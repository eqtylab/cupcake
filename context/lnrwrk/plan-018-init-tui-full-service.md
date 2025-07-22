# Plan 018: Complete TUI with Real Functionality

Created: 2025-01-22T12:00:00Z
Depends: plan-017
Enables: Full production-ready Cupcake initialization

## Goal

Transform the Plan 017 TUI wizard from mock implementation to fully functional end-to-end system with real rule extraction, LLM integration, and policy generation.

## Success Criteria

- Complete navigation and graceful exit handling
- Real rule extraction from CLAUDE.md and other agent files
- Actual LLM integration for converting rules to policies
- Detection and handling of existing guardrails configurations
- Valid YAML policy generation that works with cupcake run
- Robust error handling and user experience polish

## Context

Plan 017 delivered a complete TUI with all 6 screens and polished UI, but uses mock data throughout. This plan replaces all stub implementations with real functionality to create a production-ready initialization wizard.

The wizard currently has excellent UX but generates placeholder YAML files. Users need actual rule extraction and policy generation to make Cupcake useful in real projects.