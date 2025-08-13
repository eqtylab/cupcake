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

This specification represents the "NEW_GUIDING_FINAL" architecture that replaces the deprecated selector-based system.

## Design Principles

1. **Simplicity for the User, Intelligence in the Engine** - Users write simple declarative policies while the engine handles all routing, aggregation, and synthesis complexity.

2. **Trust-Based Evaluation** - Policies trust the engine's routing guarantees and focus purely on business logic.

3. **O(1) Performance** - Metadata-driven indexing ensures constant-time policy lookup regardless of policy count.

4. **Recursive Aggregation** - The system automatically discovers and aggregates all policies without manual maintenance.