# Contributing to MithrilVault

Thank you for your interest in contributing to MithrilVault! This document provides guidelines for contributing to the project.

## Development Setup

1. Clone the repository
2. Install dependencies: `bun install`
3. Start development server: `bun run dev-desktop`

## Commit Message Guidelines

This project uses [Conventional Commits](https://www.conventionalcommits.org/) to maintain a clear history and automate changelog generation.

### Format

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

### Types

| Type | Description | Changelog Section |
|------|-------------|-------------------|
| `feat` | New feature | Features |
| `fix` | Bug fix | Bug Fixes |
| `docs` | Documentation | Documentation |
| `style` | Formatting (no code change) | (hidden) |
| `refactor` | Code restructure | (hidden) |
| `perf` | Performance improvement | Performance |
| `test` | Tests | (hidden) |
| `build` | Build system | (hidden) |
| `ci` | CI changes | (hidden) |
| `chore` | Maintenance | (hidden) |
| `security` | Security fix | Security |

### Scopes (Optional)

- `core` - KDBX functionality
- `ui` - User interface
- `cli` - Command line
- `extension` - Browser extension
- `sync` - Cloud sync
- `deps` - Dependencies

### Examples

```
feat(core): add KDBX4 file reading support

fix(ui): resolve entry list not updating after delete

security(core): patch vulnerability in password handling

feat!: redesign database unlock flow

BREAKING CHANGE: New unlock API, see migration guide.
```

### Breaking Changes

Mark breaking changes by:
- Adding `!` after the type: `feat!: new feature`
- Adding a `BREAKING CHANGE:` footer in the commit body

### Commit Template

To use the project's commit template:

```bash
git config commit.template .gitmessage
```

## Pull Request Guidelines

- PR titles must follow the same conventional commit format
- Link related issues in the PR description
- Ensure all checks pass before requesting review

## Code Style

- Run `bun run lint` before committing
- Run `bun run format` to format code
- Write tests for new functionality

## Security

If you discover a security vulnerability, please report it privately rather than opening a public issue.
