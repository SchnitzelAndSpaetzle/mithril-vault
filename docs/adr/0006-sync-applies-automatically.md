# Sync Applies Automatically: Fast-Forward While Locked, Auto-Merge with History, No Blocking Prompts

ADR-0005 made the KDBX file the sync unit. This ADR decides how an arriving file version is *applied* on the receiving Device. It deliberately amends a security consideration written into issue #138 — "never silently overwrite; always prompt user (#59)" — so the reasoning is recorded here.

Two facts constrain the design:

1. **Merging requires decryption.** Reconciling two diverged KDBX files means opening both, which needs the master password. A locked Device can receive a file but cannot merge it.
2. **KDBX already has a loss-free conflict mechanism.** Per-entry modification times plus per-entry history mean a two-way, entry-level merge (the proven KeePassXC algorithm, including `DeletedObjects` for deletions) can resolve same-entry conflicts newest-wins while retaining the losing version in the entry's history — restorable in any KeePass app.

## Decision

- **Triggers:** sync runs on save (push to reachable paired Devices) and on encounter (a paired Device appears; version markers compared, divergence reconciled). No polling, no schedules. A manual "Sync now" exists only as reassurance/diagnostics. Cloud-Folder Sync flows through the identical state machine, with the file watcher as the arrival signal.
- **Fast-Forward:** if the local file is unchanged since the last sync point with that peer (hash match), the incoming file strictly supersedes it and replaces it on disk — even while the Vault is locked, always preceded by a pre-replace backup (#61 machinery). No decryption needed.
- **Pending Merge:** if both sides diverged, the incoming file is stored as a pending copy beside the Vault, a "changes waiting — unlock to merge" indicator is shown, and the merge runs on next unlock. Sync never prompts for a master password on its own.
- **Merge is automatic and non-blocking:** entry-level, newest-wins, loser preserved in entry history. Afterwards a non-blocking Merge Summary reports what combined and what conflicted, with restore-from-history as the undo. No blocking conflict dialogs.
- **Security-posture carve-out:** merge never auto-applies changes to KDF parameters or other master-key-affecting metadata; those are surfaced explicitly before taking effect.
- Every sync application (fast-forward or merge) is recorded as an Audit Event.

## Considered Options

- **Automatic apply + history as the safety net (chosen).** "Silently overwrite" mischaracterizes timestamp-merge-with-history: nothing is destroyed; the losing version sits in the entry's KDBX history. A blocking prompt would protect against a loss that does not occur, and at merge time the user rarely knows better than newest-wins anyway — the app can only say "two versions exist," not which is right. Automatic apply is also what keeps multi-device sync seamless for non-technical participants (a relative's device must never interrogate them about a conflict in an entry they didn't edit).
- **Blocking resolution dialog on every same-entry conflict (issue #138's original wording) — rejected.** Maximally explicit, but it turns sync encounters into interrogations on both Devices, lets divergence pile up while a human is away, and trains users to dread (or click through) the dialog. The one place explicitness genuinely matters — vault security posture — is kept as a carve-out instead of gating all merges.

## Consequences

- A Device's vault file can change with no user action (fast-forward while locked). This is inherent to seamless sync — and identical to what a cloud-synced folder already does — but it makes two things non-negotiable: the pre-replace backup, and an Audit Event for every sync application.
- Issue #59's "merge conflict resolution UI" is reinterpreted: it is a post-merge *review* surface (Merge Summary + restore-from-history), not a pre-merge gate. Issue #138's "always prompt" consideration is superseded by this ADR.
- The fast-forward/pending-merge split requires recording a per-peer last-sync point (file hash) — the only sync-state metadata a Device must persist beyond the file itself.
