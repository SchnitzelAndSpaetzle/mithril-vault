# MithrilVault Research Report — Issue #71 (Entry Favicon Download + Custom Icon Integration)

## Scope
This report documents the codebase analysis and implementation work for issue #71, with emphasis on:
- Favicon download architecture
- KDBX custom icon persistence strategy
- IPC contract changes
- Frontend integration patterns
- Settings/privacy controls
- Test and validation outcomes

It replaces the previous issue-specific research notes.

---

## 1. Code Areas Studied In Depth

### Backend (Rust)
- `src-tauri/src/services/kdbx/*`
  - Existing entry/group/database operations
  - Custom icon extraction path (`get_custom_icons`)
  - Open database in-memory state and mutation patterns
- `src-tauri/src/commands/entries.rs`
  - Existing command style and error propagation
- `src-tauri/src/commands/settings.rs`
  - `AppSettings` (flat persisted model) and `AppPreferences` (nested UI model)
  - `from_settings` / `apply_to_settings` conversion boundaries
- `src-tauri/src/lib.rs`
  - Command registration and service management

### Frontend (TypeScript/React)
- `src/lib/types.ts`
  - Zod IPC schemas and response parsing
- `src/lib/tauri.ts`
  - Command wrappers and typed invoke patterns
- Entry/edit/render flow:
  - `src/hooks/use-entry-edit-form.ts`
  - `src/components/entries/EntryEditForm.tsx`
  - `src/components/entries/entry-edit-form/EntryTitleField.tsx`
  - Icon consumers (`EntryListItem`, `EntryItemDetails`, `SearchResultItem`, `GroupTreeItem`)
- Settings UI:
  - `src/components/settings/sections/SecuritySettingsSection.tsx`
  - `src/hooks/use-app-preferences.ts`

### Tests
- Rust command and service tests under `src-tauri/tests/`
- Frontend component/hook tests under `src/components/**/__tests__` and `src/hooks/**/__tests__`

---

## 2. Existing Architecture Before #71

### What was already implemented
- Entry/group/search/list rendering already supported KDBX custom icons through `customIconUuid`.
- Standard KeePass icon selection (`iconId`) already existed in entry edit flows.
- Custom icon payload over IPC was PNG-assumed (`base64` string only), not MIME-aware.
- No favicon fetch/download pipeline existed.
- No favicon-related privacy settings existed.

### Important baseline design constraints
- Backend owns persistence and sensitive operations.
- Frontend consumes typed IPC only.
- Database write consistency relies on existing save commands; entry mutations and metadata changes follow existing modified-state patterns.

---

## 3. Design Tree and Resolved Decisions

### A. Where to store downloaded icons?
- Options considered:
  1. External filesystem icon cache
  2. KDBX-native custom icon metadata (`db.meta.custom_icons`)
- Decision: **KDBX-native custom icons**.
- Why:
  - Keeps icon portability with database files.
  - Integrates directly with existing `customIconUuid` rendering.
  - Avoids sync/migration complexity of external cache stores.

### B. How to trigger favicon fetching?
- Options considered:
  1. Inline network call during create/update entry
  2. Separate async follow-up call
- Decision: **separate async command** (`fetch_entry_favicon`), called after save in UI flows.
- Why:
  - Entry save latency remains fast/predictable.
  - Network failures do not block CRUD success.

### C. Automatic vs. manual fetching?
- Decision: **both**.
  - Auto fetch: gated by setting `autoDownloadFavicons` (default OFF).
  - Manual fetch/refresh and clear controls in entry edit.

### D. Privacy defaults and third-party fallbacks?
- Decision:
  - `autoDownloadFavicons = false` (default)
  - `allowThirdPartyFaviconFallbacks = false` (default)
- Why:
  - Prevents implicit external requests by default.
  - Keeps explicit user consent for third-party icon services.

### E. Transport and source order?
- Decision:
  - HTTPS-only fetch URLs.
  - Candidate order:
    1. `https://<host>/favicon.ico`
    2. Root-host `https://<root>/favicon.ico` fallback
    3. Third-party sources only if opt-in enabled

### F. Icon dedup and lifecycle?
- Decision:
  - Deduplicate by content hash (SHA-256) and reuse existing icon UUID.
  - No orphan icon GC in v1.

### G. IPC icon payload shape?
- Decision: move from PNG-assumed base64 string to MIME-aware object:
  - `{ mimeType: string, data: string }`
- Why:
  - Rendering is format-agnostic (PNG/JPEG/ICO/SVG/etc).
  - Supports fallback-to-original bytes when normalization is not possible.

---

## 4. Backend Implementation Details

### 4.1 New favicon service module
- Added `src-tauri/src/services/kdbx/favicons.rs`.
- New `KdbxService` methods:
  - `fetch_entry_favicon(db_id, entry_id, allow_third_party_fallbacks, force) -> Result<bool, AppError>`
  - `clear_entry_custom_icon(db_id, entry_id) -> Result<bool, AppError>`

### 4.2 Candidate generation and host strategy
- URL host extraction from entry URL.
- Candidate order built as:
  - exact host favicon
  - root-host favicon fallback
  - opt-in third-party sources (`google s2`, `icon.horse`)
- HTTPS-only for all generated source URLs.

### 4.3 Retry throttle and failure cooldown
- Added in-memory per-session cooldown map in `KdbxService`:
  - `favicon_failed_domains: Mutex<HashMap<String, Instant>>`
  - 15-minute cooldown (`FAVICON_FAILURE_COOLDOWN`)
- Behavior:
  - Failed domains are skipped during cooldown.
  - Cooldown clears on successful fetch for the attempted domain.

### 4.4 Download and normalization behavior
- HTTP client configured with connect/overall timeouts and limited redirects.
- Response guarded by:
  - status check
  - content-type check (image-like)
  - max byte size guard
  - lightweight signature checks (image/SVG)
- Normalization path:
  - If decodable image: resize/normalize to PNG 64x64.
  - If not decodable: store original bytes and detected/inferred MIME.

### 4.5 Persistence and dedup
- Icon bytes hashed (SHA-256).
- Existing `meta.custom_icons.icons` searched by hash.
- If hash matches existing icon, reuse UUID.
- If no match, insert new custom icon and assign new UUID.
- Entry links icon through `entry.custom_icon_uuid`.

### 4.6 New commands
- `src-tauri/src/commands/entries.rs`:
  - `fetch_entry_favicon`
  - `clear_entry_custom_icon`
- `src-tauri/src/lib.rs` command registration updated accordingly.

### 4.7 Settings model extension
- Added to Rust settings structs and conversions:
  - `auto_download_favicons`
  - `allow_third_party_favicon_fallbacks`
- Defaults set to `false` in `AppSettings::default()`.

---

## 5. IPC and Frontend Contract Changes

### 5.1 MIME-aware icon schema
- Rust `get_custom_icons` now returns map values with both MIME and data.
- TS schema updated:
  - `CustomIconDataSchema { mimeType, data }`
  - `CustomIconMapSchema` now record of `CustomIconData`.

### 5.2 Tauri wrapper updates
- Added wrapper methods:
  - `entries.fetchFavicon(dbId, id, force?)`
  - `entries.clearCustomIcon(dbId, id)`

### 5.3 Rendering path updates
- Consumers now build src as:
  - `data:${mimeType};base64,${data}`
- Existing fallback behavior preserved:
  - if custom icon missing/unusable, `iconId`-based icon remains.

---

## 6. Frontend Behavior and UX Integration

### 6.1 Entry save follow-up fetch (non-blocking)
- `use-entry-edit-form` now performs favicon fetch after successful create/update only when:
  - `preferences.security.autoDownloadFavicons === true`
  - URL is non-empty
- Save operation remains independent from favicon fetch result.

### 6.2 Manual entry-edit controls
- Added in title/icon section:
  - `Fetch from URL` / `Refresh favicon`
  - `Clear custom icon`
- Buttons are state-aware:
  - fetch requires edit mode + URL + not pending
  - clear requires existing custom icon

### 6.3 Persistence after icon mutation
- After successful manual/auto icon mutation path, UI calls `database.save(dbId)`.
- Related queries invalidated (`customIcons`, entry detail/list) to refresh rendering.

### 6.4 Settings UI additions
- Security settings now include toggles for:
  - auto favicon download
  - third-party fallback allowance
- i18n keys added across all supported locales.

---

## 7. Validation and Test Outcomes

### Type checking
- `bun run typecheck` passed after frontend updates.

### Frontend tests (targeted)
- Entry edit form tests expanded for:
  - manual fetch action wiring
  - manual clear action wiring
  - auto-fetch-on-save behavior when enabled
- Settings and tauri/preferences fixtures updated for new security fields.
- Icon rendering related component tests passed.

### Rust tests (targeted)
- New favicon unit tests added for deterministic behavior:
  - candidate ordering
  - opt-in fallback sources presence
  - fetch no-op for missing/invalid URL
  - icon dedup by content hash
  - clear custom icon detach behavior
- Settings command/service tests extended to verify new fields defaults and round-trip.
- Entry command regression test suite passed.

---

## 8. Risks and Known Limitations

1. Root-host fallback is currently heuristic (`last two labels`) and not full public-suffix aware registrable-domain resolution.
2. No orphan custom icon garbage collection in v1 (intentional scope limit).
3. Network fetch success path in tests is intentionally not live-network tested; unit tests cover deterministic internal logic and mutation behavior.
4. Cooldown cache is process-memory only (resets between app restarts by design).

---

## 9. Stable Implementation Conventions Learned

1. For new icon-like binary payloads over IPC, use MIME-aware shape, not format-assumed strings.
2. Keep entry/group CRUD latency isolated from network-bound enrichment; use follow-up async commands.
3. Privacy-sensitive network features should default OFF and be explicitly user-controlled.
4. KDBX custom icons are the preferred canonical store for entry icon assets tied to database portability.
5. When adding settings fields, always update all four Rust conversion points (`SecuritySettings`/`AppSettings` + `from_settings` + `apply_to_settings`) and mirror in TS Zod schemas and test fixtures.

---

## 10. Files Most Relevant for Future Favicon/Icon Work

### Backend
- `src-tauri/src/services/kdbx/favicons.rs`
- `src-tauri/src/services/kdbx/groups.rs`
- `src-tauri/src/services/kdbx/mod.rs`
- `src-tauri/src/commands/entries.rs`
- `src-tauri/src/commands/settings.rs`

### Frontend
- `src/lib/types.ts`
- `src/lib/tauri.ts`
- `src/hooks/use-entry-edit-form.ts`
- `src/components/entries/entry-edit-form/EntryTitleField.tsx`
- `src/components/settings/sections/SecuritySettingsSection.tsx`

### Tests
- `src-tauri/src/services/kdbx/favicons.rs` (unit tests)
- `src-tauri/tests/commands/settings_test.rs`
- `src-tauri/tests/services/settings_service_test.rs`
- `src-tauri/tests/services/settings_service_unitlike_test.rs`
- `src/components/entries/__tests__/EntryEditForm.test.tsx`
