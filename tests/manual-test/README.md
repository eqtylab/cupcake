# Full-Stack Test Project for Cupcake TUI

This is a hybrid Next.js + Rust project for comprehensive testing of the Cupcake TUI initialization wizard across different languages and frameworks.

## Project Structure

```
manual-test/
├── CLAUDE.md              # Main rule definitions for testing
├── .claude/
│   └── settings.local.json # Claude Code configuration
├── src/                   # Rust backend code
│   ├── main.rs            # Sample Rust application
│   └── lib.rs             # Sample Rust library
├── pages/                 # Next.js pages
│   ├── index.tsx          # React homepage
│   └── api/               # API routes
│       ├── users.ts       # Users API endpoint
│       └── users/[id].ts  # User detail endpoint
├── components/            # React components
│   └── UserForm.tsx       # Sample form component
├── package.json           # Node.js dependencies
├── tsconfig.json          # TypeScript configuration
├── next.config.js         # Next.js configuration
└── README.md              # This file
```

## Testing the TUI

1. From the repository root, run: `./test-tui.sh`
2. This script will:
   - Remove any existing `guardrails/` directory
   - Remove any existing `.cupcake/` state
   - Change to this directory
   - Launch `cupcake init` to start the TUI
3. Test all phases of the initialization wizard
4. Run the script again for quick iteration

## What to Test

- **Discovery Phase**: Verify CLAUDE.md is found and displayed
- **File Preview**: Check that file contents are shown correctly
- **Extraction Phase**: Watch the mock extraction progress
- **Review Phase**: Test editing rules and navigation
- **Compilation Phase**: Observe the compilation progress
- **Success Phase**: Verify final statistics and file generation

## Generated Files

After running the TUI, you should see:
- `guardrails/cupcake.yaml` - Root configuration file
- `guardrails/policies/` - Directory with policy fragments
- Updated `.claude/settings.local.json` with hook configuration