import { expect, test } from 'vitest';
import { policy, mustBlock, compile } from './stub';

test('mustBlock on PostToolUse compiles correctly', () => {
  const failedCommandPolicy = policy(
    'failed command handler',
    mustBlock('failed command', 'Bash')
      .on('PostToolUse')
      .severity('HIGH')
      .ruleId('BLK-001')
      .when(({ toolResponse }) => [toolResponse.exitCode.notEquals(0)])
      .reason('Command failed with non-zero exit code')
      .build()
  );

  expect(compile(failedCommandPolicy)).toBe(
    `
package cupcake.policies.failed_command_handler

import rego.v1

block contains decision if {
    input.hook_event_name == "PostToolUse"
    input.tool_name == "Bash"
    input.tool_response.exit_code != 0

    decision := {
        "rule_id": "BLK-001",
        "reason": "Command failed with non-zero exit code",
        "severity": "HIGH"
    }
}
`.trim()
  );
});

test('mustBlock on PostToolUse with stderr check', () => {
  const errorPolicy = policy(
    'error detector',
    mustBlock('stderr error', 'Bash')
      .on('PostToolUse')
      .when(({ toolResponse }) => [toolResponse.stderr.contains('error')])
      .build()
  );

  expect(compile(errorPolicy)).toBe(
    `
package cupcake.policies.error_detector

import rego.v1

block contains decision if {
    input.hook_event_name == "PostToolUse"
    input.tool_name == "Bash"
    contains(input.tool_response.stderr, "error")

    decision := {
        "rule_id": "stderr_error",
        "reason": "stderr error",
        "severity": "MEDIUM"
    }
}
`.trim()
  );
});

test('mustBlock on UserPromptSubmit compiles correctly', () => {
  const unsafePromptPolicy = policy(
    'prompt blocker',
    mustBlock('unsafe prompt')
      .on('UserPromptSubmit')
      .severity('HIGH')
      .when(({ submittedPrompt }) => [submittedPrompt.contains('UNSAFE')])
      .reason('Blocked unsafe prompt content')
      .build()
  );

  expect(compile(unsafePromptPolicy)).toBe(
    `
package cupcake.policies.prompt_blocker

import rego.v1

block contains decision if {
    input.hook_event_name == "UserPromptSubmit"
    contains(input.prompt, "UNSAFE")

    decision := {
        "rule_id": "unsafe_prompt",
        "reason": "Blocked unsafe prompt content",
        "severity": "HIGH"
    }
}
`.trim()
  );
});

test('mustBlock on Stop compiles correctly', () => {
  const forceContPolicy = policy(
    'force continue',
    mustBlock('not done yet')
      .on('Stop')
      .when(({ stopHookActive }) => [stopHookActive.equals(false)])
      .reason('Task is not complete, continue working')
      .build()
  );

  expect(compile(forceContPolicy)).toBe(
    `
package cupcake.policies.force_continue

import rego.v1

block contains decision if {
    input.hook_event_name == "Stop"
    input.stop_hook_active == false

    decision := {
        "rule_id": "not_done_yet",
        "reason": "Task is not complete, continue working",
        "severity": "MEDIUM"
    }
}
`.trim()
  );
});

test('mustBlock on SubagentStop compiles correctly', () => {
  const subagentPolicy = policy(
    'subagent handler',
    mustBlock('subagent incomplete')
      .on('SubagentStop')
      .when(({ stopHookActive }) => [stopHookActive.equals(false)])
      .reason('Subagent task not complete')
      .build()
  );

  expect(compile(subagentPolicy)).toBe(
    `
package cupcake.policies.subagent_handler

import rego.v1

block contains decision if {
    input.hook_event_name == "SubagentStop"
    input.stop_hook_active == false

    decision := {
        "rule_id": "subagent_incomplete",
        "reason": "Subagent task not complete",
        "severity": "MEDIUM"
    }
}
`.trim()
  );
});

test('mustBlock on PostToolUse with multiple tools', () => {
  const multiToolPolicy = policy(
    'write failure handler',
    mustBlock('file write failed', ['Write', 'Edit'])
      .on('PostToolUse')
      .when(({ toolResponse }) => [toolResponse.stderr.contains('Permission denied')])
      .reason('File operation failed due to permissions')
      .build()
  );

  expect(compile(multiToolPolicy)).toBe(
    `
package cupcake.policies.write_failure_handler

import rego.v1

block contains decision if {
    input.hook_event_name == "PostToolUse"
    input.tool_name in {"Write", "Edit"}
    contains(input.tool_response.stderr, "Permission denied")

    decision := {
        "rule_id": "file_write_failed",
        "reason": "File operation failed due to permissions",
        "severity": "MEDIUM"
    }
}
`.trim()
  );
});
