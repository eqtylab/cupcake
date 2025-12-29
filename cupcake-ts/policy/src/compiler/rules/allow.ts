/**
 * Compile AllowRule to Rego.
 */

import type { AllowRule } from '../../core/index';
import type { CompilerContext } from '../context';
import { compileConditions } from '../expressions';
import { formatToolSet, indent } from '../format';

/**
 * Compiles the default deny rule (generated when canOnly is used).
 */
export function compileDefaultDeny(): string {
  return `# Default deny — generated because canOnly exists
deny contains decision if {
    not allow
    decision := {
        "rule_id": "default_deny",
        "reason": "Action not permitted",
        "severity": "MEDIUM"
    }
}`;
}

/**
 * Compiles an allow rule to Rego.
 */
export function compileAllowRule(rule: AllowRule, ctx: CompilerContext): string {
  const lines: string[] = [];

  lines.push('allow if {');

  // Event check - REQUIRED per CLAUDE.md
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

    // Copy constants to context (local vars less common in allow rules)
    for (const [k, v] of compiled.constants) {
      ctx.constants.set(k, v);
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

  lines.push('}');

  return lines.join('\n');
}
