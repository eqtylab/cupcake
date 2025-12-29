import { expect, test } from 'vitest';
import { policy, mustModify, compile, reason } from './stub';

test('mustModify compiles to modify contains modification with updated_input', () => {
  const sanitizePolicy = policy(
    'sanitize paths',
    mustModify('path traversal fix', 'Write')
      .priority(80)
      .severity('HIGH')
      .ruleId('MOD-001')
      .when(({ path }) => [path.contains('../')])
      .reason('Path traversal detected, using resolved path')
      .transform(({ resolvedFilePath }) => ({ path: resolvedFilePath }))
      .build(),
  );

  expect(compile(sanitizePolicy)).toBe(
    `
package cupcake.policies.sanitize_paths

import rego.v1

modify contains modification if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Write"
    contains(input.tool_input.file_path, "../")

    modification := {
        "rule_id": "MOD-001",
        "reason": "Path traversal detected, using resolved path",
        "priority": 80,
        "severity": "HIGH",
        "updated_input": {"path": input.resolved_file_path}
    }
}
`.trim(),
  );
});

test('mustModify uses defaults for priority and severity', () => {
  const modifyPolicy = policy(
    'normalize paths',
    mustModify('strip prefix', 'Read')
      .when(({ path }) => [path.startsWith('/tmp/')])
      .transform(({ resolvedFilePath }) => ({ file_path: resolvedFilePath }))
      .build(),
  );

  expect(compile(modifyPolicy)).toBe(
    `
package cupcake.policies.normalize_paths

import rego.v1

modify contains modification if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Read"
    startswith(input.tool_input.file_path, "/tmp/")

    modification := {
        "rule_id": "strip_prefix",
        "reason": "strip prefix",
        "priority": 50,
        "severity": "MEDIUM",
        "updated_input": {"file_path": input.resolved_file_path}
    }
}
`.trim(),
  );
});

test('mustModify with static value in transform', () => {
  const forceTimeoutPolicy = policy(
    'force timeout',
    mustModify('add timeout to bash', 'Bash')
      .priority(90)
      .when(({ command }) => [command.contains('curl')])
      .reason('Adding timeout to curl command')
      .transform(() => ({ timeout: 30000 }))
      .build(),
  );

  expect(compile(forceTimeoutPolicy)).toBe(
    `
package cupcake.policies.force_timeout

import rego.v1

modify contains modification if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Bash"
    contains(input.tool_input.command, "curl")

    modification := {
        "rule_id": "add_timeout_to_bash",
        "reason": "Adding timeout to curl command",
        "priority": 90,
        "severity": "MEDIUM",
        "updated_input": {"timeout": 30000}
    }
}
`.trim(),
  );
});

test('mustModify with dynamic reason', () => {
  const dynamicPolicy = policy(
    'dynamic modify',
    mustModify('sanitize command', 'Bash')
      .when(({ command }) => [command.contains('sudo')])
      .reason(({ command }) => reason`Sanitizing command: ${command}`)
      .transform(({ command }) => ({ command: command }))
      .build(),
  );

  expect(compile(dynamicPolicy)).toBe(
    `
package cupcake.policies.dynamic_modify

import rego.v1

modify contains modification if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Bash"
    contains(input.tool_input.command, "sudo")

    modification := {
        "rule_id": "sanitize_command",
        "reason": concat("", ["Sanitizing command: ", input.tool_input.command]),
        "priority": 50,
        "severity": "MEDIUM",
        "updated_input": {"command": input.tool_input.command}
    }
}
`.trim(),
  );
});
