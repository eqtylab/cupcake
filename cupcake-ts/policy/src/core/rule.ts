/**
 * Rule type definitions.
 * These represent the different kinds of rules that can be created.
 */

import type { Tool } from './tool';
import type { Severity } from './severity';
import type { BooleanExpr, ReasonExpr, StringExpr, NumberExpr } from '../expressions/index';
import type { DenyEvent, HaltEvent, ModifyEvent, BlockEvent, ContextEvent } from './events';

/**
 * Discriminator for rule types.
 */
export type RuleType = 'deny' | 'allow' | 'context' | 'halt' | 'ask' | 'modify' | 'block';

/**
 * Base interface for all rules.
 */
export interface BaseRule {
  /** Rule type discriminator */
  readonly __type: RuleType;
  /** Human-readable rule name (also used as default reason) */
  readonly name: string;
}

/**
 * A deny rule that blocks actions matching the conditions.
 * Created with the cant() builder.
 */
export interface DenyRule extends BaseRule {
  readonly __type: 'deny';
  /** The event this rule applies to (default: PreToolUse) */
  readonly event: DenyEvent;
  /** Tools this rule applies to */
  readonly tools: readonly Tool[];
  /** Severity level for the denial */
  readonly severity: Severity;
  /** Optional rule ID for tracking */
  readonly ruleId: string | null;
  /** Conditions that trigger the denial (captured from .when()) */
  readonly conditions: readonly BooleanExpr[] | null;
  /** The reason expression (from .reason() or derived from name) */
  readonly reasonExpr: ReasonExpr | string | null;
}

/**
 * An allow rule that permits actions matching the conditions.
 * Created with the canOnly() builder.
 * When canOnly() is used, a default deny is automatically generated.
 */
export interface AllowRule extends BaseRule {
  readonly __type: 'allow';
  /** The event this rule applies to (always PreToolUse) */
  readonly event: 'PreToolUse';
  /** Tools this rule applies to */
  readonly tools: readonly Tool[];
  /** Conditions that permit the action (captured from .when()) */
  readonly conditions: readonly BooleanExpr[] | null;
}

/**
 * A context injection rule that adds context to the agent.
 * Created with the addContext() builder.
 */
export interface ContextRule extends BaseRule {
  readonly __type: 'context';
  /** The context message to inject */
  readonly message: string;
  /** Target event (null = unconditional, fires on all context events) */
  readonly event: ContextEvent | null;
  /** Conditions for when to inject (null = unconditional) */
  readonly conditions: readonly BooleanExpr[] | null;
}

/**
 * A halt rule that immediately stops Claude.
 * Created with the mustHalt() builder.
 */
export interface HaltRule extends BaseRule {
  readonly __type: 'halt';
  /** The event this rule applies to (default: PreToolUse) */
  readonly event: HaltEvent;
  /** Tools this rule applies to (null for non-tool events like UserPromptSubmit) */
  readonly tools: readonly Tool[] | null;
  /** Severity level */
  readonly severity: Severity;
  /** Optional rule ID for tracking */
  readonly ruleId: string | null;
  /** Conditions that trigger the halt */
  readonly conditions: readonly BooleanExpr[] | null;
  /** The reason expression */
  readonly reasonExpr: ReasonExpr | string | null;
}

/**
 * An ask rule that prompts the user for confirmation.
 * Created with the mustAsk() builder.
 * Only valid on PreToolUse event.
 */
export interface AskRule extends BaseRule {
  readonly __type: 'ask';
  /** Tools this rule applies to */
  readonly tools: readonly Tool[];
  /** Severity level */
  readonly severity: Severity;
  /** Optional rule ID for tracking */
  readonly ruleId: string | null;
  /** Conditions that trigger the ask */
  readonly conditions: readonly BooleanExpr[] | null;
  /** The reason expression */
  readonly reasonExpr: ReasonExpr | string | null;
  /** The question to ask the user (REQUIRED) */
  readonly questionExpr: ReasonExpr | string;
}

/**
 * Allowed value types for transform results.
 */
export type TransformValue = StringExpr | NumberExpr | BooleanExpr | string | number | boolean;

/**
 * Transform function type for modify rules.
 * Takes field expressions and returns an object of field transformations.
 */
export type TransformResult = Record<string, TransformValue>;

/**
 * A modify rule that transforms tool input before execution.
 * Created with the mustModify() builder.
 */
export interface ModifyRule extends BaseRule {
  readonly __type: 'modify';
  /** The event this rule applies to (default: PreToolUse) */
  readonly event: ModifyEvent;
  /** Tools this rule applies to */
  readonly tools: readonly Tool[];
  /** Optional rule ID for tracking */
  readonly ruleId: string | null;
  /** Priority for transform ordering (1-100, higher wins). Default 50. */
  readonly priority: number | null;
  /** Severity level. Default MEDIUM. */
  readonly severity: Severity | null;
  /** Conditions that trigger the modify */
  readonly conditions: readonly BooleanExpr[] | null;
  /** The reason expression */
  readonly reasonExpr: ReasonExpr | string | null;
  /** The transform result (updated_input). Generated from transform callback. */
  readonly transformResult: TransformResult;
}

/**
 * A block rule that provides feedback after tool execution or blocks prompts.
 * Created with the mustBlock() builder.
 * Only valid on PostToolUse, UserPromptSubmit, Stop, and SubagentStop events.
 */
export interface BlockRule extends BaseRule {
  readonly __type: 'block';
  /** The event this rule applies to (REQUIRED - no default) */
  readonly event: BlockEvent;
  /** Tools this rule applies to (null for non-tool events like UserPromptSubmit) */
  readonly tools: readonly Tool[] | null;
  /** Severity level */
  readonly severity: Severity;
  /** Optional rule ID for tracking */
  readonly ruleId: string | null;
  /** Conditions that trigger the block */
  readonly conditions: readonly BooleanExpr[] | null;
  /** The reason expression */
  readonly reasonExpr: ReasonExpr | string | null;
}

/**
 * Union of all rule types.
 */
export type Rule = DenyRule | AllowRule | ContextRule | HaltRule | AskRule | ModifyRule | BlockRule;

/**
 * Type guard for deny rules.
 */
export function isDenyRule(rule: Rule): rule is DenyRule {
  return rule.__type === 'deny';
}

/**
 * Type guard for allow rules.
 */
export function isAllowRule(rule: Rule): rule is AllowRule {
  return rule.__type === 'allow';
}

/**
 * Type guard for context rules.
 */
export function isContextRule(rule: Rule): rule is ContextRule {
  return rule.__type === 'context';
}

/**
 * Type guard for halt rules.
 */
export function isHaltRule(rule: Rule): rule is HaltRule {
  return rule.__type === 'halt';
}

/**
 * Type guard for ask rules.
 */
export function isAskRule(rule: Rule): rule is AskRule {
  return rule.__type === 'ask';
}

/**
 * Type guard for modify rules.
 */
export function isModifyRule(rule: Rule): rule is ModifyRule {
  return rule.__type === 'modify';
}

/**
 * Type guard for block rules.
 */
export function isBlockRule(rule: Rule): rule is BlockRule {
  return rule.__type === 'block';
}
