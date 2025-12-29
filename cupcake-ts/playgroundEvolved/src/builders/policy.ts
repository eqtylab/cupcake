/**
 * policy() builder - groups rules into a named policy.
 *
 * @example
 * ```typescript
 * const myPolicy = policy('technical writer',
 *   cant('write to src', ['Write', 'Edit']).when(...),
 *   canOnly('read blog files', 'Read').when(...),
 *   addContext('Follow the style guide...'),
 * );
 * ```
 */

import type { Rule, AllowRule } from '../core/index';
import { isAllowRule } from '../core/index';

/**
 * A complete policy containing multiple rules.
 */
export interface Policy {
  /** Human-readable policy name */
  readonly name: string;
  /** All rules in this policy */
  readonly rules: readonly Rule[];
  /** Whether this policy uses canOnly() (affects compilation) */
  readonly hasAllowRules: boolean;
}

/**
 * Creates a policy from a collection of rules.
 *
 * The policy name is used to generate the Rego package name:
 * - "technical writer" → cupcake.policies.technical_writer
 *
 * @param name - Human-readable policy name
 * @param rules - Rules to include in the policy
 * @returns A complete policy object
 *
 * @example
 * ```typescript
 * const securityPolicy = policy('security restrictions',
 *   cant('dangerous commands', 'Bash')
 *     .severity('CRITICAL')
 *     .when(({ command }) => [
 *       command.contains('rm -rf /'),
 *     ]),
 *
 *   cant('write to system paths', ['Write', 'Edit'])
 *     .when(({ path }) => [
 *       path.startsWith('/etc/'),
 *     ]),
 *
 *   addContext('Always verify paths before modifying system files.'),
 * );
 * ```
 */
export function policy(name: string, ...rules: Rule[]): Policy {
  const hasAllowRules = rules.some(isAllowRule);

  return {
    name,
    rules,
    hasAllowRules,
  };
}

/**
 * Converts a policy name to a valid Rego package identifier.
 * - Converts to lowercase
 * - Replaces spaces and hyphens with underscores
 * - Removes invalid characters
 *
 * @param name - Human-readable policy name
 * @returns Valid Rego package identifier
 */
export function toPackageName(name: string): string {
  return name
    .toLowerCase()
    .replace(/[\s-]+/g, '_')
    .replace(/[^a-z0-9_]/g, '');
}

/**
 * Compiles a policy to Rego source code.
 *
 * @param policy - The policy to compile
 * @returns Valid Rego v1 source code
 */
export { compile, toPackageName as toPackageNameCompiler } from '../compiler/index';

// Re-export toPackageName from local implementation for backwards compat
// (The compiler module has its own copy)
