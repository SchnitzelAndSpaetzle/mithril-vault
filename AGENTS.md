# AGENTS.md - AI Coding Agent Instructions for MithrilVault

> **MithrilVault** is a KeePass-compatible password manager supporting KDBX4/KDBX3 formats.
> Built with Tauri v2 + React + TypeScript + Rust for desktop platforms (Linux, Windows, macOS).

---

## Table of Contents

1. [Project Overview](#project-overview)
2. [Architecture Overview](#architecture-overview)
3. [Tech Stack](#tech-stack)
4. [Directory Structure](#directory-structure)
5. [Security Requirements (CRITICAL)](#security-requirements-critical)
6. [Rust Backend Conventions](#rust-backend-conventions)
7. [TypeScript Frontend Conventions](#typescript-frontend-conventions)
8. [Tauri IPC Patterns](#tauri-ipc-patterns)
9. [State Management](#state-management)
10. [Testing Requirements](#testing-requirements)
11. [Common Development Tasks](#common-development-tasks)
12. [Anti-Patterns to Avoid](#anti-patterns-to-avoid)
13. [Reference Resources](#reference-resources)

---

## Project Overview

### What is MithrilVault?

MithrilVault is a cross-platform password manager that:

- Creates, opens, and saves databases in **KDBX format** (KeePass-compatible with KDBX4 and KDBX3)
- Stores sensitive information in entries organized by groups
- Provides browser integration for autofill
- Supports TOTP generation, password health reports, and cloud sync
- Runs as a desktop application (Linux, Windows, macOS) with planned mobile support

### Target Users

- Security-conscious users who want full control over their password database
- Users migrating from KeePass, KeePassXC, or other password managers
- Users who want a modern, cross-platform KeePass client

### Key Differentiators

- Modern UI built with React
- Native performance via Tauri/Rust
- Full KeePass compatibility (not a proprietary format)
- Open source and auditable

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        MithrilVault                             │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                   Frontend (React)                      │    │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────────┐    │    │
│  │  │  Views  │ │Components │  Hooks  │ │ State (Zustand)  │    │
│  │  └────┬────┘ └────┬────┘ └────┬────┘ └──────┬──────┘    │    │
│  │       └───────────┴───────────┴─────────────┘           │    │
│  │                         │                               │    │
│  │                    Tauri IPC                            │    │
│  └─────────────────────────┼───────────────────────────────┘    │
│                            │                                    │
│  ┌─────────────────────────┼───────────────────────────────┐    │
│  │                   Backend (Rust)                        │    │
│  │                         │                               │    │
│  │  ┌──────────────────────▼──────────────────────────┐    │    │
│  │  │              Tauri Commands                     │    │    │
│  │  └──────────────────────┬──────────────────────────┘    │    │
│  │                         │                               │    │
│  │  ┌──────────┐ ┌─────────▼────────┐ ┌────────────────┐   │    │
│  │  │ Crypto   │ │  KDBX Service    │ │ Secure Storage │   │    │
│  │  │ Module   │ │  (keepass-rs)    │ │ (keyring-rs)   │   │    │
│  │  └──────────┘ └──────────────────┘ └────────────────┘   │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │              Browser Extension (WebExtension)           │    │
│  │  ┌─────────┐ ┌─────────────┐ ┌───────────────────────┐  │    │
│  │  │ Popup   │ │Content Script │ Native Messaging Host │  │    │
│  │  └─────────┘ └─────────────┘ └───────────────────────┘  │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

### Data Flow Principles

1. **All sensitive data operations happen in Rust** - Never decrypt passwords in JavaScript
2. **Frontend is a thin UI layer** - It displays data and captures user input
3. **IPC is the boundary** - All communication goes through typed Tauri commands
4. **State is derived** - Frontend state reflects backend state, not the other way around

---

## Tech Stack

### Frontend

| Technology | Version | Purpose |
|------------|---------|---------|
| React | ^18.3 | UI framework |
| TypeScript | ~5.6 | Type safety |
| Vite | ^6.0 | Build tool |
| Zustand | latest | State management |
| TailwindCSS | latest | Styling |
| React Router | latest | Navigation |
| React Hook Form | latest | Form handling |
| Zod | latest | Schema validation |

### Backend (Rust)

| Crate | Purpose |
|-------|---------|
| `tauri` | Application framework |
| `keepass-rs` | KDBX file parsing (primary) |
| `keyring` | OS keychain integration |
| `serde` / `serde_json` | Serialization |
| `tokio` | Async runtime |
| `thiserror` | Error handling |
| `zeroize` | Secure memory clearing |
| `argon2` | Key derivation (if needed beyond keepass-rs) |
| `chacha20poly1305` | Encryption (if needed) |
| `rand` | Cryptographic RNG |

### Browser Extension

| Technology | Purpose |
|------------|---------|
| WebExtension API | Cross-browser compatibility |
| TypeScript | Type safety |
| Vite | Build tool |
| TweetNaCl | End-to-end encryption |

---

## Directory Structure

```
mithril-vault/
├── AGENTS.md                    # This file
├── README.md                    # Project documentation
├── CONTRIBUTING.md              # Contribution guidelines
├── LICENSE                      # License file
│
├── src/                         # Frontend React application
│   ├── main.tsx                 # Application entry point
│   ├── App.tsx                  # Root component
│   ├── index.css                # Global styles
│   │
│   ├── components/              # Reusable UI components
│   │   ├── ui/                  # Base UI components (Button, Input, etc.)
│   │   ├── layout/              # Layout components (Sidebar, Header, etc.)
│   │   ├── entries/             # Entry-related components
│   │   ├── groups/              # Group-related components
│   │   ├── database/            # Database-related components
│   │   └── security/            # Security-related components
│   │
│   ├── views/                   # Page-level components
│   │   ├── UnlockView.tsx       # Database unlock screen
│   │   ├── MainView.tsx         # Main application view
│   │   ├── SettingsView.tsx     # Settings page
│   │   └── ...
│   │
│   ├── hooks/                   # Custom React hooks
│   │   ├── useDatabase.ts       # Database operations
│   │   ├── useEntries.ts        # Entry operations
│   │   ├── useClipboard.ts      # Clipboard with auto-clear
│   │   └── ...
│   │
│   ├── stores/                  # Zustand state stores
│   │   ├── databaseStore.ts     # Database state
│   │   ├── uiStore.ts           # UI state (theme, sidebar, etc.)
│   │   └── settingsStore.ts     # User preferences
│   │
│   ├── lib/                     # Utility functions
│   │   ├── tauri.ts             # Tauri command wrappers
│   │   ├── types.ts             # TypeScript type definitions
│   │   ├── constants.ts         # Application constants
│   │   └── utils.ts             # General utilities
│   │
│   └── assets/                  # Static assets
│       ├── icons/               # Application icons
│       └── images/              # Images
│
├── src-tauri/                   # Rust backend
│   ├── Cargo.toml               # Rust dependencies
│   ├── tauri.conf.json          # Tauri configuration
│   ├── capabilities/            # Tauri v2 capabilities
│   │
│   ├── src/
│   │   ├── main.rs              # Application entry
│   │   ├── lib.rs               # Library root, Tauri setup
│   │   │
│   │   ├── commands/            # Tauri command handlers
│   │   │   ├── mod.rs
│   │   │   ├── database.rs      # Database commands
│   │   │   ├── entries.rs       # Entry commands
│   │   │   ├── groups.rs        # Group commands
│   │   │   ├── generator.rs     # Password generator
│   │   │   └── settings.rs      # Settings commands
│   │   │
│   │   ├── services/            # Business logic
│   │   │   ├── mod.rs
│   │   │   ├── kdbx.rs          # KDBX operations
│   │   │   ├── crypto.rs        # Cryptographic operations
│   │   │   ├── keychain.rs      # OS keychain integration
│   │   │   └── clipboard.rs     # Clipboard operations
│   │   │
│   │   ├── models/              # Data structures
│   │   │   ├── mod.rs
│   │   │   ├── entry.rs         # Entry model
│   │   │   ├── group.rs         # Group model
│   │   │   ├── database.rs      # Database model
│   │   │   └── error.rs         # Error types
│   │   │
│   │   └── utils/               # Utilities
│   │       ├── mod.rs
│   │       └── ...
│   │
│   └── icons/                   # Application icons
│
├── extension/                   # Browser extension
│   ├── manifest.json            # Extension manifest
│   ├── src/
│   │   ├── popup/               # Popup UI
│   │   ├── content/             # Content scripts
│   │   ├── background/          # Background service worker
│   │   └── native/              # Native messaging
│   └── ...
│
├── docs/                        # Documentation
│   ├── architecture.md
│   ├── security.md
│   └── ...
│
└── tests/                       # Integration tests
    ├── fixtures/                # Test KDBX files
    └── ...
```

---

## Security Requirements (CRITICAL)

> **This section is NON-NEGOTIABLE. Password managers handle the most sensitive user data.**

### 1. Cryptographic Operations - Rust Only

```rust
// CORRECT: All crypto in Rust
#[tauri::command]
fn decrypt_password(entry_id: &str, state: State<AppState>) -> Result<String, Error> {
    let db = state.database.lock()?;
    db.get_entry_password(entry_id)
}

// WRONG: Never expose raw encrypted data to frontend
#[tauri::command]
fn get_encrypted_data() -> Vec<u8> { /* DON'T DO THIS */ }
```

### 2. Memory Zeroization

Always use `zeroize` for sensitive data:

```rust
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Zeroize, ZeroizeOnDrop)]
struct MasterKey {
    key: Vec<u8>,
}

// Or manually
let mut password = String::from("secret");
// ... use password ...
password.zeroize();
```

### 3. Clipboard Security

- **Always** set a timeout for clipboard clearing (default: 30 seconds)
- Clear clipboard on app exit
- Use system clipboard, not custom implementation

```rust
#[tauri::command]
async fn copy_to_clipboard(text: String, timeout_secs: u64) -> Result<(), Error> {
    clipboard::set(&text)?;
    
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(timeout_secs)).await;
        clipboard::clear();
    });
    
    Ok(())
}
```

### 4. Auto-Lock

- Lock database after inactivity (configurable, default: 5 minutes)
- Lock on system sleep/screen lock
- Lock when window loses focus (optional)

### 5. Logging Rules

```rust
// NEVER log sensitive data
log::info!("Opening database: {}", path);           // OK
log::debug!("User: {}", username);                   // OK for non-sensitive
log::error!("Password: {}", password);               // NEVER DO THIS
log::debug!("Key bytes: {:?}", key_bytes);           // NEVER DO THIS
```

### 6. IPC Security

- Validate all input from frontend
- Use typed commands (not string-based)
- Return minimal data needed

```rust
// Return only what's needed, never the full entry with password
#[derive(Serialize)]
struct EntryListItem {
    id: String,
    title: String,
    username: String,
    url: Option<String>,
    // NO password field
}
```

### 7. File Permissions

- Database files should be readable only by owner (0600 on Unix)
- Key files should be read-only (0400 on Unix)

### 8. No Secrets in Code

- No hardcoded keys, passwords, or secrets
- Use environment variables for development secrets
- Never commit `.env` files

---

## Rust Backend Conventions

### Naming

```rust
// Modules: snake_case
mod database_service;

// Types: PascalCase
struct DatabaseEntry {}
enum EntryField {}

// Functions/methods: snake_case
fn open_database() {}
fn get_entry_by_id() {}

// Constants: SCREAMING_SNAKE_CASE
const MAX_PASSWORD_LENGTH: usize = 1024;
```

### Error Handling

Use `thiserror` for error types:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Failed to open database: {0}")]
    OpenFailed(String),
    
    #[error("Invalid password")]
    InvalidPassword,
    
    #[error("Entry not found: {0}")]
    EntryNotFound(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// Make it serializable for Tauri
impl serde::Serialize for DatabaseError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
```

### Tauri Commands

```rust
use tauri::State;

// Always use Result return type
#[tauri::command]
async fn open_database(
    path: String,
    password: String,
    state: State<'_, AppState>,
) -> Result<DatabaseInfo, DatabaseError> {
    // Validate input
    if path.is_empty() {
        return Err(DatabaseError::InvalidPath);
    }
    
    // Perform operation
    let db = state.kdbx_service.open(&path, &password).await?;
    
    // Return minimal info (no sensitive data)
    Ok(DatabaseInfo::from(db))
}
```

### State Management

```rust
use std::sync::Mutex;
use tauri::Manager;

pub struct AppState {
    pub database: Mutex<Option<Database>>,
    pub settings: Mutex<Settings>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            database: Mutex::new(None),
            settings: Mutex::new(Settings::default()),
        }
    }
}

// In lib.rs
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::open_database,
            commands::close_database,
            // ...
        ])
        .run(tauri::generate_context!())
        .expect("error running tauri application");
}
```

---

## TypeScript Frontend Conventions

### Naming

```typescript
// Components: PascalCase
function EntryListItem() {}

// Hooks: camelCase with 'use' prefix
function useDatabase() {}

// Types/Interfaces: PascalCase
interface DatabaseEntry {}
type EntryField = 'title' | 'username' | 'password';

// Constants: SCREAMING_SNAKE_CASE or camelCase
const MAX_PASSWORD_LENGTH = 1024;
const defaultSettings = {};

// Files: kebab-case or PascalCase for components
// entry-list-item.tsx or EntryListItem.tsx (be consistent)
```

### Component Structure

```typescript
// components/entries/EntryListItem.tsx

import { useState } from 'react';
import { Entry } from '@/lib/types';
import { useClipboard } from '@/hooks/useClipboard';

interface EntryListItemProps {
  entry: Entry;
  isSelected: boolean;
  onSelect: (id: string) => void;
}

export function EntryListItem({ entry, isSelected, onSelect }: EntryListItemProps) {
  const { copyToClipboard } = useClipboard();
  
  const handleCopyPassword = async () => {
    await copyToClipboard(entry.id, 'password');
  };
  
  return (
    <div
      className={cn('entry-item', isSelected && 'selected')}
      onClick={() => onSelect(entry.id)}
    >
      {/* ... */}
    </div>
  );
}
```

### Tauri Command Wrappers

```typescript
// lib/tauri.ts

import { invoke } from '@tauri-apps/api/core';
import type { DatabaseInfo, Entry, Group } from './types';

export const database = {
  async open(path: string, password: string): Promise<DatabaseInfo> {
    return invoke('open_database', { path, password });
  },
  
  async close(): Promise<void> {
    return invoke('close_database');
  },
  
  async save(): Promise<void> {
    return invoke('save_database');
  },
};

export const entries = {
  async list(): Promise<Entry[]> {
    return invoke('list_entries');
  },
  
  async get(id: string): Promise<Entry> {
    return invoke('get_entry', { id });
  },
  
  async getPassword(id: string): Promise<string> {
    return invoke('get_entry_password', { id });
  },
  
  // ... more commands
};
```

### Type Definitions

```typescript
// lib/types.ts

export interface DatabaseInfo {
  name: string;
  path: string;
  isModified: boolean;
  rootGroupId: string;
}

export interface Entry {
  id: string;
  groupId: string;
  title: string;
  username: string;
  url?: string;
  notes?: string;
  createdAt: string;
  modifiedAt: string;
  // Note: password is NOT included - fetched separately
}

export interface Group {
  id: string;
  parentId?: string;
  name: string;
  icon?: string;
  children: Group[];
}

export interface PasswordGeneratorOptions {
  length: number;
  uppercase: boolean;
  lowercase: boolean;
  numbers: boolean;
  symbols: boolean;
  excludeAmbiguous: boolean;
  excludeChars?: string;
}
```

---

## Tauri IPC Patterns

### Pattern 1: Simple Query

```rust
// Backend
#[tauri::command]
fn get_entry(id: String, state: State<AppState>) -> Result<Entry, Error> {
    let db = state.database.lock()?;
    db.get_entry(&id)
}
```

```typescript
// Frontend
const entry = await invoke<Entry>('get_entry', { id: entryId });
```

### Pattern 2: Mutation with Confirmation

```rust
// Backend
#[tauri::command]
fn delete_entry(id: String, state: State<AppState>) -> Result<(), Error> {
    let mut db = state.database.lock()?;
    db.delete_entry(&id)?;
    db.mark_modified();
    Ok(())
}
```

```typescript
// Frontend
async function handleDelete(id: string) {
  if (await confirm('Delete this entry?')) {
    await invoke('delete_entry', { id });
    // Refresh state
    await refetchEntries();
  }
}
```

### Pattern 3: Streaming/Events

```rust
// Backend - for long operations
#[tauri::command]
async fn import_database(
    path: String,
    window: tauri::Window,
) -> Result<(), Error> {
    let entries = parse_import_file(&path)?;
    let total = entries.len();
    
    for (i, entry) in entries.into_iter().enumerate() {
        // Process entry...
        
        // Emit progress
        window.emit("import-progress", ImportProgress {
            current: i + 1,
            total,
        })?;
    }
    
    Ok(())
}
```

```typescript
// Frontend
import { listen } from '@tauri-apps/api/event';

async function handleImport(path: string) {
  const unlisten = await listen<ImportProgress>('import-progress', (event) => {
    setProgress(event.payload);
  });
  
  try {
    await invoke('import_database', { path });
  } finally {
    unlisten();
  }
}
```

---

## State Management

### Zustand Store Pattern

```typescript
// stores/databaseStore.ts

import { create } from 'zustand';
import { database, entries } from '@/lib/tauri';
import type { DatabaseInfo, Entry, Group } from '@/lib/types';

interface DatabaseState {
  // State
  isUnlocked: boolean;
  info: DatabaseInfo | null;
  entries: Entry[];
  groups: Group[];
  selectedEntryId: string | null;
  selectedGroupId: string | null;
  searchQuery: string;
  
  // Actions
  open: (path: string, password: string) => Promise<void>;
  close: () => Promise<void>;
  save: () => Promise<void>;
  selectEntry: (id: string | null) => void;
  selectGroup: (id: string | null) => void;
  setSearchQuery: (query: string) => void;
  refreshEntries: () => Promise<void>;
}

export const useDatabaseStore = create<DatabaseState>((set, get) => ({
  // Initial state
  isUnlocked: false,
  info: null,
  entries: [],
  groups: [],
  selectedEntryId: null,
  selectedGroupId: null,
  searchQuery: '',
  
  // Actions
  open: async (path, password) => {
    const info = await database.open(path, password);
    const [entriesList, groupsList] = await Promise.all([
      entries.list(),
      groups.list(),
    ]);
    
    set({
      isUnlocked: true,
      info,
      entries: entriesList,
      groups: groupsList,
    });
  },
  
  close: async () => {
    await database.close();
    set({
      isUnlocked: false,
      info: null,
      entries: [],
      groups: [],
      selectedEntryId: null,
      selectedGroupId: null,
    });
  },
  
  save: async () => {
    await database.save();
    const info = get().info;
    if (info) {
      set({ info: { ...info, isModified: false } });
    }
  },
  
  selectEntry: (id) => set({ selectedEntryId: id }),
  selectGroup: (id) => set({ selectedGroupId: id }),
  setSearchQuery: (query) => set({ searchQuery: query }),
  
  refreshEntries: async () => {
    const entriesList = await entries.list();
    set({ entries: entriesList });
  },
}));
```

### Using the Store

```typescript
// In components
function EntryList() {
  const entries = useDatabaseStore((s) => s.entries);
  const selectedId = useDatabaseStore((s) => s.selectedEntryId);
  const selectEntry = useDatabaseStore((s) => s.selectEntry);
  
  return (
    <ul>
      {entries.map((entry) => (
        <EntryListItem
          key={entry.id}
          entry={entry}
          isSelected={entry.id === selectedId}
          onSelect={selectEntry}
        />
      ))}
    </ul>
  );
}
```

---

## Testing Requirements

### Rust Tests

```rust
// Unit tests in the same file
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_password_generator() {
        let options = GeneratorOptions {
            length: 16,
            uppercase: true,
            lowercase: true,
            numbers: true,
            symbols: false,
        };
        
        let password = generate_password(&options);
        
        assert_eq!(password.len(), 16);
        assert!(password.chars().any(|c| c.is_uppercase()));
        assert!(password.chars().any(|c| c.is_lowercase()));
        assert!(password.chars().any(|c| c.is_numeric()));
    }
}

// Integration tests in tests/ directory
// tests/kdbx_test.rs
#[test]
fn test_open_kdbx4_database() {
    let path = "tests/fixtures/test.kdbx";
    let password = "test123";
    
    let db = Database::open(path, password).unwrap();
    
    assert_eq!(db.name(), "Test Database");
    assert!(db.entries().len() > 0);
}
```

### TypeScript Tests

```typescript
// Component tests with Vitest + Testing Library
import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { EntryListItem } from './EntryListItem';

describe('EntryListItem', () => {
  it('renders entry title and username', () => {
    const entry = {
      id: '1',
      title: 'Test Entry',
      username: 'testuser',
    };
    
    render(
      <EntryListItem
        entry={entry}
        isSelected={false}
        onSelect={vi.fn()}
      />
    );
    
    expect(screen.getByText('Test Entry')).toBeInTheDocument();
    expect(screen.getByText('testuser')).toBeInTheDocument();
  });
  
  it('calls onSelect when clicked', () => {
    const onSelect = vi.fn();
    const entry = { id: '1', title: 'Test', username: 'user' };
    
    render(
      <EntryListItem
        entry={entry}
        isSelected={false}
        onSelect={onSelect}
      />
    );
    
    fireEvent.click(screen.getByRole('button'));
    
    expect(onSelect).toHaveBeenCalledWith('1');
  });
});
```

### Test File Fixtures

Keep test KDBX files in `tests/fixtures/`:
- `test-kdbx4.kdbx` - KDBX4 format test file
- `test-kdbx3.kdbx` - KDBX3 format test file
- `test-keyfile.kdbx` - Database with key file
- `test-keyfile.key` - Test key file

---

## Common Development Tasks

### Adding a New Tauri Command

1. **Define the command in Rust:**

```rust
// src-tauri/src/commands/entries.rs
#[tauri::command]
pub async fn create_entry(
    group_id: String,
    title: String,
    username: String,
    password: String,
    state: State<'_, AppState>,
) -> Result<Entry, DatabaseError> {
    let mut db = state.database.lock().map_err(|_| DatabaseError::LockFailed)?;
    let db = db.as_mut().ok_or(DatabaseError::NotOpen)?;
    
    let entry = db.create_entry(&group_id, &title, &username, &password)?;
    Ok(entry)
}
```

2. **Register the command:**

```rust
// src-tauri/src/lib.rs
.invoke_handler(tauri::generate_handler![
    commands::entries::create_entry,
    // ... other commands
])
```

3. **Create TypeScript wrapper:**

```typescript
// src/lib/tauri.ts
export const entries = {
  // ...
  async create(groupId: string, data: CreateEntryData): Promise<Entry> {
    return invoke('create_entry', {
      groupId,
      title: data.title,
      username: data.username,
      password: data.password,
    });
  },
};
```

4. **Use in component:**

```typescript
const handleCreate = async (data: CreateEntryData) => {
  const entry = await entries.create(selectedGroupId, data);
  await refreshEntries();
};
```

### Adding a New UI Component

1. Create component file in appropriate directory
2. Export from index (if using barrel exports)
3. Add types to `lib/types.ts` if needed
4. Write tests
5. Use in parent component

### Working with KDBX Entries

```rust
// Reading an entry
let entry = db.get_entry(&entry_id)?;

// Creating an entry
let new_entry = Entry::new()
    .with_title("My Entry")
    .with_username("user")
    .with_password("secret")
    .with_url("https://example.com");
db.add_entry(&group_id, new_entry)?;

// Updating an entry
db.update_entry(&entry_id, |entry| {
    entry.set_title("New Title");
    entry.set_username("new_user");
})?;

// Deleting an entry
db.delete_entry(&entry_id)?;

// Always mark as modified after changes
db.mark_modified();
```

---

## Anti-Patterns to Avoid

### Security Anti-Patterns

```typescript
// NEVER: Store password in frontend state
const [password, setPassword] = useState('');  // For the entry's password

// NEVER: Log sensitive data
console.log('Password:', password);

// NEVER: Store master password
localStorage.setItem('masterPassword', password);
```

### Architecture Anti-Patterns

```typescript
// NEVER: Decrypt in frontend
const decrypted = decrypt(entry.encryptedPassword, key);

// NEVER: Direct file access from frontend
const file = await fs.readFile('/path/to/database.kdbx');

// NEVER: Store sensitive data in URL
navigate(`/entry/${id}?password=${password}`);
```

### Performance Anti-Patterns

```typescript
// AVOID: Fetching all data on every render
useEffect(() => {
  fetchAllEntries();  // Called on every render without deps
});

// AVOID: Not memoizing expensive computations
const filtered = entries.filter(e => e.title.includes(query));  // Recalculated every render

// BETTER:
const filtered = useMemo(
  () => entries.filter(e => e.title.includes(query)),
  [entries, query]
);
```

### Code Organization Anti-Patterns

```rust
// AVOID: Giant functions
fn handle_everything(/* 20 parameters */) { /* 500 lines */ }

// AVOID: Business logic in command handlers
#[tauri::command]
fn create_entry(/* ... */) {
    // 200 lines of logic here
}

// BETTER: Thin command handlers, logic in services
#[tauri::command]
fn create_entry(data: CreateEntryData, state: State<AppState>) -> Result<Entry, Error> {
    state.entry_service.create(data)
}
```

---

## Reference Resources

### KeePass/KDBX

- [KeePass Website](https://keepass.info/)
- [KDBX4 File Format Documentation](https://palant.info/2023/03/29/documenting-keepass-kdbx4-file-format/)
- [KeePassXC Source Code](https://github.com/keepassxreboot/keepassxc)
- [keepass-rs Crate](https://crates.io/crates/keepass)

### Tauri

- [Tauri v2 Documentation](https://v2.tauri.app/)
- [Tauri Security Guidelines](https://v2.tauri.app/security/)
- [Tauri Plugin Stronghold](https://v2.tauri.app/plugin/stronghold/)

### React/TypeScript

- [React Documentation](https://react.dev/)
- [TypeScript Handbook](https://www.typescriptlang.org/docs/)
- [Zustand Documentation](https://docs.pmnd.rs/zustand/)

### Security

- [OWASP Password Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)
- [Rust Secure Coding Guidelines](https://anssi-fr.github.io/rust-guide/)

### Similar Projects

- [KeePassXC](https://github.com/keepassxreboot/keepassxc) - C++/Qt reference implementation
- [KeePassium](https://github.com/keepassium/KeePassium) - iOS Swift implementation
- [KeeWeb](https://github.com/keeweb/keeweb) - Electron/JavaScript implementation

---

## Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2024-01 | Initial version |

---

*This document should be updated as the project evolves. If you make architectural decisions or establish new patterns, please update this file.*
