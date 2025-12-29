import { expect, test } from 'vitest';
import { policy, addContext, compile } from './stub';

test('pirate policy compiles correctly', () => {
  const pirateMode = policy(
    'pirate mode',
    addContext('Always talk like a pirate. Use "arr", "matey", and "avast" in responses.').when(({ hookEventName }) => [
      hookEventName.equals('UserPromptSubmit'),
    ]),
  );

  expect(compile(pirateMode)).toBe(
    `
package cupcake.policies.pirate_mode

import rego.v1

add_context contains ctx if {
    input.hook_event_name == "UserPromptSubmit"
    ctx := "Always talk like a pirate. Use 'arr', 'matey', and 'avast' in responses."
}
`.trim(),
  );
});
