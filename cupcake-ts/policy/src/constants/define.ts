/**
 * Named constant definition for use in policies.
 * Preserves the constant name for Rego compilation.
 */

/**
 * A named constant that carries its name for Rego output.
 * Used with containsAny() to generate properly named constants.
 */
export interface NamedConstant<T> {
  readonly __type: 'constant';
  /** The constant name (used in Rego output) */
  readonly name: string;
  /** The constant values */
  readonly values: readonly T[];
  /** Singular form of the name for iteration variable */
  readonly iteratorName: string;
}

/**
 * Derives a singular iterator name from a plural constant name.
 * E.g., "dangerous_directories" → "dangerous"
 *       "blocked_commands" → "blocked_command"
 */
function deriveIteratorName(name: string): string {
  // Common patterns for deriving singular
  if (name.endsWith('_directories')) {
    // dangerous_directories → dangerous
    return name.replace(/_directories$/, '');
  }
  if (name.endsWith('_paths')) {
    // blocked_paths → blocked_path
    return name.replace(/_paths$/, '_path');
  }
  if (name.endsWith('ies')) {
    // entries → entry
    return name.replace(/ies$/, 'y');
  }
  if (name.endsWith('es')) {
    // matches → match
    return name.replace(/es$/, '');
  }
  if (name.endsWith('s')) {
    // items → item
    return name.replace(/s$/, '');
  }
  return name + '_item';
}

/**
 * Defines a named constant for use in containsAny() and similar operations.
 *
 * @param name - The constant name (will be output in Rego as-is)
 * @param values - The array of values
 * @returns A NamedConstant that preserves the name for compilation
 *
 * @example
 * ```typescript
 * const dangerousDirectories = defineConstant('dangerous_directories', [
 *   '/etc/',
 *   '/bin/',
 *   '/System/',
 * ]);
 *
 * // In policy:
 * resolvedFilePath.lower().containsAny(dangerousDirectories)
 *
 * // Compiles to:
 * // dangerous_directories := ["/etc/", "/bin/", "/System/"]
 * // some dangerous in dangerous_directories
 * // contains(lower(resolved_path), lower(dangerous))
 * ```
 */
export function defineConstant<T>(name: string, values: readonly T[]): NamedConstant<T> {
  return {
    __type: 'constant',
    name,
    values,
    iteratorName: deriveIteratorName(name),
  };
}

/**
 * Type guard to check if a value is a NamedConstant.
 */
export function isNamedConstant<T>(value: unknown): value is NamedConstant<T> {
  return (
    typeof value === 'object' &&
    value !== null &&
    '__type' in value &&
    (value as NamedConstant<T>).__type === 'constant'
  );
}
