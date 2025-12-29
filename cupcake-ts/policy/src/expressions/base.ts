/**
 * Base expression metadata for path tracking.
 * All expressions carry metadata that enables future Rego compilation.
 */

/**
 * Core metadata present on all expressions.
 * Tracks the path through the input object for Rego compilation.
 */
export interface ExpressionMetadata {
  /** Type brand to identify expressions */
  readonly __expr: true;
  /** Path through the input object, e.g., ['input', 'tool_input', 'command'] */
  readonly __path: readonly string[];
}

/**
 * Extended metadata for comparison operations.
 * Captures the operation type and value(s) for compilation.
 */
export interface ComparisonMetadata extends ExpressionMetadata {
  /** The operation being performed, e.g., 'contains', 'equals', 'lower' */
  readonly __operation: string;
  /** The comparison value (for binary operations) */
  readonly __value?: unknown | undefined;
  /** Additional arguments (e.g., array for containsAny) */
  readonly __args?: readonly unknown[] | undefined;
}
