# Contributing to MithrilVault

Thank you for your interest in contributing to MithrilVault! This document provides guidelines for contributing to the project.

## Table of Contents

- [Development Setup](#development-setup)
- [Code Style Guidelines](#code-style-guidelines)
- [Commit Message Guidelines](#commit-message-guidelines)
- [Pull Request Process](#pull-request-process)
- [Testing](#testing)
- [Security](#security)

## Development Setup

### Prerequisites

Before you begin, ensure you have the following installed:

- **Node.js** (v18 or later) - [Download](https://nodejs.org/)
- **Bun** (recommended) - [Install](https://bun.sh/)
- **Rust** (1.70 or later) - [Install](https://www.rust-lang.org/tools/install)

### Platform-Specific Dependencies

#### Linux (Debian/Ubuntu)

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

#### macOS

```bash
xcode-select --install
```

#### Windows

1. Install [Microsoft Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
2. Install [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)

### Getting Started

1. **Fork and clone the repository:**

```bash
git clone https://github.com/YOUR_USERNAME/mithril-vault.git
cd mithril-vault
```

2. **Install dependencies:**

```bash
bun install
```

3. **Set up the commit template (recommended):**

```bash
git config commit.template .gitmessage
```

4. **Start the development server:**

```bash
bun run dev-desktop
```

The application will launch with hot-reload enabled.

### Available Scripts

| Command | Description |
|---------|-------------|
| `bun run dev-desktop` | Start desktop development server |
| `bun run dev-android` | Start Android development |
| `bun run dev-ios` | Start iOS development |
| `bun run build` | Build frontend |
| `bun run tauri build` | Build complete application |
| `bun run test` | Run tests |
| `bun run test:watch` | Run tests in watch mode |
| `bun run test:coverage` | Run tests with coverage report |
| `bun run lint` | Check for linting errors |
| `bun run lint:fix` | Fix linting errors automatically |
| `bun run format` | Format code with Prettier |
| `bun run format:check` | Check code formatting |
| `bun run check` | Run lint and format checks |

### IDE Setup

#### VS Code (Recommended)

Install these extensions:

- [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode)
- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
- [ESLint](https://marketplace.visualstudio.com/items?itemName=dbaeumer.vscode-eslint)
- [Prettier](https://marketplace.visualstudio.com/items?itemName=esbenp.prettier-vscode)

## Code Style Guidelines

### TypeScript/React (Frontend)

- **Formatting**: Prettier with project configuration
- **Linting**: ESLint with project configuration
- **Components**: Use functional components with TypeScript
- **Naming**:
  - Components: `PascalCase` (e.g., `EntryListItem.tsx`)
  - Hooks: `camelCase` with `use` prefix (e.g., `useDatabase.ts`)
  - Types/Interfaces: `PascalCase`
  - Constants: `SCREAMING_SNAKE_CASE` or `camelCase`

```typescript
// Good
function EntryListItem({ entry }: EntryListItemProps) {
  const { copyToClipboard } = useClipboard();
  return <div>{entry.title}</div>;
}

// Avoid
const entryListItem = (props) => {
  return <div>{props.entry.title}</div>;
};
```

### Rust (Backend)

- **Formatting**: `cargo fmt`
- **Linting**: `cargo clippy` with project lints
- **Error Handling**: Use `thiserror` for error types, return `Result`
- **Naming**:
  - Modules: `snake_case`
  - Types/Enums: `PascalCase`
  - Functions: `snake_case`
  - Constants: `SCREAMING_SNAKE_CASE`

```rust
// Good
#[tauri::command]
fn get_entry(id: String, state: State<AppState>) -> Result<Entry, DatabaseError> {
    let db = state.database.lock()?;
    db.get_entry(&id)
}

// Avoid: Don't use unwrap/expect in production code
fn get_entry(id: String) -> Entry {
    state.database.lock().unwrap().get_entry(&id).unwrap()
}
```

### Security Rules (Critical)

- **Never** log sensitive data (passwords, keys, etc.)
- **Never** expose passwords to the frontend unnecessarily
- **Always** use `zeroize` for sensitive data in Rust
- **Never** use `as any`, `@ts-ignore`, or `@ts-expect-error` to suppress type errors

See [AGENTS.md](AGENTS.md) for comprehensive security guidelines.

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

### Using the Commit Template

```bash
git config commit.template .gitmessage
```

## Pull Request Process

### Before Submitting

1. **Create an issue first** (for significant changes)
2. **Fork the repository** and create a feature branch
3. **Write tests** for new functionality
4. **Run all checks:**

```bash
bun run check        # Lint and format
bun run test         # Tests
cargo clippy         # Rust linting (in src-tauri/)
cargo fmt --check    # Rust formatting (in src-tauri/)
```

5. **Update documentation** if needed

### Branch Naming

Use descriptive branch names:

- `feat/add-totp-support`
- `fix/entry-list-refresh`
- `docs/update-contributing`

### PR Title

PR titles must follow the conventional commit format:

```
feat(ui): add dark mode toggle
fix(core): resolve database corruption on save
docs: update installation instructions
```

### PR Description

Include in your PR description:

- **Summary**: What does this PR do?
- **Related Issue**: Link to the issue (e.g., `Closes #123`)
- **Testing**: How was this tested?
- **Screenshots**: For UI changes

### Review Process

1. All PRs require at least one approving review
2. All CI checks must pass
3. Address review feedback promptly
4. Squash commits when merging (if requested)

## Testing

### Frontend Tests

```bash
bun run test              # Run once
bun run test:watch        # Watch mode
bun run test:coverage     # With coverage
```

Write tests using Vitest and Testing Library:

```typescript
import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';

describe('EntryListItem', () => {
  it('renders entry title', () => {
    render(<EntryListItem entry={mockEntry} />);
    expect(screen.getByText('My Entry')).toBeInTheDocument();
  });
});
```

### Rust Tests

```bash
cd src-tauri
cargo test
```

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_generator() {
        let password = generate_password(16);
        assert_eq!(password.len(), 16);
    }
}
```

## Security

If you discover a security vulnerability, **do not** open a public issue. Please see our [Security Policy](SECURITY.md) for responsible disclosure guidelines.

## Questions?

- Open a [Discussion](https://github.com/SchnitzelAndSpaetzle/mithril-vault/discussions) for questions
- Check existing [Issues](https://github.com/SchnitzelAndSpaetzle/mithril-vault/issues) for known problems

Thank you for contributing to MithrilVault!
