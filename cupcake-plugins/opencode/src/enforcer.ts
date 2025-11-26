/**
 * Decision enforcer - interprets Cupcake responses and enforces decisions
 */

import type { CupcakeResponse, ToastVariant } from "./types.js";
import { getToastVariant } from "./types.js";

/**
 * Formatted decision for display
 */
export interface FormattedDecision {
  /** Whether the operation should be blocked */
  blocked: boolean;
  /** Toast title */
  title: string;
  /** Toast/error message */
  message: string;
  /** Toast variant */
  variant: ToastVariant;
  /** Original decision */
  decision: CupcakeResponse["decision"];
  /** Rule ID if available */
  ruleId?: string;
  /** Severity if available */
  severity?: string;
}

/**
 * Format a Cupcake response for display
 *
 * @param response - Cupcake response
 * @returns Formatted decision info
 */
export function formatDecision(response: CupcakeResponse): FormattedDecision {
  const { decision, reason, rule_id, severity } = response;

  let title: string;
  let message: string;
  let blocked = false;

  switch (decision) {
    case "allow":
      title = "Allowed";
      message = reason || "Operation allowed by policy";
      break;

    case "deny":
    case "block":
      title = "Policy Violation";
      message = reason || `Operation blocked by policy`;
      blocked = true;
      break;

    case "ask":
      title = "Approval Required";
      message = reason || "This operation requires approval";
      blocked = true; // For tool.execute.before, ask also blocks
      break;

    default:
      title = "Unknown Decision";
      message = `Policy returned unknown decision: ${decision}`;
      blocked = true; // Fail closed for unknown decisions
  }

  // Add rule ID and severity to message if available
  if (rule_id || severity) {
    const details: string[] = [];
    if (rule_id) details.push(`Rule: ${rule_id}`);
    if (severity) details.push(`Severity: ${severity}`);
    message += `\n(${details.join(", ")})`;
  }

  return {
    blocked,
    title,
    message,
    variant: getToastVariant(decision),
    decision,
    ruleId: rule_id,
    severity,
  };
}

/**
 * Format an error message for throwing
 *
 * @param formatted - Formatted decision
 * @returns Error message string
 */
export function formatErrorMessage(formatted: FormattedDecision): string {
  let message = "";

  // Add decision type indicator
  if (formatted.decision === "deny" || formatted.decision === "block") {
    message += "❌ Policy Violation\n\n";
  } else if (formatted.decision === "ask") {
    message += "⚠️  Approval Required\n\n";
  }

  message += formatted.message;

  // Special handling for "ask" decisions
  if (formatted.decision === "ask") {
    message += "\n\nNote: This operation requires manual approval. ";
    message += "To proceed, review the policy and temporarily disable it if appropriate, ";
    message += "then re-run the command.";
  }

  return message;
}

/**
 * Enforce a Cupcake decision (legacy function for compatibility)
 *
 * @param response - Cupcake response
 * @throws Error if decision is deny, block, or ask
 */
export function enforceDecision(response: CupcakeResponse): void {
  const formatted = formatDecision(response);

  if (formatted.blocked) {
    throw new Error(formatErrorMessage(formatted));
  }
}
