# Guardrail Integrations

**Status:** Design Specification  
**Date:** August 2025

## Overview

Cupcake's signal system provides integration with existing industry guardrail systems. Rather than competing with established safety solutions, Cupcake acts as an orchestration layer that can use external guardrails through its signal mechanism.

## Integration Philosophy

### Orchestration, Not Competition

Cupcake recognizes that purpose-built guardrail systems like NVIDIA NeMo Guardrails and Invariant have deep expertise in specific safety domains. Instead of reimplementing their capabilities, Cupcake provides:

1. **Unified Interface**: All guardrails integrate through the same signal pattern
2. **Simple Policy Integration**: Guardrail results are accessible via standard Rego checks
3. **Flexible Composition**: Combine multiple guardrails with custom logic
4. **Performance Optimization**: Only call guardrails when relevant policies match
