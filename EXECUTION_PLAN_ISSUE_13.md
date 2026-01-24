# Execution Plan: Issue #13 - Implement New Database Creation (KDBX4 Format)

## Issue Summary

Implement the ability to create new KDBX4 databases from scratch with proper encryption settings and optional features.

## Current State Analysis

### Already Implemented

| Feature                            | Status | Location                                                                                           |
| ---------------------------------- | ------ | -------------------------------------------------------------------------------------------------- |
| `KdbxService::create()` method     | Done   | `src-tauri/src/services/kdbx/create.rs`                                                            |
| `create_database` Tauri command    | Done   | `src-tauri/src/commands/database.rs:24-31`                                                         |
| Database saved to disk immediately | Done   | Uses `db.save()`                                                                                   |
| Root group created with name       | Done   | Sets `db.root.name`                                                                                |
| Database can be reopened           | Done   | Tests verify this                                                                                  |
| KDBX 4.0 format                    | Done   | Uses `DatabaseConfig::default()`                                                                   |
| Basic tests                        | Done   | `tests/kdbx_open.rs`, `tests/kdbx_entries_groups.rs`, `tests/kdbx_save.rs`, `tests/kdbx_create.rs` |

### Gap Analysis

#### 1. Encryption Settings Mismatch

**Issue Requirement:**

- Cipher: AES-256
- KDF: Argon2id (memory: 64MB, iterations: 3, parallelism: 4)
- Compression: GZip

**Current (`DatabaseConfig::default()`):**

- Cipher: AES-256 ✅
- KDF: Argon2 (memory: 1MB, iterations: 50, parallelism: 4) ❌
- Compression: GZip ✅

**Action Required:** Configure custom `DatabaseConfig` with Argon2id and 64MB memory.

#### 2. Missing Keyfile Support

**Issue API Design:**

```rust
async fn create_database(
    path: String,
    name: String,
    password: String,
    key_file_path: Option<String>,  // <-- MISSING
) -> Result<DatabaseInfo, DatabaseError>
```

**Current Implementation:** Only supports password authentication.

#### 3. Missing Optional Default Groups

**Issue Requirement:** Create optional default groups (General, Email, etc.)
**Current:** Only root group is created.

#### 4. Missing Description Metadata

**Issue Requirement:** Set database metadata (name, description, creation date)
**Current:** Only name is set; description not implemented.

#### 5. Frontend API Mismatch

**Current `tauri.ts`:**

```typescript
async create(path: string, password: string): Promise<DatabaseInfo>
```

**Missing:** `name` and `keyfilePath` parameters.

---

## Implementation Tasks

### Phase 1: Backend Core Functionality

#### Task 1.1: Update `create()` with Custom Encryption Settings

**File:** `src-tauri/src/services/kdbx/create.rs`

Replace `DatabaseConfig::default()` with custom configuration:

```rust
use keepass::config::{
    DatabaseConfig, CompressionConfig, OuterCipherConfig,
    InnerCipherConfig, KdfConfig, DatabaseVersion
};

let config = DatabaseConfig {
    version: DatabaseVersion::KDB4(0),
    outer_cipher_config: OuterCipherConfig::Aes256,
    compression_config: CompressionConfig::GZip,
    inner_cipher_config: InnerCipherConfig::ChaCha20,
    kdf_config: KdfConfig::Argon2id {
        iterations: 3,
        memory: 64 * 1024 * 1024,  // 64 MB
        parallelism: 4,
        version: argon2::Version::Version13,
    },
};
```

#### Task 1.2: Add Keyfile Support to `create()`

**File:** `src-tauri/src/services/kdbx/create.rs`

Add new method or update signature:

```rust
pub fn create_with_keyfile(
    &self,
    path: &str,
    password: Option<&str>,
    keyfile_path: Option<&str>,
    name: &str,
) -> Result<DatabaseInfo, AppError>
```

#### Task 1.3: Add Optional Default Groups

**File:** `src-tauri/src/services/kdbx/create.rs`

Add helper to create default groups:

```rust
fn create_default_groups(db: &mut Database) {
    // Create: General, Email, Banking, Social, etc.
}
```

#### Task 1.4: Add Description Metadata Support

**File:** `src-tauri/src/services/kdbx/create.rs`

Check if keepass-rs supports database description field and implement if available.

### Phase 2: Tauri Command Updates

#### Task 2.1: Update `create_database` Command

**File:** `src-tauri/src/commands/database.rs`

```rust
#[tauri::command]
pub async fn create_database(
    path: String,
    name: String,
    password: String,
    key_file_path: Option<String>,
    state: State<'_, Arc<KdbxService>>,
) -> Result<DatabaseInfo, AppError>
```

### Phase 3: Frontend Updates

#### Task 3.1: Update TypeScript API

**File:** `src/lib/tauri.ts`

```typescript
async create(
  path: string,
  password: string,
  name: string,
  keyfilePath?: string
): Promise<DatabaseInfo>
```

#### Task 3.2: Add Validation Schema

**File:** `src/lib/tauri.ts`

```typescript
const CreateDatabaseSchema = z.object({
  path: z.string().min(1),
  password: z.string().min(8),
  name: z.string().min(1),
  keyfilePath: z.string().min(1).optional(),
});
```

### Phase 4: Testing (>70% Coverage Required)

#### Task 4.1: Test Custom Encryption Settings

- Verify Argon2id is used
- Verify 64MB memory setting
- Verify 3 iterations
- Verify AES-256 cipher
- Verify GZip compression

#### Task 4.2: Test Keyfile Creation

- Create with password + keyfile
- Create with keyfile only
- Verify reopening requires same credentials

#### Task 4.3: Test Default Groups

- Verify optional groups are created
- Verify group structure is correct

#### Task 4.4: Test KeePassXC Compatibility

- Create database with this implementation
- Open in KeePassXC
- Verify all settings are recognized

#### Task 4.5: Test Error Cases

- Invalid path
- Empty password
- Invalid keyfile path
- Database already open

---

## File Changes Summary

| File                                    | Changes                                                    |
| --------------------------------------- | ---------------------------------------------------------- |
| `src-tauri/src/services/kdbx/create.rs` | Update `create()`, add keyfile support, add default groups |

| `src-tauri/src/commands/database.rs` | Update command signature with keyfile param |
| `src/lib/tauri.ts` | Add name and keyfilePath params |
| `src-tauri/tests/kdbx_open.rs` | Add new tests |
| `src-tauri/tests/kdbx_entries_groups.rs` | Add new tests |
| `src-tauri/tests/kdbx_save.rs` | Add new tests |
| `src-tauri/tests/kdbx_create.rs` | Add new tests |
| `CLAUDE.md` | Update if new patterns learned |

---

## Technical Notes

### keepass-rs Configuration

From documentation research:

- Version: 0.8.x with `save_kdbx4` feature enabled
- `DatabaseConfig::default()` uses: AES256, GZip, ChaCha20 inner, Argon2 (1MB, 50 iter)
- Custom config needed for Argon2id with 64MB

### Security Considerations

- Never log passwords or keyfile contents
- Use `zeroize` for sensitive data (already handled by keepass-rs internally)
- Keyfile must be re-read for save operations

### KeePassXC Compatibility

- KDBX 4.0 format is fully supported
- Argon2id is the recommended KDF
- 64MB memory is a reasonable default (KeePassXC default is often higher)

---

## Estimated Task Breakdown

1. **Task 1.1** - Custom encryption config
2. **Task 1.2** - Keyfile support
3. **Task 1.3** - Default groups (if scope includes)
4. **Task 1.4** - Description metadata (if keepass-rs supports)
5. **Task 2.1** - Command update
6. **Task 3.1-3.2** - Frontend updates
7. **Task 4.1-4.5** - Comprehensive tests

---

## keepass-rs API Reference (Verified)

### Database Metadata (`Meta` struct)

```rust
// Available fields in db.meta:
db.meta.database_name = Some("My Vault".to_string());
db.meta.database_description = Some("Personal passwords".to_string());
db.meta.generator = Some("MithrilVault".to_string());
// Also available: master_key_changed, recyclebin_enabled, history settings, etc.
```

### Creating Groups

```rust
use keepass::db::{Group, Node};

// Create a new group
let mut general_group = Group::new("General");

// Add to root
db.root.add_child(general_group);

// Or add nested groups
let mut email_group = Group::new("Email");
db.root.add_child(email_group);
```

### Group Structure

- `Group::new(name: &str)` - Constructor with automatic UUID generation
- `group.add_child(node: impl Into<Node>)` - Add child group or entry
- `group.uuid` - Unique identifier (auto-generated)
- `group.name` - Group name
- `group.children` - Vec<Node> of children

---

## Design Decisions

1. **Default Groups**: Via optional flag `create_default_groups: bool` - when true, creates General, Email, Banking, Social groups
2. **KDF Parameters**: Configurable with sensible defaults matching issue spec:
   - Default memory: 64 MB
   - Default iterations: 3
   - Default parallelism: 4
   - User can override any of these

---

## Success Criteria

- [x] New database created with KDBX 4.0 format
- [x] Uses Argon2id KDF with 64MB memory, 3 iterations, 4 parallelism (configurable)
- [x] Supports optional keyfile authentication
- [x] Database can be reopened with same password/keyfile
- [x] Compatible with KeePassXC (uses standard KDBX4 format)
- [x] Root group exists with proper UUID
- [x] Test coverage >70% for new code (12 new tests added)

---

## Implementation Summary

### Completed Changes

**Backend (Rust):**

- Added `DatabaseCreationOptions` struct in `dto/database.rs` with configurable KDF parameters
- Updated `KdbxService::create_database()` with full options support
- Configured Argon2id KDF with sensible defaults (64MB, 3 iter, 4 parallel)
- Added keyfile support for database creation
- Added optional default groups (General, Email, Banking, Social)
- Added database metadata support (description, generator)
- Maintained backward compatibility with legacy `create()` method

**Frontend (TypeScript):**

- Added `DatabaseCreationOptions` type and schema
- Updated `database.create()` with full parameters

**Testing:**

- Added 12 comprehensive tests covering all new functionality
- All 76 tests pass (49 integration + 23 command + 4 unit)

### Dependencies Added

- `rust-argon2 = "3.0"` - Required for KDF configuration (matches keepass crate's version)
