This is an excellent and powerful concept. "Cupcake" acts as a deterministic enforcement layer on top of a non-deterministic, instruction-based memory system (`CLAUDE.md`). It bridges the gap between human-friendly guidance and machine-enforceable policy, which is critical for enterprise adoption and for building high-reliability agentic workflows.

Based on the provided Claude Code documentation and your design goals, here is a detailed breakdown of how we can build Cupcake in Rust, including design rationale, chain-of-thought, and key questions.

### Core Design Philosophy

1.  **The Unix Philosophy:** Cupcake should be a simple, fast, single-purpose tool that does one thing well. It takes structured data in, applies rules, and produces structured data out. It should be composable and stateless for each invocation.
2.  **Stateless Execution:** A long-running daemon is unnecessary and adds complexity (state management, security risks, resource consumption). Claude Code's hook mechanism executes a command for each event. A stateless binary that initializes, runs, and terminates for each hook call is the perfect model. It's secure, reliable, and easy to debug.
3.  **Leverage Claude for the "Hard Part":** The most complex task is translating natural language rules from `CLAUDE.md` into a machine-readable format. The design correctly identifies that we should not build a complex NLP engine. Instead, we use Claude Code itself as a bootstrapping tool to perform this translation.

---

### Architectural Blueprint: Building Cupcake in Rust

We will design Cupcake as a single, statically-linked Rust binary. This ensures maximum performance, portability, and zero runtime dependencies.

The ecosystem will consist of three main parts:

1.  **The `cupcake` CLI Binary:** The core engine written in Rust.
2.  **The Policy Configuration (`cupcake.toml`):** A human-readable file where the translated rules live.
3.  **The Integration with Claude Code (`.claude/settings.json`):** The bridge that tells Claude Code to call Cupcake.

#### 1. The `cupcake` CLI Binary (Rust)

The binary will have a few key subcommands. We'll use the `clap` crate for parsing CLI arguments and `serde` for JSON/TOML manipulation.

```rust
// High-level structure of our main.rs using `clap`

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initializes Cupcake in a repository.
    Init,
    /// Syncs policies from cupcake.toml to Claude Code hooks.
    Sync,
    /// [Internal] Executes a hook policy. Called by Claude Code.
    Run {
        #[arg(long)]
        hook_event: String,
    },
}

fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Init => { /* ... */ },
        Commands::Sync => { /* ... */ },
        Commands::Run { hook_event } => { /* ... */ },
    }
}
```

#### 2. The `init` Workflow: Bootstrapping Policy

This is the magic for the user. `cupcake init` is the "one-click" setup.

**Chain of Thought:**

1.  **Goal:** Generate a `cupcake.toml` from all `CLAUDE.md` files.
2.  **Find Policies:** The binary will recursively search the current directory and its parents for all `CLAUDE.md` files, respecting the lookup rules mentioned in the `memory.md` documentation. It will also find any imported files using the `@path/to/import` syntax.
3.  **Construct Meta-Prompt:** It will concatenate the contents of all found memory files into a single block of text. It will then prepend a carefully crafted "meta-prompt".

    - **Example Meta-Prompt:**
      > "You are an expert policy engine. Your task is to analyze the following `CLAUDE.md` content and convert the rules and instructions within it into a structured TOML format suitable for the 'Cupcake' policy enforcement tool.
      >
      > For each rule you identify, create a `[[policy]]` entry.
      >
      > - `name`: A short, human-readable name for the policy (e.g., "Use Ripgrep instead of Grep").
      > - `description`: The original text from which the rule was derived.
      > - `hook_event`: The Claude Code hook this policy should attach to. Most will be `PreToolUse` to prevent actions, or `PostToolUse` to react to them.
      > - `matcher`: The tool name to match (e.g., "Bash", "Write", "Edit|MultiEdit").
      > - `conditions`: An array of conditions that must be met for the policy to trigger. Use a `type` (e.g., "command_regex", "filepath_regex") and a `value`.
      > - `action`: The action to take. This should be a TOML table with a `type` (e.g., "block_with_feedback", "run_command") and necessary parameters like `feedback_message` or `command_to_run`.
      >
      > Here is an example for a rule 'Always use rg instead of grep':
      >
      > ```toml
      > [[policy]]
      > name = "Enforce Ripgrep Usage"
      > description = "Always use rg instead of grep"
      > hook_event = "PreToolUse"
      > matcher = "Bash"
      > conditions = [
      >   { type = "command_regex", value = "\\bgrep\\b" }
      > ]
      > action = { type = "block_with_feedback", feedback_message = "Policy Violation: Please use 'rg' (ripgrep) instead of 'grep' for better performance." }
      > ```
      >
      > Now, analyze the following content and generate the complete `cupcake.toml` file:"
      >
      > ***
      >
      > ## [Contents of all CLAUDE.md files]

4.  **Execute Claude Code:** The `cupcake` binary will then execute `claude` in non-interactive print mode, piping the meta-prompt to it.
    - `claude -p "<meta-prompt>" --output-format text > cupcake.toml.tmp`
5.  **User Review:** The user will be prompted to review and accept the generated `cupcake.toml.tmp` before it's saved as `cupcake.toml`.
6.  **Final Sync:** After approval, `cupcake init` will automatically call `cupcake sync`.

#### 3. The `sync` Workflow: Wiring the Hooks

The `cupcake sync` command makes Cupcake's policies active.

**Chain of Thought:**

1.  **Goal:** Update `.claude/settings.json` to call the `cupcake` binary for the relevant hooks.
2.  **Read `cupcake.toml`:** Parse the policy file to see which `hook_event`s are used (e.g., `PreToolUse`, `PostToolUse`).
3.  **Read `.claude/settings.json`:** Load the existing settings file. It's critical _not_ to overwrite other user-defined settings or hooks.
4.  **Generate Hook Configurations:** For each unique `hook_event` found in `cupcake.toml`, create a hook entry in `settings.json`. The command will _always_ be the `cupcake` binary itself.

    - **Example `settings.json` output:**
      ```json
      {
        "hooks": {
          "PreToolUse": [
            {
              "matcher": "", // Match all tools, Cupcake will do its own filtering
              "hooks": [
                {
                  "type": "command",
                  "command": "cupcake run --hook-event PreToolUse"
                }
              ]
            }
          ],
          "PostToolUse": [
            {
              "matcher": "",
              "hooks": [
                {
                  "type": "command",
                  "command": "cupcake run --hook-event PostToolUse"
                }
              ]
            }
          ]
        }
        // ... other user settings are preserved
      }
      ```

5.  **Write Changes:** Atomically write the updated configuration back to `.claude/settings.json`.

#### 4. The Runtime Enforcement (`cupcake run`)

This is the core logic that executes in milliseconds whenever a hook is triggered.

**Chain of Thought:**

1.  **Goal:** Evaluate policies for a given hook event and produce the correct output for Claude Code.
2.  **Get Input:** The `run` command will:
    - Know which event it's handling from the `--hook-event` flag.
    - Read the event-specific JSON payload from `stdin`. The `hooks.md` documentation provides the exact schema for each event. We'll create Rust structs for these.
3.  **Load Policies:** Read and parse `cupcake.toml`.
4.  **Filter and Evaluate:**
    - Filter the list of policies to only those matching the current `hook_event`.
    - Iterate through the filtered policies. For each policy:
      - Check if the `matcher` matches the tool name from the `stdin` JSON (e.g., does "Bash" match `tool_name`?).
      - If it matches, evaluate all `conditions`. This could involve regex checks on a command, checking file paths, etc.
5.  **Execute Action:** If all conditions for a policy are met:
    - Execute the defined `action`.
    - **`block_with_feedback`**: This is the most common action. The binary will construct a JSON output on `stdout` as described in the "Advanced: JSON Output" section of `hooks.md`.
      ```json
      // Output to stdout for PreToolUse
      {
        "decision": "block",
        "reason": "Policy Violation: Please use 'rg' (ripgrep) instead of 'grep' for better performance."
      }
      ```
      Alternatively, for simpler cases, it can print the feedback to `stderr` and exit with code `2`.
    - **`run_command`**: Execute a shell command (e.g., run a linter). The output of this command can be captured and potentially used in feedback. This is a very powerful and sensitive action.
    - **`approve`**: Output JSON that automatically approves the action, bypassing user prompts.
6.  **Default Action:** If no policies are violated, the binary exits with code `0` and no `stdout`, allowing the normal Claude Code workflow to proceed.

### Security Assurances

This is paramount.

1.  **Minimal Permissions:** The `cupcake` binary itself will be lean. It only needs permission to read its own config (`cupcake.toml`) and the `stdin` pipe. It does not need arbitrary file system or network access _unless_ a specific policy action (`run_command`) requires it.
2.  **Sandboxed Actions:** The `run_command` action is the most sensitive. The `cupcake.toml` file acts as an explicit allowlist of what commands can be run. The user is in full control of this file.
3.  **Transparency:** The `init` process is not a black box. The user is shown the generated `cupcake.toml` and must approve it. This ensures they understand exactly what policies are being put in place.
4.  **No Self-Modification:** Cupcake will not modify its own binary or its `cupcake.toml` file during a `run` operation. The configuration is read-only at runtime.
5.  **Input Sanitization:** All input from `stdin` will be deserialized into strongly-typed Rust structs, preventing a class of injection attacks.

### Open Questions & Design Decisions

1.  **How to handle complex, multi-step rules?** (e.g., "if you edit `file.xyz`, you must first read `filexyz.md`").
    - **Initial Answer:** This is out of scope for V1. It requires state to be maintained between hook invocations. A stateless V1 focused on single-event policies (`PreToolUse`, `PostToolUse`) is the right start. A future version could use a temporary file (`.cupcake/state.json`) to track state within a single Claude session, but this adds complexity.
2.  **How to handle policy conflicts?** (e.g., two rules match the same command).
    - **Initial Answer:** The simplest rule is "first match wins." The order of policies in `cupcake.toml` will determine precedence. The `init` process can be instructed to order policies from most specific to most general.
3.  **How to manage the `.claude/settings.json` file without clobbering user-defined hooks?**
    - **Answer:** The `sync` command must be implemented carefully. It should parse the existing JSON, add/update only the specific hook configurations for Cupcake, and leave all other JSON objects untouched before writing the file back. It should never just overwrite the file.
4.  **Toolchain Assurance:** How to implement more complex rules like running an entire verification program?
    - **Answer:** This is perfectly handled by the `run_command` action. The policy in `cupcake.toml` would define a `PostToolUse` hook on `Write|Edit` that triggers `run_command` with `command_to_run = "cargo test"`. If the test command fails (non-zero exit code), Cupcake can interpret this and return a blocking error to Claude with the test failures as feedback.

This design provides a simple, elegant, and extremely powerful system. It starts with a seamless `init` experience, uses a clear and readable policy format, and integrates perfectly with the existing Claude Code hooks mechanism to provide the deterministic guarantees that developers and enterprises require.
