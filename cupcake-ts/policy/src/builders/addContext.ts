/**
 * addContext() builder - creates context injection rules.
 *
 * Context rules inject additional instructions or information to the agent.
 * They can be unconditional, event-targeted, or conditional.
 *
 * @example
 * ```typescript
 * // Unconditional context (injected on all applicable events)
 * addContext('This is how we write our blogs...')
 *
 * // Event-targeted context
 * addContext('Remember the style guide...')
 *   .on('SessionStart')
 *
 * // Event-targeted with conditions
 * addContext('Use Python style.')
 *   .on('UserPromptSubmit')
 *   .when(({ userPrompt }) => [userPrompt.contains('python')])
 *
 * // Conditional context (manual event check)
 * addContext('Always talk like a pirate...')
 *   .when(({ hookEventName }) => [hookEventName.equals('UserPromptSubmit')])
 * ```
 */

import type { ContextRule, ContextEvent } from '../core/index';
import type { ContextFields } from '../fields/index';
import type { BooleanExpr } from '../expressions/index';
import { createFieldsProxy } from '../expressions/index';

/**
 * Builder for context rules.
 * Can be used as-is (unconditional), with .on() (event-targeted),
 * or with .when() (conditional).
 */
export interface ContextBuilder extends ContextRule {
  /** Target a specific event */
  on(event: ContextEvent): ContextBuilderWithEvent;
  /** Define conditions for when to inject this context */
  when(fn: (fields: ContextFields) => readonly BooleanExpr[]): ContextRule;
}

/**
 * Builder for context rules after .on() is called.
 * Can be used as-is or with .when() for additional conditions.
 */
export interface ContextBuilderWithEvent extends ContextRule {
  /** Define additional conditions for when to inject this context */
  when(fn: (fields: ContextFields) => readonly BooleanExpr[]): ContextRule;
}

/**
 * Creates a context injection rule.
 *
 * The returned object can be used directly (for unconditional injection),
 * with .on() (for event-targeted injection), or with .when() (for conditional injection).
 *
 * @param message - The context message to inject
 * @returns A context rule or builder
 *
 * @example
 * ```typescript
 * // Unconditional - fires on all context events
 * addContext('Remember to follow the style guide.')
 *
 * // Event-targeted - fires only on SessionStart
 * addContext('Remember the style guide...')
 *   .on('SessionStart')
 *
 * // Event-targeted with conditions
 * addContext('Use Python style.')
 *   .on('UserPromptSubmit')
 *   .when(({ userPrompt }) => [userPrompt.contains('python')])
 *
 * // Conditional with manual event check
 * addContext('Use camelCase for variable names.')
 *   .when(({ hookEventName }) => [
 *     hookEventName.equals('UserPromptSubmit'),
 *   ])
 * ```
 */
export function addContext(message: string): ContextBuilder {
  // Create the base rule (unconditional, no event targeting)
  const baseRule: ContextRule = {
    __type: 'context',
    name: message,
    message,
    event: null,
    conditions: null,
  };

  // Create builder that extends the base rule
  const builder: ContextBuilder = {
    ...baseRule,

    on(event: ContextEvent): ContextBuilderWithEvent {
      // Create event-targeted rule
      const eventRule: ContextRule = {
        __type: 'context',
        name: message,
        message,
        event,
        conditions: null,
      };

      // Return builder with .when() method
      const eventBuilder: ContextBuilderWithEvent = {
        ...eventRule,

        when(fn: (fields: ContextFields) => readonly BooleanExpr[]): ContextRule {
          const fieldsProxy = createFieldsProxy<ContextFields>(['input']);
          const conditions = fn(fieldsProxy);

          return {
            __type: 'context',
            name: message,
            message,
            event,
            conditions,
          };
        },
      };

      return eventBuilder;
    },

    when(fn: (fields: ContextFields) => readonly BooleanExpr[]): ContextRule {
      // Create fields proxy for the callback
      const fieldsProxy = createFieldsProxy<ContextFields>(['input']);
      const conditions = fn(fieldsProxy);

      return {
        __type: 'context',
        name: message,
        message,
        event: null,
        conditions,
      };
    },
  };

  return builder;
}
