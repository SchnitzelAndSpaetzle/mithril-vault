# Auto-Lock After Inactivity Timeout - Research Report

## GitHub Issue #39

**Goal:** Implement automatic database locking after a period of user inactivity.

---

## Current State Analysis

### What Already Exists

#### Backend (Rust)

| Component | File | Line(s) | Status |
|-----------|------|---------|--------|
| `auto_lock_timeout: u32` setting | `src-tauri/src/commands/settings.rs` | 109, 135 | Default 300s (5 min) |
| `SecuritySettings.auto_lock_timeout` | `src-tauri/src/commands/settings.rs` | 59 | DTO field present |
| `AppPreferences` conversion | `src-tauri/src/commands/settings.rs` | 166, 202 | Reads/writes correctly |
| `lock_database` command stub | `src-tauri/src/commands/database.rs` | 92-98 | Returns `NotImplemented` |
| `unlock_database` command stub | `src-tauri/src/commands/database.rs` | 104-110 | Returns `NotImplemented` |
| `OpenDatabase.password` | `src-tauri/src/domain/kdbx.rs` | 10 | `Option<SecureString>` for re-auth |
| `OpenDatabase.keyfile_path` | `src-tauri/src/domain/kdbx.rs` | 11 | `Option<String>` for re-auth |
| `DatabaseInfo.is_locked` DTO field | `src-tauri/src/dto/database.rs` | 11 | Always hardcoded `false` |
| `ClipboardService` timeout pattern | `src-tauri/src/services/clipboard.rs` | 24-50 | `tokio::spawn` + `AtomicU64` generation |
| Tokio with `time` + `rt` features | `src-tauri/Cargo.toml` | 43 | Already available |
| `AppError::NotImplemented` variant | `src-tauri/src/dto/error.rs` | 92-93 | Used by lock/unlock stubs |
| `build_database_key()` helper | `src-tauri/src/services/kdbx/key.rs` | 6-25 | Builds key from password + keyfile |
| Commands registered in `lib.rs` | `src-tauri/src/lib.rs` | 37-89 | `lock_database` and `unlock_database` already in handler list |

#### Frontend (TypeScript/React)

| Component | File | Line(s) | Status |
|-----------|------|---------|--------|
| `autoLockTimeout` Zod schema | `src/lib/types.ts` | 221 | `z.number().int().positive()` |
| `SecuritySettings` type | `src/lib/types.ts` | 220-228 | Includes `autoLockTimeout` |
| `DatabaseInfo.isLocked` Zod field | `src/lib/types.ts` | 9 | `z.boolean()` |
| Settings UI input for timeout | `src/components/settings/sections/SecuritySettingsSection.tsx` | 30-54 | Min 30s, functional |
| `useClipboardTimeout` hook pattern | `src/hooks/use-clipboard-timeout.ts` | 1-14 | Reusable pattern |
| `useAppPreferences` hook | `src/hooks/use-app-preferences.ts` | 8-51 | Query + mutation |
| `SHORTCUTS.lockDatabase` (Ctrl+L) | `src/lib/shortcuts.ts` | 43-48 | Defined and in shortcut groups |
| Lock shortcut handler (desktop) | `src/components/layout/drag-region.tsx` | 188-211 | Currently CLOSES db + removes tab |
| Lock shortcut handler (mobile) | `src/views/MobileContentArea.tsx` | 71-93 | Currently CLOSES db + removes tab |
| Lock button handler (sidebar) | `src/components/layout/database-switcher.tsx` | 50-69 | Currently CLOSES db + removes tab |
| `DatabaseTabState` type | `src/stores/database-tabs.ts` | 6 | Only `"unlocking" \| "open"` |
| `useDatabaseTabs` Zustand store | `src/stores/database-tabs.ts` | 52-127 | No "locked" state |
| `useActiveDatabase` hook | `src/hooks/use-active-database.ts` | 9-42 | Returns `isUnlocking` only |
| `__root.tsx` title checks `isLocked` | `src/routes/__root.tsx` | 55-56 | Already handles locked title |
| Tauri window API | `src/routes/__root.tsx` | 2 | `getCurrentWindow` imported |
| `UnlockDbForm` component | `src/components/security/unlock-database-form/UnlockDbForm.tsx` | 77-361 | Full unlock flow, opens fresh |
| Unlock route | `src/routes/(auth)/unlock.tsx` | 1-67 | Redirects if tab is "open" |
| Dashboard route guard | `src/routes/dashboard/index.$dbId.tsx` | 17-38 | Redirects to unlock if "unlocking" |
| i18n auto-lock note | `src/locales/en/common.json` | 53 | Says "TODO: not implemented yet" |
| `clearClipboardOnLock` setting | Multiple files | -- | Functional, clears clipboard before close |

### What Does NOT Exist Yet

1. **Activity tracking system** -- No user interaction monitoring (mouse, keyboard, scroll)
2. **Backend lock/unlock logic** -- Only stubs returning `NotImplemented`
3. **AutoLockService** -- No background timer checking inactivity
4. **`DatabaseLocked` error variant** -- Not in `AppError` enum
5. **Locked tab state** -- `"locked"` not in `DatabaseTabState`
6. **Lock-in-place behavior** -- Current "lock" actually closes the database and removes the tab
7. **Re-unlock from locked state** -- `UnlockDbForm` only handles fresh opens, not re-unlock
8. **Tauri event emission** -- No `database-locked` events from backend to frontend
9. **System sleep detection** -- No handling of OS sleep/resume events
10. **`database.lock()` / `database.unlock()` / `database.reportActivity()`** in `src/lib/tauri.ts`

---

## Architecture Deep Dive

### Database Lifecycle (Current)

```
User selects file -> /unlock route -> UnlockDbForm
  -> database.open(path, password) -> Tauri IPC -> open_database command
  -> KdbxService.open() -> reads file, decrypts, stores in HashMap as OpenDatabase
  -> returns DatabaseInfo -> updateTabInfo(tabId, info) -> tab state "open"
  -> navigate to /dashboard/index/$dbId

User clicks lock button or Ctrl+L:
  -> clipboard.clear() (if clearClipboardOnLock)
  -> database.close(dbId) -> Tauri IPC -> close_database command
  -> KdbxService.close() -> removes from HashMap (Database + password + keyfile dropped/zeroized)
  -> removeTab(tabId) -> navigate to /
```

### Database Lifecycle (Proposed)

```
Lock (manual via Ctrl+L, button, OR auto via inactivity timeout):
  -> clipboard.clear() (if clearClipboardOnLock)
  -> database.lock(dbId) -> Tauri IPC -> lock_database command
  -> KdbxService.lock() -> drops Database object (db = None), zeroizes password
  -> returns DatabaseInfo { is_locked: true }
  -> updateTabInfo(tabId, info) -> tab state becomes "locked"
  -> navigate to /unlock?path=...

Unlock from locked state:
  -> UnlockDbForm detects tab is "locked" (isLocked prop)
  -> database.unlock(path, password) -> Tauri IPC -> unlock_database command
  -> KdbxService.unlock() -> re-reads file from disk, decrypts with new password
  -> returns DatabaseInfo { is_locked: false }
  -> updateTabInfo(tabId, info) -> tab state becomes "open"
  -> navigate to /dashboard/index/$dbId

Auto-lock flow:
  Frontend: mousemove/keydown/click/scroll -> throttled (30s) -> database.reportActivity()
  Backend: AutoLockService stores last_activity AtomicU64 (epoch seconds)
  Background task (every 15s): checks now - last_activity >= timeout
  If exceeded: KdbxService.lock_all() -> emit "database-locked" event
  Frontend: listen("database-locked") -> lockTab(id) for each affected tab -> redirect
```

### OpenDatabase Struct (Proposed Change)

Current:
```rust
pub struct OpenDatabase {
    pub db: Database,
    pub path: String,
    pub is_modified: bool,
    pub password: Option<SecureString>,
    pub keyfile_path: Option<String>,
    pub version: String,
}
```

Proposed:
```rust
pub struct OpenDatabase {
    pub db: Option<Database>,           // None when locked (dropped/zeroized)
    pub path: String,
    pub is_modified: bool,
    pub password: Option<SecureString>, // None when locked (zeroized via drop)
    pub keyfile_path: Option<String>,   // Preserved across lock for re-auth
    pub version: String,                // Preserved for display
    pub name: String,                   // Cached from db.root.name for display when locked
    pub root_group_id: String,          // Cached from db.root.uuid for routing when locked
}
```

**Security guarantees when locked:**
- `Database` dropped -> all decrypted entries, groups, passwords freed from memory
- `SecureString` implements `ZeroizeOnDrop` -> password bytes overwritten with zeros
- Only metadata (path, version, name, keyfile_path) remains in memory
- All service methods that access `db` return `AppError::DatabaseLocked`

### Service Method Guards

Every method in `entries.rs`, `groups.rs`, `save.rs`, `header.rs` that accesses `open_db.db` needs a guard. The pattern using helper methods:

```rust
// On OpenDatabase:
pub fn db_or_locked(&self) -> Result<&Database, AppError> {
    self.db.as_ref().ok_or_else(|| AppError::DatabaseLocked(self.path.clone()))
}

pub fn db_mut_or_locked(&mut self) -> Result<&mut Database, AppError> {
    let path = self.path.clone();
    self.db.as_mut().ok_or_else(|| AppError::DatabaseLocked(path))
}

// Usage in service methods:
// Before: open_db.db.root
// After:  open_db.db_or_locked()?.root
```

### Files Requiring `db` -> `db_or_locked()` Changes

- `entries.rs` — ~12 methods: `list_entries`, `get_entry`, `get_entry_password`, `get_entry_protected_custom_field`, `create_entry`, `update_entry`, `delete_entry`, `move_entry`, `rename_tag`, `delete_tag` + helper fns
- `groups.rs` — ~9 methods: `list_groups`, `get_group`, `create_group`, `update_group`, `delete_group`, `move_group`, `rename_group`, `get_group_entry_counts`, `get_recycle_bin_id`
- `save.rs` — 2 methods: `save`, `save_as` (use `open_db.db.save()`)
- `header.rs` — `get_config`, `get_custom_icons`
- `open.rs` — `get_info`, `list_open_databases`
- `create.rs` — Only construction, wraps `db` in `Some(db)`, adds `name` and `root_group_id`

### AutoLockService Design

Follows the `ClipboardService` pattern:

```rust
pub struct AutoLockService {
    last_activity: Arc<AtomicU64>,  // Unix epoch seconds
}
```

Methods:
- `new()` — Sets initial timestamp to current time
- `report_activity()` — Updates timestamp to now
- `seconds_since_activity() -> u64` — Returns elapsed seconds

Background task (spawned via `tokio::spawn` during app setup):
- Runs in a loop with `tokio::time::sleep(Duration::from_secs(15))`
- Reads `auto_lock_timeout` from `SettingsService`
- If `seconds_since_activity() >= timeout` AND any database is unlocked:
  - Calls `KdbxService::lock_all()`
  - Emits `"database-locked"` Tauri event with list of locked paths
  - Resets activity (prevents repeated firing)

### Frontend Activity Hook Design

```
useAutoLock() hook — mounted in __root.tsx:
  1. Registers event listeners: mousemove, keydown, click, scroll, touchstart
  2. Throttles to 30s intervals: stores lastReportRef timestamp
  3. Calls database.reportActivity() via Tauri IPC
  4. Listens for "database-locked" Tauri event
  5. On lock event: updates tab state via lockTab(), navigates to /unlock
```

### Unlock Route Changes

The unlock route (`src/routes/(auth)/unlock.tsx`) needs to handle the "locked" tab state:
- If tab exists and state is "locked": show unlock form with `isLocked` prop
- `UnlockDbForm` when `isLocked`: calls `database.unlock()` instead of `database.open()`
- Hides file selector (path is known), hides keyfile selector (stored in backend)
- On success: `updateTabInfo()` sets state to "open", navigate to dashboard

### Settings UI

The settings UI already has the auto-lock timeout field. The only change needed:
- Update the "TODO" note text to describe the actual behavior
- The existing `clearClipboardOnLock` checkbox works as-is

### System Sleep Detection

For MVP, use `tauri::RunEvent::Resumed` in the app's run callback:
```rust
tauri::RunEvent::Resumed => {
    // Lock all databases on resume from sleep
    kdbx.lock_all();
    emit("database-locked", locked_paths);
}
```

This fires when the app resumes from system sleep on supported platforms.

---

## Key Patterns to Follow

### Timeout Pattern (from ClipboardService)
- `Arc<AtomicU64>` for thread-safe counter
- `tokio::spawn` for async background task
- Generation/timestamp comparison for cancellation
- Service registered via `Arc::new()` + `app.manage()`

### Settings Hook Pattern (from useClipboardTimeout)
- Simple hook wrapping `useAppPreferences()`
- Fallback constant value
- Returns raw value for consumers

### Tab State Pattern (from useDatabaseTabs)
- Zustand store with typed state
- State machine: `"unlocking" -> "open" -> "locked" -> "unlocking" (re-unlock) -> "open"`
- `updateTabInfo` drives state based on `DatabaseInfo.isLocked`

### Tauri Event Pattern
- Backend: `app_handle.emit("event-name", &payload)`
- Frontend: `listen<PayloadType>("event-name", (event) => { ... })`
- Cleanup: `unlisten` on unmount via effect cleanup

---

## Test Coverage Strategy (>70% target)

### Backend Tests (Rust)

| Test | File | Priority |
|------|------|----------|
| `AutoLockService::new` sets timestamp | `services/auto_lock.rs` | High |
| `report_activity` updates timestamp | `services/auto_lock.rs` | High |
| `seconds_since_activity` increases | `services/auto_lock.rs` | High |
| `KdbxService::lock` drops db and password | `services/kdbx/open.rs` (or integration) | Critical |
| `KdbxService::lock` preserves metadata | `services/kdbx/open.rs` | Critical |
| `KdbxService::unlock` re-opens successfully | `services/kdbx/open.rs` | Critical |
| `KdbxService::unlock` rejects wrong password | `services/kdbx/open.rs` | Critical |
| Locked db rejects entry queries | `services/kdbx/entries.rs` | High |
| Locked db rejects group queries | `services/kdbx/groups.rs` | High |
| `lock_all` locks multiple databases | `services/kdbx/open.rs` | Medium |
| Lock idempotent (double lock OK) | `services/kdbx/open.rs` | Medium |

### Frontend Tests (TypeScript/Vitest)

| Test | File | Priority |
|------|------|----------|
| `useAutoLockTimeout` returns setting | `hooks/__tests__/use-auto-lock-timeout.test.ts` | High |
| `useAutoLockTimeout` returns fallback | `hooks/__tests__/use-auto-lock-timeout.test.ts` | High |
| Activity events trigger reportActivity | `hooks/__tests__/use-auto-lock.test.ts` | High |
| Throttle prevents rapid-fire calls | `hooks/__tests__/use-auto-lock.test.ts` | High |
| Tab state includes "locked" | Store tests | Medium |
| Settings UI auto-lock note updated | `settings/__tests__/SettingsView.test.tsx` | Low |

---

## Security Considerations

1. **Password zeroization:** `SecureString` uses `ZeroizeOnDrop` derive — setting `password = None` triggers drop which zeros memory.
2. **Database memory:** Dropping `keepass::Database` frees decrypted entries. The `secstr::SecStr` used internally for protected values also zeroizes. Unprotected values (titles, URLs) remain in freed pages until OS-level overwrite.
3. **Race conditions:** `Mutex` on the `HashMap` ensures no concurrent read can access the `Database` between the lock check and the drop.
4. **Trust boundary:** Backend enforces timeout. Frontend reports activity but cannot prevent locking. A stopped/frozen frontend results in earlier locking (more secure).
5. **Keyfile-only databases:** The keyfile path is preserved across lock. On unlock, the user still sees the unlock form but only needs to click "Unlock" — the backend has the keyfile. For password-based auth, the user must re-enter the password.

---

## Compatibility Notes

- **No new npm dependencies** — `@tauri-apps/api` already includes the `event` module
- **No new Rust crate dependencies** — `tokio` with `time`+`rt` and `AtomicU64` already available
- **Backward compatible settings** — `auto_lock_timeout` already defaults to 300 in `AppSettings::default()`
- **Command signature change** — `lock_database` and `unlock_database` change from returning `()` to `DatabaseInfo`. Since they currently return `NotImplemented`, no existing consumers depend on the old return type.
