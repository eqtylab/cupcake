/**
 * Cupcake OpenCode Plugin
 *
 * This plugin integrates Cupcake's policy engine with OpenCode by intercepting
 * tool execution events and evaluating them against user-defined policies.
 *
 * Features:
 * - PreToolUse blocking for dangerous operations
 * - Permission auto-allow/deny via permission.ask hook
 * - Toast notifications for policy decisions
 * - Audit logging via event hook
 */

import type { Plugin, PluginInput, Hooks } from "@opencode-ai/plugin";
import type { Permission } from "@opencode-ai/sdk";
import type { CupcakeConfig, CupcakeResponse, OpencodeClient } from "./types.js";
import { DEFAULT_CONFIG, getToastVariant } from "./types.js";
import { buildPreToolUseEvent, buildPermissionEvent } from "./event-builder.js";
import { executeCupcake } from "./executor.js";
import { formatDecision, formatErrorMessage } from "./enforcer.js";
import { existsSync, readFileSync } from "fs";
import { join } from "path";

/**
 * Load configuration from .cupcake/opencode.json if it exists
 */
function loadConfig(directory: string): CupcakeConfig {
  const configPath = join(directory, ".cupcake", "opencode.json");

  if (existsSync(configPath)) {
    try {
      const configData = readFileSync(configPath, "utf-8");
      const userConfig = JSON.parse(configData);
      return { ...DEFAULT_CONFIG, ...userConfig };
    } catch (error: any) {
      console.error(`[cupcake] WARN: Failed to load config from ${configPath}: ${error.message}`);
      console.error(`[cupcake] WARN: Using default configuration`);
    }
  }

  return DEFAULT_CONFIG;
}

/**
 * Show a toast notification if enabled
 */
async function showToast(
  client: OpencodeClient | undefined,
  config: CupcakeConfig,
  title: string,
  message: string,
  variant: "info" | "success" | "warning" | "error",
): Promise<void> {
  if (!config.showToasts || !client) {
    return;
  }

  try {
    await client.tui.showToast({
      body: {
        title,
        message,
        variant,
        duration: config.toastDurationMs,
      },
    });
  } catch (error: any) {
    // Don't let toast failures break the plugin
    if (config.logLevel === "debug") {
      console.error(`[cupcake] DEBUG: Failed to show toast: ${error.message}`);
    }
  }
}

/**
 * Log a message based on log level
 */
function log(
  config: CupcakeConfig,
  level: "debug" | "info" | "warn" | "error",
  message: string,
  ...args: any[]
): void {
  const levels = ["debug", "info", "warn", "error"];
  const configLevel = levels.indexOf(config.logLevel);
  const messageLevel = levels.indexOf(level);

  if (messageLevel >= configLevel) {
    const prefix = `[cupcake] ${level.toUpperCase()}:`;
    if (args.length > 0) {
      console.error(prefix, message, ...args);
    } else {
      console.error(prefix, message);
    }
  }
}

/**
 * Cupcake OpenCode Plugin
 *
 * Intercepts tool execution and enforces policy decisions from Cupcake.
 */
export const CupcakePlugin: Plugin = async ({ directory, client }: PluginInput): Promise<Hooks> => {
  // Load configuration
  const config = loadConfig(directory);

  // Check if plugin is enabled
  if (!config.enabled) {
    log(config, "debug", "Plugin is disabled in configuration");
    return {}; // Return empty hooks - plugin is inactive
  }

  log(config, "debug", "Cupcake plugin initialized");

  return {
    /**
     * Hook: tool.execute.before
     *
     * Fired before any tool execution. This is where we enforce policies.
     * Throwing an error blocks the tool execution.
     */
    "tool.execute.before": async (
      input: { tool: string; sessionID: string; callID: string },
      output: { args: any },
    ) => {
      try {
        log(config, "debug", `tool.execute.before fired for ${input.tool}`);
        log(config, "debug", "Args:", output.args);

        // Build Cupcake event with callID
        const event = buildPreToolUseEvent(
          input.sessionID || "unknown",
          directory,
          input.tool,
          output.args,
          undefined, // agent - not provided in current hook
          undefined, // messageId - not provided in current hook
          input.callID,
        );

        // Execute cupcake to evaluate policy
        const response = await executeCupcake(config, event);
        const formatted = formatDecision(response);

        // Show toast for non-allow decisions
        if (formatted.decision !== "allow") {
          await showToast(client, config, formatted.title, formatted.message, formatted.variant);
        }

        // Block execution if needed
        if (formatted.blocked) {
          throw new Error(formatErrorMessage(formatted));
        }

        log(config, "debug", "Allowing tool execution");
      } catch (error: any) {
        throw error;
      }
    },

    /**
     * Hook: permission.ask
     *
     * Fired when OpenCode needs to request permission for an operation.
     * This integrates with OpenCode's native permission UI.
     *
     * - Set output.status = "allow" to auto-approve
     * - Set output.status = "deny" to auto-deny
     * - Leave as "ask" to show native permission dialog
     */
    "permission.ask": async (input: Permission, output: { status: "ask" | "deny" | "allow" }) => {
      try {
        log(config, "debug", `permission.ask fired for ${input.type}`);
        log(config, "debug", "Permission:", input);

        // Build permission event for Cupcake
        const event = buildPermissionEvent(
          input.sessionID,
          directory,
          input.id,
          input.type,
          input.title,
          input.metadata,
          input.pattern,
          input.messageID,
          input.callID,
        );

        // Execute cupcake to evaluate policy
        const response = await executeCupcake(config, event);

        // Map Cupcake decision to OpenCode permission status
        switch (response.decision) {
          case "allow":
            output.status = "allow";
            log(config, "debug", `Auto-allowing permission: ${input.type}`);
            break;

          case "deny":
          case "block":
            output.status = "deny";
            log(config, "debug", `Auto-denying permission: ${input.type}`);

            // Show toast for denied permissions
            await showToast(
              client,
              config,
              "Permission Denied",
              response.reason || `Permission ${input.type} blocked by policy`,
              "error",
            );
            break;

          case "ask":
          default:
            // Leave as "ask" - show native permission dialog
            output.status = "ask";
            log(config, "debug", `Deferring permission to user: ${input.type}`);

            // Optionally show a toast that approval is needed
            if (response.reason) {
              await showToast(client, config, "Approval Recommended", response.reason, "warning");
            }
            break;
        }
      } catch (error: any) {
        log(config, "error", `Permission evaluation failed: ${error.message}`);

        // On error, defer to native dialog (fail open for permissions)
        output.status = "ask";
      }
    },

    /**
     * Hook: tool.execute.after
     *
     * Fired after tool execution. Used for audit logging.
     * Cannot prevent execution (already happened).
     */
    "tool.execute.after": async (
      input: { tool: string; sessionID: string; callID: string },
      output: { title: string; output: string; metadata: any },
    ) => {
      log(config, "debug", `tool.execute.after fired for ${input.tool}`);
      log(config, "debug", "Output:", output.output?.substring(0, 200));

      // Future: Could evaluate PostToolUse policies here for validation
      // and flag suspicious outputs for review
    },

    /**
     * Hook: event
     *
     * Fired for all OpenCode events. Used for comprehensive audit logging.
     */
    event: async ({ event }) => {
      // Only log at debug level to avoid noise
      if (config.logLevel !== "debug") {
        return;
      }

      // Log events that might be relevant for audit
      const auditEvents = [
        "tool.executed",
        "permission.replied",
        "file.edited",
        "session.created",
        "session.aborted",
      ];

      if (auditEvents.includes(event.type)) {
        log(config, "debug", `Audit event: ${event.type}`, event.properties);
      }
    },
  };
};

// Export types for advanced usage
export type { CupcakeConfig, CupcakeResponse } from "./types.js";
export { formatDecision, formatErrorMessage } from "./enforcer.js";
