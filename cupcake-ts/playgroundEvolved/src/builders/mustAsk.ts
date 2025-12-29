/**
 * mustAsk() builder - creates ask rules that prompt user for confirmation.
 *
 * Only valid on PreToolUse event.
 * MUST call .question() and .build() to complete the rule.
 *
 * @example
 * ```typescript
 * mustAsk('sudo commands', 'Bash')
 *   .severity('HIGH')
 *   .when(({ command }) => [
 *     command.contains('sudo')
 *   ])
 *   .reason('Sudo command detected')
 *   .question('This command requires elevated privileges. Continue?')
 *   .build()
 * ```
 */

import type { Tool, Severity, AskRule } from '../core/index';
import { DEFAULT_SEVERITY } from '../core/index';
import type { FieldsFor } from '../fields/index';
import type { BooleanExpr, ReasonExpr } from '../expressions/index';
import { createFieldsProxy } from '../expressions/index';

/**
 * Builder for ask rules before .when() is called.
 */
export interface AskRuleBuilder<T extends Tool | readonly Tool[]> {
  /** Set the severity level for this ask */
  severity(level: Severity): AskRuleBuilder<T>;
  /** Set a custom rule ID for tracking */
  ruleId(id: string): AskRuleBuilder<T>;
  /** Define the conditions that trigger this ask */
  when(fn: (fields: FieldsFor<T>) => readonly BooleanExpr[]): AskRuleBuilderWithConditions<T>;
}

/**
 * Builder for ask rules after .when() is called.
 * Can call .reason() optionally, then must call .question() and .build().
 */
export interface AskRuleBuilderWithConditions<T extends Tool | readonly Tool[]> {
  /** Define a custom reason (optional - defaults to rule name) */
  reason(fn: ((fields: FieldsFor<T>) => ReasonExpr) | string): AskRuleBuilderWithReason<T>;
  /** Define the question to ask the user (REQUIRED) */
  question(q: ((fields: FieldsFor<T>) => ReasonExpr) | string): AskRuleBuilderWithQuestion<T>;
}

/**
 * Builder for ask rules after .reason() is called.
 * Must call .question() and .build() to complete.
 */
export interface AskRuleBuilderWithReason<T extends Tool | readonly Tool[]> {
  /** Define the question to ask the user (REQUIRED) */
  question(q: ((fields: FieldsFor<T>) => ReasonExpr) | string): AskRuleBuilderWithQuestion<T>;
}

/**
 * Builder for ask rules after .question() is called.
 * Must call .build() to get the rule.
 */
export interface AskRuleBuilderWithQuestion<T extends Tool | readonly Tool[]> {
  /** Build the ask rule */
  build(): AskRule;
}

/**
 * Internal state for building ask rules.
 */
interface AskRuleState {
  name: string;
  tools: readonly Tool[];
  severity: Severity;
  ruleId: string | null;
  conditions: readonly BooleanExpr[] | null;
  reasonExpr: ReasonExpr | string | null;
  questionExpr: ReasonExpr | string | null;
}

/**
 * Creates an ask rule builder that prompts user for confirmation.
 *
 * @param name - Human-readable name for the rule (becomes default reason)
 * @param tools - Tool or tools this rule applies to
 * @returns A builder for configuring the ask rule
 *
 * @example
 * ```typescript
 * // Simple usage
 * mustAsk('sudo commands', 'Bash')
 *   .severity('HIGH')
 *   .when(({ command }) => [command.contains('sudo')])
 *   .reason('Sudo command detected')
 *   .question('This command requires elevated privileges. Continue?')
 *   .build()
 *
 * // With dynamic question
 * mustAsk('dangerous operation', 'Write')
 *   .when(({ path }) => [path.contains('/etc/')])
 *   .question(({ path }) => reason`Are you sure you want to write to ${path}?`)
 *   .build()
 * ```
 */
export function mustAsk<T extends Tool | readonly Tool[]>(
  name: string,
  tools: T
): AskRuleBuilder<T> {
  // Normalize tools to array
  const toolsArray: readonly Tool[] = Array.isArray(tools) ? tools : [tools as Tool];

  const state: AskRuleState = {
    name,
    tools: toolsArray,
    severity: DEFAULT_SEVERITY,
    ruleId: null,
    conditions: null,
    reasonExpr: null,
    questionExpr: null,
  };

  // Store fieldsProxy for reuse across builder methods
  let fieldsProxy: FieldsFor<T>;

  const createAskRule = (): AskRule => {
    if (state.questionExpr === null) {
      throw new Error('mustAsk() requires .question() to be called before .build()');
    }
    return {
      __type: 'ask',
      name: state.name,
      tools: state.tools,
      severity: state.severity,
      ruleId: state.ruleId,
      conditions: state.conditions,
      reasonExpr: state.reasonExpr ?? state.name, // Default reason is the rule name
      questionExpr: state.questionExpr,
    };
  };

  const resolveQuestionArg = (q: ((fields: FieldsFor<T>) => ReasonExpr) | string): ReasonExpr | string => {
    if (typeof q === 'string') {
      return q;
    }
    return q(fieldsProxy);
  };

  const createBuilderWithQuestion = (): AskRuleBuilderWithQuestion<T> => ({
    build(): AskRule {
      return createAskRule();
    },
  });

  const builder: AskRuleBuilder<T> = {
    severity(level: Severity): AskRuleBuilder<T> {
      state.severity = level;
      return builder;
    },

    ruleId(id: string): AskRuleBuilder<T> {
      state.ruleId = id;
      return builder;
    },

    when(fn: (fields: FieldsFor<T>) => readonly BooleanExpr[]): AskRuleBuilderWithConditions<T> {
      // Create fields proxy for the callback
      fieldsProxy = createFieldsProxy<FieldsFor<T>>(['input']);
      state.conditions = fn(fieldsProxy);

      const builderWithConditions: AskRuleBuilderWithConditions<T> = {
        reason(reasonArg: ((fields: FieldsFor<T>) => ReasonExpr) | string): AskRuleBuilderWithReason<T> {
          if (typeof reasonArg === 'string') {
            state.reasonExpr = reasonArg;
          } else {
            state.reasonExpr = reasonArg(fieldsProxy);
          }

          return {
            question(q: ((fields: FieldsFor<T>) => ReasonExpr) | string): AskRuleBuilderWithQuestion<T> {
              state.questionExpr = resolveQuestionArg(q);
              return createBuilderWithQuestion();
            },
          };
        },

        question(q: ((fields: FieldsFor<T>) => ReasonExpr) | string): AskRuleBuilderWithQuestion<T> {
          state.questionExpr = resolveQuestionArg(q);
          return createBuilderWithQuestion();
        },
      };

      return builderWithConditions;
    },
  };

  return builder;
}
