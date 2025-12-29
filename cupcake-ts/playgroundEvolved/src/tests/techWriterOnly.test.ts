import { expect, test } from 'vitest';
import { policy, canOnly, compile } from './stub';

test('caitlin policy compiles correctly', () => {
  const caitlin = policy(
    'technical writer',
    canOnly('access blog files', ['Read', 'Write']).when(({ path }) => [path.contains('src/blog')]),
    canOnly('push to origin', 'Bash').when(({ command }) => [command.equals('git push origin')]),
  );

  expect(compile(caitlin)).toBe(
    `
package cupcake.policies.technical_writer

import rego.v1

# Default deny — generated because canOnly exists
deny contains decision if {
    not allow
    decision := {
        "rule_id": "default_deny",
        "reason": "Action not permitted",
        "severity": "MEDIUM"
    }
}

# Allow rules from canOnly

allow if {
    input.hook_event_name == "PreToolUse"
    input.tool_name in {"Read", "Write"}
    contains(input.tool_input.file_path, "src/blog")
}

allow if {
    input.hook_event_name == "PreToolUse"
    input.tool_name == "Bash"
    input.tool_input.command == "git push origin"
}
`.trim(),
  );
});
