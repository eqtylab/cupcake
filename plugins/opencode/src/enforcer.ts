/**
 * Decision enforcer - interprets Cupcake responses and enforces decisions
 */

import type { CupcakeResponse } from "./types.js";

/**
 * Format an error message for the user
 *
 * @param response - Cupcake response
 * @returns Formatted error message
 */
export function formatErrorMessage(response: CupcakeResponse): string {
  const { decision, reason } = response;

  let message = "";

  // Add decision type indicator
  if (decision === "deny" || decision === "block") {
    message += "❌ Policy Violation\n\n";
  } else if (decision === "ask") {
    message += "⚠️  Approval Required\n\n";
  }

  // Add reason if provided
  if (reason) {
    message += reason;
  } else {
    message += `Operation blocked by policy (${decision})`;
  }

  // Special handling for "ask" decisions
  if (decision === "ask") {
    message += "\n\nNote: This operation requires manual approval. ";
    message += "To proceed, review the policy and temporarily disable it if appropriate, ";
    message += "then re-run the command.";
  }

  return message;
}

/**
 * Enforce a Cupcake decision
 *
 * @param response - Cupcake response
 * @throws Error if decision is deny, block, or ask
 */
export function enforceDecision(response: CupcakeResponse): void {
  const { decision } = response;

  // Allow execution
  if (decision === "allow") {
    return; // No action needed
  }

  // Block execution
  if (decision === "deny" || decision === "block" || decision === "ask") {
    const errorMessage = formatErrorMessage(response);
    throw new Error(errorMessage);
  }

  // Unknown decision type - fail closed for security
  console.error(`[cupcake-plugin] WARN: Unknown decision type: ${decision}`);
  throw new Error(`Policy returned unknown decision type: ${decision}`);
}
