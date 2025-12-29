/**
 * mustHalt() builder - creates halt rules that immediately stop Claude.
 *
 * @example
 * ```typescript
 * // Tool-based halt (for PreToolUse, PostToolUse, PermissionRequest)
 * mustHalt('critical system path', ['Write', 'Edit'])
 *   .severity('CRITICAL')
 *   .when(({ resolvedFilePath }) => [
 *     resolvedFilePath.startsWith('/etc/')
 *   ])
 *   .reason('Attempting to modify critical system files')
 *   .build()
 *
 * // Tool-less halt (for UserPromptSubmit, Stop, SessionStart)
 * mustHalt('dangerous prompt')
 *   .on('UserPromptSubmit')
 *   .when(({ submittedPrompt }) => [submittedPrompt.contains('DANGER')])
 *   .build()
 * ```
 */

import type { Tool, Severity, HaltRule, HaltEvent } from '../core/index';
import { DEFAULT_SEVERITY } from '../core/index';
import type {
  FieldsFor,
  PostToolUseFields,
  UserPromptSubmitFields,
  StopFields,
  SessionStartFields,
} from '../fields/index';
import type { BooleanExpr, ReasonExpr } from '../expressions/index';
import { createFieldsProxy } from '../expressions/index';

// Type for combined PostToolUse fields (tool fields + toolResponse)
type PostToolUseFieldsFor<T extends Tool | readonly Tool[]> = FieldsFor<T> & PostToolUseFields;

// Non-tool events for halt
type HaltNonToolEvent = 'UserPromptSubmit' | 'Stop' | 'SubagentStop' | 'SessionStart';

/**
 * Builder for tool-based halt rules before .when() is called.
 */
export interface HaltRuleBuilder<T extends Tool | readonly Tool[]> {
  /** Set the event this rule applies to (default: PreToolUse) */
  on(event: 'PreToolUse' | 'PermissionRequest'): HaltRuleBuilder<T>;
  /** Set the event to PostToolUse (provides toolResponse fields) */
  on(event: 'PostToolUse'): HaltRuleBuilderOnPostToolUse<T>;
  /** Set the severity level for this halt */
  severity(level: Severity): HaltRuleBuilder<T>;
  /** Set a custom rule ID for tracking */
  ruleId(id: string): HaltRuleBuilder<T>;
  /** Define the conditions that trigger this halt */
  when(fn: (fields: FieldsFor<T>) => readonly BooleanExpr[]): HaltRuleBuilderWithConditions<T>;
}

/**
 * Builder for tool-less halt rules before .on() is called.
 * MUST call .on() to specify the event.
 */
export interface HaltRuleBuilderWithoutTools {
  /** Set the event this rule applies to (REQUIRED for tool-less halt) */
  on(event: 'UserPromptSubmit'): HaltRuleBuilderOnUserPrompt;
  on(event: 'Stop' | 'SubagentStop'): HaltRuleBuilderOnStop;
  on(event: 'SessionStart'): HaltRuleBuilderOnSessionStart;
}

/**
 * Builder for halt rules targeting PostToolUse event.
 * Provides access to toolResponse fields.
 */
export interface HaltRuleBuilderOnPostToolUse<T extends Tool | readonly Tool[]> {
  /** Set the severity level for this halt */
  severity(level: Severity): HaltRuleBuilderOnPostToolUse<T>;
  /** Set a custom rule ID for tracking */
  ruleId(id: string): HaltRuleBuilderOnPostToolUse<T>;
  /** Define the conditions that trigger this halt (includes toolResponse) */
  when(fn: (fields: PostToolUseFieldsFor<T>) => readonly BooleanExpr[]): HaltRuleBuilderWithConditions<T>;
}

/**
 * Builder for halt rules targeting UserPromptSubmit event.
 */
export interface HaltRuleBuilderOnUserPrompt {
  /** Set the severity level for this halt */
  severity(level: Severity): HaltRuleBuilderOnUserPrompt;
  /** Set a custom rule ID for tracking */
  ruleId(id: string): HaltRuleBuilderOnUserPrompt;
  /** Define the conditions that trigger this halt */
  when(fn: (fields: UserPromptSubmitFields) => readonly BooleanExpr[]): HaltRuleBuilderWithConditionsNoTools;
}

/**
 * Builder for halt rules targeting Stop/SubagentStop event.
 */
export interface HaltRuleBuilderOnStop {
  /** Set the severity level for this halt */
  severity(level: Severity): HaltRuleBuilderOnStop;
  /** Set a custom rule ID for tracking */
  ruleId(id: string): HaltRuleBuilderOnStop;
  /** Define the conditions that trigger this halt */
  when(fn: (fields: StopFields) => readonly BooleanExpr[]): HaltRuleBuilderWithConditionsNoTools;
}

/**
 * Builder for halt rules targeting SessionStart event.
 */
export interface HaltRuleBuilderOnSessionStart {
  /** Set the severity level for this halt */
  severity(level: Severity): HaltRuleBuilderOnSessionStart;
  /** Set a custom rule ID for tracking */
  ruleId(id: string): HaltRuleBuilderOnSessionStart;
  /** Define the conditions that trigger this halt */
  when(fn: (fields: SessionStartFields) => readonly BooleanExpr[]): HaltRuleBuilderWithConditionsNoTools;
}

/**
 * Builder for halt rules after .when() is called (tool-based).
 * Can call .reason() optionally, then must call .build() to get the rule.
 */
export interface HaltRuleBuilderWithConditions<T extends Tool | readonly Tool[]> {
  /** Define a custom reason (optional - defaults to rule name) */
  reason(fn: ((fields: FieldsFor<T>) => ReasonExpr) | string): HaltRuleBuilderWithReason<T>;
  /** Build the halt rule */
  build(): HaltRule;
}

/**
 * Builder for halt rules after .when() is called (tool-less).
 */
export interface HaltRuleBuilderWithConditionsNoTools {
  /** Define a custom reason (optional - defaults to rule name) */
  reason(fn: string): HaltRuleBuilderWithReason<never>;
  /** Build the halt rule */
  build(): HaltRule;
}

/**
 * Builder for halt rules after .reason() is called.
 * Must call .build() to get the rule.
 */
export interface HaltRuleBuilderWithReason<T extends Tool | readonly Tool[]> {
  /** Build the halt rule */
  build(): HaltRule;
}

/**
 * Internal state for building halt rules.
 */
interface HaltRuleState {
  name: string;
  event: HaltEvent;
  tools: readonly Tool[] | null;
  severity: Severity;
  ruleId: string | null;
  conditions: readonly BooleanExpr[] | null;
  reasonExpr: ReasonExpr | string | null;
}

/**
 * Creates a tool-based halt rule builder.
 *
 * @param name - Human-readable name for the rule (becomes default reason)
 * @param tools - Tool or tools this rule applies to
 * @returns A builder for configuring the halt rule
 */
export function mustHalt<T extends Tool | readonly Tool[]>(
  name: string,
  tools: T
): HaltRuleBuilder<T>;

/**
 * Creates a tool-less halt rule builder.
 * MUST call .on() to specify the event (UserPromptSubmit, Stop, SubagentStop, or SessionStart).
 *
 * @param name - Human-readable name for the rule (becomes default reason)
 * @returns A builder for configuring the halt rule
 */
export function mustHalt(name: string): HaltRuleBuilderWithoutTools;

/**
 * Implementation of mustHalt with overloaded signatures.
 */
export function mustHalt<T extends Tool | readonly Tool[]>(
  name: string,
  tools?: T
): HaltRuleBuilder<T> | HaltRuleBuilderWithoutTools {
  const toolsArray: readonly Tool[] | null = tools
    ? Array.isArray(tools)
      ? tools
      : [tools as Tool]
    : null;

  const state: HaltRuleState = {
    name,
    event: 'PreToolUse', // Default event for tool-based
    tools: toolsArray,
    severity: DEFAULT_SEVERITY,
    ruleId: null,
    conditions: null,
    reasonExpr: null,
  };

  const createHaltRule = (): HaltRule => ({
    __type: 'halt',
    name: state.name,
    event: state.event,
    tools: state.tools,
    severity: state.severity,
    ruleId: state.ruleId,
    conditions: state.conditions,
    reasonExpr: state.reasonExpr ?? state.name,
  });

  if (toolsArray) {
    // Tool-based builder
    const createPostToolUseBuilder = (): HaltRuleBuilderOnPostToolUse<T> => ({
      severity(level: Severity): HaltRuleBuilderOnPostToolUse<T> {
        state.severity = level;
        return this;
      },

      ruleId(id: string): HaltRuleBuilderOnPostToolUse<T> {
        state.ruleId = id;
        return this;
      },

      when(fn: (fields: PostToolUseFieldsFor<T>) => readonly BooleanExpr[]): HaltRuleBuilderWithConditions<T> {
        const fieldsProxy = createFieldsProxy<PostToolUseFieldsFor<T>>(['input']);
        state.conditions = fn(fieldsProxy);

        return {
          reason(reasonArg: ((fields: PostToolUseFieldsFor<T>) => ReasonExpr) | string): HaltRuleBuilderWithReason<T> {
            if (typeof reasonArg === 'string') {
              state.reasonExpr = reasonArg;
            } else {
              // Reuse fieldsProxy to maintain access to toolResponse fields
              state.reasonExpr = reasonArg(fieldsProxy);
            }
            return { build: createHaltRule };
          },
          build: createHaltRule,
        };
      },
    });

    const builder = {
      on(event: HaltEvent) {
        state.event = event;
        if (event === 'PostToolUse') {
          return createPostToolUseBuilder();
        }
        return builder;
      },

      severity(level: Severity): HaltRuleBuilder<T> {
        state.severity = level;
        return builder;
      },

      ruleId(id: string): HaltRuleBuilder<T> {
        state.ruleId = id;
        return builder;
      },

      when(fn: (fields: FieldsFor<T>) => readonly BooleanExpr[]): HaltRuleBuilderWithConditions<T> {
        const fieldsProxy = createFieldsProxy<FieldsFor<T>>(['input']);
        state.conditions = fn(fieldsProxy);

        const builderWithConditions: HaltRuleBuilderWithConditions<T> = {
          reason(reasonArg: ((fields: FieldsFor<T>) => ReasonExpr) | string): HaltRuleBuilderWithReason<T> {
            if (typeof reasonArg === 'string') {
              state.reasonExpr = reasonArg;
            } else {
              state.reasonExpr = reasonArg(fieldsProxy);
            }

            return {
              build(): HaltRule {
                return createHaltRule();
              },
            };
          },

          build(): HaltRule {
            return createHaltRule();
          },
        };

        return builderWithConditions;
      },
    } as HaltRuleBuilder<T>;

    return builder;
  } else {
    // Tool-less builder (for UserPromptSubmit/Stop/SubagentStop/SessionStart)
    const builder = {
      on(event: HaltNonToolEvent) {
        state.event = event;

        if (event === 'UserPromptSubmit') {
          const onBuilder: HaltRuleBuilderOnUserPrompt = {
            severity(level: Severity): HaltRuleBuilderOnUserPrompt {
              state.severity = level;
              return onBuilder;
            },

            ruleId(id: string): HaltRuleBuilderOnUserPrompt {
              state.ruleId = id;
              return onBuilder;
            },

            when(fn: (fields: UserPromptSubmitFields) => readonly BooleanExpr[]): HaltRuleBuilderWithConditionsNoTools {
              const fieldsProxy = createFieldsProxy<UserPromptSubmitFields>(['input']);
              state.conditions = fn(fieldsProxy);

              return {
                reason(reasonArg: string): HaltRuleBuilderWithReason<never> {
                  state.reasonExpr = reasonArg;
                  return { build: createHaltRule };
                },
                build: createHaltRule,
              };
            },
          };
          return onBuilder;
        } else if (event === 'Stop' || event === 'SubagentStop') {
          const onBuilder: HaltRuleBuilderOnStop = {
            severity(level: Severity): HaltRuleBuilderOnStop {
              state.severity = level;
              return onBuilder;
            },

            ruleId(id: string): HaltRuleBuilderOnStop {
              state.ruleId = id;
              return onBuilder;
            },

            when(fn: (fields: StopFields) => readonly BooleanExpr[]): HaltRuleBuilderWithConditionsNoTools {
              const fieldsProxy = createFieldsProxy<StopFields>(['input']);
              state.conditions = fn(fieldsProxy);

              return {
                reason(reasonArg: string): HaltRuleBuilderWithReason<never> {
                  state.reasonExpr = reasonArg;
                  return { build: createHaltRule };
                },
                build: createHaltRule,
              };
            },
          };
          return onBuilder;
        } else {
          // SessionStart
          const onBuilder: HaltRuleBuilderOnSessionStart = {
            severity(level: Severity): HaltRuleBuilderOnSessionStart {
              state.severity = level;
              return onBuilder;
            },

            ruleId(id: string): HaltRuleBuilderOnSessionStart {
              state.ruleId = id;
              return onBuilder;
            },

            when(fn: (fields: SessionStartFields) => readonly BooleanExpr[]): HaltRuleBuilderWithConditionsNoTools {
              const fieldsProxy = createFieldsProxy<SessionStartFields>(['input']);
              state.conditions = fn(fieldsProxy);

              return {
                reason(reasonArg: string): HaltRuleBuilderWithReason<never> {
                  state.reasonExpr = reasonArg;
                  return { build: createHaltRule };
                },
                build: createHaltRule,
              };
            },
          };
          return onBuilder;
        }
      },
    } as HaltRuleBuilderWithoutTools;

    return builder;
  }
}
