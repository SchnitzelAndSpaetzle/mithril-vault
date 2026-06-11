use crate::domain::secure::SecureBytes;
use crate::dto::entry::{
    AddAttachmentsOutcome, AttachmentAddPlan, CreateEntryData, CustomFieldValue, Entry,
    UpdateEntryData,
};
use crate::dto::error::AppError;
use crate::services::audit::AuditService;
use crate::services::drag_drop::PendingAttachmentPaths;
use crate::services::kdbx::entries::plan_attachment_adds;
use crate::services::kdbx::favicons::FaviconFetchOutcome;
use crate::services::kdbx::KdbxService;
use crate::services::settings::SettingsService;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Runtime, State};
use tauri_plugin_dialog::DialogExt;

/// Maps a `get_entry_password` outcome through the audit subsystem: a
/// successful read records exactly one `entry.password_revealed` event
/// against the open Vault's audit log; any error path records nothing.
///
/// Kept as a free function (mirroring `commands::database::record_open_outcome`)
/// so integration tests can drive it without spinning up a Tauri runtime.
pub fn audit_entry_password_revealed_on_success<T>(
    audit: &AuditService,
    vault_path: &str,
    entry_id: &str,
    result: Result<T, AppError>,
) -> Result<T, AppError> {
    if result.is_ok() {
        audit.record_entry_password_revealed(Path::new(vault_path), entry_id);
    }
    result
}

/// Mirror of [`audit_entry_password_revealed_on_success`] for the
/// protected-custom-field reveal path. AC #7 of the PRD: recovery codes
/// and other protected custom fields get the same audit treatment as
/// password reveals.
pub fn audit_entry_protected_field_revealed_on_success<T>(
    audit: &AuditService,
    vault_path: &str,
    entry_id: &str,
    result: Result<T, AppError>,
) -> Result<T, AppError> {
    if result.is_ok() {
        audit.record_entry_protected_field_revealed(Path::new(vault_path), entry_id);
    }
    result
}

/// Maps an attachment-export outcome through the audit subsystem: a
/// successful download (bytes written to disk) records exactly one
/// `entry.attachment_exported` event carrying the entry UUID and the
/// Attachment's filename; any error path records nothing.
///
/// The audit lives here on the download-only path rather than inside
/// `get_entry_attachment`, because the byte fetch is reused by in-app
/// preview — which must not be audited (only leaving the Vault's
/// encryption boundary is). Kept as a free function so integration tests
/// can drive it without a Tauri runtime.
pub fn audit_attachment_exported_on_success<T>(
    audit: &AuditService,
    vault_path: &str,
    entry_id: &str,
    attachment_id: &str,
    result: Result<T, AppError>,
) -> Result<T, AppError> {
    if result.is_ok() {
        audit.record_entry_attachment_exported(Path::new(vault_path), entry_id, attachment_id);
    }
    result
}

/// Lists entries, optionally filtered by group.
/// The `db_id` is the path to the database file.
#[tauri::command]
pub async fn list_entries(
    db_id: String,
    group_id: Option<String>,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Vec<Entry>, AppError> {
    state.list_entries(&db_id, group_id.as_deref())
}

/// Fetches an entry by ID.
/// The `db_id` is the path to the database file.
#[tauri::command]
pub async fn get_entry(
    db_id: String,
    id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Entry, AppError> {
    state.get_entry(&db_id, &id)
}

/// Fetches an entry password. A successful read also appends one
/// `entry.password_revealed` event to the open Vault's audit log; a
/// failure records nothing.
#[tauri::command]
pub async fn get_entry_password(
    db_id: String,
    id: String,
    state: State<'_, Arc<KdbxService>>,
    audit: State<'_, Arc<AuditService>>,
) -> Result<String, AppError> {
    audit_entry_password_revealed_on_success(
        audit.inner(),
        &db_id,
        &id,
        state.get_entry_password(&db_id, &id),
    )
}

/// Fetches a protected custom field value. A successful read also
/// appends one `entry.protected_field_revealed` event to the open
/// Vault's audit log.
#[tauri::command]
pub async fn get_entry_protected_custom_field(
    db_id: String,
    id: String,
    key: String,
    state: State<'_, Arc<KdbxService>>,
    audit: State<'_, Arc<AuditService>>,
) -> Result<CustomFieldValue, AppError> {
    audit_entry_protected_field_revealed_on_success(
        audit.inner(),
        &db_id,
        &id,
        state.get_entry_protected_custom_field(&db_id, &id, &key),
    )
}

/// Fetches a single Attachment's bytes on demand, keyed by filename, as
/// [`SecureBytes`]. This is the reusable lazy byte-fetch (Preview reuses
/// it); it records no audit event. Bytes are never included in
/// `list_entries` / `get_entry` responses.
#[tauri::command]
pub async fn get_entry_attachment(
    db_id: String,
    id: String,
    filename: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<SecureBytes, AppError> {
    state.get_entry_attachment(&db_id, &id, &filename)
}

/// Exports (downloads) a single Attachment by writing its bytes to a
/// user-chosen path. The frontend opens the save dialog and passes the
/// resulting `dest_path`; the bytes are written here in Rust so decrypted
/// data never crosses into JS. A successful write records exactly one
/// `entry.attachment_exported` event (entry UUID + filename); a failed
/// read or write records nothing.
#[tauri::command]
pub async fn export_entry_attachment(
    db_id: String,
    id: String,
    filename: String,
    dest_path: String,
    state: State<'_, Arc<KdbxService>>,
    audit: State<'_, Arc<AuditService>>,
) -> Result<(), AppError> {
    audit_attachment_exported_on_success(
        audit.inner(),
        &db_id,
        &id,
        &filename,
        state.export_entry_attachment(&db_id, &id, &filename, Path::new(&dest_path)),
    )
}

/// Removes a single Attachment from an Entry, keyed by filename. Drops the
/// Entry's reference and (when it was the last reference) the orphaned blob
/// from the Vault-level pool, then marks the Vault modified; the frontend
/// persists and refreshes. There is no undo, so the UI confirms first.
#[tauri::command]
pub async fn delete_entry_attachment(
    db_id: String,
    id: String,
    filename: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<(), AppError> {
    state.delete_entry_attachment(&db_id, &id, &filename)
}

/// Reads the configured attachment size guardrails from App Preferences as a
/// `(soft_warn_bytes, hard_cap_bytes)` pair. Both prepare commands and the
/// commit command resolve the thresholds through here so the user-configured
/// values — not a hard-coded constant — govern every add.
fn attachment_thresholds(settings: &SettingsService) -> Result<(u64, u64), AppError> {
    let prefs = settings.get_app_preferences()?;
    Ok((
        prefs.attachments.soft_warn_bytes,
        prefs.attachments.hard_cap_bytes,
    ))
}

/// Phase 1 (picker): opens the native multi-select dialog *in Rust*, buffers the
/// picked paths, and returns the size-classification plan against the configured
/// thresholds — without reading any bytes or mutating the Vault. The frontend
/// inspects the plan: if it requires confirmation (a file over the soft
/// threshold) it shows the warning prompt before calling
/// `commit_prepared_attachments`; otherwise it commits directly. A cancelled
/// dialog buffers nothing and returns an empty plan (a no-op).
///
/// The paths are acquired here, never supplied by the renderer — the trust
/// boundary in ADR-0004. They stay buffered until the commit drains them (or the
/// next pick/drop overwrites them).
#[tauri::command]
pub async fn prepare_picked_attachments<R: Runtime>(
    app: AppHandle<R>,
    settings: State<'_, Arc<SettingsService>>,
    pending: State<'_, Arc<PendingAttachmentPaths>>,
) -> Result<AttachmentAddPlan, AppError> {
    let (soft, hard) = attachment_thresholds(&settings)?;
    // `blocking_pick_files` runs off the main thread (this command runs on the
    // async runtime), dispatching the dialog to the UI thread internally. It
    // returns `None` on cancel. Each `FilePath` is converted to a real
    // `PathBuf`; any that fail conversion are dropped rather than read.
    let paths: Vec<PathBuf> = app
        .dialog()
        .file()
        .blocking_pick_files()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|file_path| file_path.into_path().ok())
        .collect();
    pending.replace(paths.clone());
    Ok(plan_attachment_adds(&paths, soft, hard))
}

/// Phase 1 (drag-drop): classifies the paths buffered from the most recent
/// native `tauri://drag-drop` event against the configured thresholds, without
/// draining them — a peek, so the buffer survives for the commit that follows a
/// confirmation. The paths were captured *in Rust* from the window event
/// (ADR-0004); the renderer supplies none. A peek with no preceding drop returns
/// an empty plan.
#[tauri::command]
pub async fn prepare_dropped_attachments(
    settings: State<'_, Arc<SettingsService>>,
    pending: State<'_, Arc<PendingAttachmentPaths>>,
) -> Result<AttachmentAddPlan, AppError> {
    let (soft, hard) = attachment_thresholds(&settings)?;
    let paths = pending.peek();
    Ok(plan_attachment_adds(&paths, soft, hard))
}

/// Phase 2 (shared): drains the buffered paths and stores each as a native KDBX
/// binary, enforcing the configured hard cap. Used by both the picker and the
/// drop after the frontend has resolved any soft-warning confirmation. Draining
/// means a stale batch cannot be replayed against a later entry, and a commit
/// with no preceding prepare reads nothing (empty outcome). Each file is read,
/// hard-size-capped, and auto-renamed on a filename collision; one bad file
/// never aborts the rest. Returns the batch outcome (stored names + per-file
/// failures); the frontend persists via `database.save` and refreshes when
/// anything landed.
#[tauri::command]
pub async fn commit_prepared_attachments(
    db_id: String,
    id: String,
    state: State<'_, Arc<KdbxService>>,
    settings: State<'_, Arc<SettingsService>>,
    pending: State<'_, Arc<PendingAttachmentPaths>>,
) -> Result<AddAttachmentsOutcome, AppError> {
    let (_soft, hard) = attachment_thresholds(&settings)?;
    let paths = pending.take();
    state.add_entry_attachments(&db_id, &id, &paths, hard)
}

/// Creates a new entry in a group.
/// The `db_id` is the path to the database file.
#[tauri::command]
pub async fn create_entry(
    db_id: String,
    group_id: String,
    data: CreateEntryData,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Entry, AppError> {
    state.create_entry(&db_id, &group_id, data)
}

/// Updates an existing entry.
/// The `db_id` is the path to the database file.
#[tauri::command]
pub async fn update_entry(
    db_id: String,
    id: String,
    data: UpdateEntryData,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Entry, AppError> {
    state.update_entry(&db_id, &id, data)
}

/// Deletes an entry by ID.
/// The `db_id` is the path to the database file.
#[tauri::command]
pub async fn delete_entry(
    db_id: String,
    id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<(), AppError> {
    state.delete_entry(&db_id, &id)
}

/// Renames a tag across all entries in the database.
/// Returns the number of entries that were modified.
#[tauri::command]
pub async fn rename_tag(
    db_id: String,
    old_name: String,
    new_name: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<u32, AppError> {
    state.rename_tag(&db_id, &old_name, &new_name)
}

/// Deletes a tag from all entries in the database.
/// Returns the number of entries that were modified.
#[tauri::command]
pub async fn delete_tag(
    db_id: String,
    tag_name: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<u32, AppError> {
    state.delete_tag(&db_id, &tag_name)
}

/// Moves an entry to another group.
/// The `db_id` is the path to the database file.
#[tauri::command]
pub async fn move_entry(
    db_id: String,
    id: String,
    target_group_id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<Entry, AppError> {
    state.move_entry(&db_id, &id, &target_group_id)
}

/// Fetches and stores a favicon for an entry URL as a database custom icon.
#[tauri::command]
pub async fn fetch_entry_favicon(
    db_id: String,
    id: String,
    force: Option<bool>,
    kdbx_state: State<'_, Arc<KdbxService>>,
    settings_state: State<'_, Arc<SettingsService>>,
) -> Result<FaviconFetchOutcome, AppError> {
    let settings = settings_state.get_settings()?;
    kdbx_state
        .fetch_entry_favicon(
            &db_id,
            &id,
            settings
                .preferences
                .security
                .allow_third_party_favicon_fallbacks,
            force.unwrap_or(false),
        )
        .await
}

/// Removes a custom icon assignment from an entry.
#[tauri::command]
pub async fn clear_entry_custom_icon(
    db_id: String,
    id: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<bool, AppError> {
    state.clear_entry_custom_icon(&db_id, &id)
}

/// Assigns an existing custom icon (already in the database) to an entry.
#[tauri::command]
pub async fn set_entry_custom_icon(
    db_id: String,
    id: String,
    icon_uuid: String,
    state: State<'_, Arc<KdbxService>>,
) -> Result<bool, AppError> {
    state.set_entry_custom_icon(&db_id, &id, &icon_uuid)
}
