/**
 * Compiler context - tracks state during compilation.
 */

/**
 * Compilation context passed through all compilation functions.
 */
export interface CompilerContext {
  /** Constants to hoist to top of file (name → values) */
  readonly constants: Map<string, readonly unknown[]>;

  /** Variable assignments within current rule (rego var name → rego path) */
  readonly localVars: Map<string, string>;

  /** Whether the policy has canOnly (allow) rules */
  readonly hasAllowRules: boolean;

  /** Indent level for current rule (0 = top level) */
  indent: number;
}

/**
 * Creates a fresh compiler context.
 */
export function createContext(hasAllowRules: boolean): CompilerContext {
  return {
    constants: new Map(),
    localVars: new Map(),
    hasAllowRules,
    indent: 0,
  };
}

/**
 * Creates a fresh local vars map for a new rule.
 */
export function resetLocalVars(ctx: CompilerContext): void {
  ctx.localVars.clear();
}
