/**
 * Expression system module - type-safe condition building.
 */

export { type ExpressionMetadata, type ComparisonMetadata } from './base';
export { type BooleanExpr } from './boolean';
export { type StringExpr } from './string';
export { type NumberExpr } from './number';
export { type ReasonExpr, reason } from './reason';
export {
  createBooleanExpr,
  createStringExpr,
  createNumberExpr,
  createFieldsProxy,
} from './factory';
