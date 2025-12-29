/**
 * SessionStart event fields.
 *
 * SessionStart fires when a new Claude Code session begins.
 * This is a non-tool event with minimal fields.
 */

import type { StringExpr } from '../expressions/index';

/**
 * Fields available during SessionStart event.
 */
export interface SessionStartFields {
  /** The hook event name (always "SessionStart") */
  readonly hookEventName: StringExpr;
  /** The session ID */
  readonly sessionId: StringExpr;
}
