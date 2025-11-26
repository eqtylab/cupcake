/**
 * Executor - runs cupcake CLI and parses responses
 */

import type { CupcakeConfig, CupcakeEvent, CupcakeResponse } from "./types.js";

/**
 * Execute cupcake eval command with the given event
 *
 * @param config - Plugin configuration
 * @param event - Cupcake event to evaluate
 * @returns Cupcake response
 * @throws Error if execution fails or times out
 */
export async function executeCupcake(
  config: CupcakeConfig,
  event: CupcakeEvent,
): Promise<CupcakeResponse> {
  const startTime = Date.now();
  const eventJson = JSON.stringify(event);

  if (config.logLevel === "debug") {
    console.error(`[cupcake] DEBUG: Executing cupcake`);
    console.error(`[cupcake] DEBUG: Event:`, eventJson);
  }

  const proc = Bun.spawn([config.cupcakePath, "eval", "--harness", config.harness], {
    stdin: "pipe",
    stdout: "pipe",
    stderr: "ignore",
  });

  // Write event to stdin
  proc.stdin.write(eventJson);
  proc.stdin.end();

  // Set up timeout
  const timeoutPromise = new Promise<never>((_, reject) => {
    setTimeout(() => {
      proc.kill();
      reject(
        new Error(
          `Policy evaluation timed out after ${config.timeoutMs}ms. ` +
            `Consider optimizing policies or increasing timeout.`,
        ),
      );
    }, config.timeoutMs);
  });

  try {
    // Race between completion and timeout
    const [stdout, exitCode] = await Promise.race([
      Promise.all([new Response(proc.stdout).text(), proc.exited]),
      timeoutPromise,
    ]);

    const elapsed = Date.now() - startTime;

    if (config.logLevel === "debug") {
      console.error(`[cupcake] DEBUG: Cupcake response (${elapsed}ms):`, stdout);
    }

    // Check exit code
    if (exitCode !== 0) {
      const error = new Error(`Cupcake exited with code ${exitCode}`);

      if (config.failMode === "open") {
        console.error(`[cupcake] ERROR: ${error.message}`);
        console.error(`[cupcake] WARN: Allowing operation in fail-open mode.`);
        return { decision: "allow" };
      }

      throw error;
    }

    // Parse JSON response
    const response: CupcakeResponse = JSON.parse(stdout);

    if (config.logLevel === "debug") {
      console.error(`[cupcake] DEBUG: Decision: ${response.decision} (${elapsed}ms)`);
    }

    return response;
  } catch (error: any) {
    // Handle timeout and other errors
    if (config.failMode === "open") {
      console.error(`[cupcake] ERROR: ${error.message}`);
      console.error(`[cupcake] WARN: Allowing operation in fail-open mode.`);
      return { decision: "allow" };
    }

    throw error;
  }
}

/**
 * Check if cupcake CLI is available
 *
 * @param cupcakePath - Path to cupcake binary
 * @returns true if available, false otherwise
 */
export async function checkCupcakeAvailable(cupcakePath: string): Promise<boolean> {
  try {
    const proc = Bun.spawn([cupcakePath, "--version"], {
      stdout: "ignore",
      stderr: "ignore",
    });
    const exitCode = await proc.exited;
    return exitCode === 0;
  } catch {
    return false;
  }
}
