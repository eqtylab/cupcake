/**
 * Type definitions for Cupcake OpenCode plugin
 */

import type { createOpencodeClient } from "@opencode-ai/sdk";

/**
 * OpenCode client type (from SDK)
 */
export type OpencodeClient = ReturnType<typeof createOpencodeClient>;

/**
 * Configuration for the Cupcake plugin
 */
export interface CupcakeConfig {
  /** Enable/disable the plugin */
  enabled: boolean;
  /** Path to cupcake CLI binary */
  cupcakePath: string;
  /** Harness type (always "opencode") */
  harness: string;
  /** Log level: "debug" | "info" | "warn" | "error" */
  logLevel: "debug" | "info" | "warn" | "error";
  /** Maximum policy evaluation time in milliseconds */
  timeoutMs: number;
  /** Fail mode: "open" (allow on error) or "closed" (deny on error) */
  failMode: "open" | "closed";
  /** Enable decision caching (experimental) */
  cacheDecisions: boolean;
  /** Enable toast notifications for policy decisions */
  showToasts: boolean;
  /** Toast duration in milliseconds */
  toastDurationMs: number;
}

/**
 * Cupcake event for PreToolUse
 */
export interface CupcakePreToolUseEvent {
  hook_event_name: "PreToolUse";
  session_id: string;
  cwd: string;
  agent?: string;
  message_id?: string;
  call_id?: string;
  tool: string;
  args: Record<string, any>;
}

/**
 * Cupcake event for PostToolUse
 */
export interface CupcakePostToolUseEvent {
  hook_event_name: "PostToolUse";
  session_id: string;
  cwd: string;
  agent?: string;
  message_id?: string;
  call_id?: string;
  tool: string;
  args: Record<string, any>;
  result: {
    success: boolean;
    output?: string;
    error?: string;
    exit_code?: number;
  };
}

/**
 * Cupcake event for permission requests
 */
export interface CupcakePermissionEvent {
  hook_event_name: "PermissionRequest";
  session_id: string;
  cwd: string;
  permission_id: string;
  permission_type: string;
  pattern?: string | string[];
  title: string;
  metadata: Record<string, unknown>;
  message_id?: string;
  call_id?: string;
}

/**
 * Union type for all Cupcake events
 */
export type CupcakeEvent =
  | CupcakePreToolUseEvent
  | CupcakePostToolUseEvent
  | CupcakePermissionEvent;

/**
 * Cupcake response from policy evaluation
 */
export interface CupcakeResponse {
  decision: "allow" | "deny" | "block" | "ask";
  reason?: string;
  context?: string[];
  rule_id?: string;
  severity?: "LOW" | "MEDIUM" | "HIGH" | "CRITICAL";
}

/**
 * Default configuration values
 *
 * Note: logLevel defaults to "warn" because console.error output
 * appears in OpenCode's TUI. Use "debug" or "info" for troubleshooting.
 */
export const DEFAULT_CONFIG: CupcakeConfig = {
  enabled: true,
  cupcakePath: "cupcake",
  harness: "opencode",
  logLevel: "warn", // Default to warn - info/debug are noisy in TUI
  timeoutMs: 5000,
  failMode: "closed",
  cacheDecisions: false,
  showToasts: true,
  toastDurationMs: 5000,
};

/**
 * Toast variant based on decision type
 */
export type ToastVariant = "info" | "success" | "warning" | "error";

/**
 * Get toast variant for a decision
 */
export function getToastVariant(decision: CupcakeResponse["decision"]): ToastVariant {
  switch (decision) {
    case "allow":
      return "success";
    case "ask":
      return "warning";
    case "deny":
    case "block":
      return "error";
    default:
      return "info";
  }
}
