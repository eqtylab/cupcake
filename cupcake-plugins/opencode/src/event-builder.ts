/**
 * Event builder - converts OpenCode events to Cupcake format
 */

import type {
  CupcakePreToolUseEvent,
  CupcakePostToolUseEvent,
  CupcakePermissionEvent,
} from "./types.js";

/**
 * Pass through tool name from OpenCode to Cupcake
 *
 * Note: The Rust preprocessing layer handles tool name normalization
 * (lowercase → PascalCase mapping like "bash" → "Bash").
 * We just pass through the tool name as-is.
 *
 * @param tool - Tool name from OpenCode (typically lowercase)
 * @returns Tool name (passed through unchanged)
 */
export function normalizeTool(tool: string): string {
  // Let Rust preprocessing handle the normalization
  return tool;
}

/**
 * Build a PreToolUse event for Cupcake
 */
export function buildPreToolUseEvent(
  sessionId: string,
  cwd: string,
  tool: string,
  args: Record<string, any>,
  agent?: string,
  messageId?: string,
  callId?: string,
): CupcakePreToolUseEvent {
  const event: CupcakePreToolUseEvent = {
    hook_event_name: "PreToolUse",
    session_id: sessionId,
    cwd,
    tool: normalizeTool(tool),
    args,
  };

  if (agent) {
    event.agent = agent;
  }

  if (messageId) {
    event.message_id = messageId;
  }

  if (callId) {
    event.call_id = callId;
  }

  return event;
}

/**
 * Build a PostToolUse event for Cupcake
 */
export function buildPostToolUseEvent(
  sessionId: string,
  cwd: string,
  tool: string,
  args: Record<string, any>,
  result: {
    success: boolean;
    output?: string;
    error?: string;
    exit_code?: number;
  },
  agent?: string,
  messageId?: string,
  callId?: string,
): CupcakePostToolUseEvent {
  const event: CupcakePostToolUseEvent = {
    hook_event_name: "PostToolUse",
    session_id: sessionId,
    cwd,
    tool: normalizeTool(tool),
    args,
    result,
  };

  if (agent) {
    event.agent = agent;
  }

  if (messageId) {
    event.message_id = messageId;
  }

  if (callId) {
    event.call_id = callId;
  }

  return event;
}

/**
 * Build a PermissionRequest event for Cupcake
 */
export function buildPermissionEvent(
  sessionId: string,
  cwd: string,
  permissionId: string,
  permissionType: string,
  title: string,
  metadata: Record<string, unknown>,
  pattern?: string | string[],
  messageId?: string,
  callId?: string,
): CupcakePermissionEvent {
  const event: CupcakePermissionEvent = {
    hook_event_name: "PermissionRequest",
    session_id: sessionId,
    cwd,
    permission_id: permissionId,
    permission_type: permissionType,
    title,
    metadata,
  };

  if (pattern) {
    event.pattern = pattern;
  }

  if (messageId) {
    event.message_id = messageId;
  }

  if (callId) {
    event.call_id = callId;
  }

  return event;
}
