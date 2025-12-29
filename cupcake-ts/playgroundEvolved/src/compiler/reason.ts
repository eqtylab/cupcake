/**
 * Compile reason expressions to Rego.
 */

import type { ReasonExpr, StringExpr } from '../expressions/index';
import type { CompilerContext } from './context';
import { compileFieldPath, getLocalVarNameFromPath } from './paths';
import { escapeRegoString } from './format';

/**
 * Compiles a reason expression to Rego.
 *
 * - String reason: returns quoted string
 * - ReasonExpr: returns concat() call
 * - null: returns default message
 */
export function compileReason(
  reason: ReasonExpr | string | null,
  ctx: CompilerContext
): string {
  // Default reason
  if (reason === null) {
    return '"Action denied"';
  }

  // Static string reason (from rule name)
  if (typeof reason === 'string') {
    return `"${escapeRegoString(reason)}"`;
  }

  // Dynamic reason template
  if (reason.__type === 'reason') {
    return compileReasonTemplate(reason, ctx);
  }

  return '"Action denied"';
}

/**
 * Compiles a reason template to a Rego concat() expression.
 */
function compileReasonTemplate(reason: ReasonExpr, ctx: CompilerContext): string {
  const parts: string[] = [];

  for (let i = 0; i < reason.strings.length; i++) {
    const str = reason.strings[i];
    if (str) {
      parts.push(`"${escapeRegoString(str)}"`);
    }

    if (i < reason.values.length) {
      const value = reason.values[i];
      if (!value || typeof value !== 'object' || !('__path' in value)) {
        throw new Error(`Invalid reason template value at index ${i}: expected field expression with __path`);
      }
      const path = (value as { __path: readonly string[] }).__path;

      // Check if this field has a local var assigned
      const localVarName = getLocalVarNameFromPath(path);
      const localVarPath = ctx.localVars.get(localVarName);

      if (localVarPath !== undefined) {
        // Use the local variable name
        parts.push(localVarName);
      } else {
        // Use the full path
        const { fullPath } = compileFieldPath(path);
        parts.push(fullPath);
      }
    }
  }

  return `concat("", [${parts.join(', ')}])`;
}

