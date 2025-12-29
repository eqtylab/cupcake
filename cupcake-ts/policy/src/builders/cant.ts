/**
 * cant() builder - creates deny rules.
 *
 * @example
 * ```typescript
 * cant('write to src', ['Write', 'Edit'])
 *   .severity('HIGH')
 *   .ruleId('SEC-001')
 *   .when(({ path }) => [path.contains('src/')])
 *   .reason(({ path }) => reason`Cannot write to ${path}`)
 *
 * // With event targeting
 * cant('dangerous permission', 'Bash')
 *   .on('PermissionRequest')
 *   .when(({ command }) => [command.contains('rm -rf')])
 * ```
 */

import type { Tool, Severity, DenyRule, DenyEvent } from '../core/index';
import { DEFAULT_SEVERITY } from '../core/index';
import type { FieldsFor } from '../fields/index';
import type { BooleanExpr, ReasonExpr } from '../expressions/index';
import { createFieldsProxy } from '../expressions/index';

/**
 * Builder for deny rules before .when() is called.
 */
export interface DenyRuleBuilder<T extends Tool | readonly Tool[]> {
  /** Set the event this rule applies to (default: PreToolUse) */
  on(event: DenyEvent): DenyRuleBuilder<T>;
  /** Set the severity level for this denial */
  severity(level: Severity): DenyRuleBuilder<T>;
  /** Set a custom rule ID for tracking */
  ruleId(id: string): DenyRuleBuilder<T>;
  /** Define the conditions that trigger this denial */
  when(fn: (fields: FieldsFor<T>) => readonly BooleanExpr[]): DenyRuleBuilderWithConditions<T>;
}

/**
 * Builder for deny rules after .when() is called.
 * At this point the rule is valid and can be used, or .reason() can be called.
 */
export interface DenyRuleBuilderWithConditions<T extends Tool | readonly Tool[]> extends DenyRule {
  /** Define a custom reason using field interpolation or a static string */
  reason(fn: ((fields: FieldsFor<T>) => ReasonExpr) | string): DenyRule;
}

/**
 * Internal state for building deny rules.
 */
interface DenyRuleState {
  name: string;
  event: DenyEvent;
  tools: readonly Tool[];
  severity: Severity;
  ruleId: string | null;
  conditions: readonly BooleanExpr[] | null;
  reasonExpr: ReasonExpr | string | null;
}

/**
 * Creates a deny rule builder.
 *
 * @param name - Human-readable name for the rule (becomes default reason)
 * @param tools - Tool or tools this rule applies to
 * @returns A builder for configuring the deny rule
 *
 * @example
 * ```typescript
 * // Single tool
 * cant('dangerous command', 'Bash')
 *   .when(({ command }) => [command.contains('rm -rf')])
 *
 * // Multiple tools
 * cant('write to protected path', ['Write', 'Edit'])
 *   .severity('CRITICAL')
 *   .when(({ path }) => [path.contains('/etc/')])
 * ```
 */
export function cant<T extends Tool | readonly Tool[]>(
  name: string,
  tools: T
): DenyRuleBuilder<T> {
  // Normalize tools to array
  const toolsArray: readonly Tool[] = Array.isArray(tools) ? tools : [tools as Tool];

  const state: DenyRuleState = {
    name,
    event: 'PreToolUse', // Default event
    tools: toolsArray,
    severity: DEFAULT_SEVERITY,
    ruleId: null,
    conditions: null,
    reasonExpr: null,
  };

  const createDenyRule = (): DenyRule => ({
    __type: 'deny',
    name: state.name,
    event: state.event,
    tools: state.tools,
    severity: state.severity,
    ruleId: state.ruleId,
    conditions: state.conditions,
    reasonExpr: state.reasonExpr ?? state.name, // Default reason is the rule name
  });

  const builder: DenyRuleBuilder<T> = {
    on(event: DenyEvent): DenyRuleBuilder<T> {
      state.event = event;
      return builder;
    },

    severity(level: Severity): DenyRuleBuilder<T> {
      state.severity = level;
      return builder;
    },

    ruleId(id: string): DenyRuleBuilder<T> {
      state.ruleId = id;
      return builder;
    },

    when(fn: (fields: FieldsFor<T>) => readonly BooleanExpr[]): DenyRuleBuilderWithConditions<T> {
      // Create fields proxy for the callback
      const fieldsProxy = createFieldsProxy<FieldsFor<T>>(['input']);
      state.conditions = fn(fieldsProxy);

      // Create the rule object that also has a .reason() method
      const rule = createDenyRule();

      const builderWithConditions: DenyRuleBuilderWithConditions<T> = {
        ...rule,
        reason(reasonArg: ((fields: FieldsFor<T>) => ReasonExpr) | string): DenyRule {
          if (typeof reasonArg === 'string') {
            state.reasonExpr = reasonArg;
          } else {
            state.reasonExpr = reasonArg(fieldsProxy);
          }
          return createDenyRule();
        },
      };

      return builderWithConditions;
    },
  };

  return builder;
}
