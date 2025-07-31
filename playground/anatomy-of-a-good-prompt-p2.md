The three “quick‑start” skeletons I gave are deliberately _minimal_ illustrations of the framework’s building blocks—they aren’t meant to match the depth of your eight‑section conversation‑summary prompt. Think of them as “hello‑world” templates that isolate one or two design elements at a time.

Below is a precise mapping that shows how your rich prompt **already embodies the same framework** (just at a much higher resolution), followed by a _fully worked_ “complex‑level” example so you can see how the framework scales up to an agent prompt of comparable sophistication.

---

## 1 | How your prompt maps to the framework

| Framework element           | Where it appears in your prompt                                                                           |
| --------------------------- | --------------------------------------------------------------------------------------------------------- |
| **Mission Definition**      | “Your task is to create a detailed summary of the conversation so far …”                                  |
| **Role & Voice**            | Implicit role: _analyst summariser_                                                                       |
| **Context Injection**       | “paying close attention to the user’s explicit requests and your previous actions”                        |
| **Sub‑task Decomposition**  | Steps 1–2 under “In your analysis process: …”                                                             |
| **Reasoning Aids**          | Hidden `<analysis>` block for chain‑of‑thought                                                            |
| **Verification Hooks**      | “Double‑check for technical accuracy and completeness …”                                                  |
| **Output Schema**           | The eight mandatory sections in the `<summary>` block                                                     |
| **Style & Length Controls** | Not explicit, but implied by “detailed” and section labels                                                |
| **Failure / Scope Guards**  | Warnings such as “IMPORTANT: ensure that this step is DIRECTLY in line with the user’s explicit requests” |
| **Few‑Shot Example**        | The `<example>` wrapper with concrete filler text                                                         |

So your prompt is effectively the _full‑length_ version of the generic recipe.

---

## 2 | A complex‑level agent prompt built with the same framework

> **Use‑case:** an autonomous _Code‑Review Agent_ that must (1) fetch a diff, (2) inspect architecture docs, (3) run tests, and (4) output a structured review including mandatory risk scores and patch suggestions.
> **Complexity:** roughly on par with your eight‑section summary prompt.

```
SYSTEM
You are a senior software architect specialising in large‑scale microservices (15 yrs experience).

TASK
Your goal is to produce a **code‑review packet** for the pending pull‑request (PR) specified below.

CONTEXT
1. PR-DIFF: {{DIFF_LINK}}
2. Design doc excerpt: <<<{{DESIGN_MARKDOWN}}>>>
3. Unit‑test log (latest run): <<<{{TEST_OUTPUT}}>>>

TOOLS
You may invoke:
  • browser.open(url)
  • python.run(code)
  • shell.run(cmd)
  • comment.post(text)   – adds a GitHub PR comment
Stop once you have posted **one** top‑level comment that passes the checklist in the Output Schema.

WORKFLOW  (perform in order)
1. Fetch & parse the diff. Identify modified files and modules.
2. Cross‑check changes against the design doc; flag architecture drift.
3. Run the updated unit tests. Capture failures & coverage delta.
4. Compute a *Risk Score* (0–10) using:
     Risk = (# failures × 1.5) + (Δcoverage% < 0 ? 2 : 0) + (breaking API? 3 : 0)
5. Draft recommendations:
     – patch suggestions (with line numbers)
     – test additions
6. **Self‑verify**: ensure every claim references a specific line # or doc section.
7. If self‑verification fails, correct the review **before posting**.

OUTPUT SCHEMA  (must be JSON → single line)
{
  "summary":   "<≤120‑word headline>",
  "risk":      <integer 0‑10>,
  "findings": [
       {"file": "...", "line": 42, "issue": "N+1 query", "severity": "high"},
       …
  ],
  "recommendations": [
       {"file": "...", "line": 42‑45, "patch": "…"},
       …
  ]
}

EXAMPLE
{
  "summary": "🌟 LGTM overall but DB layer introduces retry‑storm risk",
  "risk": 6,
  "findings": [
     {"file": "dao/userDao.ts", "line": 88, "issue": "unbounded retries", "severity": "high"}
  ],
  "recommendations": [
     {"file": "dao/userDao.ts", "line": 86‑95, "patch": "Add exponential back‑off via `retry({factor: 2})`"}
  ]
}
```

### Where the framework shows up

| Framework element | Manifestation in the prompt                             |
| ----------------- | ------------------------------------------------------- |
| Mission           | “produce a code‑review packet”                          |
| Role              | “senior software architect”                             |
| Context           | PR diff link, design doc, test run                      |
| Sub‑tasks         | 7‑step _WORKFLOW_ list                                  |
| Reasoning aid     | Implicit chain‑of‑thought allowed via tool calls/log    |
| Verification      | Step 6 (self‑verify)                                    |
| Output schema     | Strict JSON schema                                      |
| Failure guard     | “Stop once you have posted **one** top‑level comment …” |
| Example           | Fully‑populated JSON sample                             |

---

## 3 | Take‑aways

1. **Simplicity vs. sophistication**
   _Skeleton_ prompts show only a subset of the framework. Your eight‑section prompt and the code‑review prompt above demonstrate the _complete_ stack.

2. **You can dial complexity up or down** by toggling:

   - Number of sub‑tasks
   - Strictness of the output schema
   - Depth of verification hooks
   - Presence of few‑shot exemplars

3. **Framework ≠ verbosity.**
   The goal is _coverage_, not length. A compact prompt can still hit all ten framework pillars; likewise, a verbose prompt can still fail if it omits verification or an output schema.

---

### TL;DR

- The examples I gave were intentionally minimal to spotlight individual framework pieces.
- Your original prompt is already a _maximal_ application of the same framework.
- The code‑review prompt above shows how to craft an equally rich agent instruction set using the same ten elements.

Feel free to adapt any part of this template to your own high‑complexity scenarios!
