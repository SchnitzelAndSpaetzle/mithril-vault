# MithrilVault — Domain Language

This file names the concepts the codebase is *about*. Terms here are load-bearing — they appear in module names, function names, and conversations. If a term drifts, fix it here first.

Architecture vocabulary (Module, Interface, Seam, Adapter, Depth) is separate and lives in `.claude/skills/improve-codebase-architecture/LANGUAGE.md`.

## Storage

### Vault
A KDBX-format password database (KDBX3 or KDBX4) that MithrilVault has opened. Identified by its filesystem path *within one device's open-vaults map*; a Sync-Enabled Vault additionally carries a device-independent Sync ID (see Sync). May be **locked** (encrypted, no decrypted state in memory) or **unlocked** (decrypted KDBX tree is live in memory and editable). Multiple Vaults can be open at once, keyed by path.

Access to an unlocked Vault always happens inside a scoped callback: `KdbxService::with_vault` (read) or `with_vault_mut` (write). The callback receives a `Vault<'_>` / `VaultMut<'_>` handle that owns the databases-map lock for the duration of the call and exposes the tree-query and mutation API (`find_entry`, `find_group_id`, `entry_mut`, `ensure_recycle_bin`, …). After a successful mutation, callers explicitly call `vault.mark_modified()` to flip the Vault's dirty flag.

This pattern is load-bearing for unlocked KDBX tree reads and mutations. Entry, group, custom-icon, and favicon code goes through `with_vault[_mut]`; database lifecycle and metadata operations such as create, open, lock, unlock, save, and header inspection may still access `KdbxService::lock_databases` / `OpenDatabase` directly.

### Entry
A single credential record inside a Vault. Has a title, URL, username, password, optional notes, tags, custom fields, and an icon. Password material is read separately from the rest (see CLAUDE.md "minimal data in list views").

### Entry Expiry
A user-chosen flag + timestamp on an Entry (KDBX `Times.expires` + `Times.expiry`) marking when the Entry is considered stale. It is a property of the **Entry**, not of the password field — editing the password does not reset or clear it; only the user changes the date. It is explicit and one-shot, not periodic rotation (periodic rotation stays deliberately absent per NIST, see Password Health). An Entry whose `expires` flag is set and whose `expiry` is in the past is **Expired**; this is the same condition that drives the `password.expired` Password Health Finding.

### Attachment
A file stored inside a Vault and presented as belonging to a single Entry — a named binary blob (filename + bytes) the user attached to that Entry. An Entry may have many Attachments.

Modeled to the user as **per-Entry and private**: adding the same file to two Entries is two independent Attachments, and deleting one never affects the other. The KDBX format actually stores blobs in a Vault-level pool that Entries reference (and `keepass-rs` dedups identical bytes across that pool), but that sharing is an invisible storage optimization — it never surfaces in the API or UI. All Attachment operations are scoped to an Entry; there is no global attachment manager.

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

## Sync

Cross-device synchronization of Vaults (issues #138, #302). The unit of sync is the KDBX file itself — see ADR-0005.

Sync moves **ciphertext only**. A Vault's master password (and keyfile, if any) never travels through sync — it is exchanged human-to-human, out-of-band. Receiving a Shared Vault makes a Device hold the encrypted file; unlocking it still requires the secret obtained outside the app.

Two transports exist, sharing one merge engine:
- **LAN Sync** — paired Devices on the same local network discover each other and exchange the Vault file directly. No infrastructure of any kind. Devices not on the same network simply stay divergent until they next meet; the entry-level merge makes that safe.
- **Cloud-Folder Sync** — the Vault file lives in a folder synced by a cloud provider's own client (iCloud, Dropbox, Google Drive, OneDrive). Sharing with another person uses the provider's folder-sharing; MithrilVault never talks to the cloud — it watches the local file, reloads, and merges when the provider's client changes it. The provider sees only ciphertext.

Internet P2P (relay-assisted) is explicitly out of scope for v1.

Sync is alive exactly while the app is: **no daemon, no tray service** on desktop — the app's network surface exists only when the user can see the app running. Two Devices sync when both have the app open on the same LAN; otherwise they stay divergent until they next overlap, which the merge model makes safe.

Platform scope: LAN Sync ships desktop-first (macOS, Linux, Windows); mobile participates via Cloud-Folder Sync until LAN Sync is ported. On mobile, sync is **foreground-only by design** — no background tasks, ever. This is a deliberate stance (battery, simplicity, platform-restriction immunity), not a temporary gap: a mobile Device syncs when the app is open, and stays divergent otherwise, which the merge model makes safe.

### Device
One installation of MithrilVault, holding its own identity keypair. The unit of trust in sync: Vaults are shared with Devices, never with people. A person ("Dad") is at most a display label on a Device — the protocol has no concept of accounts, contacts, or persons.

### Pairing
The one-time act of establishing mutual trust between two Devices, verified by **compare-and-confirm**: both screens display a short authentication string derived from the cryptographic handshake, and both users confirm it matches (a man-in-the-middle cannot make both sides show the same code). On mobile the same material renders as a QR code instead. Pairing is mutual, and produces nothing but trust: each side persists the other's public key plus a user-editable label. It does not by itself share any Vault.

An **unpaired Device is invisible at the protocol level**: discovery reveals only "a MithrilVault instance named X is here" — never vault names, Sync IDs, or any hint of what Vaults exist. Vault metadata flows only after mutual Pairing. Device identity private keys live in the OS keychain, never in a Vault file and never on the wire.

### Sync ID
The device-independent identity of a Sync-Enabled Vault: an explicit identifier stamped into the file's KDBX4 `Meta/CustomData` when the user enables sync. Because it lives in the file, it survives every transport — LAN, cloud folder, USB stick — and other KeePass apps preserve it untouched. Filesystem path remains the *per-device* identity of an open Vault; the Sync ID is what two Devices compare to recognize "the same Vault." Deliberately forking a Vault means regenerating its Sync ID.

### Sync-Enabled Vault
A Vault the user has opted into sync, thereby stamping its Sync ID. Requires KDBX4 (`CustomData` does not exist in KDBX3); enabling sync on a KDBX3 Vault prompts a guided, explicit format upgrade first. Vaults never opted in carry no sync metadata at all.

### Sync Application
How an arriving Vault version lands on a Device. Two cases, decided by whether the local file changed since the last sync point with that peer:
- **Fast-Forward** — local copy is unchanged *and* the incoming version descends from it (on the LAN path the sender proves descent via its last-sync record; a stale sibling — e.g. a peer restored from an old backup — is demoted to Pending Merge rather than allowed to roll the Vault back). The incoming file then strictly supersedes the local one and replaces it on disk. Needs no decryption, so it applies even while the Vault is locked (always preceded by a pre-replace backup, and by a plaintext outer-header comparison so a changed security posture is never applied silently — see ADR-0006). This is what makes sync feel seamless.
- **Pending Merge** — both sides diverged; reconciling requires decrypting both copies, which requires unlock. The incoming file is stored as a pending copy beside the Vault with a "changes waiting — unlock to merge" indicator, and merges on next unlock. Sync never prompts for a master password on its own — users must not be trained to type it at unexpected moments.

Sync triggers are **on save** (push to reachable paired Devices) and **on encounter** (a paired Device appears on the network; version markers are compared and divergence reconciled). No polling, no schedules. Cloud-Folder Sync flows through the identical state machine, with the file watcher playing the role of the network arrival.

### Merge
Reconciling two diverged copies of a Sync-Enabled Vault, KeePassXC-style: a two-way, entry-level combine driven by per-entry modification times and the KDBX `DeletedObjects` list (no stored ancestor needed). Entries touched on only one side are combined trivially; an Entry edited on both sides resolves **newest-wins**, with the losing version preserved in that Entry's KDBX history — nothing is destroyed, and the history is visible in any KeePass app. Merge is automatic and non-blocking; afterwards a **Merge Summary** (non-blocking review surface) reports what combined and what conflicted, with restore-from-history as the undo path. One carve-out: changes to a Vault's security posture (KDF parameters, master-key-affecting metadata) are never auto-applied by Merge — they are surfaced explicitly.

### Shared Vault
A Sync-Enabled Vault that more than one Device syncs. There is **no owner and no access list**: each Device keeps its own local list of "peers I sync this Vault with," and that pairwise topology is the entire sharing model. Changes propagate **transitively** — if you sync with Dad and Dad syncs with Mom, Mom's edits reach you through Dad even though you never paired with her. Sharing is per-Vault, all-or-nothing (a peer receives the whole Vault, never selected Entries — selective sharing is done by organizing Entries into Vaults), and it is **irreversible delegation**: once another Device holds the file and the master password, taking it back is social, not technical. The share UX states this plainly rather than implying an enforceable boundary that whole-file sync cannot provide.

### Vault Offer
The one unsolicited message a paired Device may send: "I want to share Vault *X* (Sync ID, display name) with you." Requires an explicit, one-time accept on the receiving Device before anything is written to disk — on accept, the file lands in an app-managed sync directory by default (user-overridable, e.g. into a cloud-synced folder), and the Vault appears in the list, locked, awaiting its out-of-band master password. Decline or ignore writes nothing. After the accept, updates to that Vault flow with no further prompts, ever (see Sync Application). Everything that is not a Vault Offer is sync of an already-accepted Vault.

### Stop Sharing
Removing a peer Device from *this* Device's local peer list for one Vault. Ends direct sync of that Vault with that peer — nothing more. The peer keeps its copy, and may still receive updates **transitively** through any other peer that hasn't also dropped it.

### Unpair
Deleting a Device's identity key from this Device's trusted set. Ends all sync with it, for every Vault, until re-paired. The complement of Pairing.

### Revocation
Not a protocol primitive — there is none in the whole-file model; once a Device holds the file and the master password, access cannot be technically clawed back. "Revoking access" is a **guided remediation flow** built from the honest verbs: Stop Sharing / Unpair locally, a warning that every other peer must unpair the revoked Device too (gossip routes around a single removal — and a lost device's trusted identity key keeps receiving updates until peers drop it), a prompt to change the Vault's master password (redistributed out-of-band to remaining members), and a pointer to the Entries whose credentials the departed party saw, recommending rotation of those actual secrets.

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
- `entry.attachment_exported` — an Attachment's bytes were written to a file on disk (download); carries `entry_id` + `attachment_id` only. In-app preview, add, and delete are not audited — only leaving the Vault's encryption boundary is.
- `preferences.security_changed` — change to an allowlisted security-relevant App Preference, with `setting_name` only (no old/new values)
- `audit.cleared` — user emptied the Audit Log; the record itself survives the clear
- `vault.sync_applied` — a sync arrival changed the Vault file, with `method` (`fast_forward | merge`) and `source` (the peer Device's label, or `cloud_folder` for watcher-delivered arrivals)

Entry selection, group navigation, search, theme/language changes, and other non-security-relevant interactions are explicitly **not** Audit Event Kinds.

### Audit Retention
The policy that bounds how much history an Audit Log keeps. Two limits applied in order: an **age cutoff** (default 90 days, user-configurable 1–365 via `retentionDays`) drops events older than `now − retentionDays`; a **10 MB hard size cap** then drops the oldest survivors until under the cap. A single event larger than the cap is retained — the cap is a defense-in-depth ceiling, not a guarantee.

### Audit Compaction
The rewrite pass that enforces Audit Retention. Reads every encrypted frame under an exclusive file lock, partitions via `retention::partition_by_retention`, re-encrypts the keepers with fresh nonces, and atomically replaces the log file (temp + rename). Triggered lazily on append when the file crosses the size cap or the oldest cached timestamp falls outside the retention window; the explicit `AuditService::compact` is also directly callable.
