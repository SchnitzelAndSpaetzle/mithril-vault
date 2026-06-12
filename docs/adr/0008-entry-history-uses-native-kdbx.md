# Entry history uses native KDBX history, not a custom store

Issue #70 sketched a custom `EntryHistory { entry_id, timestamp, snapshot, changed_fields }` struct. We instead use the native `keepass::Entry.history` (a `Vec<Entry>` of full snapshots, newest-first): it is interoperable with every KeePass app, the Merge engine already writes losing versions into it (so edit-history and merge-preservation share one substrate), and the per-entry retention limit reuses the existing `Meta.history_max_items` field that travels with the file.

The trade-off: KDBX stores no per-field change record, so the `changed_fields` line is **derived** at view time by diffing each snapshot against the next-newer version (field names only — values, including passwords, never leave Rust), and the retention limit is **vault-global** meta rather than per-item configuration. We accept both in exchange for KeePass compatibility, which is a core project goal (KDBX is the sync unit — ADR-0005).

Consequences:
- A history snapshot is pushed *before* mutating, on any change to the Entry's stored content or location, matching KeePassXC: field edits, tags (including bulk tag rename/delete — one snapshot per touched Entry), color (when added), Entry move between Groups, attachment add/delete, and custom-icon/favicon. Only pure access-time bumps are excluded. All such paths funnel through one snapshot chokepoint so coverage stays uniform as fields are added.
- Historical secrets are fetched per-version on demand (`get_history_entry_password`), mirroring the live-entry rule; the history listing carries no passwords.
- The History Limit is writable through its own vault-meta surface; the read-only Database Config snapshot is unaffected.
- `Meta.history_max_size` (byte cap) is preserved on save but not enforced in v1; items-only pruning is enforced.
