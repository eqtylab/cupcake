/**
 * Signal definition for external data enrichment.
 * Signals provide async data that can be used in policy conditions.
 */

import type { StringExpr, BooleanExpr, NumberExpr } from '../expressions/index';
import { createStringExpr, createBooleanExpr, createNumberExpr } from '../expressions/index';

/**
 * Maps TypeScript types to expression types.
 * Used to determine the expression type for signal return values.
 */
export type ExpressionFor<T> =
  T extends string ? StringExpr :
  T extends number ? NumberExpr :
  T extends boolean ? BooleanExpr :
  T extends object ? { readonly [K in keyof T]: ExpressionFor<T[K]> } :
  StringExpr; // fallback

/**
 * Signal metadata interface.
 * Signals are external data sources that run at policy evaluation time.
 */
export interface Signal<T> {
  /** Type discriminator */
  readonly __type: 'signal';
  /** Phantom type for the signal's return type */
  readonly __returnType: T;
  /** Signal name (used in input.signals.<name>) */
  readonly name: string;
  /** Resolver function that fetches the signal data */
  readonly resolve: () => Promise<T> | T;
}

/**
 * Defines a signal that provides external data to policies.
 *
 * The returned object is both a Signal (with metadata) and an expression
 * that can be used directly in conditions.
 *
 * @example
 * ```typescript
 * const gitBranch = defineSignal('gitBranch', async () => {
 *   const result = await exec('git branch --show-current');
 *   return result.stdout.trim();
 * });
 *
 * // Use in conditions:
 * cant('push to main', 'Bash').when(({ command }) => [
 *   command.contains('git push'),
 *   gitBranch.equals('main'),
 * ])
 * ```
 */
export function defineSignal<T>(
  name: string,
  resolve: () => Promise<T> | T
): ExpressionFor<T> & Signal<T> {
  const signalPath = ['input', 'signals', name];

  // Create base signal metadata
  const signalMeta: Signal<T> = {
    __type: 'signal',
    __returnType: undefined as unknown as T,
    name,
    resolve,
  };

  // Create expression based on inferred type
  // For simple types, we can return appropriate expression
  // For now, default to string expression (most common case)
  const baseExpr = createStringExpr(signalPath);

  // Merge signal metadata with expression methods
  return Object.assign({}, baseExpr, signalMeta) as unknown as ExpressionFor<T> & Signal<T>;
}

/**
 * Defines a typed signal with explicit type parameter.
 * Use this when the return type can't be inferred.
 *
 * @example
 * ```typescript
 * interface User {
 *   name: string;
 *   isAdmin: boolean;
 * }
 *
 * const currentUser = defineTypedSignal<User>('currentUser', async () => {
 *   return await fetchCurrentUser();
 * });
 *
 * // Access nested properties:
 * currentUser.name.equals('admin')
 * currentUser.isAdmin.equals(true)
 * ```
 */
export function defineTypedSignal<T extends object>(
  name: string,
  resolve: () => Promise<T> | T
): ExpressionFor<T> & Signal<T> {
  const signalPath = ['input', 'signals', name];

  const signalMeta: Signal<T> = {
    __type: 'signal',
    __returnType: undefined as unknown as T,
    name,
    resolve,
  };

  // Create a proxy for nested property access
  const proxy = new Proxy({} as ExpressionFor<T>, {
    get(_target, prop: string | symbol): unknown {
      if (typeof prop === 'symbol') {
        return undefined;
      }

      // Return signal metadata properties
      if (prop === '__type') return 'signal';
      if (prop === '__returnType') return undefined;
      if (prop === 'name') return name;
      if (prop === 'resolve') return resolve;

      // For other properties, create nested expression
      const nestedPath = [...signalPath, prop];
      return createStringExpr(nestedPath);
    },
  });

  return Object.assign(proxy, signalMeta) as unknown as ExpressionFor<T> & Signal<T>;
}
