// SPDX-License-Identifier: MIT

//! In-place restore of a backup snapshot over the current Vault file.
//!
//! Behaviour summary (see parent issue #61 and slice #196):
//!
//! - The supplied `backup_path` is resolved against the open-database map: an
//!   open Vault whose basename matches the snapshot's embedded vault basename
//!   authorises the restore. This is the same authorisation model used by
//!   [`KdbxService::delete_backup`] — snapshot operations are scoped to an
//!   open Vault, never to arbitrary on-disk paths.
//! - With an open Vault matched, a pre-restore pre-image snapshot of the
//!   current on-disk state is taken via `backups::snapshot_manual` —
//!   fail-closed. A failed pre-restore snapshot aborts the restore; the
//!   Vault bytes are never touched. `snapshot_manual` is used (rather than
//!   `snapshot`) so the safety net (a) ignores `settings.enabled` —
//!   matching the dialog's unconditional promise of a pre-restore backup —
//!   and (b) does not run auto-rotation, which could otherwise delete the
//!   user's chosen restore target before we copy from it.
//! - The chosen backup is atomic-copied over the source Vault using the same
//!   `atomic_write` primitive the save path uses (temp file + sync + rename).
//! - On success the open-Vault entry is **locked in place** rather than
//!   removed: the decrypted `Database` and stored password are dropped so
//!   no stale in-memory state can drift from the new on-disk bytes, but
//!   the entry's `path` and `keyfile_path` survive so the frontend's
//!   re-unlock path (which calls `unlock_database`, expecting the entry to
//!   still exist in the map) just works. The command layer emits
//!   `database-closed` and the frontend routes to the unlock screen.
//! - The restore path never calls `add_recent_database` — backup paths must
//!   not enter the recent-Vaults list.

use crate::dto::error::AppError;
use crate::services::kdbx::backups::{
    self,
    filename::{parse_backup_filename, parse_manual_backup_filename},
};
use crate::utils::atomic_write::{atomic_write, AtomicWriteOptions};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use super::KdbxService;

impl KdbxService {
    /// Restores a backup snapshot over the Vault it belongs to.
    ///
    /// Returns the resolved source Vault path (canonical-stored form) on
    /// success so the command layer can emit `database-closed` with the
    /// matching id and the frontend can route to unlock.
    pub fn restore_backup(&self, backup_path: &str) -> Result<String, AppError> {
        let backup = Path::new(backup_path);
        let canonical_backup =
            fs::canonicalize(backup).map_err(|e| AppError::InvalidPath(e.to_string()))?;

        // Refuse anything that does not decode as a snapshot of *some* Vault.
        // Without this, the command would degrade into "copy any file over an
        // open Vault" — explicitly out of scope.
        let target_filename = canonical_backup
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| {
                AppError::InvalidInput(format!(
                    "backup path has no filename component: {backup_path}"
                ))
            })?;
        let target_vault = parse_backup_filename(target_filename)
            .or_else(|| parse_manual_backup_filename(target_filename))
            .map(|(v, _)| v)
            .ok_or_else(|| {
                AppError::InvalidInput(format!(
                    "path is not a backup snapshot filename: {backup_path}"
                ))
            })?;

        let settings = self.current_backup_settings()?;

        // Hold the open-databases mutex for the **entire** restore: matching,
        // pre-restore snapshot, atomic copy, and map invalidation all run
        // under the same guard. Every other open-Vault mutation (save,
        // create_entry, delete_entry, …) acquires this same mutex, so they
        // queue behind us — eliminating the lock-release-then-reacquire race
        // where a concurrent save could land its bytes between our pre-restore
        // snapshot and our atomic copy and silently lose user data.
        //
        // The cost is that operations against *other* open vaults also
        // serialise behind a restore, but the same is already true for save
        // (which holds the map lock through its own atomic_write), so the
        // restore path doesn't introduce a new contention pattern — it just
        // extends an existing one for the duration of one file copy.
        let mut databases = self.lock_databases()?;

        // Resolve to an open Vault by basename + backup-dir containment. The
        // backup-dir match is what closes the loop: an attacker who plants a
        // snapshot-shaped file for a *different* Vault inside our backup dir
        // cannot trick us into restoring it over the open Vault, because the
        // canonical containment check below would fail for a foreign-Vault
        // snapshot.
        //
        // The matched Vault must also be currently unlocked AND clean.
        //
        // - Locked guard: restore is a destructive write; allowing it against
        //   a locked-but-still-mapped Vault would let a walk-up attacker
        //   bypass auto-lock to roll the on-disk Vault back to an arbitrary
        //   prior state. Matches the locked-guard every mutation uses.
        // - Dirty guard: if the user has unsaved in-memory edits, restore
        //   would silently discard them — the pre-restore snapshot only
        //   captures the on-disk bytes (older than memory), not the dirty
        //   working state, and after restore the in-memory DB is dropped
        //   from the map. The user must explicitly save or discard before
        //   restoring; we don't choose for them.
        let (map_key, stored_source) = {
            let mut found: Option<(String, PathBuf, bool, bool)> = None;
            for (key, open_db) in databases.iter() {
                let source = Path::new(&open_db.path);
                let Some(open_basename) = source.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                if open_basename != target_vault {
                    continue;
                }
                let Ok(backup_dir) = backups::resolved_backup_dir(source, &settings) else {
                    continue;
                };
                if backups::assert_backup_dir_not_symlinked(&backup_dir).is_err() {
                    continue;
                }
                let Ok(canonical_dir) = fs::canonicalize(&backup_dir) else {
                    continue;
                };
                if canonical_backup.starts_with(&canonical_dir) {
                    found = Some((
                        key.clone(),
                        source.to_path_buf(),
                        open_db.is_locked(),
                        open_db.is_modified,
                    ));
                    break;
                }
            }
            let (key, source, locked, modified) = found.ok_or_else(|| {
                AppError::InvalidInput(format!(
                    "backup path is not an authorized snapshot for any open vault: {backup_path}"
                ))
            })?;
            if locked {
                return Err(AppError::DatabaseLocked(
                    source.to_string_lossy().into_owned(),
                ));
            }
            if modified {
                return Err(AppError::DatabaseModified(
                    source.to_string_lossy().into_owned(),
                ));
            }
            (key, source)
        };

        // Fail-closed pre-restore pre-image snapshot of the current on-disk
        // state. Uses `snapshot_manual` deliberately rather than `snapshot`,
        // for two reasons that hold even when the user has auto-backups
        // disabled or their rotation cap is tight:
        //
        // - Auto-rotation must not run here. `snapshot` runs `rotate` after
        //   writing the new pre-image; with `max_versions` at cap that would
        //   trim the *oldest* auto snapshot — which may be exactly the file
        //   we're about to copy from. We'd then fail mid-restore with the
        //   chosen restore target gone, leaving the user in a worse state
        //   than when they started.
        // - The dialog body promises an automatic pre-restore backup
        //   unconditionally. `snapshot` honours `settings.enabled` and
        //   silently no-ops when backups are off — breaking that promise
        //   the one time the safety net matters most. `snapshot_manual`
        //   ignores the toggle so the rollback point always exists.
        //
        // A missing source on disk is the only case we don't snapshot: the
        // file we'd capture isn't there to capture (rare — external delete
        // between unlock and now). The atomic copy below will re-create it
        // from the chosen backup.
        let source_exists = stored_source
            .try_exists()
            .map_err(|e| AppError::BackupFailed {
                path: stored_source.to_string_lossy().into_owned(),
                reason: e.to_string(),
            })?;
        if source_exists {
            backups::snapshot_manual(&stored_source, &settings)?;
        }

        // Atomic-copy the backup bytes over the source. Uses the same
        // primitive as save so a kill mid-restore leaves the original Vault
        // bytes intact rather than a half-written file.
        let source_str = stored_source.to_string_lossy().into_owned();
        let backup_for_copy = canonical_backup.clone();
        atomic_write(
            &source_str,
            &AtomicWriteOptions {
                preserve_permissions: true,
            },
            |file| {
                let mut src = fs::File::open(&backup_for_copy)
                    .map_err(|e| AppError::Io(format!("Failed to open backup for restore: {e}")))?;
                std::io::copy(&mut src, file)
                    .map_err(|e| AppError::Io(format!("Failed to copy backup bytes: {e}")))?;
                // Explicit flush before atomic_write's own sync_all keeps the
                // dependency on closure-internal flushing local to this op.
                file.flush()
                    .map_err(|e| AppError::Io(format!("Failed to flush restored bytes: {e}")))?;
                Ok(())
            },
        )?;

        // Lock the open-Vault entry in place. Dropping `db` and `password`
        // discards the now-stale decrypted state and clears the master-key
        // material from memory, while keeping `path` and `keyfile_path` so
        // the frontend's re-unlock flow (which routes to `/unlock` and calls
        // `unlock_database`) finds the entry it expects in the map. Fully
        // removing the entry would force a fresh `open_database` call, which
        // the post-restore UI flow is not wired for.
        if let Some(open_db) = databases.get_mut(&map_key) {
            open_db.db = None;
            open_db.password = None;
            open_db.is_modified = false;
        }

        Ok(source_str)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::commands::settings::BackupSettings;
    use crate::domain::secure::SecureString;
    use crate::dto::database::DatabaseCreationOptions;
    use crate::dto::entry::CreateEntryData;
    use crate::services::kdbx::KdbxService;
    use tempfile::tempdir;

    fn fast_options() -> DatabaseCreationOptions {
        // Argon2 minimum-cost parameters keep the test suite fast.
        DatabaseCreationOptions {
            create_default_groups: false,
            kdf_memory: Some(1024 * 1024),
            kdf_iterations: Some(1),
            kdf_parallelism: Some(1),
            description: None,
        }
    }

    fn settings_for(backup_dir: &std::path::Path) -> BackupSettings {
        BackupSettings {
            enabled: true,
            max_versions: 10,
            directory: Some(backup_dir.to_string_lossy().into_owned()),
            on_open: false,
        }
    }

    fn entry(title: &str, username: &str) -> CreateEntryData {
        CreateEntryData {
            title: title.into(),
            username: username.into(),
            password: SecureString::from("secret"),
            url: None,
            notes: None,
            icon_id: Some(0),
            tags: None,
            custom_fields: None,
            protected_custom_fields: None,
        }
    }

    #[test]
    fn restore_preserves_chosen_target_when_rotation_is_at_cap() {
        // Regression for the rotation-eats-target race: with `max_versions`
        // at cap (1), the previous implementation would call `snapshot()`
        // for the pre-restore snapshot, which would run rotation and
        // delete the user's chosen restore target before the copy step ran.
        // The new implementation uses `snapshot_manual` (no rotation), so
        // the target survives and the restore completes.
        let dir = tempdir().expect("tempdir");
        let vault = dir.path().join("vault.kdbx");
        let vault_str = vault.to_string_lossy().into_owned();
        let backup_dir = dir.path().join("backups");

        let service = KdbxService::new();
        let mut settings = settings_for(&backup_dir);
        settings.max_versions = 1; // tightest rotation cap
        service.set_backup_settings(settings).expect("set settings");

        let info = service
            .create_database(&vault_str, Some("pw"), None, "Test", &fast_options())
            .expect("create");
        service
            .create_entry(&vault_str, &info.root_group_id, entry("Entry A", "alice"))
            .expect("create A");
        service.save(&vault_str).expect("save 1");
        service
            .create_entry(&vault_str, &info.root_group_id, entry("Entry B", "bob"))
            .expect("create B");
        service.save(&vault_str).expect("save 2");

        // After save 2 with max_versions=1, exactly one auto snapshot
        // survives — the pre-image of save 2, i.e. the "Entry A only" state.
        // That's what the user wants to restore to.
        let listing = service.list_backups(&vault_str).expect("list backups");
        let auto_targets: Vec<_> = listing
            .iter()
            .filter(|e| e.kind == backups::BackupKind::Auto)
            .collect();
        assert_eq!(auto_targets.len(), 1, "rotation should leave one auto");
        let target_path = auto_targets[0].path.to_string_lossy().into_owned();

        service
            .restore_backup(&target_path)
            .expect("restore must succeed without rotation eating the target");

        service
            .unlock(&vault_str, Some("pw"))
            .expect("re-unlock restored vault");
        let entries = service
            .list_entries(&vault_str, None)
            .expect("list entries");
        assert!(entries.iter().any(|e| e.title == "Entry A"));
        assert!(!entries.iter().any(|e| e.title == "Entry B"));
    }

    #[test]
    fn restore_creates_pre_restore_snapshot_even_when_backups_disabled() {
        // Regression for the broken safety-net promise: the confirmation
        // dialog tells the user "the current state will be backed up
        // automatically before the restore" with no caveat about
        // settings.enabled. Previous behaviour called `snapshot()`, which
        // is a silent no-op when `enabled = false` — a destructive write
        // with no rollback point. New behaviour uses `snapshot_manual`,
        // which ignores the toggle.
        let dir = tempdir().expect("tempdir");
        let vault = dir.path().join("vault.kdbx");
        let vault_str = vault.to_string_lossy().into_owned();
        let backup_dir = dir.path().join("backups");

        let service = KdbxService::new();
        service
            .set_backup_settings(settings_for(&backup_dir))
            .expect("set settings");

        let info = service
            .create_database(&vault_str, Some("pw"), None, "Test", &fast_options())
            .expect("create");
        service
            .create_entry(&vault_str, &info.root_group_id, entry("Entry A", "alice"))
            .expect("create A");
        service.save(&vault_str).expect("save 1");
        service
            .create_entry(&vault_str, &info.root_group_id, entry("Entry B", "bob"))
            .expect("create B");
        service.save(&vault_str).expect("save 2");

        let listing = service.list_backups(&vault_str).expect("list backups");
        let target_path = listing
            .iter()
            .find(|e| e.kind == backups::BackupKind::Auto)
            .expect("auto snapshot present")
            .path
            .to_string_lossy()
            .into_owned();
        let manual_count_before = listing
            .iter()
            .filter(|e| e.kind == backups::BackupKind::Manual)
            .count();

        // Disable backups *after* the autos exist so we have something to
        // restore but no auto safety net would run on the pre-restore path.
        let mut disabled = settings_for(&backup_dir);
        disabled.enabled = false;
        service
            .set_backup_settings(disabled)
            .expect("disable backups");

        service
            .restore_backup(&target_path)
            .expect("restore must succeed even with backups disabled");

        // The vault is locked-in-place after restore. Re-unlock so we can
        // list backups; a manual pre-restore snapshot must have been
        // created despite `enabled = false`.
        service
            .unlock(&vault_str, Some("pw"))
            .expect("re-unlock after restore");
        let after = service.list_backups(&vault_str).expect("list after");
        let manual_count_after = after
            .iter()
            .filter(|e| e.kind == backups::BackupKind::Manual)
            .count();
        assert_eq!(
            manual_count_after,
            manual_count_before + 1,
            "exactly one new manual (pre-restore) snapshot must exist after restore"
        );
    }

    #[test]
    fn restore_rejects_when_matched_vault_has_unsaved_changes() {
        // Regression: without this guard, the in-memory dirty edits would
        // be silently discarded — the pre-restore snapshot only captures
        // on-disk bytes (older than memory), and the in-memory DB is
        // dropped from the open-database map after restore. Force the
        // user to save or discard explicitly first.
        let dir = tempdir().expect("tempdir");
        let vault = dir.path().join("vault.kdbx");
        let vault_str = vault.to_string_lossy().into_owned();
        let backup_dir = dir.path().join("backups");

        let service = KdbxService::new();
        service
            .set_backup_settings(settings_for(&backup_dir))
            .expect("set backup settings");

        let info = service
            .create_database(&vault_str, Some("pw"), None, "Test", &fast_options())
            .expect("create");
        service
            .create_entry(&vault_str, &info.root_group_id, entry("Entry A", "alice"))
            .expect("create A");
        service.save(&vault_str).expect("save 1");
        // Second save establishes a backup we can target.
        service
            .create_entry(&vault_str, &info.root_group_id, entry("Entry B", "bob"))
            .expect("create B");
        service.save(&vault_str).expect("save 2");

        let listing = service.list_backups(&vault_str).expect("list backups");
        let target_path = listing
            .first()
            .expect("at least one backup")
            .path
            .to_string_lossy()
            .into_owned();

        // Dirty the in-memory vault without saving. is_modified flips true.
        service
            .create_entry(&vault_str, &info.root_group_id, entry("Unsaved", "carol"))
            .expect("create unsaved entry");
        let dirty_info = service.get_info(&vault_str).expect("info");
        assert!(
            dirty_info.is_modified,
            "test precondition: vault must be modified"
        );

        let bytes_before = std::fs::read(&vault).expect("read before");

        let err = service
            .restore_backup(&target_path)
            .expect_err("restore must be rejected for dirty vault");
        assert!(
            matches!(err, AppError::DatabaseModified(_)),
            "expected DatabaseModified, got {err:?}"
        );

        let bytes_after = std::fs::read(&vault).expect("read after");
        assert_eq!(
            bytes_before, bytes_after,
            "Vault bytes must be unchanged when restore is rejected"
        );
        assert!(
            service.is_database_open(&vault_str).expect("is_open"),
            "rejected restore must not invalidate the open-Vault map"
        );
        let still_dirty = service.get_info(&vault_str).expect("info after");
        assert!(
            still_dirty.is_modified,
            "in-memory dirty state must survive the rejection"
        );
    }

    #[test]
    fn restore_rejects_when_matched_vault_is_locked() {
        // A locked vault is still in the open-databases map (auto-lock drops
        // the decrypted state but keeps the entry). Without an explicit guard
        // a walk-up attacker could bypass auto-lock by triggering Restore
        // from Settings → Backups, rolling the on-disk Vault back to an old
        // snapshot. Match the locked-guard every other mutation uses.
        let dir = tempdir().expect("tempdir");
        let vault = dir.path().join("vault.kdbx");
        let vault_str = vault.to_string_lossy().into_owned();
        let backup_dir = dir.path().join("backups");

        let service = KdbxService::new();
        service
            .set_backup_settings(settings_for(&backup_dir))
            .expect("set backup settings");

        let info = service
            .create_database(&vault_str, Some("pw"), None, "Test", &fast_options())
            .expect("create");
        service
            .create_entry(&vault_str, &info.root_group_id, entry("Entry A", "alice"))
            .expect("create A");
        service.save(&vault_str).expect("save 1");
        service
            .create_entry(&vault_str, &info.root_group_id, entry("Entry B", "bob"))
            .expect("create B");
        service.save(&vault_str).expect("save 2");

        let listing = service.list_backups(&vault_str).expect("list backups");
        let target_path = listing
            .first()
            .expect("at least one backup")
            .path
            .to_string_lossy()
            .into_owned();

        let bytes_before = std::fs::read(&vault).expect("read before");

        service.lock(&vault_str).expect("lock vault");
        assert_eq!(
            service.is_database_locked(&vault_str).expect("locked?"),
            Some(true)
        );

        let err = service
            .restore_backup(&target_path)
            .expect_err("restore must be rejected while the vault is locked");
        assert!(
            matches!(err, AppError::DatabaseLocked(_)),
            "expected DatabaseLocked, got {err:?}"
        );

        let bytes_after = std::fs::read(&vault).expect("read after");
        assert_eq!(
            bytes_before, bytes_after,
            "Vault bytes must be unchanged when restore is rejected for being locked"
        );
        assert!(
            service.is_database_open(&vault_str).expect("is_open"),
            "rejected restore must not invalidate the open-Vault map"
        );
    }

    #[cfg(unix)]
    #[test]
    fn restore_aborts_when_pre_restore_snapshot_fails() {
        // Acceptance criterion 6: a restore whose pre-restore snapshot fails
        // (e.g., read-only backup dir) must abort with the Vault file
        // unchanged. We force the failure by making the source Vault
        // unreadable so the snapshot's bytes-copy step trips PermissionDenied.
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().expect("tempdir");
        let vault = dir.path().join("vault.kdbx");
        let vault_str = vault.to_string_lossy().into_owned();
        let backup_dir = dir.path().join("backups");

        let service = KdbxService::new();
        service
            .set_backup_settings(settings_for(&backup_dir))
            .expect("set backup settings");

        let info = service
            .create_database(&vault_str, Some("pw"), None, "Test", &fast_options())
            .expect("create");
        service
            .create_entry(&vault_str, &info.root_group_id, entry("Entry A", "alice"))
            .expect("create A");
        service.save(&vault_str).expect("save 1");
        service
            .create_entry(&vault_str, &info.root_group_id, entry("Entry B", "bob"))
            .expect("create B");
        service.save(&vault_str).expect("save 2");

        let listing = service.list_backups(&vault_str).expect("list backups");
        let target_path = listing
            .first()
            .expect("at least one backup")
            .path
            .to_string_lossy()
            .into_owned();

        let bytes_before = std::fs::read(&vault).expect("read before");

        // Drop read permissions on the source so the pre-restore snapshot's
        // copy step fails. atomic_write's source-open errors out and bubbles
        // through snapshot → BackupFailed.
        std::fs::set_permissions(&vault, std::fs::Permissions::from_mode(0o000))
            .expect("chmod 000");

        let err = service
            .restore_backup(&target_path)
            .expect_err("restore must abort when pre-restore snapshot fails");
        assert!(
            matches!(err, AppError::BackupFailed { .. }),
            "expected BackupFailed, got {err:?}"
        );

        // Restore read access so we can verify the bytes survived intact.
        std::fs::set_permissions(&vault, std::fs::Permissions::from_mode(0o600))
            .expect("chmod 600");
        let bytes_after = std::fs::read(&vault).expect("read after");
        assert_eq!(
            bytes_before, bytes_after,
            "Vault bytes must be unchanged when pre-restore snapshot fails"
        );
        assert!(
            service.is_database_open(&vault_str).expect("is_open"),
            "aborted restore must not invalidate the open-Vault map"
        );
    }

    #[test]
    fn restore_rejects_backup_outside_open_vaults_backup_dir() {
        // Acceptance criterion 3: a path that does not resolve inside the
        // backup directory of any open Vault is rejected as InvalidInput. The
        // Vault file on disk is not touched.
        let dir = tempdir().expect("tempdir");
        let vault = dir.path().join("vault.kdbx");
        let vault_str = vault.to_string_lossy().into_owned();
        let backup_dir = dir.path().join("backups");

        let service = KdbxService::new();
        service
            .set_backup_settings(settings_for(&backup_dir))
            .expect("set backup settings");
        service
            .create_database(&vault_str, Some("pw"), None, "Test", &fast_options())
            .expect("create");
        service
            .create_entry(
                &vault_str,
                &service.get_info(&vault_str).expect("info").root_group_id,
                entry("Entry A", "alice"),
            )
            .expect("create A");
        service.save(&vault_str).expect("save 1");

        // Plant a snapshot-shaped file outside the authorised backup dir.
        // Has a valid snapshot filename (so the filename guard passes) but
        // lives in a sibling dir — must be rejected on containment alone.
        let foreign_dir = dir.path().join("foreign");
        std::fs::create_dir_all(&foreign_dir).expect("foreign dir");
        let foreign_path = foreign_dir.join("vault.kdbx.backup.20260101T000000.000Z.kdbx");
        std::fs::write(&foreign_path, b"not-a-snapshot").expect("write foreign");

        let vault_bytes_before = std::fs::read(&vault).expect("read before");

        let err = service
            .restore_backup(&foreign_path.to_string_lossy())
            .expect_err("must reject foreign-dir snapshot");
        assert!(
            matches!(err, AppError::InvalidInput(_)),
            "expected InvalidInput, got {err:?}"
        );

        let vault_bytes_after = std::fs::read(&vault).expect("read after");
        assert_eq!(
            vault_bytes_before, vault_bytes_after,
            "Vault file must be unchanged when restore is rejected"
        );
        assert!(
            service.is_database_open(&vault_str).expect("open?"),
            "rejected restore must not invalidate the open-Vault map"
        );
    }

    #[test]
    fn restore_reverts_vault_to_a_prior_snapshot_state() {
        // Acceptance criterion 1: take two saves with different Entry content,
        // restore to the first, reopen → Entry state matches the first save.
        let dir = tempdir().expect("tempdir");
        let vault = dir.path().join("vault.kdbx");
        let vault_str = vault.to_string_lossy().into_owned();
        let backup_dir = dir.path().join("backups");

        let service = KdbxService::new();
        service
            .set_backup_settings(settings_for(&backup_dir))
            .expect("set backup settings");

        let info = service
            .create_database(&vault_str, Some("pw"), None, "Test", &fast_options())
            .expect("create");

        // Save 1: Vault with Entry A only.
        service
            .create_entry(&vault_str, &info.root_group_id, entry("Entry A", "alice"))
            .expect("create A");
        service.save(&vault_str).expect("save 1");

        // Save 2: add Entry B. Save's pre-image snapshot captures the
        // "Entry A only" state — that is the snapshot we'll restore to.
        service
            .create_entry(&vault_str, &info.root_group_id, entry("Entry B", "bob"))
            .expect("create B");
        service.save(&vault_str).expect("save 2");

        let listing = service.list_backups(&vault_str).expect("list backups");
        let pre_save2 = listing
            .iter()
            .find(|e| e.kind == backups::BackupKind::Auto)
            .expect("auto snapshot exists");
        let target_path = pre_save2.path.to_string_lossy().into_owned();

        service.restore_backup(&target_path).expect("restore");

        // The open-Vault entry must be locked in place (not removed): the
        // in-memory decrypted state is gone so it cannot drift from the new
        // on-disk bytes, but the entry survives so the frontend's re-unlock
        // path (`unlock_database`) finds what it expects in the map.
        assert_eq!(
            service
                .is_database_locked(&vault_str)
                .expect("locked state"),
            Some(true),
            "open-Vault entry must be locked-in-place after restore"
        );

        // Re-unlock → only Entry A should be present.
        service
            .unlock(&vault_str, Some("pw"))
            .expect("re-unlock restored vault");
        let entries = service
            .list_entries(&vault_str, None)
            .expect("list entries after restore");
        assert!(
            entries.iter().any(|e| e.title == "Entry A"),
            "Entry A must be present after restore"
        );
        assert!(
            !entries.iter().any(|e| e.title == "Entry B"),
            "Entry B must NOT be present after restore"
        );
    }
}
