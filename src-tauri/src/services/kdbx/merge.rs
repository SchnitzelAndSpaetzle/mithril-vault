// SPDX-License-Identifier: MIT
//! The Merge engine (ADR-0005): a pure two-way, entry-level merge of two
//! diverged copies of the same Vault.
//!
//! The engine is a pure function over two in-memory database values — no
//! files, no network — so every merge scenario is a fast unit test. All
//! I/O (picking the second file, opening it, saving the result) lives at
//! the service and command edges, never here.

use crate::domain::secure::SecureString;
use crate::dto::error::AppError;
use crate::dto::merge::{MergeConflict, MergeSummary, SecurityPostureChange};
use keepass::db::{fields, Entry, EntryId, History, Times};
use keepass::Database;
use std::collections::HashSet;
use std::fs::File;

use super::backups::BackupInfo;
use super::key::build_database_key;
use super::open::map_open_error;
use super::KdbxService;

impl KdbxService {
    /// Merges a second KDBX file — a diverged copy of the open Vault that
    /// unlocks with the same credentials — into the open Vault, then saves
    /// the result through the existing pre-save backup machinery.
    ///
    /// Returns the Merge Summary plus the pre-merge backup info (when one
    /// was taken) so the command layer can emit its `backup-created` event.
    pub fn merge_from_file(
        &self,
        db_id: &str,
        source_path: &str,
    ) -> Result<(MergeSummary, Option<BackupInfo>), AppError> {
        // Snapshot credentials first so the databases lock is not held
        // across the (KDF-slow) unlock of the incoming file.
        let (password, keyfile_path) = {
            let normalized_path = Self::normalize_path(db_id);
            let databases = self.lock_databases()?;
            let open_db = databases
                .get(&normalized_path)
                .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;
            if open_db.is_locked() {
                return Err(AppError::DatabaseLocked(open_db.path.clone()));
            }
            if open_db.password.is_none() && open_db.keyfile_path.is_none() {
                return Err(AppError::NoCredentials);
            }
            (open_db.password.clone(), open_db.keyfile_path.clone())
        };

        let mut file = File::open(source_path).map_err(|e| AppError::InvalidPath(e.to_string()))?;
        let key = build_database_key(
            password.as_ref().map(SecureString::as_str),
            keyfile_path.as_deref(),
        )?;
        let incoming = Database::open(&mut file, key).map_err(map_open_error)?;

        let summary = self.with_vault_mut(db_id, |vault| {
            let outcome = merge_vaults(vault.db(), &incoming)?;
            *vault.db_mut() = outcome.merged;
            vault.mark_modified();
            Ok(outcome.summary)
        })?;

        let backup = self.save(db_id)?;
        Ok((summary, backup))
    }
}

/// Result of merging an incoming copy of a Vault into the local one.
pub struct MergeOutcome {
    /// The merged database. The local input is never mutated.
    pub merged: Database,
    /// What combined and what conflicted, for the post-merge summary.
    pub summary: MergeSummary,
}

/// Merges `incoming` into a copy of `local`, returning the merged database
/// plus a structured [`MergeSummary`]. Neither input is mutated.
pub fn merge_vaults(local: &Database, incoming: &Database) -> Result<MergeOutcome, AppError> {
    let conflicts = detect_conflicts(local, incoming);
    let mut merged = local.clone();
    // The keepass crate's KeePassXC-style merge applies the combine; the
    // Merge Summary is derived afterwards by diffing observable state
    // (local vs merged) rather than from the crate's merge log, whose
    // types the `_merge` feature does not export.
    merged
        .merge(incoming)
        .map_err(|e| AppError::Kdbx(e.to_string()))?;
    // The upstream merge only archives a diverged destination version and
    // only when the entry already had history; the engine's contract is
    // stronger — every conflict is loss-free in both directions.
    for conflict in &conflicts {
        ensure_version_in_history(&mut merged, conflict.id, &conflict.losing_version);
    }
    let mut summary = summarize(local, &merged, &conflicts);
    summary.security_posture_changes = detect_security_posture_changes(local, incoming);
    Ok(MergeOutcome { merged, summary })
}

/// Compares the security posture of the two sides. The upstream merge
/// never touches `config`, so the merged Vault structurally keeps the
/// local posture — this only *reports* the differences for explicit user
/// confirmation (the ADR-0006 carve-out).
fn detect_security_posture_changes(
    local: &Database,
    incoming: &Database,
) -> Vec<SecurityPostureChange> {
    let mut changes = Vec::new();
    if local.config.kdf_config != incoming.config.kdf_config {
        changes.push(SecurityPostureChange::Kdf);
    }
    if local.config.outer_cipher_config != incoming.config.outer_cipher_config {
        changes.push(SecurityPostureChange::OuterCipher);
    }
    if local.config.inner_cipher_config != incoming.config.inner_cipher_config {
        changes.push(SecurityPostureChange::InnerCipher);
    }
    if local.config.compression_config != incoming.config.compression_config {
        changes.push(SecurityPostureChange::Compression);
    }
    changes
}

/// A same-entry conflict detected before the combine: both sides changed
/// the entry since they diverged, so newest-wins applies and the losing
/// version must survive in history.
struct DetectedConflict {
    id: EntryId,
    title: String,
    losing_version: Entry,
}

/// An entry conflicts when it was edited on both sides: the two current
/// versions differ in content, and the losing (older) version is absent
/// from the winning side's history. If the winner's history contains the
/// loser, the winner simply descends from it — a clean one-sided update.
fn detect_conflicts(local: &Database, incoming: &Database) -> Vec<DetectedConflict> {
    let mut conflicts = Vec::new();
    for local_entry in local.iter_all_entries() {
        let id = local_entry.id();
        let Some(incoming_entry) = incoming.entry(id) else {
            continue;
        };
        let local_entry = &*local_entry;
        let incoming_entry = &*incoming_entry;
        if !entry_content_differs(local_entry, incoming_entry) {
            continue;
        }
        let incoming_is_newer = last_modification(incoming_entry) > last_modification(local_entry);
        let (winner, loser) = if incoming_is_newer {
            (incoming_entry, local_entry)
        } else {
            (local_entry, incoming_entry)
        };
        if version_in_history(winner, loser) {
            continue;
        }
        conflicts.push(DetectedConflict {
            id,
            title: winner.get(fields::TITLE).unwrap_or_default().to_string(),
            losing_version: loser.clone(),
        });
    }
    conflicts
}

fn last_modification(entry: &Entry) -> chrono::NaiveDateTime {
    entry.times.last_modification.unwrap_or_else(Times::epoch)
}

fn version_in_history(entry: &Entry, version: &Entry) -> bool {
    entry.history.as_ref().is_some_and(|history| {
        history
            .get_entries()
            .iter()
            .any(|archived| !entry_content_differs(archived, version))
    })
}

/// Appends `version` to the merged entry's history unless an equivalent
/// version is already archived, keeping the newest-first ordering the
/// KDBX history carries. Content-based dedup keeps re-merging the same
/// inputs idempotent.
fn ensure_version_in_history(merged: &mut Database, id: EntryId, version: &Entry) {
    let Some(mut entry) = merged.entry_mut(id) else {
        return;
    };
    let history = entry.history.get_or_insert_default();
    let already_archived = history
        .get_entries()
        .iter()
        .any(|archived| !entry_content_differs(archived, version));
    if already_archived {
        return;
    }
    let mut versions: Vec<Entry> = history.get_entries().clone();
    let mut version = version.clone();
    version.history = None;
    versions.push(version);
    // `History::add_entry` prepends, so inserting oldest-to-newest leaves
    // the rebuilt history newest-first like the upstream merge produces.
    versions.sort_by_key(last_modification);
    let mut rebuilt = History::default();
    for v in versions {
        rebuilt.add_entry(v);
    }
    entry.history = Some(rebuilt);
}

/// Saturating usize→u32 for summary counts.
fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// Compares two versions of the same Entry by content, ignoring
/// timestamps and history — the same notion of divergence the `KeePassXC`
/// merge uses.
fn entry_content_differs(a: &Entry, b: &Entry) -> bool {
    let mut a = a.clone();
    a.times = Times::default();
    a.history = None;
    let mut b = b.clone();
    b.times = Times::default();
    b.history = None;
    a != b
}

fn summarize(local: &Database, merged: &Database, conflicts: &[DetectedConflict]) -> MergeSummary {
    let local_ids: HashSet<EntryId> = local.iter_all_entries().map(|e| e.id()).collect();
    let conflicted_ids: HashSet<EntryId> = conflicts.iter().map(|c| c.id).collect();
    let entries_added = count(
        merged
            .iter_all_entries()
            .filter(|e| !local_ids.contains(&e.id()))
            .count(),
    );
    let merged_ids: HashSet<EntryId> = merged.iter_all_entries().map(|e| e.id()).collect();
    let entries_deleted = count(local_ids.difference(&merged_ids).count());
    let entries_updated = count(
        merged
            .iter_all_entries()
            .filter(|merged_entry| {
                !conflicted_ids.contains(&merged_entry.id())
                    && local.entry(merged_entry.id()).is_some_and(|local_entry| {
                        entry_content_differs(&local_entry, merged_entry)
                    })
            })
            .count(),
    );
    MergeSummary {
        entries_added,
        entries_updated,
        entries_deleted,
        conflicts: conflicts
            .iter()
            .map(|c| MergeConflict {
                entry_id: c.id.uuid().to_string(),
                title: c.title.clone(),
            })
            .collect(),
        security_posture_changes: Vec::new(),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime};
    use keepass::db::{fields, EntryId};
    use keepass::Database;

    /// Deterministic timestamp: minutes after a fixed base instant. The
    /// merge resolves conflicts by comparing these, so tests control them
    /// explicitly instead of relying on wall-clock `Times::now()`.
    fn ts(minutes: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 6, 1)
            .expect("valid date")
            .and_hms_opt(12, minutes, 0)
            .expect("valid time")
    }

    fn add_entry(db: &mut Database, title: &str, username: &str, at: NaiveDateTime) -> EntryId {
        let mut root = db.root_mut();
        let mut entry = root.add_entry();
        entry.set_unprotected(fields::TITLE, title);
        entry.set_unprotected(fields::USERNAME, username);
        entry.times.last_modification = Some(at);
        entry.times.location_changed = Some(at);
        entry.times.creation = Some(at);
        entry.id()
    }

    /// Applies an edit the way a history-maintaining `KeePass` app (e.g.
    /// `KeePassXC`) does: the current version is pushed into the entry's
    /// KDBX history before the new field values and modification time are
    /// applied.
    fn edit_entry_keeping_history(
        db: &mut Database,
        id: EntryId,
        username: &str,
        at: NaiveDateTime,
    ) {
        use std::ops::Deref;
        let mut entry = db.entry_mut(id).expect("entry exists");
        let previous = entry.deref().clone();
        entry.history.get_or_insert_default().add_entry(previous);
        entry.set_unprotected(fields::USERNAME, username);
        entry.times.last_modification = Some(at);
    }

    /// An Entry created on only one side combines trivially: it appears in
    /// the merged Vault and the Merge Summary counts it as added.
    #[test]
    fn entry_added_on_incoming_side_is_combined_into_merged() {
        let mut base = Database::new();
        add_entry(&mut base, "Existing", "alice", ts(0));
        let local = base.clone();
        let mut incoming = base;
        let new_id = add_entry(&mut incoming, "Netflix", "bob", ts(1));

        let outcome = merge_vaults(&local, &incoming).expect("merge succeeds");

        let merged_entry = outcome
            .merged
            .entry(new_id)
            .expect("entry added on incoming side is present in merged");
        assert_eq!(merged_entry.get(fields::TITLE), Some("Netflix"));
        assert_eq!(merged_entry.get(fields::USERNAME), Some("bob"));
        assert_eq!(outcome.summary.entries_added, 1);
    }

    /// Entries created independently on each side are combined without
    /// loss: the merged Vault contains both. Only the incoming side's
    /// addition counts as "added" — the local one was already there.
    #[test]
    fn entries_added_on_each_side_are_both_present() {
        let base = Database::new();
        let mut local = base.clone();
        let local_id = add_entry(&mut local, "Local Only", "alice", ts(1));
        let mut incoming = base;
        let incoming_id = add_entry(&mut incoming, "Incoming Only", "bob", ts(2));

        let outcome = merge_vaults(&local, &incoming).expect("merge succeeds");

        assert!(outcome.merged.entry(local_id).is_some());
        assert!(outcome.merged.entry(incoming_id).is_some());
        assert_eq!(outcome.summary.entries_added, 1);
    }

    /// An Entry edited on only one side is a clean update, not a conflict:
    /// the merged Vault carries the newer version and the Merge Summary
    /// counts it as updated with no conflicts.
    #[test]
    fn entry_edited_on_one_side_updates_without_conflict() {
        let mut base = Database::new();
        let id = add_entry(&mut base, "Netflix", "alice", ts(0));
        let local = base.clone();
        let mut incoming = base;
        edit_entry_keeping_history(&mut incoming, id, "alice@new", ts(5));

        let outcome = merge_vaults(&local, &incoming).expect("merge succeeds");

        let merged_entry = outcome.merged.entry(id).expect("entry present");
        assert_eq!(merged_entry.get(fields::USERNAME), Some("alice@new"));
        assert_eq!(outcome.summary.entries_updated, 1);
        assert!(outcome.summary.conflicts.is_empty());
        assert_eq!(outcome.summary.entries_added, 0);
    }

    /// True/false over whether some history version of the merged entry
    /// carries this username — how the tests observe "the losing version
    /// landed in KDBX history" (which is what `KeePassXC` renders).
    fn history_contains_username(db: &Database, id: EntryId, username: &str) -> bool {
        db.entry(id)
            .and_then(|e| e.history.clone())
            .is_some_and(|h| {
                h.get_entries()
                    .iter()
                    .any(|v| v.get(fields::USERNAME) == Some(username))
            })
    }

    /// The same Entry edited on both sides resolves newest-wins: the
    /// incoming side is newer, so its version wins, the local version is
    /// preserved in the Entry's history, and the conflict is reported.
    #[test]
    fn same_entry_edited_on_both_sides_newest_wins_and_loser_lands_in_history() {
        let mut base = Database::new();
        let id = add_entry(&mut base, "Netflix", "alice", ts(0));
        let mut local = base.clone();
        let mut incoming = base;
        edit_entry_keeping_history(&mut local, id, "alice@local", ts(3));
        edit_entry_keeping_history(&mut incoming, id, "alice@incoming", ts(7));

        let outcome = merge_vaults(&local, &incoming).expect("merge succeeds");

        let merged_entry = outcome.merged.entry(id).expect("entry present");
        assert_eq!(merged_entry.get(fields::USERNAME), Some("alice@incoming"));
        assert!(
            history_contains_username(&outcome.merged, id, "alice@local"),
            "losing local version must be preserved in entry history"
        );
        assert_eq!(outcome.summary.conflicts.len(), 1);
        assert_eq!(outcome.summary.conflicts[0].title, "Netflix");
        assert_eq!(outcome.summary.entries_updated, 0);
    }

    /// Mirror case: the local side is newer and wins; the incoming losing
    /// version must land in history (the upstream merge drops it — our
    /// engine guarantees loss-free conflicts regardless of direction).
    #[test]
    fn same_entry_edited_on_both_sides_local_newer_archives_incoming_version() {
        let mut base = Database::new();
        let id = add_entry(&mut base, "Netflix", "alice", ts(0));
        let mut local = base.clone();
        let mut incoming = base;
        edit_entry_keeping_history(&mut local, id, "alice@local", ts(7));
        edit_entry_keeping_history(&mut incoming, id, "alice@incoming", ts(3));

        let outcome = merge_vaults(&local, &incoming).expect("merge succeeds");

        let merged_entry = outcome.merged.entry(id).expect("entry present");
        assert_eq!(merged_entry.get(fields::USERNAME), Some("alice@local"));
        assert!(
            history_contains_username(&outcome.merged, id, "alice@incoming"),
            "losing incoming version must be preserved in entry history"
        );
        assert_eq!(outcome.summary.conflicts.len(), 1);
        assert_eq!(outcome.summary.entries_updated, 0);
    }

    /// Applies an edit the way `MithrilVault`'s own `update_entry` does
    /// today: fields and modification time change but no version is pushed
    /// into history.
    fn edit_entry_without_history(
        db: &mut Database,
        id: EntryId,
        username: &str,
        at: NaiveDateTime,
    ) {
        let mut entry = db.entry_mut(id).expect("entry exists");
        entry.set_unprotected(fields::USERNAME, username);
        entry.times.last_modification = Some(at);
    }

    /// Even when neither side maintained entry history (`MithrilVault`'s own
    /// edits don't yet), a both-sides edit must stay loss-free: the losing
    /// version still lands in the merged Entry's history.
    #[test]
    fn conflict_between_history_less_edits_still_preserves_losing_version() {
        let mut base = Database::new();
        let id = add_entry(&mut base, "Netflix", "alice", ts(0));
        let mut local = base.clone();
        let mut incoming = base;
        edit_entry_without_history(&mut local, id, "alice@local", ts(3));
        edit_entry_without_history(&mut incoming, id, "alice@incoming", ts(7));

        let outcome = merge_vaults(&local, &incoming).expect("merge succeeds");

        let merged_entry = outcome.merged.entry(id).expect("entry present");
        assert_eq!(merged_entry.get(fields::USERNAME), Some("alice@incoming"));
        assert!(
            history_contains_username(&outcome.merged, id, "alice@local"),
            "losing local version must be preserved even without prior history"
        );
        assert_eq!(outcome.summary.conflicts.len(), 1);
    }

    /// Deletes an entry the way a `KeePass` app does: the entry leaves the
    /// tree and a tombstone lands in `DeletedObjects`. The deletion time
    /// is then pinned for determinism.
    fn delete_entry(db: &mut Database, id: EntryId, at: NaiveDateTime) {
        db.entry_mut(id).expect("entry exists").remove();
        db.deleted_objects.insert(id.uuid(), Some(at));
    }

    /// A deletion made on the incoming side propagates: the entry leaves
    /// the merged Vault and the tombstone survives in `DeletedObjects` so
    /// later merges don't resurrect it.
    #[test]
    fn entry_deleted_on_incoming_side_propagates_via_deleted_objects() {
        let mut base = Database::new();
        let keep_id = add_entry(&mut base, "Keep", "alice", ts(0));
        let delete_id = add_entry(&mut base, "Delete Me", "bob", ts(0));
        let local = base.clone();
        let mut incoming = base;
        delete_entry(&mut incoming, delete_id, ts(5));

        let outcome = merge_vaults(&local, &incoming).expect("merge succeeds");

        assert!(outcome.merged.entry(delete_id).is_none());
        assert!(outcome.merged.entry(keep_id).is_some());
        assert!(
            outcome
                .merged
                .deleted_objects
                .contains_key(&delete_id.uuid()),
            "tombstone must survive in DeletedObjects"
        );
        assert_eq!(outcome.summary.entries_deleted, 1);
        assert!(outcome.summary.conflicts.is_empty());
    }

    /// Delete-vs-edit where the edit is newer: the entry survives with the
    /// edited content instead of being deleted.
    #[test]
    fn newer_local_edit_wins_over_incoming_deletion() {
        let mut base = Database::new();
        let id = add_entry(&mut base, "Netflix", "alice", ts(0));
        let mut local = base.clone();
        let mut incoming = base;
        delete_entry(&mut incoming, id, ts(3));
        edit_entry_without_history(&mut local, id, "alice@edited", ts(7));

        let outcome = merge_vaults(&local, &incoming).expect("merge succeeds");

        let merged_entry = outcome
            .merged
            .entry(id)
            .expect("entry edited after the deletion must survive");
        assert_eq!(merged_entry.get(fields::USERNAME), Some("alice@edited"));
        assert_eq!(outcome.summary.entries_deleted, 0);
    }

    /// Delete-vs-edit where the deletion is newer: the entry stays deleted
    /// and the older incoming edit does not resurrect it.
    #[test]
    fn newer_local_deletion_wins_over_older_incoming_edit() {
        let mut base = Database::new();
        let id = add_entry(&mut base, "Netflix", "alice", ts(0));
        let mut local = base.clone();
        let mut incoming = base;
        edit_entry_without_history(&mut incoming, id, "alice@edited", ts(3));
        delete_entry(&mut local, id, ts(7));

        let outcome = merge_vaults(&local, &incoming).expect("merge succeeds");

        assert!(
            outcome.merged.entry(id).is_none(),
            "an entry deleted after the other side's edit must stay deleted"
        );
    }

    fn add_group(db: &mut Database, name: &str, at: NaiveDateTime) -> keepass::db::GroupId {
        let mut root = db.root_mut();
        let mut group = root.add_group();
        group.name = name.to_string();
        group.times.last_modification = Some(at);
        group.times.location_changed = Some(at);
        group.times.creation = Some(at);
        group.id()
    }

    /// Group created and renamed on the incoming side: both arrive in the
    /// merged Vault, with the rename driven by modification time.
    #[test]
    fn group_creation_and_rename_on_incoming_side_merge() {
        let mut base = Database::new();
        let renamed_id = add_group(&mut base, "Old Name", ts(0));
        let local = base.clone();
        let mut incoming = base;
        let created_id = add_group(&mut incoming, "Brand New", ts(2));
        {
            let mut group = incoming.group_mut(renamed_id).expect("group exists");
            group.name = "New Name".to_string();
            group.times.last_modification = Some(ts(5));
        }

        let outcome = merge_vaults(&local, &incoming).expect("merge succeeds");

        assert_eq!(
            outcome
                .merged
                .group(renamed_id)
                .expect("group present")
                .name,
            "New Name"
        );
        assert!(outcome.merged.group(created_id).is_some());
    }

    /// A group moved on the incoming side follows location-changed
    /// semantics: the newer location wins.
    #[test]
    fn group_move_on_incoming_side_follows_location_changed() {
        let mut base = Database::new();
        let parent_a = add_group(&mut base, "Parent A", ts(0));
        let parent_b = add_group(&mut base, "Parent B", ts(0));
        let child = add_group(&mut base, "Child", ts(0));
        {
            let mut group = base.group_mut(child).expect("child exists");
            group.move_to(parent_a).expect("move under A");
            group.times.location_changed = Some(ts(1));
        }
        let local = base.clone();
        let mut incoming = base;
        {
            let mut group = incoming.group_mut(child).expect("child exists");
            group.move_to(parent_b).expect("move under B");
            group.times.location_changed = Some(ts(5));
        }

        let outcome = merge_vaults(&local, &incoming).expect("merge succeeds");

        let merged_child = outcome.merged.group(child).expect("child present");
        assert_eq!(
            merged_child.parent().map(|p| p.id()),
            Some(parent_b),
            "newer location-changed time must win"
        );
    }

    /// Builds a pair of diverged copies exercising every merge dimension
    /// at once: an addition, a clean edit, a conflict, and a deletion.
    fn diverged_pair() -> (Database, Database) {
        let mut base = Database::new();
        let edited = add_entry(&mut base, "Edited", "alice", ts(0));
        let conflicted = add_entry(&mut base, "Conflicted", "bob", ts(0));
        let deleted = add_entry(&mut base, "Deleted", "carol", ts(0));
        let mut local = base.clone();
        let mut incoming = base;
        edit_entry_without_history(&mut local, conflicted, "bob@local", ts(2));
        add_entry(&mut incoming, "Added", "dave", ts(1));
        edit_entry_keeping_history(&mut incoming, edited, "alice@new", ts(3));
        edit_entry_without_history(&mut incoming, conflicted, "bob@incoming", ts(4));
        delete_entry(&mut incoming, deleted, ts(5));
        (local, incoming)
    }

    /// Merging the same two inputs twice yields an identical result.
    #[test]
    fn merging_same_inputs_twice_yields_identical_result() {
        let (local, incoming) = diverged_pair();

        let first = merge_vaults(&local, &incoming).expect("first merge");
        let second = merge_vaults(&local, &incoming).expect("second merge");

        assert_eq!(first.merged, second.merged);
    }

    /// Re-merging the incoming copy into an already-merged result is a
    /// no-op: nothing changes and the summary reports nothing.
    #[test]
    fn remerging_incoming_into_merged_result_changes_nothing() {
        let (local, incoming) = diverged_pair();
        let first = merge_vaults(&local, &incoming).expect("first merge");

        let second = merge_vaults(&first.merged, &incoming).expect("re-merge");

        assert_eq!(second.merged, first.merged);
        assert_eq!(second.summary.entries_added, 0);
        assert_eq!(second.summary.entries_updated, 0);
        assert_eq!(second.summary.entries_deleted, 0);
        assert!(second.summary.conflicts.is_empty());
    }

    /// A changed security posture (here: different KDF parameters) is
    /// surfaced in the Merge Summary but never auto-applied — the merged
    /// Vault keeps the local configuration.
    #[test]
    fn kdf_difference_is_surfaced_but_never_auto_applied() {
        use crate::dto::merge::SecurityPostureChange;
        use keepass::config::KdfConfig;

        let mut base = Database::new();
        add_entry(&mut base, "Existing", "alice", ts(0));
        let local = base.clone();
        let mut incoming = base;
        add_entry(&mut incoming, "Added", "bob", ts(1));
        incoming.config.kdf_config = KdfConfig::Aes { rounds: 100_000 };

        let outcome = merge_vaults(&local, &incoming).expect("merge succeeds");

        assert_eq!(
            outcome.summary.security_posture_changes,
            vec![SecurityPostureChange::Kdf]
        );
        assert_eq!(
            outcome.merged.config, local.config,
            "merge must never auto-apply the incoming security posture"
        );
        assert_eq!(outcome.summary.entries_added, 1, "entry merge still runs");
    }

    /// Identical configuration on both sides reports no posture changes.
    #[test]
    fn identical_security_posture_reports_no_changes() {
        let mut base = Database::new();
        add_entry(&mut base, "Existing", "alice", ts(0));
        let local = base.clone();
        let incoming = local.clone();

        let outcome = merge_vaults(&local, &incoming).expect("merge succeeds");

        assert!(outcome.summary.security_posture_changes.is_empty());
    }

    /// End-to-end service slice for "Merge from file…": a diverged copy of
    /// the open Vault (same master password) is merged in, the result is
    /// saved to disk through the pre-save backup machinery, and the Merge
    /// Summary is returned.
    #[test]
    fn merge_from_file_merges_diverged_copy_saves_and_backs_up() {
        use crate::dto::entry::CreateEntryData;
        use crate::services::kdbx::test_support::create_test_database;

        let (service, dir, db_path, _entry_a, _entry_b) = create_test_database();
        service.save(&db_path).expect("save original to disk");

        // Diverge a copy of the same Vault: same UUIDs, same password.
        let copy_path = dir
            .path()
            .join("diverged-copy.kdbx")
            .to_string_lossy()
            .to_string();
        std::fs::copy(&db_path, &copy_path).expect("copy vault file");
        let copy_info = service.open(&copy_path, "testpass").expect("open copy");
        service
            .create_entry(
                &copy_path,
                &copy_info.root_group_id,
                CreateEntryData {
                    title: "From Copy".to_string(),
                    username: "dave".to_string(),
                    password: crate::domain::secure::SecureString::from("secret"),
                    url: None,
                    notes: None,
                    icon_id: Some(0),
                    tags: None,
                    custom_fields: None,
                    protected_custom_fields: None,
                    expires: None,
                    expiry_time: None,
                },
            )
            .expect("create entry in copy");
        service.save(&copy_path).expect("save copy");
        service.close(&copy_path).expect("close copy");

        let (summary, backup) = service
            .merge_from_file(&db_path, &copy_path)
            .expect("merge from file");

        assert_eq!(summary.entries_added, 1);
        let titles: Vec<String> = service
            .list_entries(&db_path, None)
            .expect("list entries")
            .into_iter()
            .map(|e| e.title)
            .collect();
        assert!(titles.contains(&"From Copy".to_string()));
        let info = service.get_info(&db_path).expect("info");
        assert!(!info.is_modified, "merge result must be saved to disk");
        assert!(
            backup.is_some(),
            "merge must take a pre-merge backup via the pre-save machinery"
        );
    }

    /// The picked file must unlock with the open Vault's credentials; a
    /// file with a different master password surfaces `InvalidPassword`
    /// and leaves the open Vault untouched.
    #[test]
    fn merge_from_file_with_wrong_credentials_fails_cleanly() {
        use crate::dto::database::DatabaseCreationOptions;
        use crate::services::kdbx::test_support::create_test_database;

        let (service, dir, db_path, _entry_a, _entry_b) = create_test_database();
        let other_path = dir
            .path()
            .join("other-password.kdbx")
            .to_string_lossy()
            .to_string();
        let options = DatabaseCreationOptions {
            create_default_groups: false,
            kdf_memory: Some(1024 * 1024),
            kdf_iterations: Some(1),
            kdf_parallelism: Some(1),
            description: None,
        };
        service
            .create_database(&other_path, Some("differentpass"), None, "Other", &options)
            .expect("create other db");
        service.close(&other_path).expect("close other db");

        let result = service.merge_from_file(&db_path, &other_path);

        assert!(matches!(result, Err(AppError::InvalidPassword)));
        let entries = service.list_entries(&db_path, None).expect("list entries");
        assert_eq!(entries.len(), 2, "open Vault must be untouched");
    }
}
