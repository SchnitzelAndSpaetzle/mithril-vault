// SPDX-License-Identifier: MIT

use crate::dto::database::{
    CustomIconData, DatabaseConfigDto, DatabaseCreationOptions, DatabaseHeaderInfo, DatabaseInfo,
};
use crate::dto::error::AppError;
use crate::services::audit::format::Reason;
use crate::services::audit::AuditService;
use crate::services::auto_lock::AutoLockService;
use crate::services::kdbx::backups::{BackupError, BackupInfo, BackupListEntry};
use crate::services::kdbx::KdbxService;
use serde::Serialize;
use std::path::Path;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime, State};

/// Maps the open/unlock result through the audit subsystem.
///
/// On a successful locked→unlocked transition (`was_locked == true`),
/// records exactly one `vault.opened` event and resets the per-Vault
/// failed-unlock counter. A successful no-op unlock (already unlocked)
/// records nothing — the user did not actually open anything.
///
/// On `InvalidPassword`, records one `vault.unlock_failed` event carrying
/// the running consecutive-failure count.
///
/// Returns the original result unchanged. Audit failures cannot bubble up:
/// the audit service flips an internal `degraded` flag and otherwise stays
/// silent so the user's unlock UX is unaffected.
fn record_open_outcome<T>(
    audit: &AuditService,
    path: &str,
    was_locked: bool,
    result: Result<T, AppError>,
) -> Result<T, AppError> {
    let vault_path = Path::new(path);
    match &result {
        Ok(_) if was_locked => audit.record_vault_opened(vault_path),
        Err(AppError::InvalidPassword) => audit.record_vault_unlock_failed(vault_path),
        Ok(_) | Err(_) => {}
    }
    result
}

/// Maps the lock result through the audit subsystem.
///
/// Records exactly one `vault.locked` event with the given reason iff the
/// lock represents a real unlocked→locked transition. A no-op lock (the
/// Vault was already locked) records nothing so audit history is not
/// padded with phantom events on redundant lock calls.
///
/// Returns the original result unchanged. Audit failures are swallowed
/// internally — see [`record_open_outcome`].
fn record_lock_outcome<T>(
    audit: &AuditService,
    path: &str,
    was_unlocked: bool,
    reason: Reason,
    result: Result<T, AppError>,
) -> Result<T, AppError> {
    if result.is_ok() && was_unlocked {
        audit.record_vault_locked(Path::new(path), reason);
    }
    result
}

/// Payload of the `backup-warning` event. The frontend renders this as a
/// non-blocking toast and never as a modal — open-side backup failures must
/// not interrupt the user.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BackupWarningPayload {
    path: String,
    reason: String,
}

/// Payload shared by `backup-created` and `backup-deleted` events. Carries
/// the snapshot path so the Settings → Backups list can refresh live without
/// re-fetching state the backend has already produced.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BackupEventPayload {
    path: String,
}

/// Invokes the open-side backup hook and converts a `BackupFailed` into a
/// non-blocking `backup-warning` event. Never returns an error — the unlock
/// has already succeeded by the time we reach this point and a snapshot
/// problem must not bubble up to the caller.
///
/// On success, emits `backup-created` so the Settings → Backups list can
/// refresh without polling.
fn emit_open_backup_hook<R: Runtime>(app: &AppHandle<R>, state: &KdbxService, db_id: &str) {
    match state.snapshot_after_open(db_id) {
        Ok(Some(info)) => {
            let _ = app.emit(
                "backup-created",
                BackupEventPayload {
                    path: info.path.to_string_lossy().into_owned(),
                },
            );
        }
        Ok(None) => {}
        Err(BackupError::BackupFailed { path, source }) => {
            let _ = app.emit(
                "backup-warning",
                BackupWarningPayload {
                    path: path.to_string_lossy().into_owned(),
                    reason: source.to_string(),
                },
            );
        }
    }
}

/// Opens a database with a password.
#[tauri::command]
pub async fn open_database<R: Runtime>(
    path: String,
    password: String,
    app: AppHandle<R>,
    state: State<'_, Arc<KdbxService>>,
    audit: State<'_, Arc<AuditService>>,
) -> Result<DatabaseInfo, AppError> {
    let info = record_open_outcome(&audit, &path, true, state.open(&path, &password))?;
    emit_open_backup_hook(&app, &state, &path);
    Ok(info)
}

/// Closes a specific open database.
/// The `db_id` is the path to the database file.
#[tauri::command]
pub async fn close_database(
    db_id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<(), AppError> {
    state.close(&db_id)
}

/// Create a new KDBX4 database
///
/// # Parameters
/// - `path`: File path where the database will be saved
/// - `name`: Database name (also used as root group name)
/// - `password`: Optional password (required if no keyfile)
/// - `keyfile_path`: Optional path to keyfile for authentication
/// - `options`: Optional creation options (KDF settings, default groups, description)
#[tauri::command]
pub async fn create_database(
    path: String,
    name: String,
    password: Option<String>,
    keyfile_path: Option<String>,
    options: Option<DatabaseCreationOptions>,
    state: State<'_, Arc<KdbxService>>,
) -> Result<DatabaseInfo, AppError> {
    state.create_database(
        &path,
        password.as_deref(),
        keyfile_path.as_deref(),
        &name,
        &options.unwrap_or_default(),
    )
}

/// Saves a specific open database.
/// The `db_id` is the path to the database file.
///
/// Emits `backup-created` when the save took a pre-image snapshot. The
/// frontend Backups list subscribes to that event and refreshes live.
#[tauri::command]
pub async fn save_database<R: Runtime>(
    db_id: String,
    app: AppHandle<R>,
    state: State<'_, Arc<KdbxService>>,
) -> Result<(), AppError> {
    let snapshot_info = state.save(&db_id)?;
    if let Some(info) = snapshot_info {
        let _ = app.emit(
            "backup-created",
            BackupEventPayload {
                path: info.path.to_string_lossy().into_owned(),
            },
        );
    }
    Ok(())
}

/// Opens a database with password and keyfile.
#[tauri::command]
pub async fn open_database_with_keyfile<R: Runtime>(
    path: String,
    password: String,
    keyfile_path: String,
    app: AppHandle<R>,
    state: State<'_, Arc<KdbxService>>,
    audit: State<'_, Arc<AuditService>>,
) -> Result<DatabaseInfo, AppError> {
    let info = record_open_outcome(
        &audit,
        &path,
        true,
        state.open_with_keyfile(&path, &password, &keyfile_path),
    )?;
    emit_open_backup_hook(&app, &state, &path);
    Ok(info)
}

/// Opens a database using only a keyfile.
#[tauri::command]
pub async fn open_database_with_keyfile_only<R: Runtime>(
    path: String,
    keyfile_path: String,
    app: AppHandle<R>,
    state: State<'_, Arc<KdbxService>>,
    audit: State<'_, Arc<AuditService>>,
) -> Result<DatabaseInfo, AppError> {
    let info = record_open_outcome(
        &audit,
        &path,
        true,
        state.open_with_keyfile_only(&path, &keyfile_path),
    )?;
    emit_open_backup_hook(&app, &state, &path);
    Ok(info)
}

/// Locks the database session by dropping decrypted data from memory.
///
/// On a real unlocked→locked transition, records one `vault.locked`
/// event with `reason: manual`. A no-op lock (already locked) records
/// nothing.
#[tauri::command]
pub async fn lock_database(
    db_id: String,
    state: State<'_, Arc<KdbxService>>,
    audit: State<'_, Arc<AuditService>>,
) -> Result<DatabaseInfo, AppError> {
    let was_unlocked = !state.is_database_locked(&db_id)?.unwrap_or(true);
    record_lock_outcome(
        &audit,
        &db_id,
        was_unlocked,
        Reason::Manual,
        state.lock(&db_id),
    )
}

/// Unlocks the database session by re-opening from disk with optional password.
#[tauri::command]
pub async fn unlock_database<R: Runtime>(
    db_id: String,
    password: Option<String>,
    app: AppHandle<R>,
    state: State<'_, Arc<KdbxService>>,
    audit: State<'_, Arc<AuditService>>,
) -> Result<DatabaseInfo, AppError> {
    // Snapshot the locked state BEFORE calling unlock so we can tell apart
    // an actual locked → unlocked transition from the no-op case where the
    // caller invoked unlock on an already-unlocked DB. Without this guard
    // the open-side backup hook would re-fire on every redundant unlock —
    // wasted work in the happy path, and a duplicated `backup-warning`
    // event whenever the backup dir is broken.
    let was_locked = state.is_database_locked(&db_id)?.unwrap_or(true);
    let info = record_open_outcome(
        &audit,
        &db_id,
        was_locked,
        state.unlock(&db_id, password.as_deref()),
    )?;
    if was_locked {
        emit_open_backup_hook(&app, &state, &db_id);
    }
    Ok(info)
}

/// Inspects a KDBX file without requiring credentials.
/// Returns header information including version and validity status.
#[tauri::command]
pub async fn inspect_database(
    path: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<DatabaseHeaderInfo, AppError> {
    state.inspect(&path)
}

/// Returns the cryptographic configuration of a specific open database.
/// Requires the database to be open (authenticated).
#[tauri::command]
pub async fn get_database_config(
    db_id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<DatabaseConfigDto, AppError> {
    state.get_config(&db_id)
}

/// Gets info about a specific open database.
/// The `db_id` is the path to the database file.
/// Returns None if the database is not open.
#[tauri::command]
pub async fn get_database_info(
    db_id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Option<DatabaseInfo>, AppError> {
    match state.get_info(&db_id) {
        Ok(info) => Ok(Some(info)),
        Err(AppError::DatabaseNotFound(_)) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Returns custom icons for a specific open database.
#[tauri::command]
pub async fn get_custom_icons(
    db_id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<std::collections::HashMap<String, CustomIconData>, AppError> {
    state.get_custom_icons(&db_id)
}

/// Creates a manual (rotation-exempt) backup snapshot for an open vault.
///
/// Emits `backup-created` on success so the Settings → Backups list refreshes
/// live without polling. Returns the snapshot path so the caller can show a
/// confirmation toast referencing the new file.
///
/// Manual snapshots ignore `backups.enabled` — the UI hides the button when
/// the auto-backup toggle is off, but the command itself does not gate on it.
#[tauri::command]
pub async fn create_manual_backup<R: Runtime>(
    database_path: String,
    app: AppHandle<R>,
    state: State<'_, Arc<KdbxService>>,
) -> Result<BackupInfo, AppError> {
    let info = state.create_manual_backup(&database_path)?;
    let _ = app.emit(
        "backup-created",
        BackupEventPayload {
            path: info.path.to_string_lossy().into_owned(),
        },
    );
    Ok(info)
}

/// Lists snapshot backups for a vault on disk, newest-first.
///
/// Returns an empty list when the vault has never been backed up (the backup
/// directory does not yet exist). The frontend gates this on having a vault
/// open and falls back to an empty state when none is.
#[tauri::command]
pub async fn list_backups(
    database_path: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Vec<BackupListEntry>, AppError> {
    state.list_backups(&database_path)
}

/// Deletes a backup snapshot from disk after verifying it belongs to an
/// open vault's backup directory.
///
/// Emits a `backup-deleted` Tauri event on success carrying the deleted
/// path so the Backups list refreshes without a polling cycle.
#[tauri::command]
pub async fn delete_backup<R: Runtime>(
    backup_path: String,
    app: AppHandle<R>,
    state: State<'_, Arc<KdbxService>>,
) -> Result<(), AppError> {
    state.delete_backup(&backup_path)?;
    let _ = app.emit("backup-deleted", BackupEventPayload { path: backup_path });
    Ok(())
}

/// Restores a backup snapshot over the Vault it belongs to.
///
/// The backup is matched to an open Vault by basename + canonical-path
/// containment in that Vault's resolved backup directory (see
/// `KdbxService::restore_backup`). A pre-restore pre-image snapshot of the
/// current Vault state is taken first using the save-side fail-closed
/// semantics — if that snapshot fails, the restore aborts with the Vault file
/// unchanged.
///
/// On success the open-Vault entry is removed from the service so the stale
/// in-memory state cannot drift from the new on-disk bytes, and a
/// `database-closed` event is emitted so the frontend can route to the
/// unlock screen.
///
/// The command never calls `add_recent_database`; backup paths must not
/// enter the recent-Vaults list.
#[tauri::command]
pub async fn restore_backup<R: Runtime>(
    backup_path: String,
    app: AppHandle<R>,
    state: State<'_, Arc<KdbxService>>,
) -> Result<(), AppError> {
    let source_path = state.restore_backup(&backup_path)?;
    let _ = app.emit(
        "database-closed",
        DatabaseClosedPayload {
            path: source_path,
            reason: "restore",
        },
    );
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DatabaseClosedPayload {
    path: String,
    /// Discriminator that tells the frontend why the database was closed.
    /// Today the only producer is `restore`; future producers (e.g. external
    /// file removal) can add their own value without breaking listeners.
    reason: &'static str,
}

/// Lists all currently open databases.
#[tauri::command]
pub async fn list_open_databases(
    state: State<'_, Arc<KdbxService>>,
) -> Result<Vec<DatabaseInfo>, AppError> {
    state.list_open_databases()
}

/// Generates a new `KeePass` 2.x compatible keyfile (.keyx format).
///
/// The keyfile contains 32 bytes of cryptographically random data
/// in an XML format compatible with `KeePass` 2.x and other implementations.
///
/// # Parameters
/// - `output_path`: Path where the keyfile will be saved
#[tauri::command]
pub async fn generate_keyfile(output_path: String) -> Result<(), AppError> {
    crate::services::kdbx::keyfile::generate_keyfile(&output_path)
}

/// Reports user activity to reset the auto-lock timeout.
#[tauri::command]
pub async fn report_activity(state: State<'_, Arc<AutoLockService>>) -> Result<(), AppError> {
    state.report_activity();
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod audit_helper_tests {
    use super::*;
    use crate::services::audit::format::AuditEvent;
    use crate::services::audit::key::InMemoryAuditKey;
    use crate::services::audit::AuditFilter;
    use tempfile::tempdir;

    fn fresh_service() -> (AuditService, tempfile::TempDir, std::path::PathBuf) {
        let dir = tempdir().expect("tempdir");
        let vault = dir.path().join("vault.kdbx");
        std::fs::write(&vault, b"x").expect("write vault");
        let svc = AuditService::new(dir.path().join("audit"), Arc::new(InMemoryAuditKey::new()));
        (svc, dir, vault)
    }

    #[test]
    fn record_open_outcome_emits_vault_opened_when_was_locked_and_ok() {
        let (audit, _dir, vault) = fresh_service();
        let path = vault.to_str().expect("utf8 path");

        let r: Result<(), AppError> = record_open_outcome(&audit, path, true, Ok(()));
        assert!(r.is_ok());

        let events = audit.read(&vault, &AuditFilter::default()).expect("read");
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], AuditEvent::VaultOpened { .. }));
    }

    #[test]
    fn record_open_outcome_skips_emit_on_no_op_unlock() {
        let (audit, _dir, vault) = fresh_service();
        let path = vault.to_str().expect("utf8 path");

        let r: Result<(), AppError> = record_open_outcome(&audit, path, false, Ok(()));
        assert!(r.is_ok());

        assert!(audit
            .read(&vault, &AuditFilter::default())
            .expect("read")
            .is_empty());
    }

    #[test]
    fn record_open_outcome_emits_unlock_failed_regardless_of_was_locked() {
        // A failed unlock is always meaningful — record it whether or not
        // the pre-check observed the DB as locked (the failure itself says
        // the user tried to unlock it).
        let (audit, _dir, vault) = fresh_service();
        let path = vault.to_str().expect("utf8 path");

        let r: Result<(), AppError> =
            record_open_outcome(&audit, path, true, Err(AppError::InvalidPassword));
        assert!(matches!(r, Err(AppError::InvalidPassword)));

        let events = audit.read(&vault, &AuditFilter::default()).expect("read");
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], AuditEvent::VaultUnlockFailed { .. }));
    }

    #[test]
    fn record_open_outcome_other_error_records_nothing() {
        let (audit, _dir, vault) = fresh_service();
        let path = vault.to_str().expect("utf8 path");

        let r: Result<(), AppError> =
            record_open_outcome(&audit, path, true, Err(AppError::DatabaseNotOpen));
        assert!(matches!(r, Err(AppError::DatabaseNotOpen)));

        assert!(audit
            .read(&vault, &AuditFilter::default())
            .expect("read")
            .is_empty());
    }

    #[test]
    fn record_lock_outcome_emits_manual_lock_on_real_transition() {
        let (audit, _dir, vault) = fresh_service();
        let path = vault.to_str().expect("utf8 path");

        let r: Result<(), AppError> =
            record_lock_outcome(&audit, path, true, Reason::Manual, Ok(()));
        assert!(r.is_ok());

        let events = audit.read(&vault, &AuditFilter::default()).expect("read");
        assert_eq!(events.len(), 1);
        match &events[0] {
            AuditEvent::VaultLocked { reason, .. } => assert_eq!(*reason, Reason::Manual),
            other => panic!("expected VaultLocked, got {other:?}"),
        }
    }

    #[test]
    fn record_lock_outcome_skips_emit_on_already_locked_no_op() {
        let (audit, _dir, vault) = fresh_service();
        let path = vault.to_str().expect("utf8 path");

        let r: Result<(), AppError> =
            record_lock_outcome(&audit, path, false, Reason::Manual, Ok(()));
        assert!(r.is_ok());

        assert!(audit
            .read(&vault, &AuditFilter::default())
            .expect("read")
            .is_empty());
    }

    #[test]
    fn record_lock_outcome_skips_emit_on_error() {
        let (audit, _dir, vault) = fresh_service();
        let path = vault.to_str().expect("utf8 path");

        let r: Result<(), AppError> = record_lock_outcome(
            &audit,
            path,
            true,
            Reason::Manual,
            Err(AppError::DatabaseNotOpen),
        );
        assert!(matches!(r, Err(AppError::DatabaseNotOpen)));

        assert!(audit
            .read(&vault, &AuditFilter::default())
            .expect("read")
            .is_empty());
    }
}
