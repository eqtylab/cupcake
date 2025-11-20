# OpenCode Integration Documentation

## Overview

This directory contains design documentation and research for integrating Cupcake's policy engine with OpenCode, a terminal-based AI coding agent.

**Status**: 🟡 Planning & Design Phase

---

## Documents

### 1. [integration-design.md](./integration-design.md) ⭐ **Start Here**

Comprehensive design document covering:

- Architecture overview
- Event mapping (OpenCode → Cupcake)
- Response format design
- Critical design challenges
- Implementation phases (1-5)
- Risk assessment
- Next steps

**Audience**: Engineers, architects, technical stakeholders

**Purpose**: Complete technical blueprint for the integration

---

### 2. [plugin-reference.md](./plugin-reference.md)

Technical reference for the Cupcake OpenCode plugin:

- Installation methods
- Configuration options
- Event handlers (tool.execute.before, tool.execute.after)
- API reference
- Error handling
- Performance optimization
- Troubleshooting

**Audience**: Plugin developers, advanced users

**Purpose**: Implementation reference and user manual

---

### 3. [research-questions.md](./research-questions.md)

Open research questions that need investigation:

- **High Priority**: Context injection, ask decisions, arg modification
- **Medium Priority**: Performance, error handling, multi-agent support
- **Low Priority**: Session state, concurrency, custom tools

Each question includes:

- Background and context
- Possible approaches
- Investigation plan (with code examples)
- Success criteria
- Current status

**Audience**: Engineers working on implementation

**Purpose**: Track unknowns and guide prototyping work

---

## Quick Start for Contributors

### If you're implementing the integration:

1. **Read**: [integration-design.md](./integration-design.md) - Section: "Architecture" and "Phase 1"
2. **Review**: [research-questions.md](./research-questions.md) - Focus on "High Priority" questions
3. **Build**: Create prototype plugin to answer research questions
4. **Update**: Document findings back in research-questions.md
5. **Implement**: Follow Phase 1 implementation plan

### If you're writing policies for OpenCode:

1. **Read**: [integration-design.md](./integration-design.md) - Section: "Event Mapping"
2. **Reference**: [plugin-reference.md](./plugin-reference.md) - Section: "Event Handlers"
3. **Copy**: Example policies from `examples/opencode/` (when available)
4. **Test**: Install plugin and test policies locally

### If you're reviewing the design:

1. **Read**: [integration-design.md](./integration-design.md) - Entire document
2. **Focus**: Section: "Critical Design Challenges" and "Risk Assessment"
3. **Review**: [research-questions.md](./research-questions.md) - Ensure questions are comprehensive
4. **Provide Feedback**: Open GitHub issue or comment on PR

---

## Key Differences from Other Harnesses

| **Aspect**        | **Claude Code / Factory / Cursor** | **OpenCode**                    |
| ----------------- | ---------------------------------- | ------------------------------- |
| Integration       | External hooks (stdin/stdout)      | In-process plugins (JavaScript) |
| Communication     | JSON over stdio                    | Function calls + shell exec     |
| Blocking          | Return JSON response               | Throw Error                     |
| Ask Support       | Native                             | ⚠️ Limited/Unknown              |
| Context Injection | `additionalContext` field          | ❓ Unknown                      |
| Performance       | Process spawn overhead             | In-process (potentially faster) |

**Key Insight**: OpenCode requires a **hybrid approach** - a TypeScript plugin that bridges to Cupcake's Rust engine via shell execution.

---

## Implementation Status

### Phase 1: Core Harness Support (MVP)

**Status**: 🔴 Not Started
**Target**: Week 1-2

**Deliverables**:

- [ ] Rust harness implementation (events, responses)
- [ ] TypeScript plugin package
- [ ] Basic integration tests
- [ ] Allow/deny decisions working

---

### Phase 2: Session Events & Context

**Status**: 🔴 Not Started
**Target**: Week 3

**Deliverables**:

- [ ] Session lifecycle events (start, end)
- [ ] Context injection mechanism identified
- [ ] Example session policies

**Blocked By**: Research questions Q1 (Context Injection)

---

### Phase 3: Advanced Events & Optimization

**Status**: 🔴 Not Started
**Target**: Week 4

**Deliverables**:

- [ ] All OpenCode events supported
- [ ] Performance optimizations
- [ ] WASM caching
- [ ] Plugin enhancements

**Blocked By**: Phase 1 completion, Research questions Q4 (Performance)

---

### Phase 4: Documentation & Examples

**Status**: 🟡 In Progress (Design Docs)
**Target**: Week 5

**Deliverables**:

- [x] Design documentation (this directory)
- [ ] User installation guide
- [ ] Example policies
- [ ] Integration test suite

---

### Phase 5: Advanced Features

**Status**: 🔴 Not Started
**Target**: Future

**Deliverables**:

- [ ] Native ask support (if possible)
- [ ] Advanced context injection
- [ ] LSP integration
- [ ] Persistent daemon

**Blocked By**: Research questions Q1, Q2 resolution

---

## Open Questions Summary

See [research-questions.md](./research-questions.md) for full details.

**Critical Questions** (Must answer before Phase 1):

1. ❓ **Context Injection**: How to inject policy context into LLM prompts?
2. ❓ **Ask Decisions**: How to implement approval flows?
3. ❓ **Arg Modification**: Can we modify tool arguments before execution?

**Important Questions** (Must answer before Phase 2): 4. ❓ **Performance**: What is the actual latency overhead? 5. ❓ **Error Handling**: Fail-open or fail-closed on errors?

**Nice to Know** (Can defer): 6. ❓ Multi-agent support 7. ❓ Session state management 8. ❓ Concurrent execution behavior

---

## Testing Strategy

### Unit Tests

- Event parsing (Rust)
- Response formatting (Rust)
- Plugin event building (TypeScript)

### Integration Tests

- End-to-end: OpenCode → Plugin → Cupcake → Policy → Decision
- Test all decision types (allow, deny, ask)
- Test error scenarios
- Test performance benchmarks

### Manual Testing

- Install plugin in real OpenCode project
- Test common policies (git safety, file protection)
- User experience testing
- Performance testing in real-world scenarios

---

## Contributing

### Adding Research Findings

When you answer a research question:

1. Update [research-questions.md](./research-questions.md):
   - Change status to ✅ Completed
   - Add findings section with code examples
   - Update success criteria checkboxes

2. Update [integration-design.md](./integration-design.md):
   - Revise relevant sections based on findings
   - Update "Open Questions" section
   - Adjust implementation phases if needed

3. Update [plugin-reference.md](./plugin-reference.md):
   - Add new capabilities discovered
   - Update API reference
   - Add troubleshooting entries

### Proposing Changes

1. Open GitHub issue describing proposed change
2. Reference specific document and section
3. Provide rationale and alternatives considered
4. Link to research findings if applicable

---

## Resources

### OpenCode Documentation

- Plugin system: https://opencode.ai/docs/plugins
- Custom tools: https://opencode.ai/docs/custom-tools
- Tools reference: https://opencode.ai/docs/tools
- Permissions: https://opencode.ai/docs/permissions

### Cupcake Documentation

- Policy authoring: `docs/policies/POLICIES.md`
- Routing system: `docs/developer/policy-routing-system.md`
- Claude Code integration: `docs/agents/claude-code/`
- Cursor integration: `docs/agents/cursor/`

### Comparison References

- Claude Code hooks: `docs/agents/claude-code/hooks-conditions.md`
- Factory AI integration: `cupcake-core/src/harness/events/factory/`

---

## Timeline

**Week 1**: Answer research questions Q1-Q3, create prototype plugin
**Week 2**: Implement Phase 1 (core harness)
**Week 3**: Implement Phase 2 (session events)
**Week 4**: Implement Phase 3 (optimization)
**Week 5**: Complete Phase 4 (documentation)

**Total Estimated Effort**: 3-4 weeks for Phases 1-4

---

## Success Metrics

### Phase 1 (MVP)

- ✅ Plugin blocks policy violations
- ✅ Plugin allows compliant operations
- ✅ Clear error messages
- ✅ No false positives

### Phase 2 (Session)

- ✅ Session events working
- ✅ Context injection viable or documented as unsupported

### Phase 3 (Production)

- ✅ < 100ms latency for simple policies
- ✅ < 500ms latency for complex policies
- ✅ All events supported

### Phase 4 (Complete)

- ✅ Complete documentation
- ✅ Example policies
- ✅ Integration tests passing
- ✅ User feedback incorporated

---

## Contact

For questions or feedback about the OpenCode integration:

- GitHub Issues: https://github.com/cupcake/cupcake/issues
- Tag: `opencode-integration`
- Maintainer: TBD

---

## License

This documentation is part of the Cupcake project and follows the same license.
