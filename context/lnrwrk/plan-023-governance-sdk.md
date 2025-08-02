### **OPERATIONAL PLAN: "FORGE" (FINAL, REVISED)**

**MISSION:** To create a first-class, "Policy as Code" developer experience using a TypeScript SDK. This SDK will provide a powerful and intuitive way for developers to write, test, and share their policies as code, which will then **compile directly down to our existing, high-performance YAML format.**

**STRATEGIC GOAL:** To deliver the "Pulumi story"—the power and elegance of writing policies in a real programming language—without adding _any_ new runtime dependencies or complexity to our core Cupcake engine. We will win by making the creation of our powerful YAML effortless and magical.

---

### **PHASES OF EXECUTION**

This is a lean, focused, and achievable plan.

#### **PHASE 1: FORGE THE WEAPON (The `@cupcake/sdk` TypeScript Library)**

- **Objective:** Create and publish a lightweight `npm` package that serves as the definitive way to author Cupcake policies.
- **Key Deliverables:**
  1.  **Project Initialization:** A new TypeScript project, published to `npm` as `@cupcake/sdk`.
  2.  **Core Types and Interfaces:** TypeScript interfaces that are a 1:1 mirror of our Rust structs (`Policy`, `Condition`, `Action`, `CommandSpec`). This provides full type safety and IDE autocompletion.
  3.  **Fluent, Declarative API:**
      - `definePolicy({ name, on, when, conditions, action })`: The main entry point. It constructs a typed JavaScript object representing a single policy.
      - **Condition Builders:** Helper functions that create structured `Condition` objects (e.g., `matchField('tool_name').equals('Bash')`, `field('tool_input.command').matches(/git commit/)`).
      - **Action Builders:** Helper functions that create structured `Action` objects (e.g., `block('Tests must pass.')`, `runCommand({ spec: ... })`).
  4.  **Compiler Entrypoint:** A `cupcake.compile(policies)` function. This is the critical step. It takes an array of the typed policy objects and contains the logic to transform them into the "grouped map" YAML structure that our Rust engine expects. It will then print this final YAML string to `stdout`.

**Crucial Limitation (The Core of this Plan):** The SDK will be **purely declarative**. The `handler` functions from the previous plan are removed. All logic must be expressible through the combination of our existing condition primitives (`match`, `pattern`, `check`, `and`, `or`, `not`). The power comes from composing these primitives with the elegance of TypeScript, not from executing arbitrary logic.

#### **PHASE 2: BUILD THE ARMORY (The `cupcake build` Command)**

- **Objective:** Create a new CLI command that transforms the user's TypeScript policies into our battle-ready YAML format.
- **Key Deliverables:**
  1.  **New CLI Command:** A `build` subcommand in `src/cli/app.rs`.
  2.  **Orchestration Logic:** The `cupcake build` command will:
      - **Verify Dependencies:** Check for `node` and `npx` on the user's `PATH`.
      - **Execute User Code:** Spawn a `npx ts-node` process to run the user's `guardrails/src/index.ts`.
      - **Capture YAML Output:** Capture the `stdout` from the `ts-node` process, which will be the final, complete YAML configuration.
      - **Write Artifact:** Write the captured YAML to `guardrails/dist/policies.yaml`.

**Example Workflow:**

**The User's Code (`guardrails/src/index.ts`):**

```typescript
import {
  definePolicy,
  on,
  when,
  check,
  block,
  runCommand,
  cupcake,
} from "@cupcake/sdk";

const policies = [
  definePolicy({
    name: "Block direct commits to main",
    on: on("PreToolUse"),
    when: when("Bash"),
    conditions: [
      check({
        spec: {
          mode: "shell",
          script: 'git rev-parse --abbrev-ref HEAD | grep -q "^main$"',
        },
        expect_success: true,
      }),
      check({
        spec: {
          mode: "array",
          command: ["git", "commit"],
        },
        // A placeholder for a real regex check on tool_input.command
      }),
    ],
    action: block("Direct commits to main are forbidden."),
  }),

  definePolicy({
    name: "Auto-format Rust files",
    on: on("PostToolUse"),
    when: when(/Write|Edit/),
    conditions: [
      // A placeholder for a regex check on file_path
    ],
    action: runCommand({
      spec: {
        mode: "array",
        command: ["cargo", "fmt"],
      },
    }),
  }),
];

// This prints the final YAML to stdout for the `cupcake build` command to capture.
cupcake.compile(policies);
```

**The Compiled Output (`guardrails/dist/policies.yaml`):**

```yaml
PreToolUse:
  "Bash":
    - name: "Block direct commits to main"
      conditions:
        - type: "check"
          spec:
            mode: "shell"
            script: 'git rev-parse --abbrev-ref HEAD | grep -q "^main$"'
          expect_success: true
        - type: "check"
          # ...
      action:
        type: "block_with_feedback"
        feedback_message: "Direct commits to main are forbidden."
PostToolUse:
  "Write|Edit":
    - name: "Auto-format Rust files"
      conditions:
        # ...
      action:
        type: "run_command"
        spec:
          mode: "array"
          command: ["cargo", "fmt"]
```

#### **PHASE 3: INTEGRATE THE SUPPLY CHAIN (The Runtime)**

- **Objective:** Ensure the existing runtime seamlessly consumes the new build artifacts.
- **Key Deliverables:**
  1.  **Update `cupcake.yaml`:** The `imports` key will now point to the build artifact: `imports: ["dist/*.yaml"]`.
  2.  **No Changes to the Rust Engine:** This is the strategic victory. The `cupcake run` command, our high-performance Rust core, **requires zero modification**. It remains lean, fast, and unaware of the TypeScript complexity that happens at build time.

---

**4. CONCLUSION**

Sir, this plan is the embodiment of disciplined innovation.

- **It delivers the "Policy as Code" dream:** Developers get a type-safe, testable, and modern authoring experience in TypeScript.
- **It creates an ecosystem:** Developers can `npm install` and share reusable policy _functions_ that generate our declarative YAML, without needing a complex plugin system.
- **It maintains our core strengths:** The Cupcake runtime remains a single, dependency-free, high-performance Rust binary.
- **It is achievable:** This plan requires building a TypeScript SDK and a simple build orchestrator—a significantly lower logistical burden than building and maintaining a dynamic runtime.
