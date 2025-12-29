/**
 * Compile condition expressions to Rego.
 */

import type { BooleanExpr } from '../expressions/index';
import type { ComparisonMetadata } from '../expressions/base';
import type { NamedConstant } from '../constants/index';
import { isNamedConstant } from '../constants/index';
import type { CompilerContext } from './context';
import { compileFieldPath, deriveLocalVarName } from './paths';
import { formatValue, toSnakeCase } from './format';

/**
 * Result of compiling an expression.
 * May include multiple lines (e.g., for containsAny iteration).
 */
export interface CompiledExpression {
  /** Lines of Rego code */
  readonly lines: readonly string[];
  /** Any constants that need to be hoisted */
  readonly constants: Map<string, readonly unknown[]>;
  /** Local variable assignments needed */
  readonly localVars: Map<string, string>;
}

/**
 * Type guard to check if an expression has comparison metadata.
 */
function hasComparisonMetadata(expr: BooleanExpr): expr is BooleanExpr & ComparisonMetadata {
  return '__operation' in expr;
}

/**
 * Compiles a condition expression to Rego.
 */
export function compileExpression(expr: BooleanExpr, ctx: CompilerContext): CompiledExpression {
  const constants = new Map<string, readonly unknown[]>();
  const localVars = new Map<string, string>();

  if (!hasComparisonMetadata(expr)) {
    // Plain boolean field access (e.g., isSymlink without .equals())
    const { fullPath } = compileFieldPath(expr.__path);
    return { lines: [fullPath], constants, localVars };
  }

  const op = expr.__operation;
  const { basePath, fullPath, transforms } = compileFieldPath(expr.__path);

  switch (op) {
    case 'equals': {
      const value = formatValue(expr.__value);
      return { lines: [`${fullPath} == ${value}`], constants, localVars };
    }

    case 'notEquals': {
      const value = formatValue(expr.__value);
      return { lines: [`${fullPath} != ${value}`], constants, localVars };
    }

    case 'contains': {
      const value = formatValue(expr.__value);
      return { lines: [`contains(${fullPath}, ${value})`], constants, localVars };
    }

    case 'containsAny': {
      const values = expr.__args as readonly string[];

      // Check if we have a NamedConstant (stored in __value)
      let constantName: string;
      let iterVar: string;

      if (isNamedConstant(expr.__value)) {
        // Use the explicit constant name
        const namedConst = expr.__value as NamedConstant<string>;
        constantName = namedConst.name;
        iterVar = namedConst.iteratorName;
      } else {
        // Fall back to deriving from field name
        const fieldName = expr.__path[expr.__path.length - 1] ?? 'values';
        const cleanFieldName = fieldName.startsWith('__') ? expr.__path[expr.__path.length - 2] ?? 'values' : fieldName;
        constantName = toSnakeCase(cleanFieldName) + 's';
        iterVar = toSnakeCase(cleanFieldName).replace(/_path$/, '').replace(/s$/, '');
      }

      constants.set(constantName, values);

      // If we have transforms (like lower()), we need to apply them to both sides
      // And we need a local variable for the base path
      const lines: string[] = [];

      if (transforms.length > 0) {
        // Create local var for the field - derive a sensible short name
        // e.g., resolvedFilePath -> resolved_path (preserve semantic prefix, shorten the rest)
        const fieldName = expr.__path[expr.__path.length - 1] ?? 'path';
        const cleanFieldName = fieldName.startsWith('__') ? expr.__path[expr.__path.length - 2] ?? 'path' : fieldName;
        const localVarName = deriveLocalVarName(cleanFieldName);
        localVars.set(localVarName, basePath);
        lines.push(`${localVarName} := ${basePath}`);
        lines.push('');
        lines.push(`some ${iterVar} in ${constantName}`);
        // Apply transforms to both the local var and the iterator
        let leftSide = localVarName;
        let rightSide = iterVar;
        for (const t of transforms) {
          if (t === '__lower') {
            leftSide = `lower(${leftSide})`;
            rightSide = `lower(${rightSide})`;
          } else if (t === '__upper') {
            leftSide = `upper(${leftSide})`;
            rightSide = `upper(${rightSide})`;
          }
        }
        lines.push(`contains(${leftSide}, ${rightSide})`);
      } else {
        lines.push(`some ${iterVar} in ${constantName}`);
        lines.push(`contains(${fullPath}, ${iterVar})`);
      }

      return { lines, constants, localVars };
    }

    case 'in': {
      const values = expr.__args as readonly string[];
      const items = values.map((v) => `"${v}"`).join(', ');
      // Use set notation {...} for membership testing (O(1) lookup, proper Rego semantics)
      return { lines: [`${fullPath} in {${items}}`], constants, localVars };
    }

    case 'startsWith': {
      const value = formatValue(expr.__value);
      return { lines: [`startswith(${fullPath}, ${value})`], constants, localVars };
    }

    case 'endsWith': {
      const value = formatValue(expr.__value);
      return { lines: [`endswith(${fullPath}, ${value})`], constants, localVars };
    }

    case 'gt': {
      const value = formatValue(expr.__value);
      return { lines: [`${fullPath} > ${value}`], constants, localVars };
    }

    case 'gte': {
      const value = formatValue(expr.__value);
      return { lines: [`${fullPath} >= ${value}`], constants, localVars };
    }

    case 'lt': {
      const value = formatValue(expr.__value);
      return { lines: [`${fullPath} < ${value}`], constants, localVars };
    }

    case 'lte': {
      const value = formatValue(expr.__value);
      return { lines: [`${fullPath} <= ${value}`], constants, localVars };
    }

    case 'and': {
      // Not typically used in rule conditions (they're implicitly AND'd)
      return { lines: [`${fullPath}`], constants, localVars };
    }

    case 'or': {
      // Would need separate rules in Rego
      return { lines: [`${fullPath}`], constants, localVars };
    }

    case 'not': {
      return { lines: [`not ${fullPath}`], constants, localVars };
    }

    default:
      return { lines: [`${fullPath}`], constants, localVars };
  }
}

/**
 * Compiles multiple conditions, aggregating constants and local vars.
 */
export function compileConditions(
  conditions: readonly BooleanExpr[],
  ctx: CompilerContext
): CompiledExpression {
  const allLines: string[] = [];
  const allConstants = new Map<string, readonly unknown[]>();
  const allLocalVars = new Map<string, string>();

  for (const cond of conditions) {
    const result = compileExpression(cond, ctx);
    allLines.push(...result.lines);
    for (const [k, v] of result.constants) {
      allConstants.set(k, v);
    }
    for (const [k, v] of result.localVars) {
      allLocalVars.set(k, v);
    }
  }

  return {
    lines: allLines,
    constants: allConstants,
    localVars: allLocalVars,
  };
}
