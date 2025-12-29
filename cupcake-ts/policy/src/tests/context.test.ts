import { expect, test } from 'vitest';
import { policy, addContext, compile } from './stub';

test('addContext unconditional (no event check)', () => {
  const p = policy('unconditional context', addContext('Always be helpful.'));
  const compiled = compile(p);
  expect(compiled).not.toContain('input.hook_event_name');
  expect(compiled).toContain('ctx := "Always be helpful."');
});

test('addContext with .on(SessionStart)', () => {
  const p = policy('session context', addContext('Remember the style guide...').on('SessionStart'));
  expect(compile(p)).toBe(
    `
package cupcake.policies.session_context

import rego.v1

add_context contains ctx if {
    input.hook_event_name == "SessionStart"
    ctx := "Remember the style guide..."
}
`.trim(),
  );
});

test('addContext with .on(UserPromptSubmit)', () => {
  const p = policy('prompt context', addContext('Be concise.').on('UserPromptSubmit'));
  const compiled = compile(p);
  expect(compiled).toContain('input.hook_event_name == "UserPromptSubmit"');
  expect(compiled).toContain('ctx := "Be concise."');
});

test('addContext with .on(PostToolUse)', () => {
  const p = policy('post tool context', addContext('Check the output carefully.').on('PostToolUse'));
  const compiled = compile(p);
  expect(compiled).toContain('input.hook_event_name == "PostToolUse"');
  expect(compiled).toContain('ctx := "Check the output carefully."');
});

test('addContext with .on() and .when()', () => {
  const p = policy(
    'conditional context',
    addContext('Use Python style.')
      .on('UserPromptSubmit')
      .when(({ userPrompt }) => [userPrompt.contains('python')]),
  );
  const compiled = compile(p);
  expect(compiled).toContain('input.hook_event_name == "UserPromptSubmit"');
  expect(compiled).toContain('contains(input.user_prompt, "python")');
  expect(compiled).toContain('ctx := "Use Python style."');
});

test('addContext with .when() only (no .on())', () => {
  const p = policy(
    'when only',
    addContext('Pirate mode.').when(({ hookEventName }) => [hookEventName.equals('UserPromptSubmit')]),
  );
  const compiled = compile(p);
  // Should have event check from condition, not from .on()
  expect(compiled).toContain('input.hook_event_name == "UserPromptSubmit"');
  expect(compiled).toContain('ctx := "Pirate mode."');
});
