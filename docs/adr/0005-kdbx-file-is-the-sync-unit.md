# Cross-Device Sync: the KDBX File Is the Sync Unit

Cross-device sync (issues #138, #302) moves the user's Vaults between their devices — and, for shared Vaults, between people — without a central server. The first structural decision is what the protocol exchanges: the encrypted `.kdbx` file itself, or a record-level change log from which the file is derived.

The research in issue #302 recommends the change-log model: per-device append-only event logs as the authoritative sync history, with the database as a materialized view. That recommendation is **deliberately rejected** here.

## Decision

The **KDBX file is canonical and is the unit of sync**. Devices exchange whole encrypted `.kdbx` files. When both sides have diverged, the receiving side performs an **entry-level three-way merge** in the KeePassXC style, using the causal metadata KDBX already carries (per-entry modification times, location-changed times, per-entry history). There is no second source of truth — no event log, no operation journal — beside the file.

Issue #302's transport, discovery, and device-identity research (mDNS/DNS-SD, LAN-first ladder, per-device identity keys, authenticated pairing) remains applicable and is **not** rejected; only its data-model recommendation is.

## Considered Options

- **KDBX file as sync unit, entry-level merge on conflict (chosen).** The file every other KeePass app reads *is* the database; sync is just moving and reconciling it. Merge semantics ride on metadata the format already standardizes, and the algorithm is proven in the ecosystem (KeePassXC merge). External edits — a user opening the shared Vault in KeePassXC, or a cloud-folder copy landing on disk — are indistinguishable from a sync arrival and flow through the same merge path. Costs: whole-file transfer on every change (acceptable: Vaults are KB–low-MB), and conflict granularity is the entry, not the field.
- **Per-device append-only event log as sync unit (issue #302's recommendation) — rejected.** Finer-grained conflicts, lower bandwidth, better metadata hygiene over the wire. But it demotes the KDBX file from "the database" to "an export," which contradicts MithrilVault's core identity of full KeePass compatibility: an edit made by any other KeePass app bypasses the log and permanently diverges the two sources of truth, and there is no principled re-entry point. The engineering surface (log storage, signing, compaction, replay, snapshotting) is several times larger, and the workload that justifies it — high write concurrency — is absent: #302's own analysis concedes password vaults are low-concurrency.

## Consequences

- Full KeePass compatibility is preserved at every moment: a synced Vault is always a valid KDBX file openable by KeePassXC, KeePassDX, Strongbox, etc.
- The merge prerequisites scoped in issues #56–#59 (file watcher, reload prompt, conflict detection, merge-resolution UI) become the load-bearing foundation of sync, and are independently useful for plain cloud-folder sync. As of this ADR they are **not yet implemented** — issue #138's checked boxes for them are aspirational; only #61 (pre-save backup) exists.
- Conflict resolution is bounded at entry granularity: two devices editing *different* entries always auto-merge; two devices editing the *same* entry within one sync window needs the #59 resolution UI (entry history makes this loss-free).
- Bandwidth is whole-file per change. If attachments ever make Vaults large enough for this to hurt, the remedy is content-aware transfer of the file (e.g. chunking), not a change of sync unit.
