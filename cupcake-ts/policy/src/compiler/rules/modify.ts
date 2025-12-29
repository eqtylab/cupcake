/**
 * Compile ModifyRule to Rego.
 */

import type { ModifyRule, TransformResult } from '../../core/index';
import type { CompilerContext } from '../context';
import { compileConditions } from '../expressions';
import { compileReason } from '../reason';
import { compileFieldPath } from '../paths';
import { formatToolSet, toSnakeCase, indent, formatValue } from '../format';

/**
 * Check if a value is an expression with path metadata.
 */
function isExpression(value: unknown): value is { __expr: true; __path: readonly string[] } {
  return (
    typeof value === 'object' &&
    value !== null &&
    '__expr' in value &&
    '__path' in value &&
    (value as { __expr: unknown }).__expr === true
  );
}

/**
 * Compiles the transform result to a Rego object.
 * Converts expression paths to Rego paths, and formats primitive values.
 */
function compileTransformResult(result: TransformResult): string {
  if (!result || typeof result !== 'object') {
    throw new Error('Invalid transform result: expected non-null object');
  }

  const entries: string[] = [];
  const resultEntries = Object.entries(result);

  if (resultEntries.length === 0) {
    throw new Error('Invalid transform result: expected at least one field');
  }

  for (const [key, value] of resultEntries) {
    if (isExpression(value)) {
      // Convert expression path to Rego path
      const { fullPath } = compileFieldPath(value.__path);
      entries.push(`"${key}": ${fullPath}`);
    } else {
      // Format primitive value
      entries.push(`"${key}": ${formatValue(value)}`);
    }
  }

  return `{${entries.join(', ')}}`;
}

/**
 * Compiles a modify rule to Rego.
 *
 * Modify rules output: modify contains modification if {...}
 * They transform tool input before execution.
 * The modification object includes:
 * - rule_id, reason (required)
 * - priority (optional, 1-100)
 * - severity (optional, default MEDIUM)
 * - updated_input (required, the transformed input)
 */
export function compileModifyRule(rule: ModifyRule, ctx: CompilerContext): string {
  const lines: string[] = [];

  lines.push('modify contains modification if {');

  // Hook event check
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

  // Modification object
  const ruleId = rule.ruleId ?? toSnakeCase(rule.name);
  const reason = rule.reasonExpr ?? rule.name;
  const priority = rule.priority ?? 50;
  const severity = rule.severity ?? 'MEDIUM';

  lines.push('');
  lines.push(indent('modification := {', 1));
  lines.push(indent(`"rule_id": "${ruleId}",`, 2));
  lines.push(indent(`"reason": ${compileReason(reason, ctx)},`, 2));
  lines.push(indent(`"priority": ${priority},`, 2));
  lines.push(indent(`"severity": "${severity}",`, 2));
  lines.push(indent(`"updated_input": ${compileTransformResult(rule.transformResult)}`, 2));
  lines.push(indent('}', 1));
  lines.push('}');

  return lines.join('\n');
}
