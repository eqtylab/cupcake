/**
 * Edit tool field definitions.
 */

import type { StringExpr } from '../expressions/index';
import type { CommonFields } from './common';

/**
 * Fields available when targeting the Edit tool.
 */
export interface EditFields extends CommonFields {
  /** The file path being edited */
  readonly path: StringExpr;
  /** The text being replaced */
  readonly oldString: StringExpr;
  /** The replacement text */
  readonly newString: StringExpr;
}
