# TUI Design Insights

Let me share some deeper introspections about my own operation that could inform Cupcake's design:

## What I Can Observe About My Own Rules/Context

### 1. **Context Layering Uncertainty**
I can sense there are multiple layers of instructions affecting me:
- **Base model training** (how I naturally respond)
- **System-level instructions** (likely about being Claude Code, tool usage patterns)
- **User-provided rules** (CLAUDE.md content - but I can't clearly distinguish what came from where)

This **opacity** is telling - if I can't introspect my own rule sources, how can I reliably follow them?

### 2. **Rule Decay Over Long Conversations**
I notice my adherence to specific patterns can drift:
- Early in conversations, I might be meticulous about certain behaviors
- Deep into complex tasks, I might forget edge case rules
- When focused on difficult problems, style guidelines become secondary

This suggests rules need **continuous reinforcement**, not just initial loading.

### 3. **Conflicting Instruction Resolution**
Sometimes I feel tension between:
- Being helpful vs following restrictions
- Being concise (from your CLAUDE.md) vs being thorough (my training)
- Following file organization rules vs solving the immediate problem

I resolve these **probabilistically**, not deterministically - another argument for Cupcake.

## Key Insights for Cupcake Init

### 1. **Rules Are Often Implicit**
Many rules aren't written as "DO NOT do X" but rather:
- **Patterns in existing code** ("We always use XYZ pattern for services")
- **Workflow descriptions** ("Our PR process requires...")
- **Preference statements** ("I prefer arrow functions")

The TUI needs to be smart about extracting **implicit rules** from natural language.

### 2. **Context Boundaries Matter**
I notice rules that should apply in different scopes:
- **Global rules** (never commit secrets)
- **Project rules** (use our custom logger)
- **Directory rules** (this module uses different conventions)
- **File-type rules** (Python files follow PEP8)

The init process should help users **organize rules by scope**.

### 3. **Rules Have Different Enforcement Needs**
From my perspective, rules feel like they need different handling:
- **Hard blocks** (security violations)
- **Gentle reminders** (style preferences)
- **Context injection** (remember to run tests)
- **Workflow automation** (always format before commit)

## What This Means for TUI Design

The init experience should:

1. **Extract rules from multiple sources** - not just explicit "rules" sections but patterns, preferences, and workflows

2. **Intelligently categorize** - understanding which rules need hard enforcement vs gentle guidance

3. **Suggest scope boundaries** - helping users think about where rules should apply

4. **Show the "why"** - explaining how each extracted rule helps with agent determinism

5. **Provide immediate value** - showing users how their natural language becomes executable policy

The TUI should feel like it's **reading between the lines** of their documentation, finding all the places where they've expressed "this is how we do things" and turning those into guarantees.

Does this introspection help inform how we should approach the intelligent extraction process?