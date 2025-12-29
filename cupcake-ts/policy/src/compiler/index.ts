/**
 * Main compiler - transforms Policy AST to Rego source code.
 */

import type { Policy } from '../builders/policy';
import type { Rule } from '../core/index';
import {
  isDenyRule,
  isAllowRule,
  isContextRule,
  isHaltRule,
  isAskRule,
  isModifyRule,
  isBlockRule,
} from '../core/index';
import { createContext, resetLocalVars } from './context';
import { formatConstantArray, toSnakeCase } from './format';
import { compileConditions } from './expressions';
import {
  compileDenyRule,
  compileAllowRule,
  compileDefaultDeny,
  compileContextRule,
  compileHaltRule,
  compileAskRule,
  compileModifyRule,
  compileBlockRule,
} from './rules/index';

/**
 * Compiles a policy to Rego source code.
 *
 * @param policy - The policy to compile
 * @returns Valid Rego v1 source code
 */
export function compile(policy: Policy): string {
  const ctx = createContext(policy.hasAllowRules);
  const sections: string[] = [];

  // Package declaration
  const packageName = toPackageName(policy.name);
  sections.push(`package cupcake.policies.${packageName}`);
  sections.push('');
  sections.push('import rego.v1');

  // First pass: collect all constants from rules
  collectConstants(policy.rules, ctx);

  // Output constants (if any)
  if (ctx.constants.size > 0) {
    sections.push('');
    for (const [name, values] of ctx.constants) {
      sections.push(formatConstantArray(name, values));
    }
  }

  // Default deny (if canOnly rules exist)
  if (policy.hasAllowRules) {
    sections.push('');
    sections.push(compileDefaultDeny());
    sections.push('');
    sections.push('# Allow rules from canOnly');
  }

  // Compile each rule
  for (const rule of policy.rules) {
    resetLocalVars(ctx);
    sections.push('');
    sections.push(compileRule(rule, ctx));
  }

  return sections.join('\n').trim();
}

/**
 * Collects constants from all rules (first pass).
 */
function collectConstants(rules: readonly Rule[], ctx: ReturnType<typeof createContext>): void {
  for (const rule of rules) {
    resetLocalVars(ctx);

    // Extract conditions from rules that have them
    let conditions: readonly unknown[] | null = null;

    if (isDenyRule(rule)) {
      conditions = rule.conditions;
    } else if (isAllowRule(rule)) {
      conditions = rule.conditions;
    } else if (isContextRule(rule)) {
      conditions = rule.conditions;
    } else if (isHaltRule(rule)) {
      conditions = rule.conditions;
    } else if (isAskRule(rule)) {
      conditions = rule.conditions;
    } else if (isModifyRule(rule)) {
      conditions = rule.conditions;
    } else if (isBlockRule(rule)) {
      conditions = rule.conditions;
    }

    if (conditions) {
      const compiled = compileConditions(conditions as Parameters<typeof compileConditions>[0], ctx);
      for (const [k, v] of compiled.constants) {
        ctx.constants.set(k, v);
      }
    }
  }
}

/**
 * Compiles a single rule.
 */
function compileRule(rule: Rule, ctx: ReturnType<typeof createContext>): string {
  if (isDenyRule(rule)) {
    return compileDenyRule(rule, ctx);
  }
  if (isAllowRule(rule)) {
    return compileAllowRule(rule, ctx);
  }
  if (isContextRule(rule)) {
    return compileContextRule(rule, ctx);
  }
  if (isHaltRule(rule)) {
    return compileHaltRule(rule, ctx);
  }
  if (isAskRule(rule)) {
    return compileAskRule(rule, ctx);
  }
  if (isModifyRule(rule)) {
    return compileModifyRule(rule, ctx);
  }
  if (isBlockRule(rule)) {
    return compileBlockRule(rule, ctx);
  }
  throw new Error(`Unknown rule type: ${(rule as Rule).__type}`);
}

/**
 * Converts a policy name to a valid Rego package identifier.
 */
function toPackageName(name: string): string {
  return name
    .toLowerCase()
    .replace(/[\s-]+/g, '_')
    .replace(/[^a-z0-9_]/g, '');
}

// Re-export for convenience
export { toPackageName };
