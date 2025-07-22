# Manual Testing Environment for Cupcake TUI

This directory contains a full-stack Next.js application for testing Cupcake policies across both frontend and backend code.

## Project Structure

This is a hybrid testing environment with:
- **Rust backend code** in `src/` (for testing Rust-specific policies)
- **Next.js frontend** with TypeScript (for testing frontend policies)
- **API routes** in `pages/api/` (for testing backend API policies)
- **React components** in `components/` (for testing component-specific rules)

## Sample Rules for Testing

### Git Workflow Rules
- Always run tests before committing code (`npm test` for frontend, `cargo test` for backend)
- Require code review for all pull requests
- Block force pushes to main branch
- Ensure commit messages follow conventional format
- Run type checking before commits (`npm run type-check`)

### Frontend Code Quality Rules
- Run ESLint before editing React components
- Require TypeScript strict mode for all new files
- Block editing API routes without proper validation
- Enforce React hooks rules and component naming conventions
- Require props validation for all components

### Backend Code Quality Rules
- Run linter before editing core engine files
- Require documentation updates for API changes
- Block editing production config without approval
- Enforce coding standards for all new files

### Security Rules
- Never commit API keys or environment variables to code
- Require security review for authentication changes
- Block dangerous commands like `rm -rf` in build scripts
- Validate all API inputs and sanitize user data
- Never expose sensitive data in API responses
- Require HTTPS in production configurations

### Database and API Rules
- Always validate request parameters in API routes
- Log all database mutations for audit trail
- Require error handling for all async operations
- Block direct database access without proper ORM
- Require rate limiting on public API endpoints

### Frontend Security Rules
- Never store sensitive data in localStorage
- Validate all form inputs before submission
- Require CSRF protection for state-changing operations
- Block inline JavaScript and styles (CSP compliance)
- Sanitize all user-generated content before rendering

### File Management Rules
- Read architecture docs before editing engine code
- Backup important files before major changes
- Require approval for deleting core files
- Log all configuration changes
- Block editing package.json dependencies without review
- Require testing after updating Next.js or React versions

## Testing Instructions

1. Run `./test-tui.sh` from the repository root
2. This will clean any existing policies and launch the TUI
3. Test the discovery, extraction, review, and compilation flows
4. The script can be run repeatedly for quick iteration

## Expected Behavior

The TUI should:
- Discover this CLAUDE.md file automatically
- Show preview of the content above
- Extract mock rules during the extraction phase
- Allow editing rules in the review phase
- Generate stub YAML files in guardrails/ directory
- Update Claude Code settings appropriately