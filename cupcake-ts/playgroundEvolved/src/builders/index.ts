/**
 * Builders module - fluent API for creating policy rules.
 */

export { type DenyRuleBuilder, type DenyRuleBuilderWithConditions, cant } from './cant';
export { type AllowRuleBuilder, canOnly } from './canOnly';
export { type ContextBuilder, type ContextBuilderWithEvent, addContext } from './addContext';
export { type Policy, policy, compile, toPackageName } from './policy';
export {
  type HaltRuleBuilder,
  type HaltRuleBuilderWithConditions,
  type HaltRuleBuilderWithReason,
  mustHalt,
} from './mustHalt';
export {
  type AskRuleBuilder,
  type AskRuleBuilderWithConditions,
  type AskRuleBuilderWithReason,
  type AskRuleBuilderWithQuestion,
  mustAsk,
} from './mustAsk';
export {
  type ModifyRuleBuilder,
  type ModifyRuleBuilderWithConditions,
  type ModifyRuleBuilderWithReason,
  type ModifyRuleBuilderWithTransform,
  mustModify,
} from './mustModify';
export {
  type BlockRuleBuilderWithTools,
  type BlockRuleBuilderWithoutTools,
  type BlockRuleBuilderOnPostToolUse,
  type BlockRuleBuilderOnUserPrompt,
  type BlockRuleBuilderOnStop,
  type BlockRuleBuilderWithConditions,
  type BlockRuleBuilderWithConditionsNoTools,
  type BlockRuleBuilderWithReason,
  mustBlock,
} from './mustBlock';
