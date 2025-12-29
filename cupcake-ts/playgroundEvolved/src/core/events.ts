/**
 * Hook event types and event-verb compatibility.
 *
 * Defines which decision verbs are valid for each Claude Code hook event.
 */

/**
 * All supported Claude Code hook events.
 */
export type HookEvent =
  | 'PreToolUse'
  | 'PostToolUse'
  | 'UserPromptSubmit'
  | 'PermissionRequest'
  | 'Stop'
  | 'SubagentStop'
  | 'SessionStart';

/**
 * Events that involve tool operations.
 * These events have tool_name and tool_input available.
 */
export type ToolEvent = 'PreToolUse' | 'PostToolUse' | 'PermissionRequest';

/**
 * Events that support the 'block' decision verb.
 */
export type BlockEvent = 'PostToolUse' | 'UserPromptSubmit' | 'Stop' | 'SubagentStop';

/**
 * Events that support the 'deny' decision verb.
 */
export type DenyEvent = 'PreToolUse' | 'PermissionRequest';

/**
 * Events that support the 'ask' decision verb.
 * Note: Only PreToolUse supports ask.
 */
export type AskEvent = 'PreToolUse';

/**
 * Events that support the 'modify' decision verb.
 */
export type ModifyEvent = 'PreToolUse' | 'PermissionRequest';

/**
 * Events that support the 'halt' decision verb.
 * Halt works on all events (stops Claude entirely).
 */
export type HaltEvent = HookEvent;

/**
 * Events that support 'add_context'.
 */
export type ContextEvent = 'PostToolUse' | 'UserPromptSubmit' | 'SessionStart';

/**
 * Valid decision verbs for each event (for type-level enforcement).
 * Note: addContext is a separate builder, not included here.
 */
export type DecisionVerbsFor<E extends HookEvent> = E extends 'PreToolUse'
  ? 'halt' | 'deny' | 'ask' | 'modify'
  : E extends 'PostToolUse'
    ? 'halt' | 'block'
    : E extends 'UserPromptSubmit'
      ? 'halt' | 'block'
      : E extends 'PermissionRequest'
        ? 'halt' | 'deny' | 'modify'
        : E extends 'Stop' | 'SubagentStop'
          ? 'halt' | 'block'
          : E extends 'SessionStart'
            ? 'halt'
            : never;

/**
 * Check if an event supports block verb.
 */
export function isBlockEvent(event: HookEvent): event is BlockEvent {
  return event === 'PostToolUse' || event === 'UserPromptSubmit' || event === 'Stop' || event === 'SubagentStop';
}

/**
 * Check if an event is a tool-based event.
 */
export function isToolEvent(event: HookEvent): event is ToolEvent {
  return event === 'PreToolUse' || event === 'PostToolUse' || event === 'PermissionRequest';
}
