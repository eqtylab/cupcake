/**
 * Compile AskRule to Rego.
 */

import type { AskRule } from '../../core/index';
import type { CompilerContext } from '../context';
import { compileConditions } from '../expressions';
import { compileReason } from '../reason';
import { formatToolSet, toSnakeCase, indent } from '../format';

/**
 * Compiles an ask rule to Rego.
 *
 * Ask rules output: ask contains decision if {...}
 * They prompt the user for confirmation before proceeding.
 * The decision object includes a "question" field.
 */
export function compileAskRule(rule: AskRule, ctx: CompilerContext): string {
  const lines: string[] = [];

  lines.push('ask contains decision if {');

  // Hook event check (ask is only valid on PreToolUse)
  lines.push(indent('input.hook_event_name == "PreToolUse"', 1));

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

  // Decision object with question field
  const ruleId = rule.ruleId ?? toSnakeCase(rule.name);
  const reason = rule.reasonExpr ?? rule.name;

  lines.push('');
  lines.push(indent('decision := {', 1));
  lines.push(indent(`"rule_id": "${ruleId}",`, 2));
  lines.push(indent(`"reason": ${compileReason(reason, ctx)},`, 2));
  lines.push(indent(`"question": ${compileReason(rule.questionExpr, ctx)},`, 2));
  lines.push(indent(`"severity": "${rule.severity}"`, 2));
  lines.push(indent('}', 1));
  lines.push('}');

  return lines.join('\n');
}
