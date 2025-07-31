# Plan 022: Remove String Command Mode

Created: 2025-01-31T10:00:00Z
Depends: none
Enables: Simpler, more secure command execution model

## Goal

Remove the string command execution mode from Cupcake, leaving only the secure array mode and the explicit shell mode.

## Success Criteria

- String command mode completely removed from codebase
- All string mode tests removed or converted to array mode
- Documentation updated to reflect only array and shell modes
- Migration guide provided for existing string mode users
- No regression in array or shell mode functionality

## Context

The string command mode was introduced in plan-008 as a middle ground between the secure array mode and the potentially dangerous shell mode. It attempted to provide shell-like convenience while avoiding actual shell execution through custom parsing.

However, security analysis reveals fundamental flaws:
- Incomplete quote handling allows operators inside quotes to be misinterpreted
- Template substitution lacks proper escaping
- Blacklist approach misses many dangerous constructs
- Attempting to parse shell syntax without using a shell creates subtle vulnerabilities

The string mode provides a false sense of security. Users who need shell features should explicitly opt into shell mode with full awareness of risks. Users who want security should use array mode.

## Rationale for Removal

1. **Security Theatre**: String mode appears safe but has exploitable vulnerabilities
2. **Complexity Without Benefit**: Significant code complexity for a fundamentally flawed approach  
3. **Clear Security Model**: Having only array (secure) and shell (explicit risk) makes security posture obvious
4. **Maintenance Burden**: String parser requires ongoing security patches as new exploits are discovered

## Impact

- **Breaking Change**: Existing policies using string mode will need migration
- **Clearer Mental Model**: Two distinct modes instead of three
- **Reduced Attack Surface**: Removes ~800 lines of parser code
- **Better User Understanding**: No confusion about which mode provides what guarantees