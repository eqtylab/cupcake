/**
 * mustBlock() builder - creates block rules for post-execution feedback.
 *
 * Block rules are used to:
 * - PostToolUse: Provide feedback to Claude after tool execution
 * - UserPromptSubmit: Block user prompts from being processed
 * - Stop/SubagentStop: Force Claude to continue
 *
 * IMPORTANT: `.on(event)` is REQUIRED - there is no default event.
 *
 * @example
 * ```typescript
 * // PostToolUse - block on failed command
 * mustBlock('failed command', 'Bash')
 *   .on('PostToolUse')
 *   .when(({ toolResponse }) => [toolResponse.exitCode.notEquals(0)])
 *   .reason('Command failed with non-zero exit code')
 *   .build()
 *
 * // UserPromptSubmit - block unsafe prompts (no tools)
 * mustBlock('unsafe prompt')
 *   .on('UserPromptSubmit')
 *   .when(({ submittedPrompt }) => [submittedPrompt.contains('UNSAFE')])
 *   .reason('Blocked unsafe prompt content')
 *   .build()
 * ```
 */

import type { Tool, Severity, BlockRule, BlockEvent } from '../core/index';
import { DEFAULT_SEVERITY } from '../core/index';
import type { FieldsFor, PostToolUseFields, UserPromptSubmitFields, StopFields } from '../fields/index';
import type { BooleanExpr, ReasonExpr } from '../expressions/index';
import { createFieldsProxy } from '../expressions/index';

// Type for combined PostToolUse fields (tool fields + toolResponse)
type PostToolUseFieldsFor<T extends Tool | readonly Tool[]> = FieldsFor<T> & PostToolUseFields;

/**
 * Builder for block rules before .on() is called.
 * Tool-based variant (for PostToolUse).
 */
export interface BlockRuleBuilderWithTools<T extends Tool | readonly Tool[]> {
  /** Set the event this rule applies to (REQUIRED) */
  on(event: 'PostToolUse'): BlockRuleBuilderOnPostToolUse<T>;
}

/**
 * Builder for block rules before .on() is called.
 * Tool-less variant (for UserPromptSubmit/Stop/SubagentStop).
 */
export interface BlockRuleBuilderWithoutTools {
  /** Set the event this rule applies to (REQUIRED) */
  on(event: 'UserPromptSubmit'): BlockRuleBuilderOnUserPrompt;
  on(event: 'Stop' | 'SubagentStop'): BlockRuleBuilderOnStop;
}

/**
 * Builder for PostToolUse block rules after .on() is called.
 */
export interface BlockRuleBuilderOnPostToolUse<T extends Tool | readonly Tool[]> {
  /** Set the severity level */
  severity(level: Severity): BlockRuleBuilderOnPostToolUse<T>;
  /** Set a custom rule ID for tracking */
  ruleId(id: string): BlockRuleBuilderOnPostToolUse<T>;
  /** Define the conditions that trigger this block */
  when(fn: (fields: PostToolUseFieldsFor<T>) => readonly BooleanExpr[]): BlockRuleBuilderWithConditions<T>;
}

/**
 * Builder for UserPromptSubmit block rules after .on() is called.
 */
export interface BlockRuleBuilderOnUserPrompt {
  /** Set the severity level */
  severity(level: Severity): BlockRuleBuilderOnUserPrompt;
  /** Set a custom rule ID for tracking */
  ruleId(id: string): BlockRuleBuilderOnUserPrompt;
  /** Define the conditions that trigger this block */
  when(fn: (fields: UserPromptSubmitFields) => readonly BooleanExpr[]): BlockRuleBuilderWithConditionsNoTools;
}

/**
 * Builder for Stop/SubagentStop block rules after .on() is called.
 */
export interface BlockRuleBuilderOnStop {
  /** Set the severity level */
  severity(level: Severity): BlockRuleBuilderOnStop;
  /** Set a custom rule ID for tracking */
  ruleId(id: string): BlockRuleBuilderOnStop;
  /** Define the conditions that trigger this block */
  when(fn: (fields: StopFields) => readonly BooleanExpr[]): BlockRuleBuilderWithConditionsNoTools;
}

/**
 * Builder for block rules after .when() is called (tool-based).
 */
export interface BlockRuleBuilderWithConditions<T extends Tool | readonly Tool[]> {
  /** Define a custom reason (optional - defaults to rule name) */
  reason(fn: ((fields: PostToolUseFieldsFor<T>) => ReasonExpr) | string): BlockRuleBuilderWithReason;
  /** Build the block rule */
  build(): BlockRule;
}

/**
 * Builder for block rules after .when() is called (tool-less).
 */
export interface BlockRuleBuilderWithConditionsNoTools {
  /** Define a custom reason (optional - defaults to rule name) */
  reason(fn: string): BlockRuleBuilderWithReason;
  /** Build the block rule */
  build(): BlockRule;
}

/**
 * Builder for block rules after .reason() is called.
 */
export interface BlockRuleBuilderWithReason {
  /** Build the block rule */
  build(): BlockRule;
}

/**
 * Internal state for building block rules.
 */
interface BlockRuleState {
  name: string;
  tools: readonly Tool[] | null;
  event: BlockEvent | null;
  severity: Severity;
  ruleId: string | null;
  conditions: readonly BooleanExpr[] | null;
  reasonExpr: ReasonExpr | string | null;
}

/**
 * Creates a tool-based block rule builder (for PostToolUse).
 *
 * @param name - Human-readable name for the rule (becomes default reason)
 * @param tools - Tool or tools this rule applies to
 * @returns A builder for configuring the block rule
 */
export function mustBlock<T extends Tool | readonly Tool[]>(
  name: string,
  tools: T
): BlockRuleBuilderWithTools<T>;

/**
 * Creates a tool-less block rule builder (for UserPromptSubmit/Stop/SubagentStop).
 *
 * @param name - Human-readable name for the rule (becomes default reason)
 * @returns A builder for configuring the block rule
 */
export function mustBlock(name: string): BlockRuleBuilderWithoutTools;

/**
 * Implementation of mustBlock with overloaded signatures.
 */
export function mustBlock<T extends Tool | readonly Tool[]>(
  name: string,
  tools?: T
): BlockRuleBuilderWithTools<T> | BlockRuleBuilderWithoutTools {
  const toolsArray: readonly Tool[] | null = tools
    ? Array.isArray(tools)
      ? tools
      : [tools as Tool]
    : null;

  const state: BlockRuleState = {
    name,
    tools: toolsArray,
    event: null,
    severity: DEFAULT_SEVERITY,
    ruleId: null,
    conditions: null,
    reasonExpr: null,
  };

  const createBlockRule = (): BlockRule => {
    if (!state.event) {
      throw new Error('mustBlock requires .on(event) to be called');
    }
    return {
      __type: 'block',
      name: state.name,
      event: state.event,
      tools: state.tools,
      severity: state.severity,
      ruleId: state.ruleId,
      conditions: state.conditions,
      reasonExpr: state.reasonExpr ?? state.name,
    };
  };

  if (toolsArray) {
    // Tool-based builder (for PostToolUse)
    const builder: BlockRuleBuilderWithTools<T> = {
      on(event: 'PostToolUse'): BlockRuleBuilderOnPostToolUse<T> {
        state.event = event;

        const onBuilder: BlockRuleBuilderOnPostToolUse<T> = {
          severity(level: Severity): BlockRuleBuilderOnPostToolUse<T> {
            state.severity = level;
            return onBuilder;
          },

          ruleId(id: string): BlockRuleBuilderOnPostToolUse<T> {
            state.ruleId = id;
            return onBuilder;
          },

          when(
            fn: (fields: PostToolUseFieldsFor<T>) => readonly BooleanExpr[]
          ): BlockRuleBuilderWithConditions<T> {
            const fieldsProxy = createFieldsProxy<PostToolUseFieldsFor<T>>(['input']);
            state.conditions = fn(fieldsProxy);

            return {
              reason(
                reasonArg: ((fields: PostToolUseFieldsFor<T>) => ReasonExpr) | string
              ): BlockRuleBuilderWithReason {
                if (typeof reasonArg === 'string') {
                  state.reasonExpr = reasonArg;
                } else {
                  state.reasonExpr = reasonArg(fieldsProxy);
                }
                return { build: createBlockRule };
              },
              build: createBlockRule,
            };
          },
        };

        return onBuilder;
      },
    };

    return builder;
  } else {
    // Tool-less builder (for UserPromptSubmit/Stop/SubagentStop)
    // Cast to interface since overloaded methods can't have a single compatible implementation signature
    const builder = {
      on(event: 'UserPromptSubmit' | 'Stop' | 'SubagentStop') {
        state.event = event;

        if (event === 'UserPromptSubmit') {
          const onBuilder: BlockRuleBuilderOnUserPrompt = {
            severity(level: Severity): BlockRuleBuilderOnUserPrompt {
              state.severity = level;
              return onBuilder;
            },

            ruleId(id: string): BlockRuleBuilderOnUserPrompt {
              state.ruleId = id;
              return onBuilder;
            },

            when(fn: (fields: UserPromptSubmitFields) => readonly BooleanExpr[]): BlockRuleBuilderWithConditionsNoTools {
              const fieldsProxy = createFieldsProxy<UserPromptSubmitFields>(['input']);
              state.conditions = fn(fieldsProxy);

              return {
                reason(reasonArg: string): BlockRuleBuilderWithReason {
                  state.reasonExpr = reasonArg;
                  return { build: createBlockRule };
                },
                build: createBlockRule,
              };
            },
          };
          return onBuilder;
        } else {
          // Stop or SubagentStop
          const onBuilder: BlockRuleBuilderOnStop = {
            severity(level: Severity): BlockRuleBuilderOnStop {
              state.severity = level;
              return onBuilder;
            },

            ruleId(id: string): BlockRuleBuilderOnStop {
              state.ruleId = id;
              return onBuilder;
            },

            when(fn: (fields: StopFields) => readonly BooleanExpr[]): BlockRuleBuilderWithConditionsNoTools {
              const fieldsProxy = createFieldsProxy<StopFields>(['input']);
              state.conditions = fn(fieldsProxy);

              return {
                reason(reasonArg: string): BlockRuleBuilderWithReason {
                  state.reasonExpr = reasonArg;
                  return { build: createBlockRule };
                },
                build: createBlockRule,
              };
            },
          };
          return onBuilder;
        }
      },
    } as BlockRuleBuilderWithoutTools;

    return builder;
  }
}
