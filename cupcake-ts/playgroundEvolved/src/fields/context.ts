/**
 * Context fields for addContext().when() rules.
 * These are available when conditioning context injection.
 */

import type { StringExpr } from '../expressions/index';

/**
 * Fields available when using addContext().when().
 * These relate to the event context rather than tool inputs.
 */
export interface ContextFields {
  /** The hook event name (e.g., 'UserPromptSubmit', 'PreToolUse') */
  readonly hookEventName: StringExpr;
  /** The user's prompt text (for UserPromptSubmit events) */
  readonly userPrompt: StringExpr;
  /** The session ID */
  readonly sessionId: StringExpr;
  /** Current working directory */
  readonly cwd: StringExpr;
}
