/**
 * Cupcake OpenCode Plugin
 *
 * This plugin integrates Cupcake's policy engine with OpenCode by intercepting
 * tool execution events and evaluating them against user-defined policies.
 */

import type { Plugin, PluginInput } from "@opencode-ai/plugin";
import type { CupcakeConfig } from "./types.js";
import { DEFAULT_CONFIG } from "./types.js";
import { buildPreToolUseEvent } from "./event-builder.js";
import { executeCupcake } from "./executor.js";
import { enforceDecision } from "./enforcer.js";
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
      console.error(
        `[cupcake-plugin] WARN: Failed to load config from ${configPath}: ${error.message}`,
      );
      console.error(`[cupcake-plugin] WARN: Using default configuration`);
    }
  }

  return DEFAULT_CONFIG;
}

/**
 * Cupcake OpenCode Plugin
 * 
 * Intercepts tool execution and enforces policy decisions from Cupcake.
 */
export const CupcakePlugin: Plugin = async ({ directory }: PluginInput) => {
  // Load configuration
  const config = loadConfig(directory);

  // Check if plugin is enabled
  if (!config.enabled) {
    if (config.logLevel === "info" || config.logLevel === "debug") {
      console.error("[cupcake-plugin] INFO: Plugin is disabled in configuration");
    }
    return {}; // Return empty hooks - plugin is inactive
  }

  if (config.logLevel === "info" || config.logLevel === "debug") {
    console.error("[cupcake-plugin] INFO: Cupcake plugin initialized");
  }

  return {
    /**
     * Hook: tool.execute.before
     * 
     * Fired before any tool execution. This is where we enforce policies.
     */
    "tool.execute.before": async (input: { tool: string; sessionID: string; callID: string }, output: { args: any }) => {
      try {
        if (config.logLevel === "debug") {
          console.error(`[cupcake-plugin] DEBUG: tool.execute.before fired`);
          console.error(`[cupcake-plugin] DEBUG: Tool: ${input.tool}`);
          console.error(`[cupcake-plugin] DEBUG: Args:`, output.args);
        }

        // Build Cupcake event
        const event = buildPreToolUseEvent(
          input.sessionID || "unknown",
          directory,
          input.tool,
          output.args,
        );

        // Execute cupcake to evaluate policy
        const response = await executeCupcake(config, event);

        // Enforce the decision (throws on deny/block/ask)
        enforceDecision(response);

        // If we get here, decision was "allow" - return normally
        if (config.logLevel === "debug") {
          console.error(`[cupcake-plugin] DEBUG: Allowing tool execution`);
        }
      } catch (error: any) {
        // Re-throw to block tool execution
        if (config.logLevel === "error" || config.logLevel === "debug") {
          console.error(`[cupcake-plugin] ERROR: ${error.message}`);
        }
        throw error;
      }
    },

    /**
     * Hook: tool.execute.after
     * 
     * Fired after tool execution. Can be used for validation but cannot prevent execution.
     * 
     * Note: In Phase 1, we primarily use this for logging. Phase 2 may add validation.
     */
    "tool.execute.after": async (input: { tool: string; sessionID: string; callID: string }, output: { title: string; output: string; metadata: any }) => {
      if (config.logLevel === "debug") {
        console.error(`[cupcake-plugin] DEBUG: tool.execute.after fired`);
        console.error(`[cupcake-plugin] DEBUG: Tool: ${input.tool}`);
        console.error(`[cupcake-plugin] DEBUG: Output:`, output.output);
      }

      // Phase 1: Just log
      // Phase 2: Could evaluate PostToolUse policies here
    },
  };
};

// Export types and utilities
export * from "./types.js";
export { buildPreToolUseEvent } from "./event-builder.js";
export { executeCupcake } from "./executor.js";
export { enforceDecision, formatErrorMessage } from "./enforcer.js";
