import { expect, test } from 'vitest';
import { policy, cant, compile, reason, defineConstant } from './stub';

test('symlink detection policy compiles correctly', () => {
  const dangerousDirectories = defineConstant('dangerous_directories', [
    '/etc/',
    '/bin/',
    '/sbin/',
    '/usr/',
    '/System/',
    '/Library/',
    '.cupcake/',
    '.git/',
  ]);

  const symlinkDetection = policy(
    'symlink detection',
    cant('symlink to dangerous location', ['Write', 'Edit', 'Read'])
      .severity('HIGH')
      .ruleId('SYMLINK-001')
      .when(({ isSymlink, resolvedFilePath }) => [
        isSymlink.equals(true),
        resolvedFilePath.lower().containsAny(dangerousDirectories),
      ])
      .reason(
        ({ originalFilePath, resolvedFilePath }) =>
          reason`Symlink detected: '${originalFilePath}' resolves to protected location '${resolvedFilePath}'`,
      ),
  );

  expect(compile(symlinkDetection)).toBe(
    `
package cupcake.policies.symlink_detection

import rego.v1

dangerous_directories := [
    "/etc/",
    "/bin/",
    "/sbin/",
    "/usr/",
    "/System/",
    "/Library/",
    ".cupcake/",
    ".git/",
]

deny contains decision if {
    input.hook_event_name == "PreToolUse"
    input.tool_name in {"Write", "Edit", "Read"}
    input.is_symlink == true
    resolved_path := input.resolved_file_path

    some dangerous in dangerous_directories
    contains(lower(resolved_path), lower(dangerous))

    decision := {
        "rule_id": "SYMLINK-001",
        "reason": concat("", ["Symlink detected: \\'", input.original_file_path, "\\' resolves to protected location \\'", resolved_path, "\\'"]),
        "severity": "HIGH"
    }
}
`.trim(),
  );
});
