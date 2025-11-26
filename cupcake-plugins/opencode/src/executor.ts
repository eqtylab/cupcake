/**
 * Executor - runs cupcake CLI and parses responses
 */

import { spawn } from "child_process";
import type { CupcakeConfig, CupcakeEvent, CupcakeResponse } from "./types.js";

/**
 * Execute cupcake eval command with the given event
 *
 * @param config - Plugin configuration
 * @param event - Cupcake event to evaluate
 * @param $ - Bun shell API
 * @returns Cupcake response
 * @throws Error if execution fails or times out
 */
export async function executeCupcake(
  config: CupcakeConfig,
  event: CupcakeEvent,
  $?: any, // Optional - not used with Node.js
): Promise<CupcakeResponse> {
  const startTime = Date.now();

  return new Promise((resolve, reject) => {
    // Execute cupcake with event as stdin
    const eventJson = JSON.stringify(event);

    if (config.logLevel === "debug") {
      console.error(`[cupcake] DEBUG: Executing cupcake`);
      console.error(`[cupcake] DEBUG: Event:`, eventJson);
    }

    // Spawn cupcake process
    const proc = spawn(config.cupcakePath, ["eval", "--harness", config.harness], {
      stdio: ["pipe", "pipe", "ignore"],
    });

    let stdout = "";

    // Collect stdout
    proc.stdout.on("data", (data) => {
      stdout += data.toString();
    });

    // Handle process completion
    proc.on("close", (code) => {
      const elapsed = Date.now() - startTime;

      if (config.logLevel === "debug") {
        console.error(`[cupcake] DEBUG: Cupcake response (${elapsed}ms):`, stdout);
      }

      // Check exit code
      if (code !== 0) {
        const error = new Error(`Cupcake exited with code ${code}`);

        if (config.failMode === "open") {
          console.error(`[cupcake] ERROR: ${error.message}`);
          console.error(`[cupcake] WARN: Allowing operation in fail-open mode.`);
          resolve({ decision: "allow" });
          return;
        }

        reject(error);
        return;
      }

      // Parse JSON response
      try {
        const response: CupcakeResponse = JSON.parse(stdout);

        // Only log at debug level - info logging is too noisy for TUI
        if (config.logLevel === "debug") {
          console.error(`[cupcake] DEBUG: Decision: ${response.decision} (${elapsed}ms)`);
        }

        resolve(response);
      } catch (parseError: any) {
        const error = new Error(
          `Failed to parse cupcake response: ${parseError.message}\nOutput: ${stdout}`,
        );

        if (config.failMode === "open") {
          console.error(`[cupcake] ERROR: ${error.message}`);
          console.error(`[cupcake] WARN: Allowing operation in fail-open mode.`);
          resolve({ decision: "allow" });
          return;
        }

        reject(error);
      }
    });

    // Handle process errors
    proc.on("error", (error) => {
      const executionError = new Error(`Failed to execute cupcake: ${error.message}`);

      if (config.failMode === "open") {
        console.error(`[cupcake] ERROR: ${executionError.message}`);
        console.error(`[cupcake] WARN: Allowing operation in fail-open mode.`);
        resolve({ decision: "allow" });
        return;
      }

      reject(executionError);
    });

    // Set up timeout
    const timeoutId = setTimeout(() => {
      proc.kill();

      const timeoutError = new Error(
        `Policy evaluation timed out after ${config.timeoutMs}ms. ` +
          `Consider optimizing policies or increasing timeout.`,
      );

      if (config.failMode === "open") {
        console.error(`[cupcake] WARN: ${timeoutError.message}`);
        console.error(`[cupcake] WARN: Allowing operation in fail-open mode.`);
        resolve({ decision: "allow" });
        return;
      }

      reject(timeoutError);
    }, config.timeoutMs);

    // Write event to stdin and close it
    proc.stdin.write(eventJson);
    proc.stdin.end();

    // Clear timeout when process completes
    proc.on("close", () => clearTimeout(timeoutId));
  });
}
