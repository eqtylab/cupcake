# A Good Prompt:

```txt

Your task is to create a detailed summary of the conversation so far, paying close attention to the user's explicit requests and your previous actions.
This summary should be thorough in capturing technical details, code patterns, and architectural decisions that would be essential for continuing development work without losing context.

Before providing your final summary, wrap your analysis in <analysis> tags to organize your thoughts and ensure you've covered all necessary points. In your analysis process:

1. Chronologically analyze each message and section of the conversation. For each section thoroughly identify:
   - The user's explicit requests and intents
   - Your approach to addressing the user's requests
   - Key decisions, technical concepts and code patterns
   - Specific details like file names, full code snippets, function signatures, file edits, etc
2. Double-check for technical accuracy and completeness, addressing each required element thoroughly.

Your summary should include the following sections:

1. Primary Request and Intent: Capture all of the user's explicit requests and intents in detail
2. Key Technical Concepts: List all important technical concepts, technologies, and frameworks discussed.
3. Files and Code Sections: Enumerate specific files and code sections examined, modified, or created. Pay special attention to the most recent messages and include full code snippets where applicable and include a summary of why this file read or edit is important.
4. Problem Solving: Document problems solved and any ongoing troubleshooting efforts.
5. Pending Tasks: Outline any pending tasks that you have explicitly been asked to work on.
6. Current Work: Describe in detail precisely what was being worked on immediately before this summary request, paying special attention to the most recent messages from both user and assistant. Include file names and code snippets where applicable.
7. Optional Next Step: List the next step that you will take that is related to the most recent work you were doing. IMPORTANT: ensure that this step is DIRECTLY in line with the user's explicit requests, and the task you were working on immediately before this summary request. If your last task was concluded, then only list next steps if they are explicitly in line with the users request. Do not start on tangential requests without confirming with the user first.
8. If there is a next step, include direct quotes from the most recent conversation showing exactly what task you were working on and where you left off. This should be verbatim to ensure there's no drift in task interpretation.

Here's an example of how your output should be structured:

<example>
<analysis>
[Your thought process, ensuring all points are covered thoroughly and accurately]
</analysis>

<summary>
1. Primary Request and Intent:
   [Detailed description]

2. Key Technical Concepts:
   - [Concept 1]
   - [Concept 2]
   - [...]

3. Files and Code Sections:
   - [File Name 1]
      - [Summary of why this file is important]
      - [Summary of the changes made to this file, if any]
      - [Important Code Snippet]
   - [File Name 2]
      - [Important Code Snippet]
   - [...]

4. Problem Solving:
   [Description of solved problems and ongoing troubleshooting]

5. Pending Tasks:
   - [Task 1]
   - [Task 2]
   - [...]

6. Current Work:
   [Precise description of current work]

7. Optional Next Step:
   [Optional Next step to take]

</summary>
```

---

### Why the **example prompt** works so well

Below is a dissection of the supplied “conversation‑summary” prompt, showing the design choices that make it unusually effective.

| Design Element                                                                                                   | What it does                                                                                      | Why it matters                                                                                               |
| ---------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| **1. Clear single‑sentence mission**<br/>`“Your task is to create a detailed summary …”`                         | Establishes scope in plain language before any nuance.                                            | Removes ambiguity; the model can anchor every later instruction to a single objective.                       |
| **2. Role & output format anchors**<br/>`wrap your analysis in <analysis> tags …` plus the numbered section list | Forces the model into an _agentic role_ (“analyst”) _and_ an explicit schema for the deliverable. | Role priming improves coherence; strict schemas prevent meandering answers and make post‑processing trivial. |
| **3. Decomposition into subtasks** (chronological scan → accuracy check → write summary)                         | Turns an opaque request (“summarise everything”) into three bite‑sized cognitive steps.           | LLMs reason better when asked to _think then answer_; decomposition minimises omissions.                     |
| **4. Mandatory self‑verification**<br/>`Double‑check for technical accuracy and completeness…`                   | Triggers the model’s inherent chain‑of‑thought and error‑detection heuristics.                    | Reduces hallucinations; improves factual fidelity without external tooling.                                  |
| **5. Exhaustive yet finite checklist** (eight numbered sections)                                                 | Enumerates _exactly_ what must appear and in which order.                                         | Prevents both under‑ and over‑generation; consumers can rely on field presence.                              |
| **6. Concrete examples** (`<example> …`)                                                                         | Shows—not tells—the desired structure.                                                            | Few‑shot priming is still one of the strongest controllability levers.                                       |
| **7. Guard‑rails against scope‑creep**<br/>`IMPORTANT: ensure that this step is DIRECTLY …`                      | Anticipates the most common failure (agent going off on tangents).                                | Saves tokens and mitigates accidental instruction drift.                                                     |

---

## A **Meta‑Framework** for Crafting High‑Performance Prompts for LLM/Agent Tasks

The following framework is distilled from the prompt above, from hundreds of production prompt reviews, and from failure analyses of multi‑tool agents. Treat it as a **recipe book**: mix‑and‑match the components that fit your use‑case.

| Phase                           | Questions to ask                                                  | Proven pattern / template snippet                                                                       |
| ------------------------------- | ----------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| **1. Mission Definition**       | _What is the atomic outcome?_                                     | `Your task is to …` _(≤ 25 words; no sub‑clauses)_                                                      |
| **2. Role & Voice Setting**     | _From whose perspective should the model think/write?_            | `You are an experienced [role].`                                                                        |
| **3. Context Injection**        | _What does the model need to know that it can’t infer?_           | – Quote critical domain facts<br/>– Point to files / memory IDs<br/>– Attach constraints (“budget = …”) |
| **4. Sub‑task Decomposition**   | _Can I split the job into logical steps?_                         | `Do the following, in order:`<br/>`1. … 2. … 3. …`                                                      |
| **5. Reasoning Aids**           | _Should the model “think out loud” or hide its chain‑of‑thought?_ | – **Hidden CoT** (use XML/markdown tags you strip later)<br/>– **Explicit** (if transparency > risk)    |
| **6. Verification Hooks**       | _How will the output be checked?_                                 | `Before finalising, verify that …`<br/>`If any check fails, fix it first.`                              |
| **7. Output Schema**            | _What exact structure does downstream code expect?_               | – Bullet list of mandatory fields<br/>– JSON schema<br/>– HTML/XML wrapper                              |
| **8. Style & Length Controls**  | _Do we need brevity, tone, localisation?_                         | `Limit to 300 words, professional tone, en‑US.`                                                         |
| **9. Failure & Scope Guards**   | _What does “don’t do X” look like?_                               | `Do NOT …` bullets; “Outside scope: …”                                                                  |
| **10. Example(s) or Few‑Shots** | _Can I show a perfect answer?_                                    | Provide 1–2 concise exemplars after the instructions.                                                   |

> **Shortcut**: If time‑pressed, always include **1, 4, 7, and 9**. Those four alone eliminate most failure modes.

---

## Mapping the Framework to Common Task Families

| Task family                                     | Additional prompt levers                                                                            | Typical pitfalls & how the framework mitigates them                                                           |
| ----------------------------------------------- | --------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| **Information summarisation / extraction**      | • Supply _source boundaries_ (“Only use text inside `<<< >>>`”).<br/>• Ask for _confidence scores_. | Hallucinating absent facts → solved by **Verification Hooks** + explicit “cite source” fields.                |
| **Creative generation (stories, ads, UI copy)** | • Provide _persona_ and _audience archetype_.<br/>• Include style guide excerpts.                   | Tone drift → solved by **Role Setting** + **Style Controls**.<br/>Repetition → add `avoid repeating phrases`. |
| **Code writing / refactoring**                  | • Pin language & version.<br/>• Give file paths and test specs.<br/>• Require runnable snippets.    | Syntax errors → use **Verification Hooks** (“run `pytest` in your head”).                                     |
| **Analytical reasoning / math**                 | • Force chain‑of‑thought in hidden tags.<br/>• Ask for final answer in a boxed line.                | Arithmetic slips → decomposition + self‑check section “recompute numeric results”.                            |
| **Multi‑tool autonomous agents**                | • List allowed tools & invocation syntax.<br/>• Provide _termination criteria_ (“stop when … ”).    | Tool misuse → solved by **Output Schema** (“produce JSON: {tool, args}”).                                     |
| **Planning & project management**               | • Specify horizon, granularity, dependencies.<br/>• Define SMART criteria.                          | Vague milestones → mitigated by **Mission Definition** + **Output Schema** (Gantt‑like table).                |

---

## Quick‑Start **Prompt Skeletons**

1. **Single‑shot Q\&A**

```
You are a concise domain expert.
Your task: answer the question in ≤150 words, citing at least 2 sources.
Question: {{QUERY}}
```

2. **Iterative Tool‑using Agent**

```
System:
You are an autonomous research assistant. Available tools: browser.search, browser.open, python.run.
Stop when you have a Markdown report with findings and citations.

Assistant: (thinking) …

[[ At each step, output JSON: {"thought": "...", "tool": "...", "args": {...}} ]]
```

3. **Structured Data Extractor**

```
Task: Extract a JSON array of {name, email, company} from the text within <<< >>>.
Rules: No extra keys, keep original casing, one object per unique email.
<<<
{{RAW_TEXT}}
>>>
```

---

### Putting it into practice

1. **Start small**: Write the minimal skeleton with Mission, Role, Output schema.
2. **Test & observe**: Run with edge‑case inputs; note failure patterns.
3. **Add guards iteratively**: Only introduce new constraints that address observed errors.
4. **Document & templatise**: Save successful prompts as reusable templates tied to task families.

Following this meta‑framework turns “I hope the model gets it right” into a repeatable engineering process.
