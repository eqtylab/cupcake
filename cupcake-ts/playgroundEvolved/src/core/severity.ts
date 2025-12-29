/**
 * Severity levels for policy rules.
 * Maps to Rego decision severity field.
 */
export type Severity = 'LOW' | 'MEDIUM' | 'HIGH' | 'CRITICAL';

/**
 * Default severity when not explicitly specified.
 */
export const DEFAULT_SEVERITY: Severity = 'MEDIUM';
