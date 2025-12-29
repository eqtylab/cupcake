import { expect, test } from 'vitest';
import { policy, mustHalt, compile } from './stub';

test('mustHalt compiles to halt contains decision', () => {
  const criticalPathPolicy = policy(
    'critical path halt',
    mustHalt('critical system path', ['Write', 'Edit'])
      .severity('CRITICAL')
      .ruleId('HALT-001')
      .when(({ resolvedFilePath }) => [resolvedFilePath.startsWith('/etc/')])
      .reason('Attempting to modify critical system files')
      .build(),
  );

  expect(compile(criticalPathPolicy)).toBe(
    `
package cupcake.policies.critical_path_halt

import rego.v1

halt contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name in {"Write", "Edit"}
    startswith(input.resolved_file_path, "/etc/")

    decision := {
        "rule_id": "HALT-001",
        "reason": "Attempting to modify critical system files",
        "severity": "CRITICAL"
    }
}
`.trim(),
  );
});

test('mustHalt with default reason uses rule name', () => {
  const haltPolicy = policy(
    'halt test',
    mustHalt('dangerous delete', 'Bash')
      .severity('HIGH')
      .when(({ command }) => [command.contains('rm -rf /')])
      .build(),
  );

  expect(compile(haltPolicy)).toBe(
    `
package cupcake.policies.halt_test

import rego.v1

halt contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Bash"
    contains(input.tool_input.command, "rm -rf /")

    decision := {
        "rule_id": "dangerous_delete",
        "reason": "dangerous delete",
        "severity": "HIGH"
    }
}
`.trim(),
  );
});

test('mustHalt tool-less on UserPromptSubmit', () => {
  const haltPolicy = policy(
    'halt prompt',
    mustHalt('dangerous prompt')
      .on('UserPromptSubmit')
      .severity('CRITICAL')
      .when(({ submittedPrompt }) => [submittedPrompt.contains('DANGER')])
      .reason('Dangerous prompt detected')
      .build(),
  );

  const compiled = compile(haltPolicy);
  expect(compiled).toContain('input.hook_event_name == "UserPromptSubmit"');
  expect(compiled).not.toContain('input.tool_name');
  expect(compiled).toContain('contains(input.prompt, "DANGER")');
  expect(compiled).toContain('"reason": "Dangerous prompt detected"');
});

test('mustHalt tool-less on Stop', () => {
  const haltPolicy = policy(
    'halt stop',
    mustHalt('force halt')
      .on('Stop')
      .when(({ stopHookActive }) => [stopHookActive.equals(true)])
      .build(),
  );

  const compiled = compile(haltPolicy);
  expect(compiled).toContain('input.hook_event_name == "Stop"');
  expect(compiled).not.toContain('input.tool_name');
  expect(compiled).toContain('input.stop_hook_active == true');
});

test('mustHalt tool-less on SessionStart', () => {
  const haltPolicy = policy(
    'halt session',
    mustHalt('block session')
      .on('SessionStart')
      .severity('HIGH')
      .when(({ sessionId }) => [sessionId.contains('blocked')])
      .build(),
  );

  const compiled = compile(haltPolicy);
  expect(compiled).toContain('input.hook_event_name == "SessionStart"');
  expect(compiled).not.toContain('input.tool_name');
  expect(compiled).toContain('contains(input.session_id, "blocked")');
});
