#![allow(clippy::expect_used)]

use mithril_vault_lib::commands::settings::BackupSettings;
use mithril_vault_lib::dto::error::AppError;
use mithril_vault_lib::services::kdbx::backups::BACKUP_SUBDIR;
use mithril_vault_lib::services::kdbx::KdbxService;
use tempfile::tempdir;

#[path = "support/mod.rs"]
mod support;

use support::fixture_path;

#[test]
fn test_save_database() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("save-test.kdbx");
    let db_path_str = db_path.to_string_lossy().to_string();

    let service = KdbxService::new();
    service
        .create(&db_path_str, "savepass", "Save Test")
        .expect("Failed to create database");

    service.save(&db_path_str).expect("Failed to save database");

    service.close(&db_path_str).expect("Failed to close");
    service
        .open(&db_path_str, "savepass")
        .expect("Failed to reopen after save");
}

#[test]
fn test_save_without_open_database() {
    let service = KdbxService::new();
    let result = service.save("/nonexistent/db.kdbx");
    assert!(
        matches!(result, Err(AppError::DatabaseNotFound(_))),
        "Save should fail when no database is open"
    );
}

#[test]
fn test_save_as_new_path() {
    let dir = tempdir().expect("Failed to create temp dir");
    let original_path = dir.path().join("original.kdbx");
    let new_path = dir.path().join("copy.kdbx");
    let original_path_str = original_path.to_string_lossy().to_string();
    let new_path_str = new_path.to_string_lossy().to_string();

    let service = KdbxService::new();
    service
        .create(&original_path_str, "origpass", "Original")
        .expect("Failed to create database");

    service
        .save_as(&original_path_str, &new_path_str, None)
        .expect("Failed to save as");

    assert!(original_path.exists());
    assert!(new_path.exists());

    service.close(&new_path_str).expect("Failed to close");
    service
        .open(&new_path_str, "origpass")
        .expect("Failed to open new path");
}

#[test]
fn test_save_as_with_new_password() {
    let dir = tempdir().expect("Failed to create temp dir");
    let original_path = dir.path().join("original2.kdbx");
    let new_path = dir.path().join("newpass.kdbx");
    let original_path_str = original_path.to_string_lossy().to_string();
    let new_path_str = new_path.to_string_lossy().to_string();

    let service = KdbxService::new();
    service
        .create(&original_path_str, "oldpass", "Test DB")
        .expect("Failed to create database");

    service
        .save_as(&original_path_str, &new_path_str, Some("newpass123"))
        .expect("Failed to save as with new password");

    service.close(&new_path_str).expect("Failed to close");
    service
        .open(&new_path_str, "newpass123")
        .expect("Failed to open with new password");
}

#[test]
fn test_save_as_fails_when_target_database_is_already_open() {
    let dir = tempdir().expect("Failed to create temp dir");
    let source_path = dir.path().join("source.kdbx");
    let target_path = dir.path().join("target.kdbx");
    let source_path_str = source_path.to_string_lossy().to_string();
    let target_path_str = target_path.to_string_lossy().to_string();

    let service = KdbxService::new();
    service
        .create(&source_path_str, "sourcepass", "Source")
        .expect("Failed to create source database");
    service
        .create(&target_path_str, "targetpass", "Target")
        .expect("Failed to create target database");

    let result = service.save_as(&source_path_str, &target_path_str, None);
    assert!(
        matches!(result, Err(AppError::DatabaseAlreadyOpen(_))),
        "save_as should reject destination when target database is already open"
    );

    let source_info = service
        .get_info(&source_path_str)
        .expect("Source database should remain open");
    assert_eq!(source_info.path, source_path_str);

    let target_info = service
        .get_info(&target_path_str)
        .expect("Target database should remain open");
    assert_eq!(target_info.path, target_path_str);

    service
        .close(&source_path_str)
        .expect("Failed to close source database");
    service
        .close(&target_path_str)
        .expect("Failed to close target database");

    service
        .open(&target_path_str, "targetpass")
        .expect("Target database should still open with original password");
}

#[test]
fn test_save_preserves_keyfile_authentication() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("keyfile-save-test.kdbx");
    let db_path_str = db_path.to_string_lossy().to_string();

    let fixture_db = fixture_path("test-keyfile-kdbx4-low-KDF.kdbx");
    let fixture_key = fixture_path("test-keyfile.keyx");
    if !fixture_db.exists() || !fixture_key.exists() {
        eprintln!("Skipping test: keyfile fixtures not found");
        return;
    }
    std::fs::copy(&fixture_db, &db_path).expect("Failed to copy fixture");

    let service = KdbxService::new();

    service
        .open_with_keyfile(&db_path_str, "test123", &fixture_key.to_string_lossy())
        .expect("Failed to open with keyfile");

    service.save(&db_path_str).expect("Failed to save");
    service.close(&db_path_str).expect("Failed to close");

    let result = service.open(&db_path_str, "test123");
    assert!(
        matches!(result, Err(AppError::InvalidPassword)),
        "Database should still require keyfile after save"
    );

    service
        .open_with_keyfile(&db_path_str, "test123", &fixture_key.to_string_lossy())
        .expect("Should still open with keyfile after save");
}

#[test]
fn test_save_preserves_keyfile_only_authentication() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("keyfile-only-save-test.kdbx");
    let db_path_str = db_path.to_string_lossy().to_string();

    let fixture_db = fixture_path("test-keyfile-only-kdbx4-low-KDF.kdbx");
    let fixture_key = fixture_path("test-keyfile.keyx");
    if !fixture_db.exists() || !fixture_key.exists() {
        eprintln!("Skipping test: keyfile-only fixtures not found");
        return;
    }
    std::fs::copy(&fixture_db, &db_path).expect("Failed to copy fixture");

    let service = KdbxService::new();

    service
        .open_with_keyfile_only(&db_path_str, &fixture_key.to_string_lossy())
        .expect("Failed to open with keyfile only");

    service.save(&db_path_str).expect("Failed to save");
    service.close(&db_path_str).expect("Failed to close");

    let result = service.open(&db_path_str, "any_password");
    assert!(
        matches!(result, Err(AppError::InvalidPassword)),
        "Database should still require keyfile after save"
    );

    service
        .open_with_keyfile_only(&db_path_str, &fixture_key.to_string_lossy())
        .expect("Should still open with keyfile after save");
}

#[test]
fn test_save_as_preserves_keyfile_only_authentication() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("keyfile-only-save-as-test.kdbx");
    let new_path = dir.path().join("keyfile-only-copy.kdbx");
    let db_path_str = db_path.to_string_lossy().to_string();
    let new_path_str = new_path.to_string_lossy().to_string();

    let fixture_db = fixture_path("test-keyfile-only-kdbx4-low-KDF.kdbx");
    let fixture_key = fixture_path("test-keyfile.keyx");
    if !fixture_db.exists() || !fixture_key.exists() {
        eprintln!("Skipping test: keyfile-only fixtures not found");
        return;
    }
    std::fs::copy(&fixture_db, &db_path).expect("Failed to copy fixture");

    let service = KdbxService::new();

    service
        .open_with_keyfile_only(&db_path_str, &fixture_key.to_string_lossy())
        .expect("Failed to open with keyfile only");

    service
        .save_as(&db_path_str, &new_path_str, None)
        .expect("Failed to save as");
    service.close(&new_path_str).expect("Failed to close");

    let result = service.open(&new_path_str, "any_password");
    assert!(
        matches!(result, Err(AppError::InvalidPassword)),
        "New file should still require keyfile"
    );

    service
        .open_with_keyfile_only(&new_path_str, &fixture_key.to_string_lossy())
        .expect("New file should open with keyfile");
}

#[test]
fn test_save_as_preserves_keyfile_plus_password_authentication() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("keyfile-pw-save-as-test.kdbx");
    let new_path = dir.path().join("keyfile-pw-copy.kdbx");
    let db_path_str = db_path.to_string_lossy().to_string();
    let new_path_str = new_path.to_string_lossy().to_string();

    let fixture_db = fixture_path("test-keyfile-kdbx4-low-KDF.kdbx");
    let fixture_key = fixture_path("test-keyfile.keyx");
    if !fixture_db.exists() || !fixture_key.exists() {
        eprintln!("Skipping test: keyfile fixtures not found");
        return;
    }
    std::fs::copy(&fixture_db, &db_path).expect("Failed to copy fixture");

    let service = KdbxService::new();

    service
        .open_with_keyfile(&db_path_str, "test123", &fixture_key.to_string_lossy())
        .expect("Failed to open with password + keyfile");

    service
        .save_as(&db_path_str, &new_path_str, None)
        .expect("Failed to save as");
    service.close(&new_path_str).expect("Failed to close");

    let result = service.open(&new_path_str, "test123");
    assert!(
        matches!(result, Err(AppError::InvalidPassword)),
        "New file should still require keyfile"
    );

    service
        .open_with_keyfile(&new_path_str, "test123", &fixture_key.to_string_lossy())
        .expect("New file should open with password + keyfile");
}

#[test]
fn test_save_as_with_new_password_on_keyfile_database() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("keyfile-newpw-test.kdbx");
    let new_path = dir.path().join("keyfile-newpw.kdbx");
    let db_path_str = db_path.to_string_lossy().to_string();
    let new_path_str = new_path.to_string_lossy().to_string();

    let fixture_db = fixture_path("test-keyfile-kdbx4-low-KDF.kdbx");
    let fixture_key = fixture_path("test-keyfile.keyx");
    if !fixture_db.exists() || !fixture_key.exists() {
        eprintln!("Skipping test: keyfile fixtures not found");
        return;
    }
    std::fs::copy(&fixture_db, &db_path).expect("Failed to copy fixture");

    let service = KdbxService::new();

    service
        .open_with_keyfile(&db_path_str, "test123", &fixture_key.to_string_lossy())
        .expect("Failed to open with password + keyfile");

    service
        .save_as(&db_path_str, &new_path_str, Some("newpassword456"))
        .expect("Failed to save as with new password");
    service.close(&new_path_str).expect("Failed to close");

    let result =
        service.open_with_keyfile(&new_path_str, "test123", &fixture_key.to_string_lossy());
    assert!(
        matches!(result, Err(AppError::InvalidPassword)),
        "Old password should not work on new file"
    );

    service
        .open_with_keyfile(
            &new_path_str,
            "newpassword456",
            &fixture_key.to_string_lossy(),
        )
        .expect("New password + keyfile should work");
}

#[test]
fn test_save_atomic_write_no_temp_file_remains() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("atomic-save.kdbx");
    let temp_path = dir.path().join(".atomic-save.kdbx.tmp");
    let db_path_str = db_path.to_string_lossy().to_string();

    let service = KdbxService::new();
    service
        .create(&db_path_str, "testpass", "Atomic Test")
        .expect("Failed to create database");

    service.save(&db_path_str).expect("Failed to save database");

    assert!(
        !temp_path.exists(),
        "Temp file should not exist after successful save"
    );

    service.close(&db_path_str).expect("Failed to close");
    service
        .open(&db_path_str, "testpass")
        .expect("Failed to reopen after atomic save");
}

#[test]
fn test_save_clears_is_modified_flag() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("modified-flag.kdbx");
    let db_path_str = db_path.to_string_lossy().to_string();

    let service = KdbxService::new();
    service
        .create(&db_path_str, "testpass", "Modified Flag Test")
        .expect("Failed to create database");

    let info = service.get_info(&db_path_str).expect("Failed to get info");
    assert!(
        !info.is_modified,
        "is_modified should be false after create"
    );

    service.save(&db_path_str).expect("Failed to save");
    let info_after_save = service.get_info(&db_path_str).expect("Failed to get info");
    assert!(
        !info_after_save.is_modified,
        "is_modified should be false after save"
    );
}

#[test]
fn test_save_as_creates_new_file_atomically() {
    let dir = tempdir().expect("Failed to create temp dir");
    let original_path = dir.path().join("original-atomic.kdbx");
    let new_path = dir.path().join("new-atomic.kdbx");
    let temp_path = dir.path().join(".new-atomic.kdbx.tmp");
    let original_path_str = original_path.to_string_lossy().to_string();
    let new_path_str = new_path.to_string_lossy().to_string();

    let service = KdbxService::new();
    service
        .create(&original_path_str, "testpass", "Original")
        .expect("Failed to create database");

    service
        .save_as(&original_path_str, &new_path_str, None)
        .expect("Failed to save as");

    assert!(original_path.exists(), "Original file should exist");
    assert!(new_path.exists(), "New file should exist");

    assert!(!temp_path.exists(), "Temp file should not exist");

    service.close(&new_path_str).expect("Failed to close");
    service
        .open(&new_path_str, "testpass")
        .expect("Failed to open new file");
}

#[cfg(unix)]
#[test]
fn test_save_sets_secure_permissions_on_new_file() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("secure-perms.kdbx");
    let db_path_str = db_path.to_string_lossy().to_string();

    let service = KdbxService::new();
    service
        .create(&db_path_str, "testpass", "Secure Perms Test")
        .expect("Failed to create database");

    let metadata = std::fs::metadata(&db_path).expect("Should get metadata");
    let mode = metadata.permissions().mode() & 0o777;

    assert_eq!(
        mode, 0o600,
        "New database file should have 0600 permissions, got {mode:o}"
    );
}

#[cfg(unix)]
#[test]
fn test_save_preserves_existing_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("preserved-perms.kdbx");
    let db_path_str = db_path.to_string_lossy().to_string();

    let service = KdbxService::new();
    service
        .create(&db_path_str, "testpass", "Preserve Perms Test")
        .expect("Failed to create database");

    let mut perms = std::fs::metadata(&db_path)
        .expect("Should get metadata")
        .permissions();
    perms.set_mode(0o640);
    std::fs::set_permissions(&db_path, perms).expect("Failed to set permissions");

    service.save(&db_path_str).expect("Failed to save");

    let metadata_after = std::fs::metadata(&db_path).expect("Should get metadata after save");
    let mode_after = metadata_after.permissions().mode() & 0o777;

    assert_eq!(
        mode_after, 0o640,
        "Permissions should be preserved after save, got {mode_after:o}"
    );
}

#[test]
fn test_save_creates_pre_image_backup_when_enabled() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("with-backups.kdbx");
    let db_path_str = db_path.to_string_lossy().to_string();
    let backup_dir = dir.path().join(BACKUP_SUBDIR);

    let service = KdbxService::new();
    service
        .create(&db_path_str, "pw", "Backed Up")
        .expect("create");

    // First save after create: file exists on disk, snapshot the pre-image bytes.
    let pre_image_bytes = std::fs::read(&db_path).expect("read pre-image");
    service.save(&db_path_str).expect("save");

    assert!(backup_dir.exists(), ".kdbx-backups/ should be created");
    let entries: Vec<_> = std::fs::read_dir(&backup_dir)
        .expect("read backup dir")
        .filter_map(Result::ok)
        .collect();
    assert_eq!(entries.len(), 1, "exactly one backup should be created");

    let backup_bytes = std::fs::read(entries[0].path()).expect("read backup");
    assert_eq!(
        backup_bytes, pre_image_bytes,
        "backup bytes should equal pre-save vault bytes"
    );
}

#[test]
fn test_save_with_backups_disabled_creates_no_files() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("no-backups.kdbx");
    let db_path_str = db_path.to_string_lossy().to_string();
    let backup_dir = dir.path().join(BACKUP_SUBDIR);

    let service = KdbxService::new();
    service
        .set_backup_settings(BackupSettings { enabled: false })
        .expect("set settings");
    service
        .create(&db_path_str, "pw", "Disabled")
        .expect("create");

    service.save(&db_path_str).expect("save");

    assert!(
        !backup_dir.exists(),
        "no .kdbx-backups/ when backups disabled"
    );
}

#[test]
fn test_save_as_to_fresh_path_creates_no_backup_first_time() {
    let dir = tempdir().expect("Failed to create temp dir");
    let original_path = dir.path().join("orig.kdbx");
    let new_path = dir.path().join("fresh.kdbx");
    let original_str = original_path.to_string_lossy().to_string();
    let new_str = new_path.to_string_lossy().to_string();
    let new_backup_dir = dir.path().join(BACKUP_SUBDIR);

    let service = KdbxService::new();
    service.create(&original_str, "pw", "Orig").expect("create");

    service
        .save_as(&original_str, &new_str, None)
        .expect("save_as");

    // The fresh new_path had no pre-image when save_as targeted it. AC #5.
    // Note: original's backup dir may or may not exist depending on whether
    // save_as snapshots the source — per spec the hook is in save(), not save_as.
    // The acceptance criterion here is that the *new* path's saves behave
    // like a fresh vault — no backup until a real save happens against it.
    let new_backup_count = std::fs::read_dir(&new_backup_dir)
        .ok()
        .map_or(0, |it| it.filter_map(Result::ok).count());
    assert_eq!(
        new_backup_count, 0,
        "save_as to fresh path should not snapshot at new path"
    );

    // Subsequent save() at new_path should create a backup.
    service.save(&new_str).expect("save at new path");
    let after = std::fs::read_dir(&new_backup_dir)
        .expect("backup dir should exist now")
        .filter_map(Result::ok)
        .count();
    assert_eq!(after, 1, "subsequent save should create one backup");
}

#[cfg(unix)]
#[test]
fn test_save_fails_closed_when_backup_dir_unwritable() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("locked-backups.kdbx");
    let db_path_str = db_path.to_string_lossy().to_string();

    let service = KdbxService::new();
    service
        .create(&db_path_str, "pw", "Locked")
        .expect("create");
    let pre_save_bytes = std::fs::read(&db_path).expect("read pre-save");

    // Make the parent read-only so .kdbx-backups/ cannot be created.
    let mut perms = std::fs::metadata(dir.path()).expect("meta").permissions();
    let original = perms.mode();
    perms.set_mode(0o500);
    std::fs::set_permissions(dir.path(), perms).expect("ro");

    let result = service.save(&db_path_str);

    // Restore so cleanup works regardless of outcome.
    let mut restore = std::fs::metadata(dir.path()).expect("meta").permissions();
    restore.set_mode(original);
    std::fs::set_permissions(dir.path(), restore).expect("restore");

    assert!(
        result.is_err(),
        "save should fail when backup cannot be made"
    );
    let post_bytes = std::fs::read(&db_path).expect("read post");
    assert_eq!(
        post_bytes, pre_save_bytes,
        "vault file must be unchanged when save aborts on backup failure"
    );
}

#[test]
fn test_create_database_uses_atomic_write() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("atomic-create.kdbx");
    let temp_path = dir.path().join(".atomic-create.kdbx.tmp");
    let db_path_str = db_path.to_string_lossy().to_string();

    let service = KdbxService::new();
    service
        .create(&db_path_str, "testpass", "Atomic Create")
        .expect("Failed to create database");

    assert!(db_path.exists(), "Database file should exist");

    assert!(
        !temp_path.exists(),
        "Temp file should not exist after create"
    );

    service.close(&db_path_str).expect("Failed to close");
    service
        .open(&db_path_str, "testpass")
        .expect("Failed to reopen database");
}
