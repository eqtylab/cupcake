/**
 * Common fields available to all tools.
 * These are preprocessing enrichments added by the Cupcake engine.
 */

import type { BooleanExpr, StringExpr } from '../expressions/index';

/**
 * Common fields available to all tool-based rules.
 * These fields are added by Cupcake's preprocessing pipeline.
 */
export interface CommonFields {
  /** Whether the file path is a symlink */
  readonly isSymlink: BooleanExpr;
  /** Canonicalized absolute file path (after symlink resolution) */
  readonly resolvedFilePath: StringExpr;
  /** Original file path as provided in the tool input */
  readonly originalFilePath: StringExpr;
}
