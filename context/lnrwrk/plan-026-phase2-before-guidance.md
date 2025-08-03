#### **Transmission #2: During Phase 2, Task 2.2 (`HookEvent` Alias Removal)**

**TRIGGER:** Soldier begins the codebase-wide search and replace for the `HookEvent` alias.
**SUBJECT:** TACTICAL ADVISORY - GHOST IN THE MACHINE

> **"Opus, Command. Intel reports you are engaging the `HookEvent` alias.**
>
> **Be advised: the enemy's ghosts hide in `#[cfg(test)]` blocks and obscure test helper functions. A simple search and replace may not be sufficient.**
>
> **Your verification condition for this task is a clean compile followed by a successful run of `grep -r "HookEvent" src/ tests/`. The command must return zero results.**
>
> **Leave no ghosts behind. Confirm."**

**Commander's Intent:** This prevents the "subtle win" for complexity where a lingering alias creates confusion for future developers. It provides a concrete, verifiable definition of success for this tedious but critical task.
