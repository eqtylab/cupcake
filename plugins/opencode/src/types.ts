/**
 * Type definitions for Cupcake OpenCode plugin
 */

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
  logLevel: string;
  /** Maximum policy evaluation time in milliseconds */
  timeoutMs: number;
  /** Fail mode: "open" (allow on error) or "closed" (deny on error) */
  failMode: "open" | "closed";
  /** Enable decision caching (experimental) */
  cacheDecisions: boolean;
}

/**
 * Tool name mapping from OpenCode to Cupcake format
 */
export const TOOL_NAME_MAP: Record<string, string> = {
  bash: "Bash",
  edit: "Edit",
  write: "Write",
  read: "Read",
  grep: "Grep",
  glob: "Glob",
  list: "List",
  patch: "Patch",
  todowrite: "TodoWrite",
  todoread: "TodoRead",
  webfetch: "WebFetch",
};

/**
 * Cupcake event for PreToolUse
 */
export interface CupcakePreToolUseEvent {
  hook_event_name: "PreToolUse";
  session_id: string;
  cwd: string;
  agent?: string;
  message_id?: string;
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
 * Union type for all Cupcake events
 */
export type CupcakeEvent = CupcakePreToolUseEvent | CupcakePostToolUseEvent;

/**
 * Cupcake response from policy evaluation
 */
export interface CupcakeResponse {
  decision: "allow" | "deny" | "block" | "ask";
  reason?: string;
  context?: string[];
}

/**
 * Default configuration values
 */
export const DEFAULT_CONFIG: CupcakeConfig = {
  enabled: true,
  cupcakePath: "cupcake",
  harness: "opencode",
  logLevel: "info",
  timeoutMs: 5000,
  failMode: "closed",
  cacheDecisions: false,
};
