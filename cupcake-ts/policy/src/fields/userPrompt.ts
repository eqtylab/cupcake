/**
 * UserPromptSubmit event field definitions.
 *
 * UserPromptSubmit fires when a user submits a prompt.
 * It does not have tool-specific fields.
 */

import type { StringExpr } from '../expressions/index';

/**
 * Fields available in UserPromptSubmit events.
 */
export interface UserPromptSubmitFields {
  /** The hook event name */
  readonly hookEventName: StringExpr;
  /** The session ID */
  readonly sessionId: StringExpr;
  /** The user's submitted prompt text (maps to input.prompt) */
  readonly submittedPrompt: StringExpr;
}
