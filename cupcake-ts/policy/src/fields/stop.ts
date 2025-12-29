/**
 * Stop/SubagentStop event field definitions.
 *
 * Stop fires when the main agent finishes.
 * SubagentStop fires when a subagent (Task) finishes.
 */

import type { BooleanExpr, StringExpr } from '../expressions/index';

/**
 * Fields available in Stop and SubagentStop events.
 */
export interface StopFields {
  /** The hook event name */
  readonly hookEventName: StringExpr;
  /** The session ID */
  readonly sessionId: StringExpr;
  /** True if already continuing from a stop hook */
  readonly stopHookActive: BooleanExpr;
}
