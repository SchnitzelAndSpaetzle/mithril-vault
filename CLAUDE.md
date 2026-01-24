# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

MithrilVault is a cross-platform password manager with full KeePass compatibility (KDBX4/KDBX3 formats). Built with Tauri v2, React, TypeScript, and Rust for desktop platforms (Linux, Windows, macOS).

## Common Commands

### Development

```bash
bun install                  # Install dependencies
bun run dev-desktop          # Start desktop dev server with hot reload
bun run dev-android          # Start Android development
bun run dev-ios              # Start iOS development
```

### Testing

```bash
bun run test                 # Run frontend tests (Vitest)
bun run test:watch           # Run tests in watch mode
bun run test:coverage        # Run tests with coverage
cd src-tauri && cargo test   # Run Rust tests
```

### Linting & Formatting

```bash
bun run lint                 # ESLint check
bun run lint:fix             # ESLint fix
bun run format               # Prettier format
bun run format:check         # Prettier check
bun run check                # Run both lint and format checks
cd src-tauri && cargo clippy # Rust linting
cd src-tauri && cargo fmt    # Rust formatting
```

### Building

```bash
bun run build                # Build frontend
bun run tauri build          # Build complete application
```

### License Checks

```bash
bun run licenses:check       # Check both JS and Rust licenses
bun run licenses:rust        # Check Rust licenses only (uses deny.toml)
```

## Architecture

### Data Flow

1. **All sensitive data operations happen in Rust** - Never decrypt passwords in JavaScript
2. **Frontend is a thin UI layer** - Displays data and captures user input
3. **IPC is the boundary** - All communication through typed Tauri commands
4. **State is derived** - Frontend state reflects backend state

### Frontend (src/)

- **React 18** with TypeScript strict mode
- **Zustand** for state management
- **Vite** for bundling
- Components organized by domain: `components/ui/`, `components/entries/`, `components/groups/`, etc.
- Tauri command wrappers in `lib/tauri.ts`
- Types in `lib/types.ts`

### Backend (src-tauri/src/)

- **Tauri v2** with security-hardened capabilities
- **Commands** (`commands/`): Thin handlers that delegate to services
- **Services** (`services/`): Business logic - KDBX operations, crypto, clipboard, keychain
- **DTOs** (`dto/`): IPC data structures - Entry, Group, Database, Error types
- **Domain** (`domain/`): Internal backend state and helpers
- Commands registered in `lib.rs`

### Tauri IPC Pattern

```rust
// Backend: src-tauri/src/commands/entries.rs
#[tauri::command]
fn get_entry(id: String, state: State<AppState>) -> Result<Entry, DatabaseError> {
    // Validate, fetch, return minimal data (no passwords in list views)
}
```

```typescript
// Frontend: src/lib/tauri.ts
export const entries = {
  async get(id: string): Promise<Entry> {
    return invoke("get_entry", { id });
  },
};
```

## Security Requirements (Critical)

- **Never log passwords, keys, or sensitive data**
- **Never expose passwords to frontend unnecessarily** - Fetch passwords separately via `get_entry_password`
- **Use `zeroize` crate** for sensitive data in Rust
- **Clipboard auto-clears** after timeout (default 30 seconds)
- **Return minimal data** in list views (no password fields in `EntryListItem`)

### Secure Memory Types

The codebase uses secure memory types that automatically zeroize on drop to prevent memory disclosure:

- **`SecureString`** - Use for all password parameters and storage. Wraps `String` with automatic zeroization.
- **`SecureBytes`** - Use for binary sensitive data like keys and keyfile contents. Wraps `Vec<u8>` with automatic zeroization.

Both types:
- Auto-zeroize memory when dropped (via `zeroize` crate with `ZeroizeOnDrop` derive)
- Print `[REDACTED]` in `Debug` and `Display` output to prevent accidental logging
- Support serde for Tauri IPC (deserialize from JSON, serialize with warning)
- Implement `Deref` for easy access to inner value

Usage example:
```rust
use crate::domain::secure::SecureString;

// Create from string
let password = SecureString::from("my-password");

// Access inner value
let password_str: &str = password.as_str();

// Debug won't leak
println!("{:?}", password); // Prints: SecureString("[REDACTED]")

// Automatically zeroized when dropped
```

Location: `src-tauri/src/domain/secure.rs`

## Code Conventions

### Rust

- Use `thiserror` for error types, always return `Result`
- No `unwrap()` or `expect()` in production code (enforced via clippy lints)
- Clippy pedantic enabled with some allows (see `Cargo.toml` lints section)

### TypeScript/React

- Functional components with TypeScript
- Components: `PascalCase`, Hooks: `use` prefix, Files: kebab-case or PascalCase
- Never use `as any`, `@ts-ignore`, or `@ts-expect-error`

### Commits

Uses Conventional Commits: `feat(scope):`, `fix(scope):`, `security(scope):`, etc.
Scopes: `core`, `ui`, `cli`, `extension`, `sync`, `deps`

## File Locking

The codebase implements a hybrid file locking mechanism to prevent concurrent database access:

### How It Works

1. **OS-level advisory locks** via `fs4` crate (cross-platform: `flock()` on Unix, `LockFileEx()` on Windows)
2. **Lock metadata files** (`.kdbx.lock`) containing PID, hostname, timestamp for stale lock detection

### Key Components

- **`FileLockService`** (`src-tauri/src/services/file_lock/mod.rs`): Core locking service
- **`LockFileInfo`**: Lock metadata (PID, hostname, timestamp, app version)
- **`LockStatus`**: Enum for lock states (Available, LockedByCurrentProcess, LockedByOtherProcess, StaleLock)
- **`FileLock`**: RAII wrapper that auto-releases lock on drop

### Usage

```rust
// Lock is automatically acquired when opening a database
let info = service.open( & path, & password) ?; // Acquires lock

// Lock is released when closing
service.close() ?; // Releases lock

// Check lock status without opening
let status = FileLockService::check_lock_status( & path) ?;

// Force unlock for recovery
FileLockService::force_unlock( & path) ?;
```

### Stale Lock Detection

- When a lock file exists but the PID is not running, it's considered stale
- Stale locks are automatically cleaned up when trying to open the database
- Uses `sysinfo` crate for cross-platform PID validation

### Test Considerations

- Tests that open database fixtures must copy them to temp directories first
- This prevents test conflicts when running in parallel with file locking enabled
- Use `copy_fixture_to_temp()` helper function in tests

## keepass-rs Crate Notes

When working with the `keepass` crate for KDBX operations:

- **Version matching**: The crate uses `rust-argon2` v3.0 internally. When configuring KDF parameters, add `rust-argon2 = "3.0"` to match (not `argon2` which is a different crate)
- **Enum casing**: Config enums use UPPERCASE variants (e.g., `OuterCipherConfig::AES256`, not `Aes256`)
- **DatabaseConfig fields**: When building custom config, include all fields including `public_custom_data: None`
- **Group creation**: Use `Group::new(name)` which auto-generates UUID, then `parent.add_child(group)`
- **Metadata**: Access via `db.meta.database_name`, `db.meta.database_description`, `db.meta.generator`

## License Compliance

MIT License. Dependencies must have compatible licenses. CI blocks incompatible licenses.
See CONTRIBUTING.md for allowed/denied license list.
