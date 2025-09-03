# Cupcake Evaluation - Getting Started

Welcome to Cupcake! This directory contains everything you need to evaluate the Cupcake policy engine with Claude Code.

## Quick Start

1. **Run the Setup Script**
   ```bash
   ./setup.sh
   ```
   This will:
   - Build the Cupcake binary from source
   - Initialize a Cupcake project (creates `.cupcake/` directory)
   - Copy example policies for demonstration
   - Configure builtins for security and git workflow

2. **Test with Claude Code**
   Run Claude Code in this directory and try commands that trigger policies:
   - Safe commands like `ls`, `cat`, `echo`
   - Dangerous commands like `rm -rf`, `sudo rm`
   - File edits to system paths like `/etc/hosts`
   - Git operations like `git push --force`

## What You'll See

- **Modern Rego v1 Syntax** - Policies use the latest OPA syntax with `import rego.v1`
- **Decision Verbs** - Clean syntax: `deny contains decision if { ... }`
- **Builtin Abstractions** - Pre-built security policies you can enable
- **Signal Integration** - External data feeding into policies
- **Action Automation** - Automated responses to policy decisions

## Architecture Highlights

- **Hybrid Model**: Rego handles policy logic, Rust handles routing/synthesis
- **Metadata-Driven Routing**: Policies declare what events they care about
- **O(1) Event Routing**: Efficient policy matching
- **Decision Synthesis**: Intelligent conflict resolution

## Global vs Project Policies

- **Project**: Local to this directory (`.cupcake/`)
- **Global**: Machine-wide policies with absolute precedence
  ```bash
  cupcake init --global  # Set up global policies
  ```

## Next Steps

1. Examine the example policies in `../fixtures/`
2. Try modifying policies and re-running tests
3. Enable different builtins in `guidebook.yml`
4. Create your own custom policies

Happy evaluating! 🧁