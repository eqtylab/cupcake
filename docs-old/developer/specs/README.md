# Cupcake Architecture Specifications

This directory contains the architectural specifications and design documents for the Cupcake policy engine.

## Documents

### ARCHITECTURE_SPEC.md

The primary architectural specification that defines:

- **The Hybrid Model**: Division of responsibilities between Rego (WASM) and Rust (Host)
- **Metadata-Driven Routing**: How policies declare their requirements and the engine routes events
- **Decision Verb System**: The modern Rego v1.0 syntax for expressing policy decisions
- **Synthesis Layer**: How the engine prioritizes and synthesizes multiple decisions
- **Claude Code Integration**: Mapping policy decisions to Claude Code API responses

This specification defines the current Cupcake architecture.

## Design Principles

1. **Simplicity for the User, Intelligence in the Engine** - Users write simple declarative policies while the engine handles signal collection, aggregation, and synthesis complexity.

2. **Policy Self-Filtering** - Policies include their own event/tool checks in Rego. Routing metadata controls signal gating and early exit, NOT which Rego rules execute (all compiled policies run when WASM evaluates).

3. **O(1) Signal Gating** - Metadata-driven indexing ensures constant-time lookup for determining which signals to execute and whether to run WASM at all.

4. **Recursive Aggregation** - The system automatically discovers and aggregates all policies via `walk()` without manual maintenance.