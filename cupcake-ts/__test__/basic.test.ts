/**
 * Basic tests for Cupcake TypeScript bindings
 *
 * Note: These tests use the test-fixtures/.cupcake directory
 */

import { Cupcake, CupcakeError } from '../index';
import * as path from 'path';

const TEST_CUPCAKE_DIR = path.join(__dirname, '..', 'test-fixtures', '.cupcake');

describe('Cupcake TypeScript Bindings', () => {
  describe('Initialization', () => {
    it('should initialize successfully', async () => {
      const cupcake = new Cupcake();
      await cupcake.init(TEST_CUPCAKE_DIR);

      expect(cupcake.isReady).toBe(true);
      expect(cupcake.version).toBeTruthy();
    });

    it('should throw error when initializing twice', async () => {
      const cupcake = new Cupcake();
      await cupcake.init(TEST_CUPCAKE_DIR);

      await expect(cupcake.init(TEST_CUPCAKE_DIR)).rejects.toThrow(CupcakeError);
      await expect(cupcake.init(TEST_CUPCAKE_DIR)).rejects.toThrow('already initialized');
    });

    it('should initialize synchronously', () => {
      const cupcake = new Cupcake();
      cupcake.initSync(TEST_CUPCAKE_DIR);

      expect(cupcake.isReady).toBe(true);
    });

    it('should throw error for invalid path', async () => {
      const cupcake = new Cupcake();

      await expect(cupcake.init('/nonexistent/path')).rejects.toThrow(CupcakeError);
    });
  });

  describe('Evaluation', () => {
    let cupcake: Cupcake;

    beforeAll(async () => {
      cupcake = new Cupcake();
      await cupcake.init(TEST_CUPCAKE_DIR);
    });

    it('should evaluate an event asynchronously', async () => {
      const event = {
        hookEventName: 'PreToolUse',
        tool_name: 'Bash',
        command: 'ls',
        args: ['-la'],
      };

      const decision = await cupcake.evaluate(event);

      // Check if decision has the expected structure (OPA output format)
      expect(decision).toBeTruthy();
      // OPA returns {"Allow": {...}} or {"Deny": {...}} format
      const hasDecision = decision.Allow || decision.Deny || decision.Halt || decision.Ask;
      expect(hasDecision).toBeDefined();
    });

    it('should evaluate an event synchronously', () => {
      const event = {
        hookEventName: 'PreToolUse',
        tool_name: 'Bash',
        command: 'ls',
        args: ['-la'],
      };

      const decision = cupcake.evaluateSync(event);

      // Check if decision has the expected structure (OPA output format)
      expect(decision).toBeTruthy();
      const hasDecision = decision.Allow || decision.Deny || decision.Halt || decision.Ask;
      expect(hasDecision).toBeDefined();
    });

    it('should handle custom event structures', async () => {
      const event = {
        hookEventName: 'UserPromptSubmit',
        prompt: 'test prompt',
        user: 'test-user',
      };

      const decision = await cupcake.evaluate(event);

      // Check if decision has the expected structure (OPA output format)
      expect(decision).toBeTruthy();
      const hasDecision = decision.Allow || decision.Deny || decision.Halt || decision.Ask;
      expect(hasDecision).toBeDefined();
    });

    it('should throw error when evaluating before initialization', async () => {
      const uninitializedCupcake = new Cupcake();
      const event = { hookEventName: 'PreToolUse', tool_name: 'Bash' };

      await expect(uninitializedCupcake.evaluate(event)).rejects.toThrow(CupcakeError);
      await expect(uninitializedCupcake.evaluate(event)).rejects.toThrow('not initialized');
    });

    it('should handle concurrent evaluations', async () => {
      const events = Array.from({ length: 10 }, (_, i) => ({
        hookEventName: 'PreToolUse',
        tool_name: 'Bash',
        command: 'echo',
        args: [`test-${i}`],
      }));

      const decisions = await Promise.all(events.map((event) => cupcake.evaluate(event)));

      expect(decisions).toHaveLength(10);
      decisions.forEach((decision) => {
        expect(decision).toBeTruthy();
        const hasDecision = decision.Allow || decision.Deny || decision.Halt || decision.Ask;
        expect(hasDecision).toBeDefined();
      });
    });
  });

  describe('Module-level API', () => {
    it('should work with module-level functions', async () => {
      const { init, evaluate, version, isReady } = await import('../index');

      await init(TEST_CUPCAKE_DIR);
      expect(isReady()).toBe(true);
      expect(version()).toBeTruthy();

      const decision = await evaluate({
        hookEventName: 'PreToolUse',
        tool_name: 'Bash',
        command: 'test',
      });
      expect(decision).toBeTruthy();
      const hasDecision = decision.Allow || decision.Deny || decision.Halt || decision.Ask;
      expect(hasDecision).toBeDefined();
    });
  });

  describe('Error Handling', () => {
    it('should provide detailed error information', async () => {
      const cupcake = new Cupcake();

      try {
        await cupcake.init('/invalid/path');
        fail('Should have thrown an error');
      } catch (error) {
        expect(error).toBeInstanceOf(CupcakeError);
        const cupcakeError = error as CupcakeError;
        expect(cupcakeError.code).toBeTruthy();
        expect(cupcakeError.message).toBeTruthy();
      }
    });
  });

  describe('Version Info', () => {
    it('should return version string', async () => {
      const cupcake = new Cupcake();
      await cupcake.init(TEST_CUPCAKE_DIR);

      const version = cupcake.version;
      expect(typeof version).toBe('string');
      expect(version.length).toBeGreaterThan(0);
    });
  });
});
