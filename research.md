# Research: Issue #32 Settings/Preferences (App + Database Split)

## 1. Executive Summary

Issue #32 requested a full settings/preferences page with persistence, categories, reset behavior, and a clear structure
for future growth.

This repository now implements a real settings system with a strict split:

1. **Application settings**: editable and persisted through Rust `SettingsService`
2. **Database settings**: read-only view from `get_database_config`, with explicit TODOs for missing mutation flows

The implementation intentionally keeps Rust as the source of truth and does not migrate to `tauri-plugin-store`.

## 2. Architecture Findings (Before)

### 2.1 Backend state

Before this issue:

- `src-tauri/src/services/settings.rs` already persisted `settings.json`
- Existing settings focused on:
  - `auto_lock_timeout`
  - `clipboard_clear_timeout`
  - `show_password_by_default`
  - `minimize_to_tray`
  - `start_minimized`
  - `theme`
  - `recent_databases`
- Tests existed for persistence/recent-database behavior and IO error handling

### 2.2 Frontend state

Before this issue:

- `/settings` route existed but was placeholder scaffolding
- `AppSettingsSidebar` and `SiteSettingsHeader` were demo/template content
- No `SettingsView` or settings sections
- No dedicated frontend settings tests
- Clipboard timeout behavior in UI was hardcoded (`30`) in multiple components

### 2.3 Split gap

A true split between app-level and database-level settings was not represented in UI, even though backend already
exposed read-only DB cryptographic config.

## 3. Implementation Completed

## 3.1 Rust: expanded settings domain and preference commands

Updated files:

- `src-tauri/src/commands/settings.rs`
- `src-tauri/src/services/settings.rs`
- `src-tauri/src/lib.rs`

Key additions:

- New typed preference structures:
  - `GeneralSettings`
  - `SecuritySettings`
  - `AppearanceSettings`
  - `BrowserIntegrationSettings`
  - `AdvancedSettings`
  - `AppPreferences`
- Added `StartupBehavior` enum
- Expanded persisted `AppSettings` fields with defaults
- Added preference-focused commands:
  - `get_app_preferences`
  - `update_app_preferences`
  - `reset_app_preferences`

Design choice:

- `AppSettings` remains flat for robust compatibility with existing persisted shape.
- `AppPreferences` is a structured view mapped to/from `AppSettings`.
- `reset_app_preferences` preserves `recent_databases`.

## 3.2 Frontend: typed settings interfaces and wrappers

Updated files:

- `src/lib/types.ts`
- `src/lib/tauri.ts`
- `src/lib/query-keys.ts`

Key additions:

- `AppPreferencesSchema` + nested schemas
- New setting-related types:
  - startup behavior
  - appearance/list-column settings
  - browser integration settings
  - advanced settings
- New wrappers:
  - `settings.getPreferences()`
  - `settings.updatePreferences()`
  - `settings.resetPreferences()`
- Query keys for:
  - app preferences
  - database config snapshot

## 3.3 Frontend: settings UI and split

New/updated files:

- `src/views/SettingsView.tsx` (new)
- `src/components/settings/SettingsSection.tsx` (new)
- `src/routes/settings/index.tsx` (real settings route)
- `src/components/layout/app-settings-sidebar.tsx` (real section nav)
- `src/components/layout/site-settings-header.tsx` (real header)

Categories implemented:

- General
- Security
- Appearance
- Browser Integration
- Advanced
- Database Settings (read-only section)

Behavior implemented:

- Save settings
- Reset to defaults (preferences only)
- Database config read-only rendering when DB is open
- TODO markers for unimplemented runtime behavior

## 3.4 Immediate-effect wiring

Updated files:

- `src/App.tsx`
- `src/components/theme-provider.tsx`
- `src/components/ui/sonner.tsx`
- `src/hooks/use-app-preferences.ts` (new)
- `src/hooks/use-clipboard-timeout.ts` (new)
- `src/hooks/use-database-config.ts` (new)
- `src/components/entries/EntryItemDetails.tsx`
- `src/components/entries/PasswordGeneratorPopover.tsx`
- `src/components/layout/database-switcher.tsx`

Improvements:

- Theme provider now uses a stable app-specific storage key
- Sonner now uses local theme provider context
- Clipboard timeout in entry/password generator uses settings value (not hardcoded)
- Database switcher settings button now routes to `/settings`

## 4. Testing and Coverage Findings

### 4.1 Rust tests

Expanded and passing:

- command tests for new preference commands
- service tests for preference mapping/reset + preservation of recents

Command used:

- `cd src-tauri && cargo test --test services --test commands settings`

Result: all targeted settings tests passed.

### 4.2 Frontend tests

New tests:

- `src/components/settings/__tests__/SettingsSection.test.tsx`
- `src/components/settings/__tests__/SettingsView.test.tsx`
- `src/hooks/__tests__/use-app-settings.test.tsx`
- `src/components/entries/__tests__/PasswordGeneratorPopover.test.tsx`

Updated tests:

- `src/lib/tauri.test.ts`
- `src/components/entries/__tests__/EntryItemDetails.test.tsx`

Coverage snapshot after implementation:

- `views/SettingsView.tsx`: **93.93% statements**, **78.26% branches**, **93.90% lines**
- New settings hooks/components added for this feature are heavily covered

Commands used:

- `bun run typecheck`
- `bun run test`
- `bun run test:coverage`

All passed.

## 5. KeepassXC-Inspired Settings Split Mapping

Target split inspired by KeePassXC principles:

- **Application settings**: UX and app runtime defaults
- **Database settings**: properties tied to a specific database file/security profile

Implemented mapping:

- App side: General, Security, Appearance, Browser, Advanced (persisted)
- Database side: cryptographic configuration read-only summary (version/ciphers/compression/KDF)

Deferred mapping:

- Database mutation workflows (KDF/cipher/history/recycle bin/security policy)

## 6. Issue #32 Checklist Status

1. Create `SettingsView`: **Done**
2. Create `SettingsSection`: **Done**
3. Implement categories: **Done**
4. General settings: **Done** (with TODOs for runtime wiring)
5. Security settings: **Done**
6. Appearance settings: **Done**
7. Browser integration settings: **Done** (persisted; runtime wiring TODO)
8. Persist settings to backend: **Done** (Rust service)
9. Reset defaults: **Done** (preferences only; recents preserved)

## 7. TODO Inventory (Explicit and Persisted in Design)

1. Auto-lock behavior integration on inactivity/OS lock/focus events
2. Minimize-to-tray and start-minimized runtime window behavior wiring
3. Startup behavior execution (`open last/default database`) at app launch
4. Language/i18n runtime integration
5. Browser integration enforcement and native messaging policy execution
6. Entry list column preferences wired into actual list rendering
7. Database settings mutation commands and UI for editable DB security/config settings

## 8. Risks and Tradeoffs

1. Persisted settings now include richer fields; runtime behavior is intentionally partial for several categories.
2. Current approach prioritizes schema + persistence + explicit TODO visibility over speculative behavior wiring.
3. Rust remains the settings source of truth; this avoids split-brain between frontend storage and backend state.

## 9. Conclusion

Issue #32 is implemented with production-usable settings UI and persistence, strict app/database split, strong test
coverage for the new feature surface, and clear TODO boundaries for the next settings expansion phase.
