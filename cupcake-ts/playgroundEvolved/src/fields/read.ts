/**
 * Read tool field definitions.
 */

import type { StringExpr } from '../expressions/index';
import type { CommonFields } from './common';

/**
 * Fields available when targeting the Read tool.
 */
export interface ReadFields extends CommonFields {
  /** The file path being read */
  readonly path: StringExpr;
}
