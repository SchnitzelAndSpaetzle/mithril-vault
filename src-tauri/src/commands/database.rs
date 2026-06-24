// SPDX-License-Identifier: MIT

use crate::dto::database::{
    CustomIconData, DatabaseConfigDto, DatabaseCreationOptions, DatabaseHeaderInfo, DatabaseInfo,
    VaultHistorySettings,
};
use crate::dto::error::AppError;
use crate::services::audit::format::Reason;
use crate::services::audit::AuditService;
use crate::services::auto_lock::AutoLockService;
use crate::services::kdbx::backups::{BackupError, BackupInfo, BackupListEntry};
use crate::services::kdbx::KdbxService;
use crate::services::password_health::service::PasswordHealthService;
use serde::Serialize;
use std::path::Path;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime, State};

/// Maps an `open_database*` result through the audit subsystem.
///
/// `KdbxService::open*` errors with `DatabaseAlreadyOpen` if the Vault is
/// already in the open-map, so a successful open is by construction a
/// fresh transition — no TOCTOU pre-check is needed. Records exactly
/// one `vault.opened` on success and one `vault.unlock_failed` on
/// `InvalidPassword`.
fn record_open_audit<T>(
    audit: &AuditService,
    path: &str,
    result: Result<T, AppError>,
) -> Result<T, AppError> {
    let vault_path = Path::new(path);
    match &result {
        Ok(_) => audit.record_vault_opened(vault_path),
        Err(AppError::InvalidPassword) => audit.record_vault_unlock_failed(vault_path),
        Err(_) => {}
    }
    result
}

/// Maps an `unlock` result through the audit subsystem.
///
/// `KdbxService::unlock` returns `(info, did_transition)` where the bool
/// is computed inside the same mutex that mutates the open-database
/// state. Audit decisions gate on that flag — two concurrent callers
/// observing a locked DB and both succeeding cannot both record
/// `vault.opened`, because only one of them actually performs the
/// transition. On `InvalidPassword`, records `vault.unlock_failed`.
fn record_unlock_audit(
    audit: &AuditService,
    path: &str,
    result: Result<(DatabaseInfo, bool), AppError>,
) -> Result<(DatabaseInfo, bool), AppError> {
    let vault_path = Path::new(path);
    match &result {
        Ok((_, true)) => audit.record_vault_opened(vault_path),
        Err(AppError::InvalidPassword) => audit.record_vault_unlock_failed(vault_path),
        Ok((_, false)) | Err(_) => {}
    }
    result
}

/// Maps a `lock` result through the audit subsystem.
///
/// Same TOCTOU-safety story as [`record_unlock_audit`]: the
/// `did_transition` flag is computed inside `KdbxService::lock`'s
/// mutex, so audit emit on `Ok((_, true))` records exactly one
/// `vault.locked` per real unlocked→locked transition even under
/// concurrent lock requests.
fn record_lock_audit(
    audit: &AuditService,
    path: &str,
    reason: Reason,
    result: Result<(DatabaseInfo, bool), AppError>,
) -> Result<(DatabaseInfo, bool), AppError> {
    if let Ok((_, true)) = &result {
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
    let info = record_open_audit(&audit, &path, state.open(&path, &password))?;
    emit_open_backup_hook(&app, &state, &path);
    Ok(info)
}

/// Closes a specific open database.
/// The `db_id` is the path to the database file.
///
/// Evicts the password-health cache slot *after* the close succeeds so
/// a subsequent open of the same path computes a fresh report instead
/// of returning the previous session's snapshot — generation restarts
/// at 0 on a fresh open, which would otherwise look like a cache hit.
#[tauri::command]
pub async fn close_database(
    db_id: String,
    state: State<'_, Arc<KdbxService>>,
    health: State<'_, Arc<PasswordHealthService>>,
) -> Result<(), AppError> {
    state.close(&db_id)?;
    health.on_lock(&db_id);
    Ok(())
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
    let info = record_open_audit(
        &audit,
        &path,
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
    let info = record_open_audit(
        &audit,
        &path,
        state.open_with_keyfile_only(&path, &keyfile_path),
    )?;
    emit_open_backup_hook(&app, &state, &path);
    Ok(info)
}

/// Locks the database session by dropping decrypted data from memory.
///
/// `KdbxService::lock` returns `(info, did_transition)` computed inside
/// the open-database mutex; the audit emit gates on that flag so
/// concurrent lock calls cannot both record a `vault.locked` event for
/// the same transition.
#[tauri::command]
pub async fn lock_database(
    db_id: String,
    state: State<'_, Arc<KdbxService>>,
    audit: State<'_, Arc<AuditService>>,
    health: State<'_, Arc<PasswordHealthService>>,
) -> Result<DatabaseInfo, AppError> {
    let (info, did_transition) =
        record_lock_audit(&audit, &db_id, Reason::Manual, state.lock(&db_id))?;
    if did_transition {
        // Drop the cached report so a future unlock-then-read pulls a
        // fresh analysis instead of returning a snapshot from the
        // pre-lock session. Eviction is idempotent — calling it on a
        // redundant lock is harmless.
        health.on_lock(&db_id);
    }
    Ok(info)
}

/// Unlocks the database session by re-opening from disk with optional password.
///
/// `KdbxService::unlock` returns `(info, did_transition)` computed inside
/// its mutex; both the audit `vault.opened` emit and the open-side backup
/// hook gate on that flag, so a redundant unlock on an already-unlocked
/// DB neither double-records nor re-fires the backup snapshot — and two
/// racing unlock calls can't both observe themselves as the transitioning
/// one.
#[tauri::command]
pub async fn unlock_database<R: Runtime>(
    db_id: String,
    password: Option<String>,
    app: AppHandle<R>,
    state: State<'_, Arc<KdbxService>>,
    audit: State<'_, Arc<AuditService>>,
) -> Result<DatabaseInfo, AppError> {
    let (info, did_transition) =
        record_unlock_audit(&audit, &db_id, state.unlock(&db_id, password.as_deref()))?;
    if did_transition {
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

/// Reads the per-Vault Entry-History retention (`Meta.history_max_items`) — the
/// writable vault-meta surface, distinct from the read-only Database Config.
#[tauri::command]
pub async fn get_vault_history_settings(
    db_id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<VaultHistorySettings, AppError> {
    state.get_vault_history_settings(&db_id)
}

/// Writes the per-Vault `History Limit` into `Meta.history_max_items`. `None`
/// clears the field (effective default 10); negative = unlimited; `0` =
/// disabled; positive `n` = keep newest `n`. The change persists on next save.
#[tauri::command]
pub async fn update_vault_history_settings(
    db_id: String,
    max_items: Option<i32>,
    state: State<'_, Arc<KdbxService>>,
) -> Result<(), AppError> {
    state.update_vault_history_settings(&db_id, max_items)
}

/// Clears every Entry's history across the whole Vault, emptying each native
/// KDBX `Entry.history` (ADR-0008). Live content is untouched; the change
/// persists on next save. Clearing history is not audited (per the PRD).
#[tauri::command]
pub async fn clear_all_history(
    db_id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<(), AppError> {
    state.clear_all_history(&db_id)
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
    health: State<'_, Arc<PasswordHealthService>>,
) -> Result<(), AppError> {
    let source_path = state.restore_backup(&backup_path)?;
    // Restore replaces the on-disk file under the Vault that owns
    // `source_path`. The in-memory slot is locked-in-place rather than
    // removed, so the lock-time eviction wired into `lock_database`
    // does not run here. Evict explicitly so the next unlock recomputes
    // against the freshly-restored bytes instead of serving the
    // pre-restore report from cache.
    health.on_lock(&source_path);
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

    fn stub_info() -> DatabaseInfo {
        DatabaseInfo {
            name: "stub".to_string(),
            path: "stub".to_string(),
            is_modified: false,
            is_locked: false,
            root_group_id: "rg".to_string(),
            version: "v".to_string(),
        }
    }

    #[test]
    fn record_open_audit_emits_vault_opened_on_ok() {
        let (audit, _dir, vault) = fresh_service();
        let path = vault.to_str().expect("utf8 path");

        let r: Result<(), AppError> = record_open_audit(&audit, path, Ok(()));
        assert!(r.is_ok());

        let events = audit.read(&vault, &AuditFilter::default()).expect("read");
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], AuditEvent::VaultOpened { .. }));
    }

    #[test]
    fn record_open_audit_emits_unlock_failed_on_invalid_password() {
        let (audit, _dir, vault) = fresh_service();
        let path = vault.to_str().expect("utf8 path");

        let r: Result<(), AppError> =
            record_open_audit(&audit, path, Err(AppError::InvalidPassword));
        assert!(matches!(r, Err(AppError::InvalidPassword)));

        let events = audit.read(&vault, &AuditFilter::default()).expect("read");
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], AuditEvent::VaultUnlockFailed { .. }));
    }

    #[test]
    fn record_open_audit_other_error_records_nothing() {
        let (audit, _dir, vault) = fresh_service();
        let path = vault.to_str().expect("utf8 path");

        let r: Result<(), AppError> =
            record_open_audit(&audit, path, Err(AppError::DatabaseNotOpen));
        assert!(matches!(r, Err(AppError::DatabaseNotOpen)));

        assert!(audit
            .read(&vault, &AuditFilter::default())
            .expect("read")
            .is_empty());
    }

    #[test]
    fn record_unlock_audit_emits_vault_opened_only_on_real_transition() {
        let (audit, _dir, vault) = fresh_service();
        let path = vault.to_str().expect("utf8 path");

        let r = record_unlock_audit(&audit, path, Ok((stub_info(), true)));
        assert!(r.is_ok());
        let events = audit.read(&vault, &AuditFilter::default()).expect("read");
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], AuditEvent::VaultOpened { .. }));
    }

    #[test]
    fn record_unlock_audit_skips_emit_on_no_op_unlock() {
        let (audit, _dir, vault) = fresh_service();
        let path = vault.to_str().expect("utf8 path");

        let r = record_unlock_audit(&audit, path, Ok((stub_info(), false)));
        assert!(r.is_ok());
        assert!(audit
            .read(&vault, &AuditFilter::default())
            .expect("read")
            .is_empty());
    }

    #[test]
    fn record_unlock_audit_emits_unlock_failed_on_invalid_password() {
        let (audit, _dir, vault) = fresh_service();
        let path = vault.to_str().expect("utf8 path");

        let r = record_unlock_audit(&audit, path, Err(AppError::InvalidPassword));
        assert!(matches!(r, Err(AppError::InvalidPassword)));

        let events = audit.read(&vault, &AuditFilter::default()).expect("read");
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], AuditEvent::VaultUnlockFailed { .. }));
    }

    #[test]
    fn record_lock_audit_emits_manual_lock_on_real_transition() {
        let (audit, _dir, vault) = fresh_service();
        let path = vault.to_str().expect("utf8 path");

        let r = record_lock_audit(&audit, path, Reason::Manual, Ok((stub_info(), true)));
        assert!(r.is_ok());

        let events = audit.read(&vault, &AuditFilter::default()).expect("read");
        assert_eq!(events.len(), 1);
        match &events[0] {
            AuditEvent::VaultLocked { reason, .. } => assert_eq!(*reason, Reason::Manual),
            other => panic!("expected VaultLocked, got {other:?}"),
        }
    }

    #[test]
    fn record_lock_audit_skips_emit_on_no_op_lock() {
        let (audit, _dir, vault) = fresh_service();
        let path = vault.to_str().expect("utf8 path");

        let r = record_lock_audit(&audit, path, Reason::Manual, Ok((stub_info(), false)));
        assert!(r.is_ok());

        assert!(audit
            .read(&vault, &AuditFilter::default())
            .expect("read")
            .is_empty());
    }

    #[test]
    fn record_lock_audit_skips_emit_on_error() {
        let (audit, _dir, vault) = fresh_service();
        let path = vault.to_str().expect("utf8 path");

        let r = record_lock_audit(&audit, path, Reason::Manual, Err(AppError::DatabaseNotOpen));
        assert!(matches!(r, Err(AppError::DatabaseNotOpen)));

        assert!(audit
            .read(&vault, &AuditFilter::default())
            .expect("read")
            .is_empty());
    }
}
