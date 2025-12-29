import { expect, test } from 'vitest';
import { policy, mustAsk, compile, reason } from './stub';

test('mustAsk compiles to ask contains decision with question', () => {
  const sudoPolicy = policy(
    'sudo confirmation',
    mustAsk('sudo commands', 'Bash')
      .severity('HIGH')
      .ruleId('ASK-001')
      .when(({ command }) => [command.contains('sudo')])
      .reason('Sudo command detected')
      .question('This command requires elevated privileges. Continue?')
      .build(),
  );

  expect(compile(sudoPolicy)).toBe(
    `
package cupcake.policies.sudo_confirmation

import rego.v1

ask contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Bash"
    contains(input.tool_input.command, "sudo")

    decision := {
        "rule_id": "ASK-001",
        "reason": "Sudo command detected",
        "question": "This command requires elevated privileges. Continue?",
        "severity": "HIGH"
    }
}
`.trim(),
  );
});

test('mustAsk with dynamic question', () => {
  const writePolicy = policy(
    'write confirmation',
    mustAsk('write to important path', 'Write')
      .severity('MEDIUM')
      .when(({ path }) => [path.contains('/important/')])
      .question(({ path }) => reason`Are you sure you want to write to ${path}?`)
      .build(),
  );

  expect(compile(writePolicy)).toBe(
    `
package cupcake.policies.write_confirmation

import rego.v1

ask contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Write"
    contains(input.tool_input.file_path, "/important/")

    decision := {
        "rule_id": "write_to_important_path",
        "reason": "write to important path",
        "question": concat("", ["Are you sure you want to write to ", input.tool_input.file_path, "?"]),
        "severity": "MEDIUM"
    }
}
`.trim(),
  );
});

test('mustAsk without reason uses rule name as reason', () => {
  const askPolicy = policy(
    'ask test',
    mustAsk('confirm operation', 'Bash')
      .severity('LOW')
      .when(({ command }) => [command.startsWith('npm')])
      .question('Run this npm command?')
      .build(),
  );

  expect(compile(askPolicy)).toBe(
    `
package cupcake.policies.ask_test

import rego.v1

ask contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Bash"
    startswith(input.tool_input.command, "npm")

    decision := {
        "rule_id": "confirm_operation",
        "reason": "confirm operation",
        "question": "Run this npm command?",
        "severity": "LOW"
    }
}
`.trim(),
  );
});
