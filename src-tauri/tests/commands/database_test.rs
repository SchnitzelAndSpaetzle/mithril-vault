// SPDX-License-Identifier: MIT
//! Tests for database command handlers
//!
//! These tests exercise the `KdbxService` methods that the database commands delegate to.
//! The test structure mirrors the command API so that command-specific logic can be tested
//! when it is added.

#![allow(clippy::expect_used)] // expect() is acceptable in tests

use mithril_vault_lib::commands::settings::BackupSettings;
use mithril_vault_lib::dto::error::AppError;
use mithril_vault_lib::services::kdbx::backups::snapshot;
use mithril_vault_lib::services::kdbx::KdbxService;
use tempfile::tempdir;

use super::copy_fixture_to_temp;

// ============================================================================
// open_database command tests
// ============================================================================

#[test]
fn test_open_database_success() {
    let Some((_temp_dir, db_path)) = copy_fixture_to_temp("test-kdbx4-low-KDF.kdbx") else {
        eprintln!("Skipping test: fixture not found");
        return;
    };

    let service = KdbxService::new();

    let result = service.open(&db_path.to_string_lossy(), "test123");

    assert!(result.is_ok(), "Should successfully open database");
    let info = result.expect("database info");
    assert!(!info.name.is_empty(), "Database should have a name");
    assert!(
        !info.root_group_id.is_empty(),
        "Database should have a root group"
    );
    assert!(
        !info.is_modified,
        "Newly opened database should not be modified"
    );
}

#[test]
fn test_open_database_invalid_password() {
    let Some((_temp_dir, db_path)) = copy_fixture_to_temp("test-kdbx4-low-KDF.kdbx") else {
        eprintln!("Skipping test: fixture not found");
        return;
    };

    let service = KdbxService::new();

    let result = service.open(&db_path.to_string_lossy(), "wrong_password");

    assert!(
        matches!(result, Err(AppError::InvalidPassword)),
        "Should fail with InvalidPassword error for wrong password"
    );
}

#[test]
fn test_open_database_file_not_found() {
    let service = KdbxService::new();
    let dir = tempdir().expect("Failed to create temp dir");
    let path = dir.path().join("nonexistent-database.kdbx");

    let result = service.open(&path.to_string_lossy(), "test123");

    assert!(
        matches!(result, Err(AppError::InvalidPath(_))),
        "Should fail with InvalidPath error for missing file"
    );
}

// ============================================================================
// close_database command tests
// ============================================================================

#[test]
fn test_close_database_success() {
    let Some((_temp_dir, db_path)) = copy_fixture_to_temp("test-kdbx4-low-KDF.kdbx") else {
        eprintln!("Skipping test: fixture not found");
        return;
    };

    let db_path_str = db_path.to_string_lossy().to_string();
    let service = KdbxService::new();

    service
        .open(&db_path_str, "test123")
        .expect("Failed to open database");

    let result = service.close(&db_path_str);

    assert!(result.is_ok(), "Should successfully close database");

    // Verify database is closed by checking get_info fails
    let info_result = service.get_info(&db_path_str);
    assert!(
        matches!(info_result, Err(AppError::DatabaseNotFound(_))),
        "Database should be closed"
    );
}

#[test]
fn test_close_database_not_open() {
    let service = KdbxService::new();

    let result = service.close("nonexistent.kdbx");

    assert!(
        matches!(result, Err(AppError::DatabaseNotFound(_))),
        "Should fail with DatabaseNotFound error when no database is open"
    );
}

// ============================================================================
// create_database command tests
// ============================================================================

#[test]
fn test_create_database_success() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("new-test-database.kdbx");

    let service = KdbxService::new();

    let result = service.create(&db_path.to_string_lossy(), "newpassword", "My Test Vault");

    assert!(result.is_ok(), "Should successfully create database");
    let info = result.expect("database info");
    assert_eq!(info.name, "My Test Vault");
    assert!(!info.root_group_id.is_empty());
    assert!(
        !info.is_modified,
        "Newly created database should not be modified"
    );
    assert!(db_path.exists(), "Database file should exist on disk");
}

#[test]
fn test_create_database_same_path_already_open() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path1 = dir.path().join("first.kdbx");
    let db_path1_str = db_path1.to_string_lossy().to_string();

    let service = KdbxService::new();
    service
        .create(&db_path1_str, "pass1", "First DB")
        .expect("Failed to create first database");

    // Try to create/open the same database again
    let result = service.create(&db_path1_str, "pass1", "First DB Again");

    assert!(
        matches!(result, Err(AppError::DatabaseAlreadyOpen(_))),
        "Should fail with DatabaseAlreadyOpen when trying to create same database while one is open"
    );
}

// ============================================================================
// save_database command tests
// ============================================================================

#[test]
fn test_save_database_success() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("save-test.kdbx");
    let db_path_str = db_path.to_string_lossy().to_string();

    let service = KdbxService::new();
    service
        .create(&db_path_str, "savepass", "Save Test DB")
        .expect("Failed to create database");

    let result = service.save(&db_path_str);

    assert!(result.is_ok(), "Should successfully save database");

    // Verify file can be reopened
    service.close(&db_path_str).expect("Failed to close");
    let reopen_result = service.open(&db_path_str, "savepass");
    assert!(
        reopen_result.is_ok(),
        "Should be able to reopen saved database"
    );
}

#[test]
fn test_save_database_not_open() {
    let service = KdbxService::new();

    let result = service.save("nonexistent.kdbx");

    assert!(
        matches!(result, Err(AppError::DatabaseNotFound(_))),
        "Should fail with DatabaseNotFound when no database is open"
    );
}

#[test]
fn test_save_as_updates_database_identity_and_still_closes() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("original.kdbx");
    let new_path = dir.path().join("moved.kdbx");
    let db_path_str = db_path.to_string_lossy().to_string();
    let new_path_str = new_path.to_string_lossy().to_string();

    let service = KdbxService::new();
    service
        .create(&db_path_str, "savepass", "Save Test DB")
        .expect("Failed to create database");

    service
        .save_as(&db_path_str, &new_path_str, None)
        .expect("Failed to save database as new path");

    // After save_as, the database is now at new_path
    let old_info = service.get_info(&db_path_str);
    assert!(
        matches!(old_info, Err(AppError::DatabaseNotFound(_))),
        "Old database id should not resolve after save_as"
    );

    let new_info = service
        .get_info(&new_path_str)
        .expect("New database id should resolve after save_as");
    assert_eq!(new_info.path, new_path_str);

    service
        .close(&new_path_str)
        .expect("Failed to close database");

    let after_close = service.get_info(&new_path_str);
    assert!(
        matches!(after_close, Err(AppError::DatabaseNotFound(_))),
        "Database should be closed after close()"
    );
}

// ============================================================================
// list_backups command tests
// ============================================================================

fn enabled_backup_settings() -> BackupSettings {
    BackupSettings {
        enabled: true,
        ..BackupSettings::default()
    }
}

#[test]
fn list_backups_returns_snapshot_taken_for_open_vault() {
    // Service-level contract: with the vault open, list_backups surfaces a
    // freshly-taken snapshot — same path, with metadata.
    let dir = tempdir().expect("tempdir");
    let vault_path = dir.path().join("vault.kdbx");
    let db_path_str = vault_path.to_string_lossy().to_string();

    let service = KdbxService::new();
    service
        .set_backup_settings(enabled_backup_settings())
        .expect("set settings");
    service
        .create(&db_path_str, "pw", "List Backups Test Vault")
        .expect("create vault");
    let info = snapshot(&vault_path, &enabled_backup_settings())
        .expect("snapshot ok")
        .expect("snapshot created");

    let listed = service.list_backups(&db_path_str).expect("list ok");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].path, info.path);
}

// ============================================================================
// delete_backup command tests
// ============================================================================

#[test]
fn delete_backup_removes_a_snapshot_inside_an_open_vault_backup_dir() {
    // Service-level contract: with the vault open, deleting a snapshot path
    // that lives inside its backup directory removes the file from disk.
    let dir = tempdir().expect("tempdir");
    let vault_path = dir.path().join("vault.kdbx");
    let service = KdbxService::new();
    service
        .set_backup_settings(enabled_backup_settings())
        .expect("set settings");
    service
        .create(
            &vault_path.to_string_lossy(),
            "pw",
            "Delete Backup Test Vault",
        )
        .expect("create vault");
    let info = snapshot(&vault_path, &enabled_backup_settings())
        .expect("snapshot ok")
        .expect("snapshot created");
    assert!(info.path.exists(), "precondition: snapshot exists");

    service
        .delete_backup(&info.path.to_string_lossy())
        .expect("delete ok");

    assert!(!info.path.exists(), "snapshot must be deleted from disk");
}

#[test]
fn delete_backup_rejects_paths_outside_any_open_vaults_backup_dir() {
    // Path-safety guard: a path that does NOT resolve inside the backup
    // directory of any currently-open vault must be rejected. The check
    // protects against accidentally (or maliciously) deleting unrelated
    // files via the delete endpoint.
    let dir = tempdir().expect("tempdir");
    let unrelated = dir.path().join("not-a-backup.kdbx");
    std::fs::write(&unrelated, b"unrelated data").expect("write unrelated");

    let service = KdbxService::new();
    service
        .set_backup_settings(enabled_backup_settings())
        .expect("set settings");

    let result = service.delete_backup(&unrelated.to_string_lossy());
    assert!(
        result.is_err(),
        "delete must reject paths outside any open vault's backup dir, got {result:?}"
    );
    assert!(unrelated.exists(), "the unrelated file must remain on disk");
}

#[test]
fn delete_backup_rejects_when_no_vault_is_open() {
    // Defense-in-depth: with no vault open at all, every delete is rejected
    // because there is no backup directory to scope deletes against. This
    // matches the issue's 'for some open Vault' wording.
    let dir = tempdir().expect("tempdir");
    let some_file = dir.path().join("something.kdbx");
    std::fs::write(&some_file, b"x").expect("write");

    let service = KdbxService::new();
    service
        .set_backup_settings(enabled_backup_settings())
        .expect("set settings");
    let result = service.delete_backup(&some_file.to_string_lossy());
    assert!(
        result.is_err(),
        "with no vault open, delete must always reject; got {result:?}"
    );
    assert!(some_file.exists());
}

// ============================================================================
// backup-created event semantics (returned by save)
// ============================================================================

#[test]
fn save_returns_snapshot_info_when_a_snapshot_was_taken() {
    // The command layer emits `backup-created` only when a snapshot
    // actually landed on disk. The service must therefore tell the command
    // which path (if any) was just created. Returning Option<BackupInfo>
    // makes "nothing to emit" a typed state rather than a guess.
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("vault.kdbx");
    let db_path_str = db_path.to_string_lossy().to_string();

    let service = KdbxService::new();
    service
        .set_backup_settings(enabled_backup_settings())
        .expect("set settings");
    service
        .create(&db_path_str, "pw", "Save Snapshot Test")
        .expect("create");

    // create() wrote the initial bytes, so save() sees an existing source
    // and snapshots its pre-image. The returned info carries the path the
    // command layer hands to the `backup-created` event payload.
    let info = service
        .save(&db_path_str)
        .expect("save ok")
        .expect("save must produce a snapshot once the source exists");
    assert!(info.path.exists(), "returned snapshot must exist on disk");
}

#[test]
fn save_returns_none_when_backups_are_disabled() {
    // If the user has disabled backups in settings, save must complete
    // without snapshotting and the service must signal that no event
    // should be emitted.
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("vault.kdbx");
    let db_path_str = db_path.to_string_lossy().to_string();

    let service = KdbxService::new();
    service
        .set_backup_settings(BackupSettings {
            enabled: false,
            ..BackupSettings::default()
        })
        .expect("set settings");
    service
        .create(&db_path_str, "pw", "Disabled Backups Vault")
        .expect("create");

    let snapshot = service.save(&db_path_str).expect("save ok");
    assert!(
        snapshot.is_none(),
        "no snapshot must be reported when backups are disabled"
    );
}

#[cfg(unix)]
#[test]
fn delete_backup_rejects_paths_when_open_vaults_backup_dir_is_a_symlink() {
    // Regression for Codex P1 on #212. Without a pre-canonicalize symlink
    // guard, an attacker who plants a symlink at the per-Vault `.kdbx-backups/`
    // path can shift the allowed delete boundary to the symlink target. The
    // delete authorization would then accept any path under that target,
    // letting `delete_backup` remove arbitrary files outside the real backup
    // directory. The guard must reject this before canonicalization.
    let dir = tempdir().expect("tempdir");
    let vault_path = dir.path().join("vault.kdbx");
    let db_path_str = vault_path.to_string_lossy().to_string();

    let service = KdbxService::new();
    service
        .set_backup_settings(enabled_backup_settings())
        .expect("set settings");
    service
        .create(&db_path_str, "pw", "Symlink Guard Test Vault")
        .expect("create vault");

    // Replace the (not-yet-existing) backup dir with a symlink pointing at
    // an attacker-controlled directory.
    let elsewhere = tempdir().expect("tempdir elsewhere");
    let backup_link = dir
        .path()
        .join(mithril_vault_lib::services::kdbx::backups::BACKUP_SUBDIR);
    std::os::unix::fs::symlink(elsewhere.path(), &backup_link).expect("symlink");

    // Plant a file inside the symlink target. With the bug this file would
    // pass the `starts_with` check after `canonicalize(&backup_dir)` follows
    // the symlink.
    let victim = elsewhere.path().join("victim.txt");
    std::fs::write(&victim, b"do not delete me").expect("write victim");

    let result = service.delete_backup(&victim.to_string_lossy());
    assert!(
        result.is_err(),
        "delete_backup must reject when the open vault's backup dir is a symlink, got {result:?}"
    );
    assert!(
        victim.exists(),
        "the file inside the symlink target must remain on disk"
    );
}

#[test]
fn list_backups_rejects_paths_without_an_open_vault() {
    // The list_backups command is exposed over IPC. Accepting any path would
    // let a caller enumerate snapshot metadata (filenames, timestamps, sizes)
    // for vaults the user has not opened — a metadata-disclosure footgun
    // even if the bytes are encrypted. Scope listing to currently-open
    // vaults, matching delete_backup's authorization model.
    let dir = tempdir().expect("tempdir");
    let vault_path = dir.path().join("vault.kdbx");
    std::fs::write(&vault_path, b"data").expect("write source");
    // Seed a snapshot so the directory walk would otherwise succeed.
    snapshot(&vault_path, &enabled_backup_settings())
        .expect("snapshot ok")
        .expect("created");

    // No vault is open in this service instance.
    let service = KdbxService::new();
    service
        .set_backup_settings(enabled_backup_settings())
        .expect("set settings");

    let result = service.list_backups(&vault_path.to_string_lossy());
    assert!(
        matches!(result, Err(AppError::DatabaseNotFound(_))),
        "list_backups must reject paths that don't map to an open vault, got {result:?}"
    );
}

#[test]
fn delete_backup_rejects_non_snapshot_files_inside_backup_dir() {
    // Issue #194 specifies deleting a backup, not "any file inside the backup
    // directory". A malformed IPC call (or a future bug) must not be able to
    // delete a README, configuration file, or any other file someone has
    // placed alongside the snapshots. Authorization checks must verify both
    // location AND filename shape.
    let dir = tempdir().expect("tempdir");
    let vault_path = dir.path().join("vault.kdbx");
    let db_path_str = vault_path.to_string_lossy().to_string();

    let service = KdbxService::new();
    service
        .set_backup_settings(enabled_backup_settings())
        .expect("set settings");
    service
        .create(&db_path_str, "pw", "Filename Guard Test Vault")
        .expect("create vault");
    // Seed a real snapshot so the backup directory exists.
    snapshot(&vault_path, &enabled_backup_settings())
        .expect("snapshot ok")
        .expect("created");

    // Plant a non-snapshot file inside the backup dir.
    let backup_dir = dir
        .path()
        .join(mithril_vault_lib::services::kdbx::backups::BACKUP_SUBDIR);
    let bystander = backup_dir.join("README.txt");
    std::fs::write(&bystander, b"important notes, do not delete").expect("write bystander");

    let result = service.delete_backup(&bystander.to_string_lossy());
    assert!(
        result.is_err(),
        "delete_backup must reject non-snapshot filenames, got {result:?}"
    );
    assert!(
        bystander.exists(),
        "the non-snapshot file must remain on disk"
    );
}

#[test]
fn delete_backup_rejects_foreign_vaults_snapshot_inside_our_backup_dir() {
    // Defense-in-depth: a snapshot-shaped filename belonging to a *different*
    // vault that happens to live in our backup dir must not be deletable
    // through our open-vault authorization. The vault basename embedded in
    // the filename must match the open vault we authorize against.
    let dir = tempdir().expect("tempdir");
    let vault_path = dir.path().join("vault.kdbx");
    let db_path_str = vault_path.to_string_lossy().to_string();

    let service = KdbxService::new();
    service
        .set_backup_settings(enabled_backup_settings())
        .expect("set settings");
    service
        .create(&db_path_str, "pw", "Foreign Snapshot Test")
        .expect("create vault");
    snapshot(&vault_path, &enabled_backup_settings())
        .expect("snapshot ok")
        .expect("created");

    let backup_dir = dir
        .path()
        .join(mithril_vault_lib::services::kdbx::backups::BACKUP_SUBDIR);
    let foreign = backup_dir.join("other.kdbx.backup.20260101T000000.000Z.kdbx");
    std::fs::write(&foreign, b"foreign vault snapshot").expect("write foreign");

    let result = service.delete_backup(&foreign.to_string_lossy());
    assert!(
        result.is_err(),
        "delete_backup must reject foreign-vault snapshots, got {result:?}"
    );
    assert!(foreign.exists(), "foreign snapshot must remain on disk");
}
