/**
 * Reason expression for dynamic rule reasons.
 * Uses template literals to interpolate field values into reason strings.
 */

import type { StringExpr } from './string';

/**
 * A reason expression built from a template literal.
 * Captures the static strings and dynamic field references for compilation.
 *
 * @example
 * ```typescript
 * .reason(({ originalFilePath, resolvedFilePath }) =>
 *   reason`Symlink detected: '${originalFilePath}' → '${resolvedFilePath}'`
 * )
 * ```
 */
export interface ReasonExpr {
  /** Type discriminator */
  readonly __type: 'reason';
  /** Static string parts from the template literal */
  readonly strings: TemplateStringsArray;
  /** Dynamic field references (StringExpr) interpolated into the template */
  readonly values: readonly StringExpr[];
}

/**
 * Template tag for creating dynamic reason expressions.
 *
 * @example
 * ```typescript
 * reason`File '${path}' is in a protected directory`
 * ```
 */
export function reason(
  strings: TemplateStringsArray,
  ...values: StringExpr[]
): ReasonExpr {
  return {
    __type: 'reason',
    strings,
    values,
  };
}
