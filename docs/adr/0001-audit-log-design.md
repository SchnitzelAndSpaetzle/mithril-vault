# Audit log: per-device, per-vault, keychain-encrypted JSONL

We're adding an Audit Log (see CONTEXT.md "Audit") for end-user security awareness — "did anyone try to unlock my vault while I was away?", "when did the clipboard auto-copy fire?". Developer debugging is out of scope and remains a separate, unimplemented concern.

## What we decided

- **Audience:** end-user only. No mixing with developer tracing.
- **Storage:** app-local data dir, one file per Vault, filename = SHA-256 of the canonicalized Vault path. Per-device, not per-Vault-file — the threat model "what happened on this machine?" is inherently per-device.
- **Encryption at rest:** XChaCha20-Poly1305 with a key obtained from the OS keychain via the existing `secure_storage` service. Each Audit Event is its own AEAD frame (one JSONL line per record), so appends are O(1) and frames are independently decryptable.
- **Event taxonomy:** the eight kinds enumerated in `CONTEXT.md` under Audit Event Kind. Entry selection, theme changes, language changes, and navigation are deliberately not events.
- **Event fields:** minimal — timestamp, kind, and kind-specific extras. **No** Vault paths, **no** Entry titles, **no** old/new preference values. Entry titles resolved at view time against the unlocked Vault.
- **Retention:** age-based, default 90 days, user-configurable in Settings. Secondary 10 MB hard size cap as defense-in-depth. Lazy compaction on append.
- **Default state:** on by default. Single global Settings toggle to disable; disabling preserves the existing log.
- **Manual clear:** allowed; emits a surviving `audit.cleared` event so a wipe is never silent.
- **Write-failure behavior:** silent skip per event. The triggering user action always proceeds. Repeated failures flip a "degraded" indicator in Settings. The Audit Log must never become a DoS vector against the user's own Vault access.
- **Concurrency:** intra-process `Mutex` + advisory file lock (`flock` on POSIX, `LockFileEx` on Windows) inside the audit service. Self-contained; doesn't require adopting `tauri-plugin-single-instance` app-wide.
- **UI surface:** Settings → Audit Log panel, accessible any time the app is running. Vault picker sourced from `recent_databases`. Entry IDs render as UUID prefixes when the matching Vault is locked, resolved titles when unlocked.

## Considered and rejected

- **Storing the log inside the KDBX Vault.** Encrypted-by-vault-key for free, but a locked Vault can't append — failed-unlock events would have nowhere to go. Every write would also dirty the Vault, conflicting with the user's "do I have unsaved changes" model.
- **Per-vault sidecar file next to the `.kdbx`.** Travels with the Vault when copied, but leaks Entry UUIDs and unlock timestamps in plaintext to anyone with filesystem access — defeats the privacy property the Vault provides.
- **Plaintext JSONL log.** Cheapest, but the *behavioral* leak (unlock cadence, entry-access patterns, failed-attempt counts) is meaningful even though the Vault itself is encrypted. Shipping a security-awareness feature whose own data file undermines its threat model is wrong.
- **Vault-password-derived encryption key.** Strongest privacy, but pre-unlock events have no key to encrypt with, forcing a hybrid scheme. The keychain key is good enough for "an attacker with filesystem access but not OS-session access."
- **AES-256-GCM AEAD frames.** Hardware-accelerated, but a 96-bit nonce is too short for safe random-per-record generation at scale. XChaCha20-Poly1305's 192-bit nonce eliminates that concern.
- **Hard-fail (block the user action) on write failure.** Strongest log integrity but turns the Audit Log into a DoS vector — an attacker who corrupts the log file could prevent password access.
- **Global single-stream log across all Vaults.** Simpler UI picker, but requires a plaintext hash-→-name index to label streams — leaks which Vaults the user has.
- **Per-event-category disable toggles.** Considered for v1, deferred as YAGNI. Single global on/off is sufficient until users ask for finer control.
- **App-wide single-instance plugin.** A reasonable change on its own merits, but out of scope for this issue. The audit service's file lock works correctly with or without it.

## Known limits accepted for v1

- Moving a `.kdbx` to a new path orphans its Audit history (new hash → new file). Mitigation deferred: a future reconciliation tool keyed on the KDBX header UUID.
- The OS keychain key gates *reading* the log but not its existence — an attacker with filesystem access can still observe that an audit file exists for a given hash, and its size and mtime.
- Compaction (retention rewrite) is the one operation that holds the exclusive lock for the full file; rare by construction.
