/**
 * PostToolUse event field definitions.
 *
 * PostToolUse fires after a tool completes execution.
 * It includes all the tool input fields plus the tool's response.
 */

import type { NumberExpr, StringExpr } from '../expressions/index';

/**
 * Tool response fields available in PostToolUse events.
 * Contains the output from the executed tool.
 */
export interface ToolResponseFields {
  /** Standard output from the tool */
  readonly stdout: StringExpr;
  /** Standard error from the tool */
  readonly stderr: StringExpr;
  /** Exit code from the tool (0 = success) */
  readonly exitCode: NumberExpr;
}

/**
 * Additional fields available in PostToolUse events.
 * These are merged with the tool-specific fields.
 */
export interface PostToolUseFields {
  /** The tool's execution response */
  readonly toolResponse: ToolResponseFields;
}
