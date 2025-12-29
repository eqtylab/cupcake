/**
 * Task tool field definitions.
 */

import type { StringExpr } from '../expressions/index';
import type { CommonFields } from './common';

/**
 * Fields available when targeting the Task tool.
 */
export interface TaskFields extends CommonFields {
  /** The task description */
  readonly description: StringExpr;
  /** The task prompt */
  readonly prompt: StringExpr;
}
