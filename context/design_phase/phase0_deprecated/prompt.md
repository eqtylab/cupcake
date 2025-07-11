<tool_we_want_to_design>
Cupcake: Policy enforcement engine for AI coding agents (specifically Claude Code to start)

Problem: Developers tend to write rules/conventions/critical-context in CLAUDE.md, however CLAUDE.md serves as memory without deterministic guarentees (which developers, and enterprises, need for agents).

Solution: Generates Claude Code hooks from your policies (that are expressed as rules in CLAUDE.md). Cupcake hooks enforces rules in real-time, in a deterministic way.

1. You write your rules in CLAUDE.md how you normally would
2. Cupcake generates Claude Code hooks automatically - turning your rules into guarantees
3. When Claude violates a rule, Cupcake can automatically correct it's behavior using the Claude Code hooks config, or conduct any of the other configurable activities.
   All lifecycle hooks are supported.

Cupcake should:

- compile to a binary that can execute extrememly fast - either executing some form of regex over related files, some form of configurable tool chain assurance (eg "if you edit file.xyz, you must first read filexyz.md, etc), automated repo commands (eg a lint), or automated programs (more complex rules assurance, even network integrated, even things like having a new instance of claude code review itself with a new isolated verification session).
- determine if that binary is right approach? or a daemon? (whatever is best for itnegrated performance, security, reliability, etc).
- provide developers a very seemless `init` ability... init within a repo where developers use Claude Code, recursively find all CLAUDE.md, structure some meta-program (prompting) and initiate a claude code session to automatically create the internal policies that Cupcake will then enforce through a hooks integration.
- should work with all hooks
- should provide security assurances
- enables higher quality agentic behavior, enables enterprise oversight and assurance for governing coding agents, etc
  </tool_we_want_to_design>

Task: Think hard about how we should build Cupcake in rust. Determine any questions you need answered. Rationalize yourself based on what was provided. Conduct chain-of-thought with yourself and review the initial software designs you come up with. Ultimately we are hoping for simple and elegant design, for maximum capability empowerment here.
