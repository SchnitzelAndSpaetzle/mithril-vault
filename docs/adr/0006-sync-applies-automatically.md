# Sync Applies Automatically: Fast-Forward While Locked, Auto-Merge with History, No Blocking Prompts

ADR-0005 made the KDBX file the sync unit. This ADR decides how an arriving file version is *applied* on the receiving Device. It deliberately amends a security consideration written into issue #138 — "never silently overwrite; always prompt user (#59)" — so the reasoning is recorded here.

Two facts constrain the design:

1. **Merging requires decryption.** Reconciling two diverged KDBX files means opening both, which needs the master password. A locked Device can receive a file but cannot merge it.
2. **KDBX already has a loss-free conflict mechanism.** Per-entry modification times plus per-entry history mean a two-way, entry-level merge (the proven KeePassXC algorithm, including `DeletedObjects` for deletions) can resolve same-entry conflicts newest-wins while retaining the losing version in the entry's history — restorable in any KeePass app.

## Decision

- **Triggers:** sync runs on save (push to reachable paired Devices) and on encounter (a paired Device appears; version markers compared, divergence reconciled). No polling, no schedules. A manual "Sync now" exists only as reassurance/diagnostics. Cloud-Folder Sync flows through the identical state machine, with the file watcher as the arrival signal.
- **Fast-Forward:** if the local file is unchanged since the last sync point with that peer (hash match) **and the incoming version descends from the local one**, the incoming file strictly supersedes it and replaces it on disk — even while the Vault is locked, always preceded by a pre-replace backup (#61 machinery). No decryption needed. Descent is proven on the LAN path: the sender's recorded last-sync point with this Device must equal this Device's current file hash; if it doesn't (e.g. a peer restored from an old backup offering a stale sibling), the arrival is treated as a Pending Merge even though the local copy is clean — a clean local file alone must never authorize a rollback. On the cloud-folder path there is no sender to interrogate, so a provider-side restore of an old version is indistinguishable from a normal external change; the pre-replace backup is the accepted residual safety net there.
- **Security-posture check before every Fast-Forward:** KDF parameters and cipher settings live in the KDBX4 *plaintext outer header*, readable without the master password. Before any fast-forward replacement — including on the locked path — the incoming file's outer header is compared to the local one; a changed security posture is never applied silently. The arrival is held and surfaced explicitly instead, mirroring the merge carve-out below.
- **Pending Merge:** if both sides diverged, the incoming file is stored as a pending copy beside the Vault, a "changes waiting — unlock to merge" indicator is shown, and the merge runs on next unlock. Sync never prompts for a master password on its own.
- **Merge is automatic and non-blocking:** entry-level, newest-wins, loser preserved in entry history. Afterwards a non-blocking Merge Summary reports what combined and what conflicted, with restore-from-history as the undo. No blocking conflict dialogs.
- **Security-posture carve-out:** merge never auto-applies changes to KDF parameters or other master-key-affecting metadata; those are surfaced explicitly before taking effect.
- Every sync application (fast-forward or merge) is recorded as an Audit Event of kind `vault.sync_applied`, carrying `method` (`fast_forward | merge`) and `source` (the peer Device's label, or `cloud_folder` for watcher-delivered arrivals). The kind is added to the Audit Event Kind taxonomy in CONTEXT.md, which is deliberately a closed list.

## Considered Options

- **Automatic apply + history as the safety net (chosen).** "Silently overwrite" mischaracterizes timestamp-merge-with-history: nothing is destroyed; the losing version sits in the entry's KDBX history. A blocking prompt would protect against a loss that does not occur, and at merge time the user rarely knows better than newest-wins anyway — the app can only say "two versions exist," not which is right. Automatic apply is also what keeps multi-device sync seamless for non-technical participants (a relative's device must never interrogate them about a conflict in an entry they didn't edit).
- **Blocking resolution dialog on every same-entry conflict (issue #138's original wording) — rejected.** Maximally explicit, but it turns sync encounters into interrogations on both Devices, lets divergence pile up while a human is away, and trains users to dread (or click through) the dialog. The one place explicitness genuinely matters — vault security posture — is kept as a carve-out instead of gating all merges.

## Consequences

- A Device's vault file can change with no user action (fast-forward while locked). This is inherent to seamless sync — and identical to what a cloud-synced folder already does — but it makes two things non-negotiable: the pre-replace backup, and an Audit Event for every sync application.
- Issue #59's "merge conflict resolution UI" is reinterpreted: it is a post-merge *review* surface (Merge Summary + restore-from-history), not a pre-merge gate. Issue #138's "always prompt" consideration is superseded by this ADR.
- The fast-forward/pending-merge split requires recording a per-peer last-sync point (file hash) — the only sync-state metadata a Device must persist beyond the file itself. The descent proof reuses the same record from the sender's side: version markers exchanged on encounter carry it.
