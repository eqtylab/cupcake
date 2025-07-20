Of course. Here is a clear, concise outline designed to be shared with your team. It captures the vision, architecture, and strategic benefits of the proposed "Governance as Code" platform for Cupcake.

---

### **Cupcake: Evolving from a Tool to a "Governance as Code" Platform**

**1. The Vision: Empowering Developers with Policies as Code**

Our goal is to make Cupcake the industry standard for governing AI coding agents. To achieve this, we will evolve it from a YAML-based tool into a true "Policies as Code" platform, prioritizing a world-class developer experience in TypeScript—the language our users already know and love.

This strategy is directly inspired by the success of platforms like Pulumi and CDK, which brought the power of general-purpose programming to infrastructure. We will do the same for agent governance.

**2. The Core Philosophy: Two Tiers of Governance**

We will support two distinct but complementary ways to define policies, catering to 99% of use cases from simple to complex.

- **Tier 1: Declarative Policies (The Fast Path)**

  - **What:** Simple, secure, and ultra-fast rules defined in YAML.
  - **How:** Generated from a type-safe TypeScript SDK (`@cupcake/sdk`).
  - **Use Case:** The majority of policies: instantly blocking dangerous commands, enforcing file naming conventions, providing quick feedback.
  - **Benefit:** **Sub-10ms performance.** The Rust engine executes these policies with near-zero overhead, keeping the agent's workflow seamless.

- **Tier 2: Programmatic Hooks (The Power Path)**
  - **What:** Fully dynamic hooks written in TypeScript, with access to the entire `npm` ecosystem.
  - **How:** Executed at runtime by the Rust engine in a secure, sandboxed process.
  - **Use Case:** Advanced scenarios: calling the OpenAI API for a final code review on the `Stop` hook, sending a Slack message on a `Notification` hook, or validating a change against an external API.
  - **Benefit:** **Unlimited flexibility.** Users can integrate any API or library to create sophisticated, stateful governance logic.

**3. The Architecture: Best of Both Worlds**

Our architecture is designed to be decoupled, performant, and secure, leveraging the strengths of both Rust and TypeScript.

- **The Engine (Rust):**

  - A single, portable, high-performance binary.
  - Its **only** runtime job is to parse YAML and execute policies.
  - For programmatic hooks, it acts as a **secure process supervisor**, spawning a Node.js process, enforcing strict timeouts, and managing I/O.
  - **It has zero runtime dependency on Node.js or TypeScript.**

- **The Experience (TypeScript - `@cupcake/sdk`):**

  - The primary interface for our users.
  - Provides a fluent, type-safe builder API for defining both declarative policies and programmatic hooks.
  - A **build-time step** (`npm run policies:build`) compiles the developer's TypeScript into the static YAML files that the Rust engine consumes.

- **The Contract (JSON Schema):**
  - The Rust engine will automatically generate a `cupcake.schema.json` file.
  - This schema is the unbreakable contract that ensures the TypeScript SDK always produces valid YAML, guaranteeing perfect alignment between the two halves of the system.

**4. The Developer Experience**

A developer's workflow will be simple and familiar:

1.  `npm install @cupcake/sdk`
2.  Define all policies in a `guardrails/policies.ts` file using a fluent, autocompleted API.
3.  Run `npm run policies:build` to generate the final YAML configuration.
4.  Commit both the `policies.ts` and the generated YAML to version control.

**Example Policy Definition in TypeScript:**

```typescript
import {
  cupcake,
  Hook,
  Tool,
  Action,
  Condition,
  PolicySet,
} from "@cupcake/sdk";

// A declarative policy (Tier 1)
const requireCleanGit = new PolicySet("Require Clean Git Status")
  .on(Hook.PreToolUse, Tool.named("Bash"))
  .when(Condition.field("command").matches(/^git\s+push/))
  .when(Condition.check("git diff --quiet").expectFailure())
  .then(Action.blockWithFeedback("Commit your changes before pushing."));

// A programmatic hook (Tier 2)
const aiReviewOnStop = new PolicySet("Final AI Review")
  .on(Hook.Stop, Tool.any())
  .then(
    Action.runScript({
      file: "./hooks.ts",
      function: "onSessionStopReview",
      timeout: 120,
    })
  );

// Generate the final configuration
cupcake.generate({
  "10-workflow.yaml": [requireCleanGit],
  "99-advanced.yaml": [aiReviewOnStop],
});
```

**5. Strategic Benefits**

- **Massive Adoption:** We meet developers where they are, in the language and ecosystem they use every day.
- **Unmatched Performance:** We retain the speed and security of a compiled Rust core for the critical hot path.
- **Scalability & Reusability:** Policies become reusable, testable, and shareable components.
- **Ecosystem Potential:** Enables the creation of community-driven "Policy Packs" on npm (e.g., `@cupcake/policies-owasp-security`).
- **Future-Proof:** The decoupled architecture allows us to add SDKs for other languages (Python, Go) in the future without changing the core engine.

This approach positions Cupcake not just as a tool, but as a foundational platform for the future of agentic software development.

---

You are absolutely right to pull back and focus. That's a crucial clarification. The "policy packs" and ecosystem are the long-term vision, but they are irrelevant if the core dynamic capability isn't rock-solid first.

Let's strip away the "ecosystem" talk and focus entirely on the core task: **enabling a developer to write and execute arbitrary, dynamic TypeScript code within the Cupcake governance lifecycle.**

This is the foundational feature. Everything else is built on top of it.

---

### The Core Problem to Solve: Securely Executing User Code

The challenge is to bridge the gap between the high-performance, secure Rust world and the flexible, ecosystem-rich TypeScript world, without compromising the principles of either.

**The Goal:** A developer should be able to write a TypeScript function, point to it from their YAML policy, and have the Cupcake engine execute it securely at the right time, with the right context, and act on its result.

### The Refined Approach: A Minimalist `run_script` Action

We'll design the simplest possible feature that delivers this core capability. All complexity will be hidden from the user.

**1. The User's Perspective: What they write**

The user experience needs to be dead simple. They need to do two things:

**First, define the action in their YAML policy:** This tells the Rust engine _what_ to run.

```yaml
# in guardrails/policies/advanced.yaml
Stop:
  "":
    - name: "Custom Stop Hook Logic"
      action:
        type: "run_script"
        # Path to the TS file, relative to the `guardrails` directory
        script: "hooks/onStop.ts"
        # The function to call within that file
        function: "handleStop"
        # A hard timeout enforced by the Rust engine
        timeout_seconds: 60
```

_Note: I've simplified `script_path` to `script` for brevity._

**Second, write the corresponding TypeScript function:** This is _how_ the logic is implemented.

```typescript
// in guardrails/hooks/onStop.ts
import type { StopHookEvent, HookResult } from "@cupcake/sdk";
import { getTranscript } from "@cupcake/sdk"; // A helper from the SDK
import { someApiCall } from "./lib/api"; // User's own code

// The function must be an async named export.
// It receives the typed hook event and must return a typed result.
export async function handleStop(event: StopHookEvent): Promise<HookResult> {
  // 1. Get context from the event
  console.error(`Running custom stop hook for session ${event.session_id}...`);

  // 2. Use SDK helpers to get more data
  const transcript = await getTranscript(event.transcript_path);

  // 3. Run arbitrary, dynamic logic (e.g., call an external API)
  const validationResult = await someApiCall({ transcript });

  // 4. Return a structured result that the Rust engine understands
  if (!validationResult.success) {
    return {
      decision: "block", // Tell Claude it cannot stop yet
      reason: `Validation failed: ${validationResult.error}. Please fix this.`,
    };
  }

  return { decision: "allow" }; // All good, Claude can stop
}
```

This is the entire user-facing contract. It's clean, type-safe, and focuses them on writing their logic, not on the plumbing.

**2. The SDK's Perspective: `@cupcake/sdk`**

The SDK's job is to make the TypeScript part easy and reliable. It has two main components:

- **Types and Helpers:** It provides the TypeScript types (`StopHookEvent`, `HookResult`) and utility functions (`getTranscript`). These types are auto-generated from the Rust engine's JSON schema to guarantee they are always in sync.
- **The Runner:** This is the crucial piece. The SDK ships a small, executable Node.js script (e.g., `node_modules/.bin/cupcake-runner`). This is **not** a user-facing tool. It's the lightweight bridge that the Rust engine calls.

**The Runner's Logic (`cupcake-runner`):**

1.  Receives command-line args from the Rust parent: `cupcake-runner --file hooks/onStop.ts --function handleStop`.
2.  Reads the serialized JSON of the hook event from `stdin`.
3.  Uses a dynamic `import()` to load the user's specified file (`hooks/onStop.ts`).
4.  Validates that the specified function (`handleStop`) is an exported member of that module.
5.  `await`s the function, passing in the deserialized event data.
6.  Takes the returned `HookResult` object, `JSON.stringify()`s it, and prints it to `stdout`.
7.  If anything fails (file not found, function throws an error), it catches the error, formats it as a JSON error object, prints it to `stderr`, and exits with a non-zero code.

**3. The Engine's Perspective: The Rust `ActionExecutor`**

The Rust engine's job is to be a **paranoid, secure parent process.** It trusts nothing.

When it sees `action: run_script`:

1.  **Security Check:** It first verifies that `settings.allow_scripts` is `true` in `cupcake.yaml`. If not, it immediately fails.
2.  **Path Sanitization:** It resolves the `script` path relative to the `guardrails` directory and ensures it doesn't escape that directory (preventing path traversal attacks like `script: ../../../.ssh/id_rsa`).
3.  **Process Spawning:** It constructs the command to run the runner. It will look for the runner in a predictable location, likely `node_modules/.bin/cupcake-runner` relative to the `guardrails` directory.
    - **The command:** `npx cupcake-runner --file hooks/onStop.ts --function handleStop`
    - Using `npx` is the right call. It leverages the user's project-local dependencies and Node.js version, so Cupcake doesn't have to manage the environment.
4.  **Data Piping:** It serializes the in-memory hook event to JSON and pipes it to the child process's `stdin`.
5.  **Timeout Enforcement:** It wraps the entire process execution in a `tokio::time::timeout`. This is a hard kill switch. If the Node.js script takes longer than `timeout_seconds`, the process is terminated. This is a critical protection against runaway scripts.
6.  **Result Parsing:** It waits for the child process to exit and captures its `stdout` and `stderr`.
    - If `stdout` contains a valid `HookResult` JSON, it proceeds.
    - If `stderr` has content or the exit code is non-zero, it treats it as a script execution failure and reports an error to the user.
7.  **Action Translation:** It takes the parsed `HookResult` and translates it into the final outcome for Claude Code (e.g., exit code 2 with the `reason` on `stderr` for a `block` decision).

### What Sets Cupcake's Current Posture Up for This Success?

Your current architecture is perfectly suited for this because of these key decisions you've already made:

1.  **The Engine is Written in Rust:** This gives you the performance and security primitives needed to build a trustworthy process supervisor. Managing child processes, enforcing timeouts, and handling I/O safely are all strengths of the Rust/Tokio ecosystem.
2.  **The `CommandExecutor` Already Exists:** You have already built the core logic for securely spawning and managing child processes. Extending it to handle this specific `npx` command is an incremental change, not a rewrite.
3.  **The YAML is the "Intermediate Representation":** The system is already designed to parse a declarative format. Adding a new `action` type is a simple extension. This keeps the fast, declarative path completely separate from this new dynamic path.
4.  **The Decoupling is Key:** The Rust engine doesn't need to know anything about TypeScript, `npm`, or `node_modules`. It only needs to know how to run a single, well-defined command (`npx cupcake-runner`) and communicate with it over standard I/O streams. This is a robust, time-tested model for interoperability.

By focusing on this minimalist, secure implementation of `run_script`, you deliver the core promise of "arbitrary dynamic behavior" first. You give developers the power they need, wrapped in a secure harness that you control. The fancy "policy packs" can come later, built on top of this powerful and solid foundation.

---

Excellent. Let's dive deep into the two most compelling use cases for programmatic hooks. These examples aren't just theoretical; they represent real-world, high-value workflows that transform Cupcake from a simple guardrail into an active participant in the development lifecycle.

### Use Case 1: The AI Code Reviewer on Session Stop

This is the flagship use case for programmatic hooks. It creates a powerful feedback loop where one AI supervises another, ensuring quality and correctness before the agent considers its task complete.

**The Goal:**
Before Claude Code finishes its work (on the `Stop` hook), automatically perform a final code review of all the changes it has staged for commit. If the review passes, the agent can stop. If it fails, the agent is blocked from stopping and is given the review feedback as a new instruction.

**The Developer Experience:**

1.  **YAML Policy (`guardrails/policies/reviews.yaml`):**

    ```yaml
    Stop:
      "": # This hook runs when the main agent session stops
        - name: "AI Code Review of Staged Changes"
          description: "Uses a separate AI model to review the agent's work before it finishes."
          action:
            type: "run_script"
            script: "hooks/codeReview.ts"
            function: "performCodeReview"
            timeout_seconds: 180 # Allow up to 3 minutes for the review
    ```

2.  **TypeScript Hook (`guardrails/hooks/codeReview.ts`):**

    ```typescript
    import type { StopHookEvent, HookResult } from "@cupcake/sdk";
    import { getTranscript, runCommand } from "@cupcake/sdk"; // SDK helpers
    import OpenAI from "openai";

    // Initialize the OpenAI client once
    const openai = new OpenAI({ apiKey: process.env.OPENAI_API_KEY });

    export async function performCodeReview(
      event: StopHookEvent
    ): Promise<HookResult> {
      console.error(
        `[AI Review] Starting review for session ${event.session_id}...`
      );

      // Step 1: Get the staged code changes using a Git command
      const { stdout: diff, success } = await runCommand("git", [
        "diff",
        "--cached",
      ]);
      if (!success || !diff.trim()) {
        console.error("[AI Review] No staged changes found. Skipping review.");
        return { decision: "allow" }; // Nothing to review, allow stop
      }

      // Step 2: Get the conversation transcript for context
      const transcript = await getTranscript(event.transcript_path);
      const userRequest =
        transcript.match(/User Request: (.*)/)?.[1] || "Not found.";

      // Step 3: Call the external AI model (e.g., GPT-4) for the review
      try {
        const reviewCompletion = await openai.chat.completions.create({
          model: "gpt-4-turbo",
          messages: [
            {
              role: "system",
              content: `You are a senior software engineer acting as an automated code reviewer.
                        Your task is to review the following git diff, which was produced by an AI coding agent.
                        The agent's original task was: "${userRequest}".
                        Your review should be concise and actionable.
                        If the code is good and meets the request, respond with only the word "PASS".
                        If there are issues, respond with "FAIL:" followed by a brief, bulleted list of the required changes.
                        Focus only on critical errors, security vulnerabilities, or significant deviations from the original request.`,
            },
            { role: "user", content: `GIT DIFF TO REVIEW:\n\n${diff}` },
          ],
        });

        const reviewText = reviewCompletion.choices[0].message.content || "";

        // Step 4: Parse the review and return a structured result
        if (reviewText.trim().toUpperCase() === "PASS") {
          console.error("[AI Review] PASSED.");
          return { decision: "allow" };
        } else if (reviewText.startsWith("FAIL:")) {
          const feedback = reviewText.substring(5).trim();
          console.error(`[AI Review] FAILED. Feedback: ${feedback}`);
          return {
            decision: "block", // This is the key: prevent the agent from stopping
            reason: `Your work has been reviewed and requires the following changes:\n- ${feedback}\nPlease address these issues.`,
          };
        } else {
          // Handle unexpected AI response
          console.error(
            `[AI Review] Warning: Unexpected response from review AI: ${reviewText}`
          );
          return { decision: "allow" }; // Fail open for safety
        }
      } catch (error) {
        console.error(`[AI Review] Error calling OpenAI API: ${error}`);
        return { decision: "allow" }; // Fail open if the review service is down
      }
    }
    ```

**How it Works & The Power:**

- **Autonomous Quality Gate:** This creates a fully automated quality assurance loop. The coding agent can work for extended periods, and you have confidence that a second, independent "AI brain" will check its work before it's considered done.
- **Contextual Review:** The hook provides not just the code (`git diff`) but also the _intent_ (`transcript`). This allows the reviewer AI to check if the code actually solves the user's original problem.
- **Actionable Feedback Loop:** The `decision: "block"` and `reason` fields are the magic. Cupcake translates this into the correct exit code and `stderr` message, which Claude Code then feeds back to the original agent as a new instruction. The agent is literally told, "Your review failed, here's what you need to fix." This enables true self-correction.

---

### Use Case 2: Rich Notifications to Slack

This use case demonstrates how to bridge the gap between the agent's terminal-based world and a team's collaborative workspace.

**The Goal:**
When Claude Code needs permission to run a command (the `Notification` hook), send a detailed, interactive message to a Slack channel. This allows for team visibility and potentially even a "human-in-the-loop" approval process.

**The Developer Experience:**

1.  **YAML Policy (`guardrails/policies/notifications.yaml`):**

    ```yaml
    Notification:
      "": # This hook runs on all notifications from Claude Code
        - name: "Send Rich Slack Notifications"
          description: "Notifies a Slack channel when the agent requires user input."
          action:
            type: "run_script"
            script: "hooks/notifications.ts"
            function: "sendToSlack"
            timeout_seconds: 30
    ```

2.  **TypeScript Hook (`guardrails/hooks/notifications.ts`):**

    ```typescript
    import type { NotificationHookEvent, HookResult } from "@cupcake/sdk";
    import { WebClient } from "@slack/web-api";

    const slackClient = new WebClient(process.env.SLACK_BOT_TOKEN);
    const slackChannel = "#claude-code-activity";

    export async function sendToSlack(
      event: NotificationHookEvent
    ): Promise<HookResult> {
      console.error(
        `[Slack] Sending notification for session ${event.session_id}...`
      );

      // The message from Claude Code, e.g., "Claude needs your permission to use Bash"
      const { message } = event;

      // We can make the message richer by parsing it
      let title = "Claude Code is waiting for input";
      let details = `Message: "${message}"`;
      let color = "#ffc107"; // Yellow for waiting

      const bashMatch = message.match(/permission to use Bash to run `(.*)`/);
      if (bashMatch) {
        title = "🚨 Permission Request: Bash Command";
        details = `\`\`\`${bashMatch[1]}\`\`\``;
        color = "#dc3545"; // Red for dangerous
      }

      try {
        await slackClient.chat.postMessage({
          channel: slackChannel,
          text: `${title} - ${details}`, // Fallback for notifications
          attachments: [
            {
              color: color,
              title: title,
              fields: [
                { title: "Project Directory", value: event.cwd, short: true },
                { title: "Session ID", value: event.session_id, short: true },
                { title: "Details", value: details, short: false },
              ],
              footer: "Cupcake Notification Service",
              ts: Math.floor(Date.now() / 1000).toString(),
            },
          ],
        });
        console.error("[Slack] Notification sent successfully.");
      } catch (error) {
        console.error(`[Slack] Error sending notification: ${error}`);
      }

      // Notification hooks should never block, so we always allow and suppress output.
      return { decision: "allow", suppress_output: true };
    }
    ```

**How it Works & The Power:**

- **Team Visibility:** Moves agent activity out of a single developer's terminal and into a shared, observable space. This is crucial for team collaboration and oversight.
- **Rich, Contextual Information:** Instead of a generic "Claude is waiting," the team sees _exactly_ what command it wants to run, in which project, and for which session.
- **Extensibility:** This is the foundation for more advanced workflows. You could add buttons to the Slack message ("Approve", "Deny"). Clicking a button could trigger a webhook that writes to a file, which a subsequent Cupcake `PreToolUse` hook could read to make its final decision, creating a remote, human-in-the-loop approval system.
- **Leveraging the Ecosystem:** The developer doesn't need to write an HTTP client or learn Slack's API from scratch. They just `npm install @slack/web-api` and use a high-level, well-maintained library to get the job done. This is the power of tapping into the JS/TS ecosystem.
