Here’s the cleaned-up **final design document** (ready to paste into your repo / Confluence page).

# Cupcake Execution Model — K8s-Familiar · Shell-Free · Enterprise-Safe

This spec locks down syntax, execution flow, and security guarantees for **v 1.0**.
We keep Kubernetes-style arrays for familiarity, add seven _tiny_ operator keys for
composition, and expose two opt-in shortcuts (`string:` and `shell:`) so authors can
work the way they like without re-introducing shell-injection risk.

---

## Step 0 Three input styles & their security story

| Mode          | Author uses it for…                   | How Cupcake runs it                                                        | Security label           | Main benefit                              |
| ------------- | ------------------------------------- | -------------------------------------------------------------------------- | ------------------------ | ----------------------------------------- |
| **`array:`**  | Audited & machine-generated policies  | Direct `execve`/`CreateProcess` with `command` + `args` (pure K8s style)   | **Safe** (no shell)      | Zero shell → zero injection surface       |
| **`string:`** | Quick 1-liners pasted from a terminal | Tokenise & **parse** limited operators → same argv list as `array:`        | **Safe** (no shell)      | Bash ergonomics _with_ array-level safety |
| **`shell:`**  | Rare cases needing full Bash grammar  | Run `/bin/sh -c` (or `cmd /C`) **only here**; can be disabled or sandboxed | **Caution** (shell runs) | Risk isolated to one opt-in field         |

**Operators parsed for `string:` (v 1.0)** `|   >   >>   &&   ||`
_(Globs `_ ? [ ]` pass through literally in v 1.0; secure globbing can be added later.)\*

> **Rule of thumb for authors**
> Use **`array:`** for anything that gets security-reviewed, **`string:`** for day-to-day tasks, and **`shell:`** only when absolutely necessary.

---

## 1 `array:` syntax (Kubernetes + 7 operator keys)

### 1.1 Plain K8s fields

```yaml
command: ["/usr/bin/git"] # (k8s: command)
args: ["status", "-s"] # (k8s: args)
workingDir: repo/ # (k8s: workingDir)
env: # (k8s: env list)
  - { name: GIT_TRACE, value: "1" }
```

### 1.2 Cupcake-only composition keys

| Key              | Shell analogue | Position         | Effect                                    |                                          |                                    |
| ---------------- | -------------- | ---------------- | ----------------------------------------- | ---------------------------------------- | ---------------------------------- |
| `pipe`           | \`             | \`               | after **cmd**                             | Connect previous `stdout` → next `stdin` |                                    |
| `redirectStdout` | `>`            | after **cmd**    | Truncate file, write previous `stdout`    |                                          |                                    |
| `appendStdout`   | `>>`           | after **cmd**    | Append file, write previous `stdout`      |                                          |                                    |
| `redirectStderr` | `2>`           | after **cmd**    | Redirect previous `stderr` to file        |                                          |                                    |
| `mergeStderr`    | `2>&1`         | after **cmd**    | Merge previous `stderr` into its `stdout` |                                          |                                    |
| `onSuccess`      | `&&`           | between **cmds** | Run next cmd only if exit code == 0       |                                          |                                    |
| `onFailure`      | \`             |                  | \`                                        | between **cmds**                         | Run next cmd only if exit code ≠ 0 |

_Control symbols never appear as bare strings, so filenames like `>` still work._

#### Example — pipe + redirect, shell-free

```yaml
test:
  array:
    command: [npm]
    args: [test]
    pipe:
      - cmd: [grep, -v, WARNING]
    redirectStdout: test.log
```

---

## 2 `string:` one-liner parser (v 1.0)

```yaml
run:
  string: "npm test | tee result.log && echo DONE"
```

- Parser supports only `| > >> && ||` (no globbing yet).
- Anything else → `UnsupportedSyntax`.

---

## 3 `shell:` escape hatch (gated)

```yaml
legacy_cleanup:
  shell: |
    set -euo pipefail
    terraform state list | grep '^module.old' | xargs -r terraform state rm
```

- Disabled in production via `allow_shell:false`.
- Always wrapped with timeout + low-priv UID (+ seccomp when available).

---

## 4 Tooling

```bash
# Convert a shell line to structured YAML
cupcake encode 'npm test | tee result.log && echo DONE'
```

_Outputs the equivalent `array:` block with `pipe` / `redirectStdout` keys._

VS Code JSON-schema autocompletes all seven operator keys.

---

## 5 Validation & telemetry

- **Schema validation** — bad keys / positions fail on load.
- **Parser metrics** — log unsupported tokens for roadmap.
- **Shell counter** — flags any `shell:` usage for governance.

---

## 6 Execution internals

1. YAML → `Vec<Token>` (`Cmd` or operator).
2. `Vec<Token>` → `CommandGraph` via linear scan.
3. Spawn each `Cmd` with `std::process::Command`; wire pipes & redirects (`Stdio`).
4. Evaluate `onSuccess` / `onFailure` from exit codes.

_The only shell path is the explicit `shell:` field._

---

## 7 Example policy file (v 1.0)

```yaml
# Quick one-liner (safe)
tests:
  string: "npm test | tee result.log && echo DONE"

# K8s style + operators (safe)
docker_build:
  array:
    command: [docker]
    args: ["build", "-t", "myimage:latest", "."]
    workingDir: backend/
    env:
      - { name: DOCKER_BUILDKIT, value: "1" }
    pipe:
      - cmd: [grep, -v, WARNING]
    redirectStdout: build.log

# Full Bash (gated)
legacy_cleanup:
  shell: |
    for f in {a..z}*.tmp; do
      [ -e "$f" ] && rm "$f"
    done
```

---

## 8 Impact summary

| Goal                          | Delivered by                                                                    |
| ----------------------------- | ------------------------------------------------------------------------------- |
| **Security**                  | `array:` & `string:` bypass the shell entirely → immune to injection.           |
| **Developer familiarity**     | Base keys identical to Kubernetes; seven extra keys are self-explanatory.       |
| **Expressiveness, no shells** | Pipes, redirects, chaining covered by explicit YAML keys.                       |
| **Governance controls**       | `shell:` gated by `allow_shell`; operator keys greppable by linters.            |
| **Future-proofing**           | New features (`timeout`, `retry`, `tee`) are just more keys—no parser refactor. |
| **Implementation cost**       | ≲ 1 kLOC total: executor first, ≈ 300 LOC mini-parser, 50 LOC encode-CLI.       |

**Bottom line:** Cupcake ships Kubernetes-style arrays, Bash-style convenience, and enterprise-grade security — all with the smallest viable syntax delta and code footprint.
