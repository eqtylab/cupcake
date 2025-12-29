/**
 * Rego formatting and string utilities.
 */

/**
 * Escapes a string for use in Rego.
 * - Converts double quotes to single quotes (for readability)
 * - Escapes backslashes and control characters
 */
export function escapeRegoString(str: string): string {
  return str
    .replace(/\\/g, '\\\\')
    .replace(/'/g, "\\'") // Escape existing single quotes first
    .replace(/"/g, "'") // Then convert double quotes to single
    .replace(/\n/g, '\\n')
    .replace(/\r/g, '\\r')
    .replace(/\t/g, '\\t');
}

/**
 * Converts a camelCase string to snake_case.
 */
export function toSnakeCase(str: string): string {
  return str
    .replace(/([A-Z])/g, '_$1')
    .toLowerCase()
    .replace(/^_/, '')
    .replace(/[\s-]+/g, '_');
}

/**
 * Creates an indented line.
 */
export function indent(line: string, level: number): string {
  return '    '.repeat(level) + line;
}

/**
 * Formats a list of tools as a Rego set.
 */
export function formatToolSet(tools: readonly string[]): string {
  const items = tools.map((t) => `"${t}"`).join(', ');
  return `{${items}}`;
}

/**
 * Formats a list of tools as a Rego array.
 */
export function formatToolArray(tools: readonly string[]): string {
  const items = tools.map((t) => `"${t}"`).join(', ');
  return `[${items}]`;
}

/**
 * Formats a constant array for hoisting.
 */
export function formatConstantArray(name: string, values: readonly unknown[]): string {
  const lines = [
    `${name} := [`,
    ...values.map((v) => `    "${v}",`),
    ']',
  ];
  return lines.join('\n');
}

/**
 * Formats a JSON value for Rego output.
 */
export function formatValue(value: unknown): string {
  if (typeof value === 'string') {
    return `"${escapeRegoString(value)}"`;
  }
  if (typeof value === 'boolean') {
    return value ? 'true' : 'false';
  }
  if (typeof value === 'number') {
    return String(value);
  }
  return JSON.stringify(value);
}
