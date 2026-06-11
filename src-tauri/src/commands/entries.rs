use crate::domain::secure::SecureBytes;
use crate::dto::entry::{
    AddAttachmentsOutcome, CreateEntryData, CustomFieldValue, Entry, UpdateEntryData,
};
use crate::dto::error::AppError;
use crate::services::audit::AuditService;
use crate::services::drag_drop::DropPathsBuffer;
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

/// Adds files on disk to an Entry as native KDBX binaries. The file paths are
/// acquired *here in Rust* from the native multi-select dialog — the frontend
/// passes no path, so a renderer-supplied (or fabricated) path can never reach
/// the read. This is the trust boundary recorded in ADR-0004: only an
/// OS-provided source (this dialog today; the `tauri://drag-drop` event for
/// #286) can name a file to import. A cancelled dialog yields no paths and is a
/// no-op. Each picked file is read, hard-size-capped, and auto-renamed on a
/// filename collision; one bad pick never aborts the rest. Returns the batch
/// outcome (stored names + per-file failures); the frontend persists via
/// `database.save` and refreshes the entry when anything landed.
#[tauri::command]
pub async fn add_entry_attachments<R: Runtime>(
    db_id: String,
    id: String,
    app: AppHandle<R>,
    state: State<'_, Arc<KdbxService>>,
) -> Result<AddAttachmentsOutcome, AppError> {
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
    state.add_entry_attachments(&db_id, &id, &paths)
}

/// Adds the files from the most recent native `tauri://drag-drop` event to an
/// Entry. The paths were captured *in Rust* from the window event (buffered in
/// `DropPathsBuffer`) — the frontend passes only the target db/entry ids, so a
/// renderer-supplied path can never reach the read. This is the same trust
/// boundary as the picker (ADR-0004): the renderer decides *whether* to commit
/// (the drop must land on the selected Entry's panel) but never names a file.
/// Draining the buffer here means a stale drop cannot be replayed against a
/// later entry, and a commit with no preceding drop reads nothing (empty
/// outcome). The dropped files go through the same feeder as the picker — hard
/// size cap, auto-rename on collision, one bad file never aborting the rest —
/// and the frontend persists via `database.save` and refreshes when anything
/// landed.
#[tauri::command]
pub async fn commit_dropped_attachments(
    db_id: String,
    id: String,
    drops: State<'_, Arc<DropPathsBuffer>>,
    state: State<'_, Arc<KdbxService>>,
) -> Result<AddAttachmentsOutcome, AppError> {
    let paths = drops.take();
    state.add_entry_attachments(&db_id, &id, &paths)
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
