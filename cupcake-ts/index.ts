/**
 * @eqtylab/cupcake - Policy enforcement for AI agents and automation tools
 *
 * This module provides TypeScript/Node.js bindings for the Cupcake policy engine,
 * enabling custom applications to embed policy evaluation for governance and safety.
 *
 * @example
 * ```typescript
 * import { Cupcake } from '@eqtylab/cupcake';
 *
 * const cupcake = new Cupcake();
 * await cupcake.init('.cupcake');
 *
 * const decision = await cupcake.evaluate({
 *   kind: 'shell',
 *   command: 'rm -rf /',
 *   user: 'agent'
 * });
 *
 * if (decision.decision === 'Deny') {
 *   console.error('Blocked:', decision.reason);
 * }
 * ```
 *
 * @packageDocumentation
 */

// Load the native module using platform-specific binary name
const nativeBinding = require('./cupcake-native.darwin-arm64.node');
const NativePolicyEngine = nativeBinding.PolicyEngine;

// Type definition for the native PolicyEngine class
interface NativePolicyEngineClass {
  new (path: string, harness?: string): NativePolicyEngineInstance;
}

interface NativePolicyEngineInstance {
  evaluateSync(input: string): string;
  evaluateAsync(input: string): Promise<string>;
  version(): string;
  isReady(): boolean;
}

import { ensureOpaInstalled } from './installer';

/**
 * Hook event input - generic object that your application defines
 *
 * Cupcake doesn't enforce a specific event schema for embedded use.
 * Your policies will evaluate whatever structure you provide.
 *
 * @example
 * ```typescript
 * const event: HookEvent = {
 *   kind: 'tool_use',
 *   tool: 'database_query',
 *   query: 'DELETE FROM users',
 *   user_id: 'agent-123'
 * };
 * ```
 */
export interface HookEvent {
  [key: string]: any;
}

/**
 * Policy decision returned from evaluation
 */
export interface Decision {
  /** Decision verb: Allow, Deny, Halt, or Ask */
  decision: 'Allow' | 'Deny' | 'Halt' | 'Ask';

  /** Human-readable reason for the decision */
  reason?: string;

  /** Additional context or guidance (for Allow decisions) */
  context?: string[];

  /** Question to ask user (for Ask decisions) */
  question?: string;

  /** Severity level (for warnings) */
  severity?: 'LOW' | 'MEDIUM' | 'HIGH' | 'CRITICAL';

  /** Rule ID that triggered this decision */
  rule_id?: string;

  /** Additional fields from policy evaluation */
  [key: string]: any;
}

/**
 * Cupcake error class for all engine-related errors
 */
export class CupcakeError extends Error {
  constructor(
    message: string,
    public readonly code: string,
    public readonly cause?: Error,
  ) {
    super(message);
    this.name = 'CupcakeError';
  }
}

/**
 * Main Cupcake class for policy evaluation
 *
 * This class wraps the native Rust engine and provides a TypeScript-friendly API.
 * You can create multiple instances to evaluate against different policy sets.
 *
 * @example
 * ```typescript
 * // Instance-based API (recommended for multiple policy sets)
 * const production = new Cupcake();
 * await production.init('./policies/production');
 *
 * const staging = new Cupcake();
 * await staging.init('./policies/staging');
 *
 * // Evaluate against different policy sets
 * await production.evaluate(event);
 * await staging.evaluate(event);
 * ```
 */
export class Cupcake {
  private engine?: NativePolicyEngineInstance;
  private initialized = false;

  /**
   * Initialize the Cupcake engine
   *
   * This method:
   * 1. Ensures OPA binary is installed (auto-downloads if needed)
   * 2. Loads and compiles policies from the specified directory
   * 3. Initializes the WASM runtime
   *
   * @param path - Path to project directory or .cupcake folder (default: '.cupcake')
   * @param harness - Harness type for policy namespace (default: 'claude')
   *
   * @throws {CupcakeError} If initialization fails
   *
   * @example
   * ```typescript
   * const cupcake = new Cupcake();
   * await cupcake.init('./my-policies', 'claude');
   * ```
   */
  async init(path: string = '.cupcake', harness: 'claude' | 'cursor' = 'claude'): Promise<void> {
    if (this.initialized) {
      throw new CupcakeError('Cupcake already initialized', 'ALREADY_INITIALIZED');
    }

    try {
      // Ensure OPA binary is available
      await ensureOpaInstalled();

      // Initialize native engine (runs in background thread to avoid blocking)
      this.engine = await new Promise((resolve, reject) => {
        try {
          const engine = new (NativePolicyEngine as unknown as NativePolicyEngineClass)(path, harness);
          resolve(engine);
        } catch (error) {
          reject(error);
        }
      });

      this.initialized = true;
    } catch (error) {
      const err = error as Error;
      throw new CupcakeError(`Failed to initialize Cupcake: ${err.message}`, 'INIT_FAILED', err);
    }
  }

  /**
   * Initialize the Cupcake engine synchronously (BLOCKS event loop)
   *
   * ⚠️  WARNING: This method blocks the Node.js event loop during initialization.
   * Only use this in:
   * - CLI scripts where blocking is acceptable
   * - Top-level startup code before server starts
   *
   * For web servers, use async `init()` instead.
   *
   * @param path - Path to project directory or .cupcake folder
   * @param harness - Harness type for policy namespace
   *
   * @throws {CupcakeError} If initialization fails
   */
  initSync(path: string = '.cupcake', harness: 'claude' | 'cursor' = 'claude'): void {
    if (this.initialized) {
      throw new CupcakeError('Cupcake already initialized', 'ALREADY_INITIALIZED');
    }

    try {
      // Note: OPA installation is async-only, must be done beforehand
      this.engine = new (NativePolicyEngine as unknown as NativePolicyEngineClass)(path, harness);
      this.initialized = true;
    } catch (error) {
      const err = error as Error;
      throw new CupcakeError(`Failed to initialize Cupcake: ${err.message}`, 'INIT_FAILED', err);
    }
  }

  /**
   * Asynchronously evaluate a hook event (RECOMMENDED, non-blocking)
   *
   * This method runs the policy evaluation on a background thread,
   * allowing the event loop to handle other requests concurrently.
   *
   * @param event - Hook event object (your application defines the structure)
   * @returns Promise resolving to the policy decision
   *
   * @throws {CupcakeError} If evaluation fails or engine not initialized
   *
   * @example
   * ```typescript
   * const decision = await cupcake.evaluate({
   *   kind: 'api_call',
   *   endpoint: '/admin/delete-user',
   *   user: 'agent-x'
   * });
   *
   * if (decision.decision === 'Deny') {
   *   throw new Error(`Policy blocked: ${decision.reason}`);
   * }
   * ```
   */
  async evaluate(event: HookEvent): Promise<Decision> {
    if (!this.engine) {
      throw new CupcakeError('Cupcake not initialized. Call init() first.', 'NOT_INITIALIZED');
    }

    try {
      const inputJson = JSON.stringify(event);
      const resultJson = await this.engine.evaluateAsync(inputJson);
      return JSON.parse(resultJson) as Decision;
    } catch (error) {
      const err = error as Error;
      throw new CupcakeError(`Policy evaluation failed: ${err.message}`, 'EVALUATION_FAILED', err);
    }
  }

  /**
   * Synchronously evaluate a hook event (BLOCKS event loop)
   *
   * ⚠️  WARNING: This method blocks the Node.js event loop until evaluation completes.
   * Only use this in:
   * - CLI scripts where blocking is acceptable
   * - Simple one-off evaluations
   *
   * For web servers or concurrent evaluations, use async `evaluate()` instead.
   *
   * @param event - Hook event object
   * @returns The policy decision
   *
   * @throws {CupcakeError} If evaluation fails or engine not initialized
   */
  evaluateSync(event: HookEvent): Decision {
    if (!this.engine) {
      throw new CupcakeError('Cupcake not initialized. Call initSync() first.', 'NOT_INITIALIZED');
    }

    try {
      const inputJson = JSON.stringify(event);
      const resultJson = this.engine.evaluateSync(inputJson);
      return JSON.parse(resultJson) as Decision;
    } catch (error) {
      const err = error as Error;
      throw new CupcakeError(`Policy evaluation failed: ${err.message}`, 'EVALUATION_FAILED', err);
    }
  }

  /**
   * Get the Cupcake version
   */
  get version(): string {
    if (!this.engine) {
      throw new CupcakeError('Cupcake not initialized', 'NOT_INITIALIZED');
    }
    return this.engine.version();
  }

  /**
   * Check if the engine is ready to evaluate policies
   */
  get isReady(): boolean {
    return this.engine?.isReady() ?? false;
  }
}

// Module-level singleton for convenience API
let defaultInstance: Cupcake | null = null;

/**
 * Initialize the default Cupcake instance
 *
 * Convenience function for module-level API. For multiple policy sets,
 * use the class-based API instead.
 *
 * @param path - Path to project directory or .cupcake folder
 * @param harness - Harness type for policy namespace
 *
 * @example
 * ```typescript
 * import { init, evaluate } from '@eqtylab/cupcake';
 *
 * await init('.cupcake');
 * const decision = await evaluate(event);
 * ```
 */
export async function init(path?: string, harness?: 'claude' | 'cursor'): Promise<void> {
  defaultInstance = new Cupcake();
  await defaultInstance.init(path, harness);
}

/**
 * Initialize the default Cupcake instance synchronously (BLOCKS event loop)
 *
 * ⚠️  Use async `init()` instead for servers.
 */
export function initSync(path?: string, harness?: 'claude' | 'cursor'): void {
  defaultInstance = new Cupcake();
  defaultInstance.initSync(path, harness);
}

/**
 * Evaluate an event using the default instance (async, non-blocking)
 *
 * @param event - Hook event object
 * @returns Promise resolving to the policy decision
 */
export async function evaluate(event: HookEvent): Promise<Decision> {
  if (!defaultInstance) {
    throw new CupcakeError('Cupcake not initialized. Call init() first.', 'NOT_INITIALIZED');
  }
  return defaultInstance.evaluate(event);
}

/**
 * Evaluate an event using the default instance (sync, BLOCKS event loop)
 *
 * ⚠️  Use async `evaluate()` instead for servers.
 */
export function evaluateSync(event: HookEvent): Decision {
  if (!defaultInstance) {
    throw new CupcakeError('Cupcake not initialized. Call initSync() first.', 'NOT_INITIALIZED');
  }
  return defaultInstance.evaluateSync(event);
}

/**
 * Get the Cupcake version
 */
export function version(): string {
  if (!defaultInstance) {
    throw new CupcakeError('Cupcake not initialized', 'NOT_INITIALIZED');
  }
  return defaultInstance.version;
}

/**
 * Check if the default instance is ready
 */
export function isReady(): boolean {
  return defaultInstance?.isReady ?? false;
}

// Re-export the native engine class for advanced usage
export const PolicyEngine = NativePolicyEngine as unknown as NativePolicyEngineClass;
