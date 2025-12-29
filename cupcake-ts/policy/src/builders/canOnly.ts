/**
 * canOnly() builder - creates allow rules with automatic default deny.
 *
 * When canOnly() is used in a policy, a default deny rule is automatically
 * generated for actions that don't match any canOnly() conditions.
 *
 * @example
 * ```typescript
 * canOnly('access blog files', ['Read', 'Write'])
 *   .when(({ path }) => [path.contains('src/blog')])
 * ```
 */

import type { Tool, AllowRule } from '../core/index';
import type { FieldsFor } from '../fields/index';
import type { BooleanExpr } from '../expressions/index';
import { createFieldsProxy } from '../expressions/index';

/**
 * Builder for allow rules.
 */
export interface AllowRuleBuilder<T extends Tool | readonly Tool[]> {
  /** Define the conditions that permit this action */
  when(fn: (fields: FieldsFor<T>) => readonly BooleanExpr[]): AllowRule;
}

/**
 * Creates an allow rule builder.
 *
 * When used in a policy, canOnly() creates a whitelist pattern:
 * - Actions matching the conditions are allowed
 * - A default deny is generated for non-matching actions
 *
 * @param name - Human-readable name for the rule
 * @param tools - Tool or tools this rule applies to
 * @returns A builder for configuring the allow rule
 *
 * @example
 * ```typescript
 * // Only allow reading from specific directories
 * canOnly('read project files', 'Read')
 *   .when(({ path }) => [
 *     path.contains('/project/'),
 *   ])
 *
 * // Only allow specific git commands
 * canOnly('safe git operations', 'Bash')
 *   .when(({ command }) => [
 *     command.in(['git status', 'git diff', 'git log']),
 *   ])
 * ```
 */
export function canOnly<T extends Tool | readonly Tool[]>(
  name: string,
  tools: T
): AllowRuleBuilder<T> {
  // Normalize tools to array
  const toolsArray: readonly Tool[] = Array.isArray(tools) ? tools : [tools as Tool];

  const builder: AllowRuleBuilder<T> = {
    when(fn: (fields: FieldsFor<T>) => readonly BooleanExpr[]): AllowRule {
      // Create fields proxy for the callback
      const fieldsProxy = createFieldsProxy<FieldsFor<T>>(['input']);
      const conditions = fn(fieldsProxy);

      return {
        __type: 'allow',
        name,
        event: 'PreToolUse',
        tools: toolsArray,
        conditions,
      };
    },
  };

  return builder;
}
