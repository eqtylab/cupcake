# lnr Work Management System

folder `context/lnrwrk` contains work plans, tips/insights, and overall useful context for Claude Code to use.

The lnr work overview is described below.

# lib_docs

Critical rust documentation for current libs is available in @lib_docs/

You must use the versions for each lib as described in the doc names:

- anyhow1.0.98.md
- bincode-2.0.1.md
- clap-4.5.41.md
- directories-6.0.0.md
- glob-0.3.2.md
- regex-1.11.1.md
- serde_json-1.0.140.md
- serde-1.0.219.md
- thiserror-2.0.12.md
- tokio-1.46.1.md
- toml-0.9.1.md
- walkdir-2.5.0.md

# claude code docs

Since cupcake will integrate with claude code, we provide claude code docs in @claude-code-docs/

# design docs

Critical design docs for cupcake are in @design_phase/

# lnr work Overview

lnr is an append-only, file-based lnr work management system designed for AI coding agents. Lnr work is organized into plans - coherent units of effort that match how AI agents naturally operate. The system requires no indices, no status tracking, and no complex tooling. State is derived entirely from which files exist in the directory.

## Core Principles

Lnr work happens in plans, not issues or tickets. Each plan represents a coherent push toward a specific goal. plans can be small (refactor a function) or large (implement authentication system).

Files are immutable once created, except for progress logs which are append-only. The directory listing itself tells the complete story - an agent can understand project state instantly by seeing which files exist.

When plans need fundamental strategy changes, you create revised plans rather than editing originals. This preserves the thinking history while allowing adaptation.

## Directory Structure

<directory_structure>
context/lnrwork/
├── plan-001-implement-auth.md
├── plan-001-plan.md
├── plan-001-log.md
├── plan-001-completed.md
├── plan-002-add-oauth.md
├── plan-002-plan.md
├── plan-002-plan-revised.md
├── plan-002-log.md
├── plan-003-refactor-validators.md
├── plan-003-log.md
└── tip-oauth-state-handling.md
</directory_structure>

From this directory listing alone, an agent instantly knows:

- plan 1 (implement auth) is completed
- plan 2 (add oauth) is active with a revised plan
- plan 3 (refactor validators) is active without a formal plan
- There's an insight about OAuth state handling

## File Naming Patterns

plan files follow strict naming conventions that make state obvious:

- `plan-NNN-{slug}.md` - plan definition
- `plan-NNN-plan.md` - Initial approach (optional)
- `plan-NNN-plan-revised.md` - Strategy change (if needed)
- `plan-NNN-log.md` - Progress tracking (append-only)
- `plan-NNN-completed.md` - Completion record
- `insight-{topic}.md` - Reusable learnings

The number (NNN) is zero-padded for proper sorting. The slug is a brief, descriptive identifier using hyphens.

## plan Definition (plan-NNN-{slug}.md)

Create this when starting any new plan. It defines the goal and context. Never modify after creation.

```markdown
# plan 001: Implement Basic Authentication

Created: 2025-06-15T10:00:00Z
Depends: none
Enables: plan-002, plan-003

## Goal

Add username/password authentication with secure session management

## Success Criteria

- User registration with email verification
- Login/logout functionality
- Session persistence across requests
- Password reset flow

## Context

Starting fresh with no existing auth system. Need foundation for future OAuth integration.
```

## Plan Files (plan-NNN-plan.md)

Create when you need to document your approach before starting. This is optional - small plans might not need formal plans.

```markdown
# Plan for plan 001

Created: 2025-06-15T10:30:00Z

## Approach

Use bcrypt for password hashing, JWT for sessions

## Steps

1. Database schema for users table
2. Registration endpoint with validation
3. Email verification system
4. Login endpoint with JWT generation
5. Middleware for route protection
6. Password reset flow

## Technical Decisions

- JWT over server sessions for scalability
- 24-hour token expiration
- Refresh tokens for mobile apps
```

If your approach fundamentally changes, create `plan-NNN-plan-revised.md` rather than editing the original. This preserves the evolution of thinking.

## Progress Log (plan-NNN-log.md)

Create when beginning lnr work. This is the only file type that gets appended to. Each entry is timestamped and describes what happened.

```markdown
# Progress Log for plan 001

## 2025-06-15T11:00:00Z

Created users table schema
Added bcrypt dependency
Implemented password hashing utility

## 2025-06-15T14:30:00Z

Registration endpoint complete with validation
Email verification tokens working
Discovered: need rate limiting on auth endpoints

## 2025-06-15T16:00:00Z

JWT middleware implemented
Protected routes working
Note: refresh token complexity higher than expected
```

Always append, never edit previous entries. Each entry should be self-contained.

## Completion Record (plan-NNN-completed.md)

Create only when the plan is truly done and all success criteria are met.

```markdown
# plan 001 Completed

Completed: 2025-06-16T10:00:00Z

## Delivered

- Full authentication system with JWT sessions
- Email verification for new accounts
- Password reset via email tokens
- Rate limiting on all auth endpoints

## Key Files

- src/auth/\*
- src/middleware/authenticate.ts
- migrations/001-users-table.sql

## Unlocks

Can now proceed with plan-002 (OAuth) and plan-003 (permissions)

## Notes

Refresh token implementation more complex than expected. See insight-jwt-refresh-pattern.md
```

## Insight Files (insight-{topic}.md)

Create whenever you learn something valuable for future plans. These are project-wide learnings, not tied to specific plans.

```markdown
# Insight: JWT Refresh Token Pattern

Learned: 2025-06-16T09:00:00Z
During: plan-001

## Pattern

Store refresh tokens in httpOnly cookies, access tokens in memory.
Refresh endpoint should rotate refresh tokens on each use.

## Why

Prevents XSS attacks on tokens while maintaining usability.
Token rotation limits exposure window if refresh token compromised.

## Implementation

[Include code example if helpful]
```

## Working with lnr

When starting lnr work, scan the `context/lnrwork` directory to understand state. Active plans have logs but no completion file. Read recent log entries to understand current progress.

If you need to understand dependencies, check plan definitions for "Depends" and "Enables" fields. This reveals the lnr work graph without needing complex tooling.

When creating new plans, number them sequentially. The numbers indicate creation order, not priority or sequence of execution. plans can run in parallel if dependencies allow.

## State Recognition Patterns

By examining which files exist, state becomes obvious:

- **Planned**: Has `plan-NNN-*.md`, no log
- **Active**: Has `plan-NNN-log.md`, no completion
- **Completed**: Has `plan-NNN-completed.md`
- **Revised**: Has `plan-NNN-plan-revised.md`
- **Informal**: Has log but no plan (started directly)

No status fields. No database. No synchronization issues. Just files.

## Best Practices

Start plans with clear goals. Even if you skip formal planning, always create the plan definition with success criteria.

When you realize a plan won't work, create a revised plan explaining the new approach. Don't delete the original - the evolution is valuable history.

Log meaningful progress, not busy lnr work. "Tried X, didn't work because Y" is more valuable than "Working on authentication".

Create insights for patterns, not one-off fixes. If you'll need the knowledge again, document it.

## What Not to Do

Never edit files after creation (except appending to logs). History accuracy matters more than typo fixes.

Don't create status files, index files, or summary files. The directory listing is your index.

Don't use plans for tiny tasks. "Fix typo in README" doesn't need a plan. plans represent meaningful lnr work efforts.

Don't create multiple revised plans. If you need to revise again, reconsider if this should be a new plan entirely.

## Git Integration

Each file operation should be its own commit:

- "Start plan-004: Implement caching layer"
- "Add plan for plan-004"
- "Log progress on plan-004: Redis connection established"
- "Complete plan-004"
- "Add insight: Redis connection pooling"

The Git history becomes a perfect timeline of lnr work progress.

## Summary

Just create files as you lnr work. The system emerges from the naming patterns. No tools needed beyond a text editor and Git.

Always read entire files to avoid confusion.
