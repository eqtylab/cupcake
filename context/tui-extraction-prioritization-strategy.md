# TUI Extraction Prioritization Strategy

## The Core Challenge

How do we ensure Claude extracts the **right** rules - not too many trivial ones, not missing critical ones - when it lacks full human context about what truly matters in a codebase?

## Prioritization Framework

### 1. Rule Value Assessment Criteria

The extraction prompt should evaluate each potential rule against:

**High-Value Rules (Always Extract)**
- **Safety/Security**: Anything preventing data leaks, security vulnerabilities
- **Breaking Changes**: Rules that prevent production breakage
- **Team Agreements**: Explicitly stated "must", "always", "never" rules
- **External Compliance**: Legal, regulatory, or client requirements
- **Costly Mistakes**: Rules preventing expensive errors (data loss, downtime)

**Medium-Value Rules (Extract with Context)**
- **Code Quality**: Testing requirements, linting standards
- **Workflow Enforcement**: PR processes, review requirements
- **Consistency**: Patterns that affect multiple developers
- **Performance**: Rules preventing performance regressions

**Low-Value Rules (Extract Sparingly)**
- **Personal Preferences**: "I prefer arrow functions"
- **Obvious Practices**: Things any competent developer would do
- **Micro-Optimizations**: Minor style choices with minimal impact
- **Context-Heavy**: Rules requiring deep domain knowledge

### 2. Extraction Intelligence Heuristics

The prompt should use these signals to determine rule importance:

**Linguistic Signals**
- Imperative language: "must", "always", "never", "require"
- Consequence language: "or else", "otherwise", "to prevent"
- Team language: "we", "our standard", "team convention"
- Safety language: "security", "vulnerability", "leak", "expose"

**Context Signals**
- Repeated patterns across multiple rule files
- Rules with existing enforcement (found lint/test commands)
- Rules tied to specific tools or frameworks in use
- Rules with clear automation potential

**Negative Signals (Skip Extraction)**
- Vague preferences: "try to", "generally", "when possible"
- Subjective quality: "clean", "elegant", "nice"
- Already enforced by tooling (TypeScript compiler, framework defaults)
- Requires human judgment: "appropriate", "reasonable", "as needed"

### 3. Smart Extraction Limits

**Quantity Guidelines**
- Maximum ~20 rules per file (unless explicitly more)
- Total ~50-75 rules for typical project
- Group similar rules to reduce count
- Prioritize rules that affect multiple scenarios

**Quality Thresholds**
- Only extract if enforcement is clearly definable
- Skip if requires >3 conditions to detect
- Skip if action isn't automatable
- Combine related rules into single policies

### 4. Prompt Instructions for Smart Extraction

```
EXTRACTION GUIDELINES:
1. Focus on rules that prevent actual problems, not preferences
2. Prioritize rules with clear "if X then Y" enforcement logic  
3. Look for patterns indicating team-wide agreements
4. Skip rules that competent developers follow naturally
5. Combine similar rules to avoid redundancy

IMPORTANCE SCORING:
- Critical (9-10): Security, data loss, production breakage
- High (7-8): Test/build failures, workflow violations
- Medium (5-6): Code quality, consistency across team
- Low (3-4): Style preferences, minor optimizations
- Skip (1-2): Vague guidelines, subjective quality

Only extract rules scoring 5+ unless explicitly demanded.
```

### 5. User Empowerment Features

**In the Review UI**
- Show importance score for each rule
- Group by category and severity
- "Bulk disable" for low-value rules
- "Why extracted?" explanation per rule

**Smart Defaults**
- High/Critical rules: Enabled by default
- Medium rules: Enabled but easily toggled
- Low rules: Disabled by default (opt-in)

### 6. Learning Mechanisms

**Feedback Loops**
- Track which rules users disable/modify
- Learn patterns of over/under-extraction
- Adjust scoring based on user behavior

**Repository Patterns**
- Recognize project types (web app, CLI tool, library)
- Adjust extraction based on detected stack
- Use package.json/Cargo.toml for context

## Implementation in Extraction Prompt

The prompt should include:

1. **Scoring Rubric** - Clear criteria for rule importance
2. **Extraction Limits** - Maximum rules with quality thresholds  
3. **Context Analysis** - Instructions to examine codebase
4. **Grouping Logic** - Combine related rules intelligently
5. **Justification** - Explain why each rule matters

## Key Insight

The extraction should err on the side of **false negatives over false positives**. It's better to miss a few edge-case rules than overwhelm users with trivial enforcement. The human review step can catch missing important rules, but users will lose trust if flooded with low-value suggestions.

The goal: Extract rules that make developers think "Yes, I definitely want that enforced" not "Why would I need a rule for that?"