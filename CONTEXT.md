# MithrilVault — Domain Language

This file names the concepts the codebase is *about*. Terms here are load-bearing — they appear in module names, function names, and conversations. If a term drifts, fix it here first.

Architecture vocabulary (Module, Interface, Seam, Adapter, Depth) is separate and lives in `.claude/skills/improve-codebase-architecture/LANGUAGE.md`.

## Storage

### Vault
A KDBX-format password database (KDBX3 or KDBX4) that MithrilVault has opened. Identified by its filesystem path. May be **locked** (encrypted, no decrypted state in memory) or **unlocked** (decrypted KDBX tree is live in memory and editable). Multiple Vaults can be open at once, keyed by path.

Access to an unlocked Vault always happens inside a scoped callback: `KdbxService::with_vault` (read) or `with_vault_mut` (write). The callback receives a `Vault<'_>` / `VaultMut<'_>` handle that owns the databases-map lock for the duration of the call and exposes the tree-query and mutation API (`find_entry`, `find_group_id`, `entry_mut`, `ensure_recycle_bin`, …). After a successful mutation, callers explicitly call `vault.mark_modified()` to flip the Vault's dirty flag.

This pattern is load-bearing for unlocked KDBX tree reads and mutations. Entry, group, custom-icon, and favicon code goes through `with_vault[_mut]`; database lifecycle and metadata operations such as create, open, lock, unlock, save, and header inspection may still access `KdbxService::lock_databases` / `OpenDatabase` directly.

### Entry
A single credential record inside a Vault. Has a title, URL, username, password, optional notes, tags, custom fields, and an icon. Password material is read separately from the rest (see CLAUDE.md "minimal data in list views").

### Group
A folder inside a Vault that contains Entries and other Groups. Forms a tree rooted at the Vault's root Group.

## Icons

KDBX 0.12+ makes built-in and custom icons mutually exclusive on an Entry — an Entry has either an Icon ID or a Custom Icon UUID, never both.

### Custom Icon
A binary image stored inside the Vault's metadata (`db.meta.custom_icons`), referenced from an Entry by UUID. Deduped within a Vault by content hash (Sha256). MIME-aware over IPC (`{ mimeType, data }`).

### Favicon
A web-sourced icon for an Entry, derived from the Entry's URL. When a favicon is downloaded, it is **stored as a Custom Icon** — there is no separate "favicon" storage. "Favicon" describes the *acquisition pipeline*, not the storage.

### Favicon Lookup
The host derivations extracted from an Entry's URL, used to enumerate where to fetch a Favicon from. Currently three fields: the exact authority (hostname + port), the hostname alone (lowercased), and the registrable root domain (PSL-derived eTLD+1, if any).

### Favicon Candidate
One attempt in the Favicon pipeline: a `fetch_url` to try and a `cooldown_domain` to record success/failure against. Multiple Candidates are derived from one Favicon Lookup — direct host, root host, and (opt-in) third-party services like Google or icon.horse.

### Favicon Cooldown
A per-process, per-`cooldown_domain` failure timestamp. Used to skip Candidates that recently failed, so a bulk auto-fetch doesn't hammer the same dead host across every Entry that uses it. Bypassed by the manual "Refetch" path (`force=true`).

### Favicon Fetch Result
The public output of the Favicon pipeline. Either `Found { bytes, mime_type, cooldown_domain }` (a successful Candidate produced usable image bytes) or `NotFound { attempted_domains }` (every Candidate failed or was skipped). The pipeline does **not** write to the Vault — the caller decides what to do with the bytes.

### Favicon Fetch Outcome
The Entry-level outcome of `fetch_entry_favicon`: `Updated`, `Unchanged`, or `NotFound`. Distinct from Favicon Fetch Result — this is what the IPC layer surfaces to the UI after the pipeline result has been reconciled with the Entry's existing icon state.

## Settings

### App Preferences
The editable user-facing settings (theme, language, clipboard timeout, screen-capture protection, favicon auto-fetch policy). Stored locally per machine. Hierarchical shape on the IPC boundary; persisted as a flat `AppSettings` shape.

### Database Config
A read-only snapshot of properties intrinsic to a specific Vault (KDF parameters, KDBX version, generator string). Surfaced by `get_database_config`. Not editable through the same path as App Preferences.

## Security

### Secure String / Secure Bytes
Wrappers around `String` / `Vec<u8>` that zeroize on drop and redact in Debug/Display output. Used for every value the code wants to keep out of accidental logs and core dumps — passwords, keyfile contents, derived keys.

## Password Health

A read-only assessment of the passwords inside an unlocked Vault. Strictly local — no network calls, no third-party services. Breach-corpus / "have-I-been-pwned" checks are explicitly **not** part of Password Health and would belong to a separate, opt-in feature. Scoped per-Vault — each open Vault has its own independent Health report; cross-Vault analysis is not a thing.

In-scope Entries: non-Recycle-Bin Entries that carry a password field (including the empty string). Entries with `password: None` (TOTP-only, attachment-only) are skipped entirely. `{REF:...}` password references are resolved against the referenced Entry before analysis, when `keepass-rs` exposes the resolved value.

### Password Health Finding Kind
The namespaced enum of recordable findings:
- `password.very_weak` — Critical. Empty password, or zxcvbn score 0 (top dictionaries, trivially guessable).
- `password.weak` — High. zxcvbn score 1 (common patterns).
- `password.reused` — High. Exact byte-equal password shared by ≥ 2 in-scope Entries in this Vault.
- `password.expired` — High. The Entry's own `expires` flag is set and `expiry_time` is in the past.

Findings are emitted independently — an Entry that is both Reused and Very Weak produces two findings, not one merged finding, because the remediations differ (regenerate this Entry vs. regenerate every Entry sharing the group). Password age ("password last changed N months ago") is **not** a Health Finding Kind — periodic rotation contradicts NIST SP 800-63B guidance and is intentionally absent from the model.

## Audit

End-user awareness of security-relevant events. Developer debugging is a separate concern and is **not** what this section covers.

### Audit Log
A per-Vault append-only stream of security-relevant events recorded on **this device**. Lives in the app's local data dir, one file per Vault, filename keyed by the SHA-256 hash of the canonicalized Vault path so the on-disk layout doesn't leak which Vaults exist. Encrypted at rest with a key from the OS keychain (via `secure_storage`). Survives Vault locks — failed-unlock attempts append to the same log.

### Audit Event
A single record in an Audit Log. Carries a UTC timestamp, an Audit Event Kind, and the minimal kind-specific fields needed to tell the story. Deliberately does **not** carry passwords, Vault paths, or Entry titles — titles are resolved at view time by joining `entry_id` against the matching unlocked Vault, so an Audit Log without its Vault is intentionally less informative.

### Audit Event Kind
The namespaced enum of recordable events:
- `vault.opened` — successful unlock
- `vault.locked` — re-lock, with `reason` (`manual | auto_lock | app_quit | screen_lock`)
- `vault.unlock_failed` — wrong password/keyfile, with `attempt_count` (resets on success, per-session)
- `entry.password_revealed` — eye-icon click on the password field
- `entry.password_copied` — clipboard write of an Entry's password
- `entry.protected_field_revealed` — reveal on a protected custom field
- `preferences.security_changed` — change to an allowlisted security-relevant App Preference, with `setting_name` only (no old/new values)
- `audit.cleared` — user emptied the Audit Log; the record itself survives the clear

Entry selection, group navigation, search, theme/language changes, and other non-security-relevant interactions are explicitly **not** Audit Event Kinds.

### Audit Retention
The policy that bounds how much history an Audit Log keeps. Two limits applied in order: an **age cutoff** (default 90 days, user-configurable 1–365 via `retentionDays`) drops events older than `now − retentionDays`; a **10 MB hard size cap** then drops the oldest survivors until under the cap. A single event larger than the cap is retained — the cap is a defense-in-depth ceiling, not a guarantee.

### Audit Compaction
The rewrite pass that enforces Audit Retention. Reads every encrypted frame under an exclusive file lock, partitions via `retention::partition_by_retention`, re-encrypts the keepers with fresh nonces, and atomically replaces the log file (temp + rename). Triggered lazily on append when the file crosses the size cap or the oldest cached timestamp falls outside the retention window; the explicit `AuditService::compact` is also directly callable.
