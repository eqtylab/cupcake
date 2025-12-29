/**
 * Bash tool field definitions.
 */

import type { StringExpr } from '../expressions/index';
import type { CommonFields } from './common';

/**
 * Fields available when targeting the Bash tool.
 */
export interface BashFields extends CommonFields {
  /** The shell command being executed */
  readonly command: StringExpr;
  /** The working directory path */
  readonly path: StringExpr;
}
