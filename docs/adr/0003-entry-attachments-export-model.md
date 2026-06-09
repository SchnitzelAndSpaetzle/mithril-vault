# Entry Attachments: Export-Only, No Open-In-Place

Entry Attachments are stored as native KDBX binaries (via `keepass-rs`' Vault-level binary pool) and presented as per-Entry private files. The one deliberate deviation from the obvious path is the **export model**: the only way to get an Attachment's bytes out of the Vault is an explicit, user-chosen **Download** (a save dialog → Rust writes the bytes to the path the user picked), which is audited as `entry.attachment_exported`. There is no "open in external application" action. In-app **Preview** is offered only for cheap, safe formats (raster images and plain text), never by writing to disk.

## Considered Options

- **Open-in-place via a temp file (the KeePassXC model)** — rejected for v1. KeePassXC's "Open" writes the Attachment to a temporary file and launches the OS default app. That scatters **decrypted Vault data into temp directories outside the app's control**, defeating the purpose of screen-capture protection, `SecureBytes`, and the Audit Log. Every byte that leaves the encryption boundary should be a deliberate, audited user action with no residue. The cost — viewing a PDF is "Download, then open it yourself" — is judged acceptable. If reintroduced later, it needs its own audit event and a cleanup-on-lock guarantee.
- **Surfacing the shared binary pool to the user** — rejected. KDBX stores binaries in a Vault-level pool that multiple Entries can reference, and `keepass-rs` dedups identical bytes across it. We keep that as an invisible storage optimization and model Attachments as per-Entry and private (adding the same file to two Entries is two independent Attachments; deleting one never affects the other). Matches the user's mental model and how every mainstream password manager — KeePassXC included — presents attachments.
- **Memory-protecting Attachment bytes at rest in the live tree** — rejected. The decrypted KDBX tree already lives unprotected in memory for the whole session (same as KeePassXC), so marking one blob `Value::Protected` buys almost nothing while risking round-trip surprises. Attachment blobs are stored unprotected; the Vault's own at-rest encryption covers them. The stricter-than-KeePassXC posture shows only in flight: fetched bytes cross IPC as `SecureBytes` and are dropped promptly.

## Consequences

- Attachment bytes never ride along with `get_entry` / `list_entries`; only metadata (filename, size, derived type) does. Bytes are fetched per-file on explicit Preview or Download, mirroring the `get_entry_password` lazy-reveal pattern.
- Per-file size guardrails (soft-warn + hard-cap, both configurable in App Preferences) protect save performance, because the entire Vault is re-encrypted and rewritten on every save.
- Adding open-in-place, PDF preview, or surfacing pool-sharing are all forward-compatible additions that don't require revisiting this decision.
