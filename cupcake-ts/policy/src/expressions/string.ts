/**
 * String expression type for string field operations.
 */

import type { ExpressionMetadata } from './base';
import type { BooleanExpr } from './boolean';
import type { NamedConstant } from '../constants/index';

/**
 * A string expression with comparison and transformation methods.
 * Provides type-safe string operations that compile to Rego.
 */
export interface StringExpr extends ExpressionMetadata {
  /** Check if string contains a substring */
  contains(val: string): BooleanExpr;
  /** Check if string contains any of the given substrings */
  containsAny(vals: readonly string[] | NamedConstant<string>): BooleanExpr;
  /** Check if string equals a value */
  equals(val: string): BooleanExpr;
  /** Check if string does not equal a value */
  notEquals(val: string): BooleanExpr;
  /** Check if string is in a list of values */
  in(vals: readonly string[]): BooleanExpr;
  /** Check if string starts with a prefix */
  startsWith(prefix: string): BooleanExpr;
  /** Check if string ends with a suffix */
  endsWith(suffix: string): BooleanExpr;
  /** Transform to lowercase (returns new StringExpr) */
  lower(): StringExpr;
  /** Transform to uppercase (returns new StringExpr) */
  upper(): StringExpr;
}
