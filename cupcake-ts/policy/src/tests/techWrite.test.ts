import { expect, test } from 'vitest';
import { policy, cant, addContext, compile, defineSignal } from './stub';

const simulateGitBranch = (): Promise<string> => {
  return Promise.resolve('main');
};

test('caitlin policy compiles correctly', () => {
  const gitBranch = defineSignal('gitBranch', async () => {
    const result = await simulateGitBranch();
    return result.trim();
  });

  const caitlin = policy(
    'technical writer',
    cant('write to src', ['Write', 'Edit', 'Bash']).when(({ path }) => [path.contains('src/')]),
    cant('push to main', 'Bash').when(({ command }) => [command.contains('git push'), gitBranch.equals('main')]),
    addContext('This is how we write our blogs...'),
  );

  expect(compile(caitlin)).toBe(
    `
package cupcake.policies.technical_writer

import rego.v1

deny contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name in {"Write", "Edit", "Bash"}
    contains(input.tool_input.file_path, "src/")

    decision := {
        "rule_id": "write_to_src",
        "reason": "write to src",
        "severity": "MEDIUM"
    }
}

deny contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Bash"
    contains(input.tool_input.command, "git push")
    input.signals.gitBranch == "main"

    decision := {
        "rule_id": "push_to_main",
        "reason": "push to main",
        "severity": "MEDIUM"
    }
}

add_context contains ctx if {
    ctx := "This is how we write our blogs..."
}
`.trim(),
  );
});
