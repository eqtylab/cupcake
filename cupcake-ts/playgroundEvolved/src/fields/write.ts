/**
 * Write tool field definitions.
 */

import type { StringExpr } from '../expressions/index';
import type { CommonFields } from './common';

/**
 * Fields available when targeting the Write tool.
 */
export interface WriteFields extends CommonFields {
  /** The file path being written to */
  readonly path: StringExpr;
  /** The content being written */
  readonly content: StringExpr;
}
