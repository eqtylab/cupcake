/**
 * Compile ContextRule to Rego.
 */

import type { ContextRule } from '../../core/index';
import type { CompilerContext } from '../context';
import { compileConditions } from '../expressions';
import { escapeRegoString, indent } from '../format';

/**
 * Compiles a context rule to Rego.
 */
export function compileContextRule(rule: ContextRule, ctx: CompilerContext): string {
  const message = escapeRegoString(rule.message);
  const lines: string[] = [];

  lines.push('add_context contains ctx if {');

  // Event check (if specified)
  if (rule.event) {
    lines.push(indent(`input.hook_event_name == "${rule.event}"`, 1));
  }

  // Compile conditions if present
  if (rule.conditions && rule.conditions.length > 0) {
    const compiled = compileConditions(rule.conditions, ctx);

    // Copy constants to context
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

  // Context assignment
  lines.push(indent(`ctx := "${message}"`, 1));
  lines.push('}');

  return lines.join('\n');
}
