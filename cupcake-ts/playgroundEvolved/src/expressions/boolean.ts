/**
 * Boolean expression type for conditions.
 */

import type { ExpressionMetadata } from './base';

/**
 * A boolean expression that can be used in conditions.
 * Returned by comparison methods like .equals(), .contains(), etc.
 */
export interface BooleanExpr extends ExpressionMetadata {
  /** Compare to a boolean value */
  equals(val: boolean): BooleanExpr;
  /** Logical AND with another boolean expression */
  and(other: BooleanExpr): BooleanExpr;
  /** Logical OR with another boolean expression */
  or(other: BooleanExpr): BooleanExpr;
  /** Logical NOT */
  not(): BooleanExpr;
}
