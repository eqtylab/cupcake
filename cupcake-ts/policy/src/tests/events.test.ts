import { expect, test } from 'vitest';
import { policy, cant, mustHalt, mustModify, compile } from './stub';

test('cant with .on(PermissionRequest) compiles correctly', () => {
  const permPolicy = policy(
    'permission deny',
    cant('dangerous permission', 'Bash')
      .on('PermissionRequest')
      .severity('HIGH')
      .when(({ command }) => [command.contains('rm -rf')])
      .reason('Dangerous command blocked at permission dialog')
  );

  expect(compile(permPolicy)).toBe(
    `
package cupcake.policies.permission_deny

import rego.v1

deny contains decision if {
    input.hook_event_name == "PermissionRequest"
    input.tool_name == "Bash"
    contains(input.tool_input.command, "rm -rf")

    decision := {
        "rule_id": "dangerous_permission",
        "reason": "Dangerous command blocked at permission dialog",
        "severity": "HIGH"
    }
}
`.trim()
  );
});

test('cant defaults to PreToolUse event', () => {
  const defaultPolicy = policy(
    'default event',
    cant('default deny', 'Bash')
      .when(({ command }) => [command.contains('sudo')])
  );

  expect(compile(defaultPolicy)).toContain('input.hook_event_name == "PreToolUse"');
});

test('mustHalt with .on(PostToolUse) compiles correctly', () => {
  const postPolicy = policy(
    'post halt',
    mustHalt('fatal error', 'Bash')
      .on('PostToolUse')
      .severity('CRITICAL')
      .when(({ toolResponse }) => [toolResponse.stderr.contains('fatal')])
      .reason('Fatal error detected in output')
      .build()
  );

  expect(compile(postPolicy)).toBe(
    `
package cupcake.policies.post_halt

import rego.v1

halt contains decision if {
    input.hook_event_name == "PostToolUse"
    input.tool_name == "Bash"
    contains(input.tool_response.stderr, "fatal")

    decision := {
        "rule_id": "fatal_error",
        "reason": "Fatal error detected in output",
        "severity": "CRITICAL"
    }
}
`.trim()
  );
});

test('mustHalt with .on(PermissionRequest) compiles correctly', () => {
  const permPolicy = policy(
    'permission halt',
    mustHalt('dangerous permission', 'Bash')
      .on('PermissionRequest')
      .when(({ command }) => [command.contains('rm -rf /')])
      .build()
  );

  expect(compile(permPolicy)).toContain('input.hook_event_name == "PermissionRequest"');
});

test('mustHalt defaults to PreToolUse event', () => {
  const defaultPolicy = policy(
    'default halt',
    mustHalt('default', 'Bash')
      .when(({ command }) => [command.contains('shutdown')])
      .build()
  );

  expect(compile(defaultPolicy)).toContain('input.hook_event_name == "PreToolUse"');
});

test('mustModify with .on(PermissionRequest) compiles correctly', () => {
  const permPolicy = policy(
    'permission modify',
    mustModify('add timeout', 'Bash')
      .on('PermissionRequest')
      .when(({ command }) => [command.contains('npm')])
      .transform(() => ({ timeout: 120000 }))
      .build()
  );

  expect(compile(permPolicy)).toBe(
    `
package cupcake.policies.permission_modify

import rego.v1

modify contains modification if {
    input.hook_event_name == "PermissionRequest"
    input.tool_name == "Bash"
    contains(input.tool_input.command, "npm")

    modification := {
        "rule_id": "add_timeout",
        "reason": "add timeout",
        "priority": 50,
        "severity": "MEDIUM",
        "updated_input": {"timeout": 120000}
    }
}
`.trim()
  );
});

test('mustModify defaults to PreToolUse event', () => {
  const defaultPolicy = policy(
    'default modify',
    mustModify('default', 'Bash')
      .when(({ command }) => [command.contains('curl')])
      .transform(() => ({ timeout: 30000 }))
      .build()
  );

  expect(compile(defaultPolicy)).toContain('input.hook_event_name == "PreToolUse"');
});

test('mixed policy with different events', () => {
  const mixedPolicy = policy(
    'mixed events',
    // Default PreToolUse
    cant('block sudo', 'Bash')
      .when(({ command }) => [command.contains('sudo')]),
    // Explicit PermissionRequest
    cant('block rm at permission', 'Bash')
      .on('PermissionRequest')
      .when(({ command }) => [command.contains('rm -rf')])
  );

  const compiled = compile(mixedPolicy);

  // First rule uses PreToolUse
  expect(compiled).toContain('input.hook_event_name == "PreToolUse"');
  // Second rule uses PermissionRequest
  expect(compiled).toContain('input.hook_event_name == "PermissionRequest"');
});
