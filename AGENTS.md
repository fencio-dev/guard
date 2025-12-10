# Codex Instructions for mgmt-plane

## Session Continuity Guide

**Maintain context across sessions using `STATUS.md`:**

- **At session start**: Read `STATUS.md` to understand project state, current status, known issues, and next steps
- **During work**: Keep current session work organized following the format: Current Status → Known Issues → Next Steps
- **Before ending**: Update `STATUS.md` with session summary, completed work, any new blockers, and prioritized next tasks
- **Follow the format**: Use the `<format>` template in `STATUS.md` (Current Status bullets, Known Issues bullets, Next Steps bullets)

## Project Documentation

**For understanding project evolution and architecture:**

- **Release Notes** (`docs/releases/`): Review release notes to understand version changes, features, and breaking changes
- **Project History** (`docs/PROJECT_HISTORY.md`): Read for project evolution, technical decisions, lessons learned, and architectural changes
- **Mental Models** (`docs/models/`): Study component mental models to understand system architecture, data flow, and component interactions
- **Use when**: Starting on unfamiliar components, making architectural changes, or needing historical context for decisions

---

## Development Guidelines

- For developing new features or refactoring old features, always use a **Test-Driven Development (TDD)** approach.
  - Write tests for the feature you're trying to develop
  - Develop the feature
  - Test the feature against the tests you've written earlier
  - Iterate until all tests pass.

---

## Tool Usage Guidelines

- Use the Context7 MCP for external library documentation

