/**
 * Event builder - converts OpenCode events to Cupcake format
 */

import type { CupcakeEvent, CupcakePreToolUseEvent, CupcakePostToolUseEvent } from "./types.js";
import { TOOL_NAME_MAP } from "./types.js";

/**
 * Normalize tool name from OpenCode format to Cupcake format
 * @param tool - Tool name from OpenCode (lowercase)
 * @returns Normalized tool name for Cupcake (PascalCase)
 */
export function normalizeTool(tool: string): string {
  const normalized = TOOL_NAME_MAP[tool.toLowerCase()];
  if (normalized) {
    return normalized;
  }
  // Custom tools - capitalize first letter
  return tool.charAt(0).toUpperCase() + tool.slice(1);
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
  messageId?: string
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
  messageId?: string
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

  return event;
}
