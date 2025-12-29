/**
 * Cupcake Policy Evolved - Type-safe policy DSL
 *
 * A natural language policy DSL that compiles to Rego.
 *
 * @example
 * ```typescript
 * import { policy, cant, canOnly, addContext, reason, defineSignal } from '@cupcake/policy-evolved';
 *
 * const gitBranch = defineSignal('gitBranch', async () => 'main');
 *
 * const myPolicy = policy('technical writer',
 *   cant('write to src', ['Write', 'Edit'])
 *     .severity('HIGH')
 *     .when(({ path }) => [path.contains('src/')]),
 *
 *   cant('push to main', 'Bash')
 *     .when(({ command }) => [
 *       command.contains('git push'),
 *       gitBranch.equals('main'),
 *     ]),
 *
 *   canOnly('access blog files', ['Read', 'Write'])
 *     .when(({ path }) => [path.contains('blog/')]),
 *
 *   addContext('Follow the company style guide for all content.'),
 * );
 * ```
 *
 * @packageDocumentation
 */

// Core types
export {
  type Severity,
  DEFAULT_SEVERITY,
  type Tool,
  ALL_TOOLS,
  type RuleType,
  type BaseRule,
  type DenyRule,
  type AllowRule,
  type ContextRule,
  type HaltRule,
  type AskRule,
  type ModifyRule,
  type BlockRule,
  type BlockEvent,
  type TransformResult,
  type Rule,
  isDenyRule,
  isAllowRule,
  isContextRule,
  isHaltRule,
  isAskRule,
  isModifyRule,
  isBlockRule,
  // Event types
  type HookEvent,
  type ToolEvent,
  type DenyEvent,
  type AskEvent,
  type ModifyEvent,
  type HaltEvent,
  type ContextEvent,
  type DecisionVerbsFor,
  isBlockEvent,
  isToolEvent,
} from './core/index';

// Expression types
export {
  type ExpressionMetadata,
  type ComparisonMetadata,
  type BooleanExpr,
  type StringExpr,
  type NumberExpr,
  type ReasonExpr,
  reason,
} from './expressions/index';

// Field types
export {
  type CommonFields,
  type BashFields,
  type WriteFields,
  type EditFields,
  type ReadFields,
  type TaskFields,
  type ContextFields,
  type ToolFieldsMap,
  type FieldsFor,
  // Event-specific fields
  type PostToolUseFields,
  type ToolResponseFields,
  type UserPromptSubmitFields,
  type StopFields,
} from './fields/index';

// Signal types
export {
  type ExpressionFor,
  type Signal,
  defineSignal,
  defineTypedSignal,
} from './signals/index';

// Constants
export {
  type NamedConstant,
  defineConstant,
  isNamedConstant,
} from './constants/index';

// Builders
export {
  type DenyRuleBuilder,
  type DenyRuleBuilderWithConditions,
  cant,
  type AllowRuleBuilder,
  canOnly,
  type ContextBuilder,
  addContext,
  type Policy,
  policy,
  compile,
  toPackageName,
  type HaltRuleBuilder,
  type HaltRuleBuilderWithConditions,
  type HaltRuleBuilderWithReason,
  mustHalt,
  type AskRuleBuilder,
  type AskRuleBuilderWithConditions,
  type AskRuleBuilderWithReason,
  type AskRuleBuilderWithQuestion,
  mustAsk,
  type ModifyRuleBuilder,
  type ModifyRuleBuilderWithConditions,
  type ModifyRuleBuilderWithReason,
  type ModifyRuleBuilderWithTransform,
  mustModify,
  // Block builders
  type BlockRuleBuilderWithTools,
  type BlockRuleBuilderWithoutTools,
  type BlockRuleBuilderOnPostToolUse,
  type BlockRuleBuilderOnUserPrompt,
  type BlockRuleBuilderOnStop,
  type BlockRuleBuilderWithConditions,
  type BlockRuleBuilderWithConditionsNoTools,
  type BlockRuleBuilderWithReason,
  mustBlock,
} from './builders/index';
