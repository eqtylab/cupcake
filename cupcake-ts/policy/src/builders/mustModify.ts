/**
 * mustModify() builder - creates modify rules that transform tool input.
 *
 * MUST call .transform() and .build() to complete the rule.
 *
 * @example
 * ```typescript
 * mustModify('sanitize paths', 'Write')
 *   .priority(80)
 *   .when(({ path }) => [
 *     path.contains('../')
 *   ])
 *   .reason('Path traversal detected, using resolved path')
 *   .transform(({ resolvedFilePath }) => ({ path: resolvedFilePath }))
 *   .build()
 *
 * // With event targeting
 * mustModify('modify at permission', 'Bash')
 *   .on('PermissionRequest')
 *   .when(({ command }) => [command.contains('npm')])
 *   .transform(() => ({ timeout: 60000 }))
 *   .build()
 * ```
 */

import type { Tool, Severity, ModifyRule, ModifyEvent, TransformResult } from '../core/index';
import type { FieldsFor } from '../fields/index';
import type { BooleanExpr, ReasonExpr } from '../expressions/index';
import { createFieldsProxy } from '../expressions/index';

/**
 * Builder for modify rules before .when() is called.
 */
export interface ModifyRuleBuilder<T extends Tool | readonly Tool[]> {
  /** Set the event this rule applies to (default: PreToolUse) */
  on(event: ModifyEvent): ModifyRuleBuilder<T>;
  /** Set the priority for transform ordering (1-100, higher wins). Default 50. */
  priority(n: number): ModifyRuleBuilder<T>;
  /** Set the severity level for this modification. Default MEDIUM. */
  severity(level: Severity): ModifyRuleBuilder<T>;
  /** Set a custom rule ID for tracking */
  ruleId(id: string): ModifyRuleBuilder<T>;
  /** Define the conditions that trigger this modification */
  when(fn: (fields: FieldsFor<T>) => readonly BooleanExpr[]): ModifyRuleBuilderWithConditions<T>;
}

/**
 * Builder for modify rules after .when() is called.
 * Can call .reason() optionally, then must call .transform() and .build().
 */
export interface ModifyRuleBuilderWithConditions<T extends Tool | readonly Tool[]> {
  /** Define a custom reason (optional - defaults to rule name) */
  reason(fn: ((fields: FieldsFor<T>) => ReasonExpr) | string): ModifyRuleBuilderWithReason<T>;
  /** Define the transform function that returns the updated input (REQUIRED) */
  transform(fn: (fields: FieldsFor<T>) => TransformResult): ModifyRuleBuilderWithTransform<T>;
}

/**
 * Builder for modify rules after .reason() is called.
 * Must call .transform() and .build() to complete.
 */
export interface ModifyRuleBuilderWithReason<T extends Tool | readonly Tool[]> {
  /** Define the transform function that returns the updated input (REQUIRED) */
  transform(fn: (fields: FieldsFor<T>) => TransformResult): ModifyRuleBuilderWithTransform<T>;
}

/**
 * Builder for modify rules after .transform() is called.
 * Must call .build() to get the rule.
 */
export interface ModifyRuleBuilderWithTransform<T extends Tool | readonly Tool[]> {
  /** Build the modify rule */
  build(): ModifyRule;
}

/**
 * Internal state for building modify rules.
 */
interface ModifyRuleState {
  name: string;
  event: ModifyEvent;
  tools: readonly Tool[];
  priority: number | null;
  severity: Severity | null;
  ruleId: string | null;
  conditions: readonly BooleanExpr[] | null;
  reasonExpr: ReasonExpr | string | null;
  transformResult: TransformResult | null;
}

/**
 * Creates a modify rule builder that transforms tool input.
 *
 * @param name - Human-readable name for the rule (becomes default reason)
 * @param tools - Tool or tools this rule applies to
 * @returns A builder for configuring the modify rule
 *
 * @example
 * ```typescript
 * // Sanitize path traversal
 * mustModify('sanitize paths', 'Write')
 *   .priority(80)
 *   .when(({ path }) => [path.contains('../')])
 *   .reason('Path traversal detected, using resolved path')
 *   .transform(({ resolvedFilePath }) => ({ path: resolvedFilePath }))
 *   .build()
 *
 * // Force safe mode
 * mustModify('safe mode', 'Bash')
 *   .when(({ command }) => [command.contains('rm')])
 *   .transform(({ command }) => ({ command: command }))
 *   .build()
 * ```
 */
export function mustModify<T extends Tool | readonly Tool[]>(
  name: string,
  tools: T
): ModifyRuleBuilder<T> {
  // Normalize tools to array
  const toolsArray: readonly Tool[] = Array.isArray(tools) ? tools : [tools as Tool];

  const state: ModifyRuleState = {
    name,
    event: 'PreToolUse', // Default event
    tools: toolsArray,
    priority: null,
    severity: null,
    ruleId: null,
    conditions: null,
    reasonExpr: null,
    transformResult: null,
  };

  // Store fieldsProxy for reuse across builder methods
  let fieldsProxy: FieldsFor<T>;

  const createModifyRule = (): ModifyRule => {
    if (state.transformResult === null) {
      throw new Error('mustModify() requires .transform() to be called before .build()');
    }
    return {
      __type: 'modify',
      name: state.name,
      event: state.event,
      tools: state.tools,
      priority: state.priority,
      severity: state.severity,
      ruleId: state.ruleId,
      conditions: state.conditions,
      reasonExpr: state.reasonExpr ?? state.name, // Default reason is the rule name
      transformResult: state.transformResult,
    };
  };

  const createBuilderWithTransform = (): ModifyRuleBuilderWithTransform<T> => ({
    build(): ModifyRule {
      return createModifyRule();
    },
  });

  const builder: ModifyRuleBuilder<T> = {
    on(event: ModifyEvent): ModifyRuleBuilder<T> {
      state.event = event;
      return builder;
    },

    priority(n: number): ModifyRuleBuilder<T> {
      state.priority = n;
      return builder;
    },

    severity(level: Severity): ModifyRuleBuilder<T> {
      state.severity = level;
      return builder;
    },

    ruleId(id: string): ModifyRuleBuilder<T> {
      state.ruleId = id;
      return builder;
    },

    when(fn: (fields: FieldsFor<T>) => readonly BooleanExpr[]): ModifyRuleBuilderWithConditions<T> {
      // Create fields proxy for the callback
      fieldsProxy = createFieldsProxy<FieldsFor<T>>(['input']);
      state.conditions = fn(fieldsProxy);

      const builderWithConditions: ModifyRuleBuilderWithConditions<T> = {
        reason(reasonArg: ((fields: FieldsFor<T>) => ReasonExpr) | string): ModifyRuleBuilderWithReason<T> {
          if (typeof reasonArg === 'string') {
            state.reasonExpr = reasonArg;
          } else {
            state.reasonExpr = reasonArg(fieldsProxy);
          }

          return {
            transform(transformFn: (fields: FieldsFor<T>) => TransformResult): ModifyRuleBuilderWithTransform<T> {
              state.transformResult = transformFn(fieldsProxy);
              return createBuilderWithTransform();
            },
          };
        },

        transform(transformFn: (fields: FieldsFor<T>) => TransformResult): ModifyRuleBuilderWithTransform<T> {
          state.transformResult = transformFn(fieldsProxy);
          return createBuilderWithTransform();
        },
      };

      return builderWithConditions;
    },
  };

  return builder;
}
