/**
 * Core types module - foundation types for the policy DSL.
 */

export { type Severity, DEFAULT_SEVERITY } from './severity';
export { type Tool, ALL_TOOLS } from './tool';
export {
  type RuleType,
  type BaseRule,
  type DenyRule,
  type AllowRule,
  type ContextRule,
  type HaltRule,
  type AskRule,
  type ModifyRule,
  type BlockRule,
  type TransformResult,
  type Rule,
  isDenyRule,
  isAllowRule,
  isContextRule,
  isHaltRule,
  isAskRule,
  isModifyRule,
  isBlockRule,
} from './rule';

// Event types
export {
  type HookEvent,
  type ToolEvent,
  type BlockEvent,
  type DenyEvent,
  type AskEvent,
  type ModifyEvent,
  type HaltEvent,
  type ContextEvent,
  type DecisionVerbsFor,
  isBlockEvent,
  isToolEvent,
} from './events';
