# Research Notes

## Auto-Lock After Inactivity Timeout - Research Report

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
---

## MithrilVault — Codebase Research Notes (Issue #42)

This document captures what I learned about the MithrilVault codebase while implementing issue #42 ("Implement secure window mode (prevent screenshots)"). It is not a tutorial; it is a reference for whoever picks up this code next.

---

## 1. What the project is

MithrilVault is a KeePass-compatible (KDBX4 / KDBX3) cross-platform password manager built on **Tauri v2 + React 18 + TypeScript + Rust**. It targets Linux/Windows/macOS desktop today, with mobile planned. The KDBX work is done by the `keepass` crate; everything else (UI, settings, clipboard, secure storage, KDF) is in-house.

The repo layout splits cleanly:
- `src/` — React frontend (TanStack Router file-based, Zustand, React Query, Tailwind v4, shadcn/ui)
- `src-tauri/` — Rust backend (commands, services, DTOs, domain helpers)
- `src/locales/{en,de,es,fr,sr}/` — i18n bundles
- `extension/` — browser extension (not touched in this task)
- `docs/` — reference docs

Two top-level docs steer agents: `CLAUDE.md` (concise) and `AGENTS.md` (verbose, exhaustive). They overlap; AGENTS.md is the reference, CLAUDE.md is the cheat sheet.

---

## 2. Data-flow contract

The project's hard rule: **all sensitive data lives and is decrypted in Rust**. The frontend is a thin UI; it asks for data through typed Tauri commands and renders what comes back. There are no passwords in `localStorage`, no decryption in JS, no `as any`. Schemas at the IPC boundary are double-validated:
- Rust uses `serde` with `#[serde(rename_all = "camelCase")]` on every DTO.
- TypeScript wraps every `invoke` call with a Zod schema that re-parses the result.

This means adding any new command requires updating *both* the Rust struct and the matching Zod schema in `src/lib/types.ts`. The pattern is consistent enough that copying an existing namespace (e.g. `clipboard` in `src/lib/tauri.ts`) is the right move.

### Settings architecture (the key pattern for #42)

Settings are backend-owned and persisted to `$APP_LOCAL_DATA_DIR/settings.json` by `SettingsService`. The data model uses two shapes:

- **`AppSettings`** — flat, all fields at the top level. This is what hits the disk.
- **`AppPreferences`** — nested by section (`general`, `security`, `appearance`, `browserIntegration`, `advanced`). This is what the UI works with.

`AppPreferences::from_settings(settings, data_location)` and `AppPreferences::apply_to_settings(&self, settings)` shuttle data between the two shapes (`src-tauri/src/commands/settings.rs:157-224`). When you add a new field you must touch *four* spots: `SecuritySettings` (or whichever nested section), `AppSettings`, `from_settings`, and `apply_to_settings`. The default lives in `impl Default for AppSettings`.

There are three commands the frontend uses:
- `get_app_preferences` → returns nested `AppPreferences`
- `update_app_preferences(newPreferences)` → writes through and persists
- `reset_app_preferences()` → returns defaults, **but preserves `recent_databases`**

The flat `get_settings`/`update_settings` exist mainly for tests and lower-level use.

`#[serde(default)]` on `AppSettings` means a `settings.json` missing fields is filled in with defaults at load time — which is how new fields stay backwards-compatible. We rely on this for `prevent_screen_capture`: an existing user's settings file lacks the field, so they get the default `true`.

### Frontend hook

`src/hooks/use-app-preferences.ts` wraps the three commands in React Query. Stale time is 30s. The mutation uses `queryClient.setQueryData` then `invalidateQueries` to keep the cache hot. **Diff-and-side-effect logic** for things like applying window protection on toggle goes inside the mutation's `mutationFn`, comparing `queryClient.getQueryData(...)` (the previous value) against the new payload — this is how I wired the `windowProtection.setProtected` call without polluting the section component.

---

## 3. The Rust side, briefly

### Folder roles

```
src-tauri/src/
├── commands/      # Tauri command handlers — thin, delegate to services
├── services/      # Business logic (kdbx, clipboard, settings, secure_storage, …)
├── dto/           # IPC data structures (Entry, Group, AppError, …)
├── domain/        # Internal state + secure types (SecureString, SecureBytes)
├── utils/         # Crosscutting helpers
├── lib.rs         # `build_app`, `register_services`, `run`
└── main.rs        # Entry point
```

Lints in `Cargo.toml` enforce zero `unwrap`/`expect`/`panic` in production code (warn-level, but CI treats them as errors). Tests bypass with `#![allow(clippy::expect_used)]` because they're allowed to be loud.

Errors are a single `AppError` enum in `dto/error.rs` using `thiserror`. Adding a new variant is straightforward; serialization to the frontend uses a custom `impl Serialize` that emits the `Display` form.

### Service registration

Services that hold state (e.g. `KdbxService`, `ClipboardService`, `SettingsService`, `SecureStorageService`) are wrapped in `Arc` and registered in `register_services` in `lib.rs`, then injected into commands via `State<'_, Arc<T>>`.

Stateless helpers (like `WindowProtectionService` I added for #42) don't need this — they can live as a struct namespace with associated functions. Don't add them to `register_services` if there's nothing to manage.

### Tests

- Rust integration tests live in `src-tauri/tests/`. Files are split by topic and pulled into top-level `tests/services.rs` / `tests/commands.rs` via `#[path = ...]` re-exports.
- `tauri::test::mock_app()` creates a test runtime. **Crucially, it does not create real OS windows** — `set_content_protected` and similar window APIs return early on the mock runtime. We test the wiring (handle plumbing, error mapping), not the OS behavior.
- Settings tests share a `SETTINGS_TEST_LOCK` (defined in `tests/services.rs`) because they all touch the same `$APP_LOCAL_DATA_DIR/settings.json` — without the lock, parallel tests collide.
- Dependency on a real clipboard (and therefore CI-flakiness) is avoided in `services/clipboard.rs::tests` by exercising only the generation counter.

---

## 4. The frontend, briefly

### Stack notes

- **TanStack Router** with file-based routes in `src/routes/`. The root is `__root.tsx`, which already imports `getCurrentWindow` from `@tauri-apps/api/window` for `setTitle`. That's the natural mounting point for any global window-level effect.
- **Zustand** for ephemeral UI state — currently only `useDatabaseTabs` (one store, multi-tab DB selection state).
- **React Query** for everything that crosses the IPC boundary.
- **Tailwind v4** with `@tailwindcss/vite`, custom variants, and theme via `@theme inline`. Custom utilities live in `src/index.css`.
- **shadcn/ui** components (`src/components/ui/*`). Tooltips use Radix; the Tooltip wrapper auto-supplies its own `TooltipProvider`, so just rendering `<Tooltip><TooltipTrigger>...</TooltipTrigger><TooltipContent>...</TooltipContent></Tooltip>` works in any subtree without provider plumbing.
- **Forms** use react-hook-form + Zod via `standardSchemaResolver`; `Controller` is the default field wrapper. (Not used in #42, but documented in `src/components/entries/entry-edit-form/`.)

### Tauri wrapper

`src/lib/tauri.ts` exposes thin namespaces (`database`, `entries`, `groups`, `clipboard`, `settings`, `keyfile`, `secureStorage`, …). I added `windowProtection` next to them. Each namespace is a plain object with async methods that call `invoke()` and validate the result with Zod. Inputs that need shape-checking get their own `*Schema.parse(...)` call before invocation.

### i18n

`react-i18next` with five locales (`en`, `de`, `es`, `fr`, `sr`). All user-facing strings come from `src/locales/{locale}/common.json`, accessed via `useTranslation()` and `t("dot.path.key")`. The keys must exist in **every** locale file or i18next renders the key string. The Cyrillic/Latin mix in `sr/common.json` is pre-existing and tracks the original file's conventions; do not "fix" it without coordinating with the project owner.

### Test setup

- Vitest + React Testing Library + jsdom.
- `src/test/setup.ts` globally mocks `react-i18next` so `t(key)` returns the key string. This is what makes assertions like `screen.getByText("settings.security.preventScreenCapture")` work.
- For components that use mutations / queries, wrap with a fresh `QueryClient` per test (see `src/hooks/__tests__/use-app-preferences.test.tsx` for the canonical wrapper pattern).
- For Tauri commands, mock `@/lib/tauri` and replace the namespace methods with `vi.fn()`.
- A known wart: react-hook-form's `Controller` does not detect `fireEvent.change` as dirty in jsdom. Test dirty-state logic in a real browser, not via Vitest.

---

## 5. Key research finding for #42 — Tauri's built-in API

The original issue lists three platform-specific tasks (NSWindowSharingNone, SetWindowDisplayAffinity, Linux best-effort). I almost reached for `objc2`/`windows-rs`. **Tauri v2 already wraps these.**

- JS: `getCurrentWindow().setContentProtected(true)` from `@tauri-apps/api/window`
- Rust: `window.set_content_protected(true)` on `Window` / `WebviewWindow`
- macOS → `NSWindow.sharingType = NSWindowSharingNone`
- Windows → `SetWindowDisplayAffinity(WDA_EXCLUDEFROMCAPTURE)` on Win10 build 2004+, falls back to `WDA_MONITOR` (screenshots only, no recording protection) on older builds — this fallback is handled by Tauri, not us
- Linux → no-op (no compositor-level API exists)

The JS-side path requires the capability `core:window:allow-set-content-protected` in `src-tauri/capabilities/default.json`. The Rust-side path does not. I added the capability anyway to keep the option open for frontend code.

This finding is the load-bearing decision for the whole feature: it kept the dependency footprint at zero, eliminated the unsafe FFI risk, and meant the implementation collapsed into a single API call wrapped by a tiny service.

---

## 6. Implementation summary (for #42)

What I actually shipped:

### Rust
- New variant `AppError::WindowProtection(String)` in `dto/error.rs`.
- New stateless service `services/window_protection.rs` exposing `apply_to_all(handle, enabled)` and `is_supported() -> bool` (`cfg!`).
- New thin commands in `commands/window.rs`: `set_window_content_protected(enabled, app)` and `get_window_content_protection_supported() -> bool`.
- Added `prevent_screen_capture: bool` (default `true`) to both `SecuritySettings` and `AppSettings`, including the bidirectional conversion.
- `lib.rs` `setup()` now calls `apply_initial_window_protection(handle)` after `register_services`. It reads the persisted value, falls back to `true` on any error, and **never fails the app launch**.
- Capability JSON gains `core:window:allow-set-content-protected`.

### Frontend
- Zod schemas in `lib/types.ts` updated for the new field.
- New `windowProtection` namespace in `lib/tauri.ts`.
- `useAppPreferences` mutation now diffs the previous cached value of `preventScreenCapture` against the next payload and calls `windowProtection.setProtected(next)` only on change. Same diff-and-apply runs on `resetPreferences`.
- New `useWindowProtection()` hook returning `{ enabled, isSupported }`. `enabled` reads from preferences, `isSupported` is a React Query call to the backend with `staleTime: Infinity` (the value cannot change at runtime).
- New `<SecureModeIndicator />` component renders a small fixed-position shield icon in the bottom-right when protection is enabled. Tooltip text differs by `isSupported` (Linux gets a "not supported on this platform" message).
- Mounted in `__root.tsx` so it persists across all routes.
- New checkbox + helper note added to `SecuritySettingsSection` with platform-aware "(not supported on this platform)" suffix.
- All three test fixtures that build a `SecuritySettings` object updated.

### i18n
- Added `settings.security.preventScreenCapture`, `preventScreenCaptureNote`, `preventScreenCaptureUnsupported` to all five locales with proper translations.
- Added new top-level `secureMode.indicator.activeTooltip` and `secureMode.indicator.notSupportedTooltip` keys.

### Tests
- Rust: new `tests/services/window_protection_test.rs`, new `tests/commands/window_test.rs`, plus three new tests in `settings_service_test.rs` (default true, missing-field defaults true, persists across reload). Also extended an existing test in `commands/settings_test.rs` to round-trip the new field. **All 275 Rust tests pass.**
- Frontend: new `hooks/__tests__/use-window-protection.test.tsx`, new `components/layout/__tests__/secure-mode-indicator.test.tsx`, extended `use-app-preferences.test.tsx` to assert the diff-and-apply path, extended `SettingsView.test.tsx` to assert the new checkbox toggles correctly. **All 288 frontend tests pass.**

### Coverage realism
The OS code paths themselves cannot be tested — `mock_app()` does not create real platform windows. Our coverage applies to the Rust glue (handle plumbing, error mapping, `is_supported` reporting, settings round-trip) and the frontend glue (diff-and-apply logic, indicator render conditions, hook shape). Manual macOS verification (Cmd+Shift+4 capture → black region) is documented in the plan.

---

## 7. Things to be careful about (footguns I noticed)

1. **The `sr` locale mixes Cyrillic and Latin scripts** within the same file. The `settings.security.*` block is Latin; the `keyboardShortcuts.toast.*` block is Cyrillic. I followed the per-section convention; do not "normalize" without discussing with the project owner first.
2. **`SecuritySettingsSection.tsx` calls `useWindowProtection()`**, which calls `useAppPreferences()`. Tests for `SettingsView` must mock `@/hooks/use-window-protection` directly (the alternative — letting the React Query call resolve — adds flakiness with no benefit).
3. **`fireEvent.click(screen.getByText("settings.security.minimizeToTray"))`** is the common pattern for clicking shadcn checkboxes in tests. The label wraps the Checkbox primitive, so clicking the label flips the underlying input. This is how we toggle `preventScreenCapture` in the new test.
4. **`AppSettings` has `#[serde(default)]`** at the struct level. New `bool` fields silently default to `false` if you forget to set the default in `impl Default`. For security-relevant fields like `prevent_screen_capture`, always pair the new field with the explicit default in `impl Default for AppSettings` — `serde`'s "use Rust default" path here would silently flip the policy from "secure by default" to "insecure by default."
5. **`reset_app_preferences` clobbers** every preference back to `AppSettings::default()` while preserving `recent_databases`. New defaults flow naturally — but if you ever want a field to *not* be reset, you must explicitly preserve it the way `recent_databases` is preserved.
6. **The eslint config has 8 pre-existing warnings** about `react-refresh/only-export-components` in shadcn-derived files. These are not ours to fix in this branch; running `bun run check` passes despite them.
7. **`apply_initial_window_protection` deliberately does not error.** If `SettingsService` isn't registered yet, or fails to read the file, or the platform call fails, we log to `stderr` and continue. A password manager that refuses to launch when its on-by-default protection setting can't be applied is worse than one that launches with the protection failing silently — but make sure the indicator UI is visible in either case so the user can spot it.
8. **The visual indicator is `position: fixed`**. If a future modal uses a higher `z-index` than the indicator's `z-50`, it could occlude it. If that becomes a problem, raise the indicator into a portal anchored to the document root, or move it into the title-bar chrome.

---

## 8. Pointers for the next agent

- Need to add a new boolean preference? `prevent_screen_capture` is the cleanest reference. Trace it through these files in order: `commands/settings.rs` (struct + default + conversions), `tests/.../settings_*` (round-trip), `lib/types.ts` (Zod), three test fixture files (`tauri.test.ts`, `use-app-preferences.test.tsx`, `SettingsView.test.tsx`), `SecuritySettingsSection.tsx` (UI), all five locale JSON files (i18n).
- Need to add a Tauri command that touches a window? `commands/window.rs` is the model. Stay in `commands/`, never call `set_content_protected` directly from a non-window command — use `WindowProtectionService::apply_to_all` so the user's preference is honored.
- Need to react to a settings change with a side effect? Diff inside `useAppPreferences`'s `mutationFn`, not at the section component level. Read `queryClient.getQueryData(...)` *before* `setQueryData` overwrites it.
- Need to add a new Tauri service? Match `services/clipboard.rs` for stateful + `services/window_protection.rs` for stateless; stateful ones go in `register_services` in `lib.rs`, stateless ones do not.

---
