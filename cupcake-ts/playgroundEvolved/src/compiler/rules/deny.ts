/**
 * Compile DenyRule to Rego.
 */

import type { DenyRule } from '../../core/index';
import type { CompilerContext } from '../context';
import { compileConditions } from '../expressions';
import { compileReason } from '../reason';
import { formatToolSet, toSnakeCase, indent } from '../format';

/**
 * Compiles a deny rule to Rego.
 */
export function compileDenyRule(rule: DenyRule, ctx: CompilerContext): string {
  const lines: string[] = [];

  lines.push('deny contains decision if {');

  // Hook event check (required for deny rules)
  lines.push(indent(`input.hook_event_name == "${rule.event}"`, 1));

  // Tool check
  if (rule.tools.length === 1) {
    lines.push(indent(`input.tool_name == "${rule.tools[0]}"`, 1));
  } else {
    lines.push(indent(`input.tool_name in ${formatToolSet(rule.tools)}`, 1));
  }

  // Compile conditions
  if (rule.conditions && rule.conditions.length > 0) {
    const compiled = compileConditions(rule.conditions, ctx);

    // Copy constants and local vars to context
    for (const [k, v] of compiled.constants) {
      ctx.constants.set(k, v);
    }
    for (const [k, v] of compiled.localVars) {
      ctx.localVars.set(k, v);
    }

    // Add condition lines
    for (const line of compiled.lines) {
      if (line === '') {
        lines.push('');
      } else {
        lines.push(indent(line, 1));
      }
    }
  }

  // Decision object
  const ruleId = rule.ruleId ?? toSnakeCase(rule.name);
  const reason = rule.reasonExpr ?? rule.name;

  lines.push('');
  lines.push(indent('decision := {', 1));
  lines.push(indent(`"rule_id": "${ruleId}",`, 2));
  lines.push(indent(`"reason": ${compileReason(reason, ctx)},`, 2));
  lines.push(indent(`"severity": "${rule.severity}"`, 2));
  lines.push(indent('}', 1));
  lines.push('}');

  return lines.join('\n');
}
