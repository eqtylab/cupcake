/**
 * Number expression type for numeric field operations.
 */

import type { ExpressionMetadata } from './base';
import type { BooleanExpr } from './boolean';

/**
 * A number expression with comparison methods.
 * Provides type-safe numeric comparisons that compile to Rego.
 */
export interface NumberExpr extends ExpressionMetadata {
  /** Check if number equals a value */
  equals(val: number): BooleanExpr;
  /** Check if number does not equal a value */
  notEquals(val: number): BooleanExpr;
  /** Check if number is greater than a value */
  gt(val: number): BooleanExpr;
  /** Check if number is greater than or equal to a value */
  gte(val: number): BooleanExpr;
  /** Check if number is less than a value */
  lt(val: number): BooleanExpr;
  /** Check if number is less than or equal to a value */
  lte(val: number): BooleanExpr;
}
