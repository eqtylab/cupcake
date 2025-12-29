/**
 * Expression factory - creates proxy-based expressions at runtime.
 * These proxies track property access paths for future Rego compilation.
 */

import type { StringExpr } from './string';
import type { BooleanExpr } from './boolean';
import type { NumberExpr } from './number';
import type { ComparisonMetadata } from './base';
import type { NamedConstant } from '../constants/index';
import { isNamedConstant } from '../constants/index';

/**
 * Creates a boolean expression with the given path.
 */
export function createBooleanExpr(path: readonly string[]): BooleanExpr {
  const expr: BooleanExpr = {
    __expr: true,
    __path: path,
    equals(val: boolean): BooleanExpr {
      return createComparisonExpr(path, 'equals', val);
    },
    and(other: BooleanExpr): BooleanExpr {
      return createComparisonExpr(path, 'and', undefined, [other]);
    },
    or(other: BooleanExpr): BooleanExpr {
      return createComparisonExpr(path, 'or', undefined, [other]);
    },
    not(): BooleanExpr {
      return createComparisonExpr(path, 'not');
    },
  };
  return expr;
}

/**
 * Creates a string expression with the given path.
 */
export function createStringExpr(path: readonly string[]): StringExpr {
  const expr: StringExpr = {
    __expr: true,
    __path: path,
    contains(val: string): BooleanExpr {
      return createComparisonExpr(path, 'contains', val);
    },
    containsAny(vals: readonly string[] | NamedConstant<string>): BooleanExpr {
      // If it's a NamedConstant, store the entire object for name extraction
      if (isNamedConstant(vals)) {
        return createComparisonExpr(path, 'containsAny', vals, vals.values);
      }
      return createComparisonExpr(path, 'containsAny', undefined, vals);
    },
    equals(val: string): BooleanExpr {
      return createComparisonExpr(path, 'equals', val);
    },
    notEquals(val: string): BooleanExpr {
      return createComparisonExpr(path, 'notEquals', val);
    },
    in(vals: readonly string[]): BooleanExpr {
      return createComparisonExpr(path, 'in', undefined, vals);
    },
    startsWith(prefix: string): BooleanExpr {
      return createComparisonExpr(path, 'startsWith', prefix);
    },
    endsWith(suffix: string): BooleanExpr {
      return createComparisonExpr(path, 'endsWith', suffix);
    },
    lower(): StringExpr {
      return createStringExpr([...path, '__lower']);
    },
    upper(): StringExpr {
      return createStringExpr([...path, '__upper']);
    },
  };
  return expr;
}

/**
 * Creates a number expression with the given path.
 */
export function createNumberExpr(path: readonly string[]): NumberExpr {
  const expr: NumberExpr = {
    __expr: true,
    __path: path,
    equals(val: number): BooleanExpr {
      return createComparisonExpr(path, 'equals', val);
    },
    notEquals(val: number): BooleanExpr {
      return createComparisonExpr(path, 'notEquals', val);
    },
    gt(val: number): BooleanExpr {
      return createComparisonExpr(path, 'gt', val);
    },
    gte(val: number): BooleanExpr {
      return createComparisonExpr(path, 'gte', val);
    },
    lt(val: number): BooleanExpr {
      return createComparisonExpr(path, 'lt', val);
    },
    lte(val: number): BooleanExpr {
      return createComparisonExpr(path, 'lte', val);
    },
  };
  return expr;
}

/**
 * Creates a comparison expression (result of a comparison operation).
 */
function createComparisonExpr(
  path: readonly string[],
  operation: string,
  value?: unknown,
  args?: readonly unknown[]
): BooleanExpr & ComparisonMetadata {
  const expr: BooleanExpr & ComparisonMetadata = {
    __expr: true,
    __path: path,
    __operation: operation,
    __value: value,
    __args: args,
    equals(val: boolean): BooleanExpr {
      return createComparisonExpr(path, 'equals', val);
    },
    and(other: BooleanExpr): BooleanExpr {
      return createComparisonExpr(path, 'and', undefined, [this, other]);
    },
    or(other: BooleanExpr): BooleanExpr {
      return createComparisonExpr(path, 'or', undefined, [this, other]);
    },
    not(): BooleanExpr {
      return createComparisonExpr(path, 'not', undefined, [this]);
    },
  };
  return expr;
}

/**
 * Field type hint for proxy creation.
 */
type FieldType = 'string' | 'boolean' | 'number' | 'object';

/**
 * Nested field type definitions.
 * Maps parent.child to the child's type.
 */
const NESTED_FIELD_TYPES: Record<string, Record<string, FieldType>> = {
  toolResponse: {
    stdout: 'string',
    stderr: 'string',
    exitCode: 'number',
  },
};

/**
 * Field type mapping for known fields.
 * Used by the proxy to determine which expression type to return.
 */
const FIELD_TYPES: Record<string, FieldType> = {
  // Boolean fields
  isSymlink: 'boolean',
  stopHookActive: 'boolean',
  // String fields (tool inputs)
  command: 'string',
  path: 'string',
  content: 'string',
  oldString: 'string',
  newString: 'string',
  description: 'string',
  prompt: 'string',
  // String fields (preprocessing)
  resolvedFilePath: 'string',
  originalFilePath: 'string',
  // String fields (context)
  hookEventName: 'string',
  userPrompt: 'string',
  sessionId: 'string',
  cwd: 'string',
  // String fields (UserPromptSubmit)
  submittedPrompt: 'string',
  // Object fields (contain nested fields)
  toolResponse: 'object',
};

/**
 * Creates a nested proxy for object-typed fields.
 * Used for fields like toolResponse that have nested properties.
 */
function createNestedFieldsProxy<T extends object>(
  basePath: readonly string[],
  parentField: string
): T {
  const nestedTypes = NESTED_FIELD_TYPES[parentField] ?? {};

  return new Proxy({} as T, {
    get(_target, prop: string | symbol): unknown {
      if (typeof prop === 'symbol') {
        return undefined;
      }

      const newPath = [...basePath, prop];
      const fieldType = nestedTypes[prop] ?? 'string';

      switch (fieldType) {
        case 'boolean':
          return createBooleanExpr(newPath);
        case 'number':
          return createNumberExpr(newPath);
        case 'string':
        default:
          return createStringExpr(newPath);
      }
    },
  });
}

/**
 * Creates a proxy that generates field expressions on property access.
 * Used to provide the fields object to .when() callbacks.
 */
export function createFieldsProxy<T extends object>(basePath: readonly string[]): T {
  return new Proxy({} as T, {
    get(_target, prop: string | symbol): unknown {
      if (typeof prop === 'symbol') {
        return undefined;
      }

      const newPath = [...basePath, prop];
      const fieldType = FIELD_TYPES[prop] ?? 'string';

      switch (fieldType) {
        case 'boolean':
          return createBooleanExpr(newPath);
        case 'number':
          return createNumberExpr(newPath);
        case 'object':
          // Return a nested proxy for object-typed fields
          return createNestedFieldsProxy(newPath, prop);
        case 'string':
        default:
          return createStringExpr(newPath);
      }
    },
  });
}
