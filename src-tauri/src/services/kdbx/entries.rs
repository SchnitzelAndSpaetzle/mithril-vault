use crate::domain::secure::SecureBytes;
use crate::dto::entry::{
    AddAttachmentsOutcome, AttachmentAddFailure, AttachmentAddPlan, AttachmentPlanItem,
    AttachmentSizeStatus, CreateEntryData, CustomFieldValue, Entry, EntryHistoryItem,
    UpdateEntryData,
};
use crate::dto::error::AppError;
use crate::utils::atomic_write::{atomic_write, AtomicWriteOptions};
use keepass::db::{Entry as KeepassEntry, Times, Value};
use std::io::{Read, Write};

use super::conversions::{
    apply_custom_fields, apply_expiry, convert_entry, is_standard_entry_field, parse_expiry_time,
    replace_custom_fields, validate_expiry_enabled,
};
use super::recycle::{is_group_in_recycle_bin, is_in_recycle_bin};
use super::KdbxService;

/// Classifies a candidate attachment's size against the configured guardrails.
/// At-threshold values are treated as within the lower band: a file exactly the
/// size of the soft threshold is [`Ok`] (warning fires only *above* it), and a
/// file exactly the size of the hard cap is [`OverSoft`] (rejection fires only
/// *above* it — matching the `> hard_cap` check in the add path). Callers must
/// pass a coherent pair (`soft <= hard`), which the settings boundary
/// guarantees.
///
/// [`Ok`]: AttachmentSizeStatus::Ok
/// [`OverSoft`]: AttachmentSizeStatus::OverSoft
fn classify_attachment_size(size: u64, soft: u64, hard: u64) -> AttachmentSizeStatus {
    if size > hard {
        AttachmentSizeStatus::OverHard
    } else if size > soft {
        AttachmentSizeStatus::OverSoft
    } else {
        AttachmentSizeStatus::Ok
    }
}

/// Builds the size-classification plan for a batch of candidate files without
/// reading their bytes or touching the Vault. Each path is stat'd and
/// classified against the configured `soft`/`hard` thresholds; the result drives
/// the frontend's decision to prompt before committing. A path that cannot be
/// stat'd (missing, permission-denied) is recorded with size `0` and status
/// [`Ok`] — it is advisory only, and the authoritative read at commit time will
/// surface the real I/O error as a per-file failure. `requires_confirmation` is
/// `true` iff at least one file is [`OverSoft`]; files over the hard cap do not
/// gate the prompt.
///
/// [`Ok`]: AttachmentSizeStatus::Ok
/// [`OverSoft`]: AttachmentSizeStatus::OverSoft
pub(crate) fn plan_attachment_adds(
    paths: &[std::path::PathBuf],
    soft: u64,
    hard: u64,
) -> AttachmentAddPlan {
    let items: Vec<AttachmentPlanItem> = paths
        .iter()
        .map(|path| {
            let source_name = path
                .file_name()
                .and_then(std::ffi::OsStr::to_str)
                .unwrap_or("")
                .to_string();
            let size = std::fs::metadata(path).map_or(0, |m| m.len());
            AttachmentPlanItem {
                source_name,
                size,
                status: classify_attachment_size(size, soft, hard),
            }
        })
        .collect();
    let requires_confirmation = items
        .iter()
        .any(|item| item.status == AttachmentSizeStatus::OverSoft);
    // `batch_id` is owned by the buffer generation, which this pure builder has
    // no access to; the prepare command overwrites it with the real id before
    // returning the plan over IPC.
    AttachmentAddPlan {
        items,
        requires_confirmation,
        batch_id: 0,
    }
}

/// Pushes a captured pre-image of the Entry into its native KDBX history — the
/// single snapshot chokepoint (ADR-0008). The pre-image is a clone of the
/// Entry's state from *before* the mutation, with its own nested history
/// stripped (KDBX never nests history, and keeping it would grow each version
/// exponentially), inserted newest-first by [`keepass::db::History::add_entry`].
///
/// S1 wired this only into `update_entry`; S2 routes the remaining
/// content/location mutators (bulk tags, move between real Groups, attachment
/// add, custom-icon/Favicon) through the same helper so coverage stays uniform.
/// Retention pruning is intentionally not enforced here yet (S6) — history may
/// grow unbounded for now.
///
/// Takes `&mut KeepassEntry` rather than an `EntryMut` so both the `EntryMut`
/// call sites (via deref coercion) and the raw-`Entry` closures handed to
/// [`Vault::modify_all_entries`] can funnel through the one chokepoint.
///
/// NOTE (#323): attachment **deletion** is deliberately *not* yet routed here.
/// A pre-image clone shares the live Entry's binary-pool `AttachmentId` but is
/// not registered as a pool back-reference, and the crate's delete path GCs a
/// blob keyed only by `entry_id` — so snapshotting a delete would leave the
/// history version pointing at a reclaimed (and later possibly reused) blob.
/// Snapshot-on-delete plus blob retention land together in a follow-up.
pub(crate) fn snapshot_entry_history(entry: &mut KeepassEntry, mut pre_image: KeepassEntry) {
    // `History::add_entry` also strips nested history, but clearing it here
    // makes the intent explicit and keeps the pushed snapshot minimal.
    pre_image.history = None;
    entry.history.get_or_insert_default().add_entry(pre_image);
}

/// Whether an edit changed any *stored content* of the Entry, ignoring the
/// volatile `last_modification` bump and the history list itself. Gates the
/// history snapshot so content-preserving updates don't accrue junk versions:
/// the edit form submits a full payload on every Save even when nothing is
/// dirty, and emits a no-op update before a group move. Compares normalized
/// clones so real differences in fields, tags, icon, expiry, or attachments
/// register while the always-bumped mtime does not.
fn entry_content_changed(before: &KeepassEntry, after: &KeepassEntry) -> bool {
    let mut a = before.clone();
    let mut b = after.clone();
    a.history = None;
    b.history = None;
    b.times.last_modification = a.times.last_modification;
    a != b
}

impl KdbxService {
    /// Lists entries, optionally filtered by group.
    pub fn list_entries(
        &self,
        db_id: &str,
        group_id: Option<&str>,
    ) -> Result<Vec<Entry>, AppError> {
        self.with_vault(db_id, |vault| {
            let mut entries = Vec::new();

            if let Some(gid) = group_id {
                let group = vault.find_group(gid)?;
                let group_uuid = group.id().uuid().to_string();
                for entry in group.entries() {
                    entries.push(convert_entry(&entry, &group_uuid));
                }
            } else {
                // The unfiltered "all entries" view hides anything inside the
                // recycle bin so deleted entries don't appear to come back. The
                // recycle bin group is still navigable directly through the
                // `Some(gid)` branch above.
                let recycle_uuid = vault.db().meta.recyclebin_uuid;
                for entry in vault.db().iter_all_entries() {
                    if let Some(rid) = recycle_uuid {
                        if is_in_recycle_bin(vault.db(), &entry, rid) {
                            continue;
                        }
                    }
                    let group_uuid = entry.parent().id().uuid().to_string();
                    entries.push(convert_entry(&entry, &group_uuid));
                }
            }

            Ok(entries)
        })
    }

    /// Fetches an entry by ID.
    pub fn get_entry(&self, db_id: &str, id: &str) -> Result<Entry, AppError> {
        self.with_vault(db_id, |vault| {
            let entry = vault.find_entry(id)?;
            let group_uuid = entry.parent().id().uuid().to_string();
            Ok(convert_entry(&entry, &group_uuid))
        })
    }

    /// Lists an Entry's history — its past versions, newest-first — read from
    /// native KDBX `Entry.history` (ADR-0008). Each item carries its `index`
    /// in the newest-first list, the snapshot's `modified_at` timestamp, and
    /// non-secret display fields only; passwords and protected values never
    /// cross this boundary. An Entry with `history: None` (imported or
    /// malformed) yields an empty list rather than an error.
    pub fn list_entry_history(
        &self,
        db_id: &str,
        id: &str,
    ) -> Result<Vec<EntryHistoryItem>, AppError> {
        self.with_vault(db_id, |vault| {
            let entry = vault.find_entry(id)?;
            let items = entry.history.as_ref().map_or_else(Vec::new, |history| {
                history
                    .get_entries()
                    .iter()
                    .enumerate()
                    .map(|(index, snapshot)| EntryHistoryItem {
                        index,
                        modified_at: snapshot
                            .times
                            .last_modification
                            .map(|t| t.to_string())
                            .unwrap_or_default(),
                        title: snapshot.get_title().unwrap_or_default().to_string(),
                        username: snapshot.get_username().unwrap_or_default().to_string(),
                        url: snapshot.get_url().map(std::string::ToString::to_string),
                    })
                    .collect()
            });
            Ok(items)
        })
    }

    /// Fetches an entry password.
    pub fn get_entry_password(&self, db_id: &str, id: &str) -> Result<String, AppError> {
        self.with_vault(db_id, |vault| {
            let entry = vault.find_entry(id)?;
            Ok(entry
                .get_password()
                .map(std::string::ToString::to_string)
                .unwrap_or_default())
        })
    }

    /// Fetches a single Attachment's bytes on demand, keyed by its filename
    /// (the per-Entry Attachment identifier). Bytes are returned as
    /// [`SecureBytes`] so they zeroize promptly after the caller is done —
    /// mirroring the `get_entry_password` lazy-reveal pattern. This is the
    /// reusable byte-fetch path; it never records an audit event (only the
    /// export/download path does), so in-app preview can reuse it freely.
    pub fn get_entry_attachment(
        &self,
        db_id: &str,
        entry_id: &str,
        filename: &str,
    ) -> Result<SecureBytes, AppError> {
        self.with_vault(db_id, |vault| {
            let entry = vault.find_entry(entry_id)?;
            let attachment = entry
                .attachment_by_name(filename)
                .ok_or_else(|| AppError::AttachmentNotFound(filename.to_string()))?;
            Ok(SecureBytes::new(attachment.data.get().clone()))
        })
    }

    /// Exports a single Attachment by writing its bytes to `dest` on disk —
    /// the only path by which Attachment bytes leave the Vault's encryption
    /// boundary. Bytes are fetched via [`get_entry_attachment`] and written
    /// from Rust so they never cross IPC into JS. The caller (command layer)
    /// records the `entry.attachment_exported` audit event only when this
    /// succeeds; a failed read or write records nothing.
    ///
    /// [`get_entry_attachment`]: Self::get_entry_attachment
    pub fn export_entry_attachment(
        &self,
        db_id: &str,
        entry_id: &str,
        filename: &str,
        dest: &std::path::Path,
    ) -> Result<(), AppError> {
        let bytes = self.get_entry_attachment(db_id, entry_id, filename)?;
        let dest_str = dest
            .to_str()
            .ok_or_else(|| AppError::InvalidPath(dest.to_string_lossy().into_owned()))?;
        // Atomic temp-file + rename (secure 0600 perms by default): a failed
        // write — disk full, interrupted I/O — leaves any pre-existing file at
        // the chosen destination untouched rather than truncated or partial.
        atomic_write(dest_str, &AtomicWriteOptions::default(), |file| {
            file.write_all(bytes.as_bytes())
                .map_err(|e| AppError::Io(e.to_string()))
        })
    }

    /// Removes a single Attachment from an Entry, keyed by its filename. The
    /// `keepass` crate drops the Entry's reference and — when it was the last
    /// reference — the now-orphaned blob from the Vault-level binary pool, so
    /// no separate pool cleanup is needed. The Entry's modification time is
    /// bumped and the Vault is marked modified (the caller persists). Deleting
    /// an unknown filename is an [`AppError::AttachmentNotFound`] that leaves
    /// the Vault untouched.
    ///
    /// NOTE (#323): this path deliberately does **not** route through the
    /// snapshot chokepoint yet. The pool GC above reclaims (and later reuses)
    /// the blob keyed only by `entry_id`, so a snapshot taken here would point
    /// the history version at a freed/reused blob. Snapshot-on-delete plus the
    /// required binary-pool blob retention land together in a follow-up.
    pub fn delete_entry_attachment(
        &self,
        db_id: &str,
        entry_id: &str,
        filename: &str,
    ) -> Result<(), AppError> {
        self.with_vault_mut(db_id, |vault| {
            {
                let mut entry = vault.entry_mut(entry_id)?;
                if entry.attachment_by_name_mut(filename).is_none() {
                    return Err(AppError::AttachmentNotFound(filename.to_string()));
                }
                entry.remove_attachment_by_name(filename);
                entry.times.last_modification = Some(Times::now());
            }
            vault.mark_modified();
            Ok(())
        })
    }

    /// Adds a file on disk to an Entry as a native KDBX binary, keyed by its
    /// filename. The bytes are read in Rust from `source_path` — the frontend
    /// passes filesystem paths, never file bytes — and stored unprotected (the
    /// Vault's at-rest encryption already covers them). The Entry's
    /// modification time is bumped and the Vault is marked modified
    /// immediately, independent of the Entry edit-form save cycle (mirroring
    /// `set_entry_custom_icon`). Returns the filename the attachment was
    /// actually stored under, which may differ from the source basename when a
    /// collision triggered an auto-rename. The `hard_cap` (in bytes) is the
    /// per-file rejection threshold; it is injected by the caller from App
    /// Preferences rather than hard-coded, so the user-configured cap governs
    /// every add.
    pub fn add_entry_attachment(
        &self,
        db_id: &str,
        entry_id: &str,
        source_path: &std::path::Path,
        hard_cap: u64,
    ) -> Result<String, AppError> {
        let filename = source_path
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            .ok_or_else(|| AppError::InvalidPath(source_path.to_string_lossy().into_owned()))?
            .to_string();

        let metadata = std::fs::metadata(source_path).map_err(|e| AppError::Io(e.to_string()))?;

        // Reject anything that isn't a regular file. A directory, FIFO, or
        // device like `/dev/zero` reports a meaningless metadata length, so the
        // stat-based cap below can't be trusted for it and an unbounded read
        // could hang or exhaust memory.
        if !metadata.is_file() {
            return Err(AppError::InvalidInput(format!(
                "not a regular file: {}",
                source_path.to_string_lossy()
            )));
        }

        // Fast path: reject an obviously oversized file from its stat'd size
        // before opening it at all.
        if metadata.len() > hard_cap {
            return Err(AppError::AttachmentTooLarge {
                filename,
                size: metadata.len(),
                cap: hard_cap,
            });
        }

        // Authoritative guard: bound the read itself to one byte past the cap.
        // A regular file that grows after the stat (TOCTOU) can never push more
        // than the cap into memory — reading `cap + 1` bytes proves it's over.
        let file = std::fs::File::open(source_path).map_err(|e| AppError::Io(e.to_string()))?;

        // Re-check on the opened handle, not the pre-open path: if the path was
        // swapped (e.g. a symlink repointed to a FIFO/device) between the
        // `metadata()` above and this `open`, the descriptor we actually hold
        // could be a non-regular file that would block or hang `read_to_end`.
        // Validating the fd's own metadata closes that race.
        let opened = file.metadata().map_err(|e| AppError::Io(e.to_string()))?;
        if !opened.is_file() {
            return Err(AppError::InvalidInput(format!(
                "not a regular file: {}",
                source_path.to_string_lossy()
            )));
        }

        // Read one byte past the cap to prove an over-cap file. `saturating_add`
        // guards the edge where `hard_cap` is `u64::MAX` (a hand-edited
        // settings.json the validator accepts): `hard_cap + 1` would overflow —
        // panicking in overflow-checked builds, or wrapping to `take(0)` in
        // release and silently storing an empty attachment. Saturating leaves
        // the cap effectively unlimited, which is the intended meaning.
        let mut bytes = Vec::new();
        file.take(hard_cap.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|e| AppError::Io(e.to_string()))?;
        if bytes.len() as u64 > hard_cap {
            return Err(AppError::AttachmentTooLarge {
                filename,
                size: bytes.len() as u64,
                cap: hard_cap,
            });
        }

        self.with_vault_mut(db_id, |vault| {
            let stored_name = {
                let mut entry = vault.entry_mut(entry_id)?;
                // Snapshot the pre-add state before the new binary lands (#323).
                // The pre-image predates the add, so it never references the new
                // blob — the deferred delete-path retention concern doesn't apply.
                let before: KeepassEntry = (*entry.as_ref()).clone();
                let stored_name = unique_attachment_name(&entry.as_ref(), &filename);
                entry.add_attachment(stored_name.clone(), Value::unprotected(bytes));
                entry.times.last_modification = Some(Times::now());
                snapshot_entry_history(&mut entry, before);
                stored_name
            };
            vault.mark_modified();
            Ok(stored_name)
        })
    }

    /// Adds a batch of files to an Entry, one per path, returning the stored
    /// names and per-file failures. This is the single feeder the trusted
    /// add-attachment command uses: it is handed only OS-provided paths (the
    /// native file dialog today, the `tauri://drag-drop` event for #286), never
    /// a renderer-supplied string. A failure on one file (e.g. over the hard
    /// cap) is collected and the batch continues, so one bad pick never aborts
    /// the rest. Each successful add marks the Vault modified via the
    /// underlying single add; the frontend persists once afterward.
    pub fn add_entry_attachments(
        &self,
        db_id: &str,
        entry_id: &str,
        paths: &[std::path::PathBuf],
        hard_cap: u64,
    ) -> Result<AddAttachmentsOutcome, AppError> {
        let mut outcome = AddAttachmentsOutcome::default();
        for path in paths {
            match self.add_entry_attachment(db_id, entry_id, path, hard_cap) {
                Ok(stored_name) => outcome.added.push(stored_name),
                Err(error) => outcome.failed.push(AttachmentAddFailure {
                    source_name: path
                        .file_name()
                        .and_then(std::ffi::OsStr::to_str)
                        .unwrap_or("")
                        .to_string(),
                    reason: error.to_string(),
                }),
            }
        }
        Ok(outcome)
    }

    /// Fetches a protected custom field value.
    pub fn get_entry_protected_custom_field(
        &self,
        db_id: &str,
        entry_id: &str,
        key: &str,
    ) -> Result<CustomFieldValue, AppError> {
        self.with_vault(db_id, |vault| {
            if is_standard_entry_field(key) {
                return Err(AppError::CustomFieldNotFound(key.to_string()));
            }

            let entry = vault.find_entry(entry_id)?;

            let value = entry
                .fields
                .get(key)
                .ok_or_else(|| AppError::CustomFieldNotFound(key.to_string()))?;

            match value {
                Value::Protected(_) => Ok(CustomFieldValue {
                    key: key.to_string(),
                    value: value.get().clone(),
                }),
                Value::Unprotected(_) => Err(AppError::CustomFieldNotProtected(key.to_string())),
            }
        })
    }

    /// Creates a new entry in a group.
    #[allow(clippy::needless_pass_by_value)]
    pub fn create_entry(
        &self,
        db_id: &str,
        group_id: &str,
        data: CreateEntryData,
    ) -> Result<Entry, AppError> {
        self.with_vault_mut(db_id, |vault| {
            // Validate the expiry payload before touching the tree so an
            // invalid request can't leave a phantom entry behind. A fresh entry
            // has no stored timestamp, so enabling expiry requires one here.
            let expiry = parse_expiry_time(data.expiry_time.as_deref())?;
            validate_expiry_enabled(data.expires, expiry, None)?;
            let new_eid = {
                let mut group = vault.group_mut(group_id)?;
                let mut entry = group.add_entry();
                populate_entry(&mut entry, &data);
                apply_expiry(&mut entry, data.expires, expiry);
                entry.id()
            };

            let entry_ref = vault
                .db()
                .entry(new_eid)
                .ok_or_else(|| AppError::EntryNotFound(new_eid.uuid().to_string()))?;
            let entry_model = convert_entry(&entry_ref, group_id);
            vault.mark_modified();

            Ok(entry_model)
        })
    }

    /// Updates an existing entry.
    pub fn update_entry(
        &self,
        db_id: &str,
        id: &str,
        data: UpdateEntryData,
    ) -> Result<Entry, AppError> {
        self.with_vault_mut(db_id, |vault| {
            let eid = vault.find_entry_id(id)?;
            // Validate the expiry timestamp before applying any field updates so
            // a malformed payload leaves the entry unchanged.
            let expiry = parse_expiry_time(data.expiry_time.as_deref())?;

            let group_uuid = {
                let mut entry = vault
                    .db_mut()
                    .entry_mut(eid)
                    .ok_or_else(|| AppError::EntryNotFound(id.to_string()))?;

                // Reject enabling expiry with no timestamp to anchor it before
                // applying any field updates, so the entry stays unchanged on
                // an invalid request. Re-enabling an entry that already has a
                // stored timestamp is allowed.
                validate_expiry_enabled(data.expires, expiry, entry.times.expiry)?;

                // Capture the pre-image before any field is overwritten, so a
                // snapshot can preserve exactly what the edit replaces
                // (ADR-0008). Whether it is actually kept is decided after the
                // mutation, gated on a real content change.
                let before: KeepassEntry = (*entry.as_ref()).clone();

                if let Some(title) = data.title {
                    entry
                        .fields
                        .insert("Title".to_string(), Value::Unprotected(title));
                }
                if let Some(username) = data.username {
                    entry
                        .fields
                        .insert("UserName".to_string(), Value::Unprotected(username));
                }
                if let Some(ref password) = data.password {
                    entry.fields.insert(
                        "Password".to_string(),
                        Value::protected(password.as_str().to_string()),
                    );
                }
                if let Some(url) = data.url {
                    entry
                        .fields
                        .insert("URL".to_string(), Value::Unprotected(url));
                }
                if let Some(notes) = data.notes {
                    entry
                        .fields
                        .insert("Notes".to_string(), Value::Unprotected(notes));
                }
                if let Some(icon_id) = data.icon_id {
                    entry.set_icon_builtin(icon_id as usize);
                }
                if let Some(tags) = data.tags {
                    entry.tags = tags;
                }
                if data.custom_fields.is_some() || data.protected_custom_fields.is_some() {
                    replace_custom_fields(
                        &mut entry,
                        data.custom_fields.as_ref(),
                        data.protected_custom_fields.as_ref(),
                    );
                }
                apply_expiry(&mut entry, data.expires, expiry);

                entry.times.last_modification = Some(Times::now());

                // Single snapshot chokepoint, gated on a real content change so
                // no-op saves and the no-op pre-move update don't accrue junk
                // versions. S1 wires the chokepoint here; S2 widens it to the
                // other mutators.
                if entry_content_changed(&before, &entry.as_ref()) {
                    snapshot_entry_history(&mut entry, before);
                }

                entry.as_ref().parent().id().uuid().to_string()
            };

            let entry_ref = vault
                .db()
                .entry(eid)
                .ok_or_else(|| AppError::EntryNotFound(id.to_string()))?;
            let result = convert_entry(&entry_ref, &group_uuid);
            vault.mark_modified();

            Ok(result)
        })
    }

    /// Deletes an entry by moving it to recycle bin.
    pub fn delete_entry(&self, db_id: &str, id: &str) -> Result<(), AppError> {
        self.with_vault_mut(db_id, |vault| {
            let eid = vault.find_entry_id(id)?;
            let recycle_uuid = vault.ensure_recycle_bin();
            let recycle_gid = vault.find_group_id(&recycle_uuid)?;

            let now = Times::now();
            {
                let mut entry = vault
                    .db_mut()
                    .entry_mut(eid)
                    .ok_or_else(|| AppError::EntryNotFound(id.to_string()))?;
                entry.times.last_modification = Some(now);
                entry.times.location_changed = Some(now);
                entry
                    .move_to(recycle_gid)
                    .map_err(|e| AppError::Kdbx(e.to_string()))?;
            }

            vault.mark_modified();
            Ok(())
        })
    }

    /// Renames a tag across all entries in the database.
    /// Returns the number of entries that were modified.
    pub fn rename_tag(&self, db_id: &str, old_name: &str, new_name: &str) -> Result<u32, AppError> {
        self.with_vault_mut(db_id, |vault| {
            if old_name == new_name {
                return Ok(0);
            }

            // One snapshot per touched Entry, captured before its tags are
            // rewritten (#323). The pre-image is cloned eagerly and only kept
            // when the rename actually changed this Entry.
            let count = vault.modify_all_entries(&|entry| {
                let before = entry.clone();
                if rename_tag_in_entry(entry, old_name, new_name) {
                    snapshot_entry_history(entry, before);
                    true
                } else {
                    false
                }
            });

            if count > 0 {
                vault.mark_modified();
            }

            Ok(count)
        })
    }

    /// Deletes a tag from all entries in the database.
    /// Returns the number of entries that were modified.
    pub fn delete_tag(&self, db_id: &str, tag_name: &str) -> Result<u32, AppError> {
        self.with_vault_mut(db_id, |vault| {
            let count = vault.modify_all_entries(&|entry| {
                let before = entry.clone();
                if delete_tag_in_entry(entry, tag_name) {
                    snapshot_entry_history(entry, before);
                    true
                } else {
                    false
                }
            });

            if count > 0 {
                vault.mark_modified();
            }

            Ok(count)
        })
    }

    /// Moves an entry to another group.
    pub fn move_entry(
        &self,
        db_id: &str,
        id: &str,
        target_group_id: &str,
    ) -> Result<Entry, AppError> {
        self.with_vault_mut(db_id, |vault| {
            let eid = vault.find_entry_id(id)?;
            let target_gid = vault.find_group_id(target_group_id)?;

            // Snapshot a real relocation, but exclude Recycle-Bin transitions
            // (trashing or restoring) and a no-op same-Group move — those carry
            // no content change worth a history version (#323). Resolved while
            // the borrow is immutable, before the move mutates the tree.
            let should_snapshot = {
                let entry = vault
                    .db()
                    .entry(eid)
                    .ok_or_else(|| AppError::EntryNotFound(id.to_string()))?;
                let current_parent = entry.parent().id();
                let recycle_uuid = vault.db().meta.recyclebin_uuid;
                let touches_recycle = recycle_uuid.is_some_and(|rid| {
                    is_in_recycle_bin(vault.db(), &entry, rid)
                        || is_group_in_recycle_bin(vault.db(), Some(target_gid), rid)
                });
                current_parent != target_gid && !touches_recycle
            };

            let now = Times::now();
            {
                let mut entry = vault
                    .db_mut()
                    .entry_mut(eid)
                    .ok_or_else(|| AppError::EntryNotFound(id.to_string()))?;
                let before: KeepassEntry = (*entry.as_ref()).clone();
                entry.times.last_modification = Some(now);
                entry.times.location_changed = Some(now);
                entry
                    .move_to(target_gid)
                    .map_err(|e| AppError::Kdbx(e.to_string()))?;
                if should_snapshot {
                    snapshot_entry_history(&mut entry, before);
                }
            }

            let entry_ref = vault
                .db()
                .entry(eid)
                .ok_or_else(|| AppError::EntryNotFound(id.to_string()))?;
            let entry_model = convert_entry(&entry_ref, target_group_id);
            vault.mark_modified();

            Ok(entry_model)
        })
    }
}

fn populate_entry(entry: &mut keepass::db::EntryMut<'_>, data: &CreateEntryData) {
    entry
        .fields
        .insert("Title".to_string(), Value::Unprotected(data.title.clone()));
    entry.fields.insert(
        "UserName".to_string(),
        Value::Unprotected(data.username.clone()),
    );
    entry.fields.insert(
        "Password".to_string(),
        Value::protected(data.password.as_str().to_string()),
    );

    if let Some(url) = &data.url {
        entry
            .fields
            .insert("URL".to_string(), Value::Unprotected(url.clone()));
    }
    if let Some(notes) = &data.notes {
        entry
            .fields
            .insert("Notes".to_string(), Value::Unprotected(notes.clone()));
    }
    if let Some(icon_id) = data.icon_id {
        entry.set_icon_builtin(icon_id as usize);
    }
    if let Some(tags) = &data.tags {
        entry.tags.clone_from(tags);
    }
    apply_custom_fields(
        entry,
        data.custom_fields.as_ref(),
        data.protected_custom_fields.as_ref(),
    );
}

fn rename_tag_in_entry(entry: &mut KeepassEntry, old_name: &str, new_name: &str) -> bool {
    let mut modified = rename_tag_in_list(&mut entry.tags, old_name, new_name);
    modified |= rename_tag_in_custom_field(entry, "Tags", old_name, new_name);
    modified |= rename_tag_in_custom_field(entry, "tags", old_name, new_name);
    modified
}

fn delete_tag_in_entry(entry: &mut KeepassEntry, tag_name: &str) -> bool {
    let mut modified = delete_tag_in_list(&mut entry.tags, tag_name);
    modified |= delete_tag_in_custom_field(entry, "Tags", tag_name);
    modified |= delete_tag_in_custom_field(entry, "tags", tag_name);
    modified
}

fn rename_tag_in_custom_field(
    entry: &mut KeepassEntry,
    key: &str,
    old_name: &str,
    new_name: &str,
) -> bool {
    let Some(value) = entry.fields.get_mut(key) else {
        return false;
    };

    match value {
        Value::Unprotected(text) => rename_tag_in_tag_text(text, old_name, new_name),
        Value::Protected(_) => {
            let mut text = value.get().clone();
            if !rename_tag_in_tag_text(&mut text, old_name, new_name) {
                return false;
            }
            *value = Value::protected(text);
            true
        }
    }
}

fn delete_tag_in_custom_field(entry: &mut KeepassEntry, key: &str, tag_name: &str) -> bool {
    let Some(value) = entry.fields.get_mut(key) else {
        return false;
    };

    match value {
        Value::Unprotected(text) => delete_tag_in_tag_text(text, tag_name),
        Value::Protected(_) => {
            let mut text = value.get().clone();
            if !delete_tag_in_tag_text(&mut text, tag_name) {
                return false;
            }
            *value = Value::protected(text);
            true
        }
    }
}

fn rename_tag_in_tag_text(text: &mut String, old_name: &str, new_name: &str) -> bool {
    let mut tags = split_tag_values(text);
    if !rename_tag_in_list(&mut tags, old_name, new_name) {
        return false;
    }
    *text = join_tag_values(&tags);
    true
}

fn delete_tag_in_tag_text(text: &mut String, tag_name: &str) -> bool {
    let mut tags = split_tag_values(text);
    if !delete_tag_in_list(&mut tags, tag_name) {
        return false;
    }
    *text = join_tag_values(&tags);
    true
}

fn rename_tag_in_list(tags: &mut Vec<String>, old_name: &str, new_name: &str) -> bool {
    let mut normalized = normalize_tag_list(tags);
    let mut replaced = false;

    for tag in &mut normalized {
        if tag == old_name {
            *tag = new_name.to_string();
            replaced = true;
        }
    }

    if !replaced {
        return false;
    }

    dedupe_preserving_order(&mut normalized);
    *tags = normalized;
    true
}

fn delete_tag_in_list(tags: &mut Vec<String>, tag_name: &str) -> bool {
    let mut normalized = normalize_tag_list(tags);
    let len_before = normalized.len();
    normalized.retain(|tag| tag != tag_name);

    if normalized.len() == len_before {
        return false;
    }

    *tags = normalized;
    true
}

fn normalize_tag_list(tags: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();
    for tag in tags {
        normalized.extend(split_tag_values(tag));
    }
    normalized
}

fn split_tag_values(raw: &str) -> Vec<String> {
    raw.split([',', ';'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(std::string::ToString::to_string)
        .collect()
}

fn join_tag_values(tags: &[String]) -> String {
    tags.join("; ")
}

fn dedupe_preserving_order(tags: &mut Vec<String>) {
    let mut deduped = Vec::with_capacity(tags.len());
    for tag in tags.drain(..) {
        if !deduped.contains(&tag) {
            deduped.push(tag);
        }
    }
    *tags = deduped;
}

/// Resolves a collision-free attachment filename within an Entry. If `filename`
/// is unused on the Entry it is returned as-is; otherwise the stem gains a
/// ` (n)` suffix (`scan (1).pdf`, `scan (2).pdf`, …), counting up until a free
/// name is found. The extension is preserved; names without one (or dotfiles
/// like `.gitignore`, whose whole name is the stem) just gain the suffix.
fn unique_attachment_name(entry: &keepass::db::EntryRef<'_>, filename: &str) -> String {
    if entry.attachment_by_name(filename).is_none() {
        return filename.to_string();
    }

    let path = std::path::Path::new(filename);
    let stem = path
        .file_stem()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or(filename);
    let ext = path.extension().and_then(std::ffi::OsStr::to_str);

    let mut n: u32 = 1;
    loop {
        let candidate = match ext {
            Some(ext) => format!("{stem} ({n}).{ext}"),
            None => format!("{stem} ({n})"),
        };
        if entry.attachment_by_name(&candidate).is_none() {
            return candidate;
        }
        n += 1;
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::domain::secure::SecureString;
    use crate::dto::entry::{AttachmentMeta, AttachmentSizeStatus};
    use crate::services::kdbx::test_support::create_test_database;
    use keepass::db::Value;
    use std::collections::HashMap;

    /// The hard cap the attachment add/round-trip tests inject. A generous
    /// fixed value so the round-trip cases never trip the cap; the over-cap
    /// cases seed a file one byte past it.
    const TEST_HARD_CAP: u64 = 25 * 1024 * 1024;

    #[test]
    fn classify_attachment_size_walks_the_thresholds() {
        // soft = 5, hard = 25. Every boundary is pinned because the soft/hard
        // edges decide whether the user is warned, silently allowed, or
        // rejected — an off-by-one here is a user-visible behavior change.
        let soft = 5;
        let hard = 25;
        // At or below the soft threshold: silent add.
        assert_eq!(
            classify_attachment_size(0, soft, hard),
            AttachmentSizeStatus::Ok
        );
        assert_eq!(
            classify_attachment_size(5, soft, hard),
            AttachmentSizeStatus::Ok
        );
        // Above soft, up to and including the hard cap: warn.
        assert_eq!(
            classify_attachment_size(6, soft, hard),
            AttachmentSizeStatus::OverSoft
        );
        assert_eq!(
            classify_attachment_size(25, soft, hard),
            AttachmentSizeStatus::OverSoft
        );
        // Above the hard cap: reject.
        assert_eq!(
            classify_attachment_size(26, soft, hard),
            AttachmentSizeStatus::OverHard
        );
    }

    #[test]
    fn classify_attachment_size_handles_soft_equal_to_hard() {
        // A coherent edge config: soft == hard means the warning band is empty,
        // so a file is either Ok (<= the shared threshold) or OverHard.
        assert_eq!(
            classify_attachment_size(10, 10, 10),
            AttachmentSizeStatus::Ok
        );
        assert_eq!(
            classify_attachment_size(11, 10, 10),
            AttachmentSizeStatus::OverHard
        );
    }

    #[test]
    fn plan_attachment_adds_classifies_each_file_and_flags_confirmation() {
        // A mixed batch: one file under the soft threshold, one above it (but
        // under the hard cap). The plan must classify each by its on-disk size,
        // preserve pick order, and flag that confirmation is required because at
        // least one file is over the soft threshold — the single signal the
        // frontend reads before showing the warning prompt.
        let dir = tempfile::tempdir().expect("tempdir");
        let small = dir.path().join("notes.txt");
        std::fs::write(&small, vec![0u8; 3]).expect("seed small");
        let large = dir.path().join("scan.pdf");
        std::fs::write(&large, vec![0u8; 50]).expect("seed large");

        let plan = plan_attachment_adds(
            &[small.clone(), large.clone()],
            10,  // soft
            100, // hard
        );

        assert!(
            plan.requires_confirmation,
            "a file over the soft threshold must require confirmation"
        );
        assert_eq!(plan.items.len(), 2, "one plan item per path, in pick order");
        assert_eq!(plan.items[0].source_name, "notes.txt");
        assert_eq!(plan.items[0].size, 3);
        assert_eq!(plan.items[0].status, AttachmentSizeStatus::Ok);
        assert_eq!(plan.items[1].source_name, "scan.pdf");
        assert_eq!(plan.items[1].size, 50);
        assert_eq!(plan.items[1].status, AttachmentSizeStatus::OverSoft);
    }

    #[test]
    fn plan_attachment_adds_requires_no_confirmation_when_all_under_soft() {
        // Every file under the soft threshold: the add proceeds silently, so
        // the plan must not require confirmation.
        let dir = tempfile::tempdir().expect("tempdir");
        let a = dir.path().join("a.txt");
        std::fs::write(&a, vec![0u8; 2]).expect("seed a");
        let b = dir.path().join("b.txt");
        std::fs::write(&b, vec![0u8; 4]).expect("seed b");

        let plan = plan_attachment_adds(&[a, b], 10, 100);

        assert!(
            !plan.requires_confirmation,
            "all-small batch must not require confirmation"
        );
        assert!(plan
            .items
            .iter()
            .all(|item| item.status == AttachmentSizeStatus::Ok));
    }

    #[test]
    fn plan_attachment_adds_does_not_require_confirmation_for_over_hard_only() {
        // A file over the hard cap is OverHard, not OverSoft. It will be
        // rejected at commit as a per-file failure — it must NOT trigger the
        // soft-warning prompt, so a batch whose only large file is over-hard
        // requires no confirmation.
        let dir = tempfile::tempdir().expect("tempdir");
        let huge = dir.path().join("huge.bin");
        std::fs::write(&huge, vec![0u8; 200]).expect("seed huge");

        let plan = plan_attachment_adds(&[huge], 10, 100);

        assert!(
            !plan.requires_confirmation,
            "an over-hard-only batch must not require the soft prompt"
        );
        assert_eq!(plan.items[0].status, AttachmentSizeStatus::OverHard);
    }

    fn create_entry_with_expiry(
        service: &KdbxService,
        db_path: &str,
        group_id: &str,
        expires: Option<bool>,
        expiry_time: Option<String>,
    ) -> Result<Entry, AppError> {
        service.create_entry(
            db_path,
            group_id,
            CreateEntryData {
                title: "Expiring".to_string(),
                username: "carol".to_string(),
                password: SecureString::from("secret"),
                url: None,
                notes: None,
                icon_id: Some(0),
                tags: None,
                custom_fields: None,
                protected_custom_fields: None,
                expires,
                expiry_time,
            },
        )
    }

    /// Seeds a single unprotected attachment on `entry_id` and returns the
    /// Vault's generation immediately after, so a delete test can assert the
    /// generation moved (a successful delete marks the Vault modified) or
    /// stayed put (a rejected delete leaves it untouched).
    fn seed_attachment(
        service: &KdbxService,
        db_path: &str,
        entry_id: &str,
        filename: &str,
        bytes: &[u8],
    ) -> u64 {
        service
            .with_vault_mut(db_path, |vault| {
                let mut entry = vault.entry_mut(entry_id)?;
                entry.add_attachment(filename, Value::unprotected(bytes.to_vec()));
                Ok(())
            })
            .expect("seed attachment");
        service
            .with_vault(db_path, |vault| Ok(vault.generation()))
            .expect("generation after seed")
    }

    /// An `UpdateEntryData` that touches nothing; tests override single fields
    /// via `..empty_update()` struct-update syntax.
    fn empty_update() -> UpdateEntryData {
        UpdateEntryData {
            title: None,
            username: None,
            password: None,
            url: None,
            notes: None,
            icon_id: None,
            tags: None,
            custom_fields: None,
            protected_custom_fields: None,
            expires: None,
            expiry_time: None,
        }
    }

    #[test]
    fn create_entry_round_trips_expiry() {
        let (service, _dir, db_path, _a, _b) = create_test_database();
        let info = service.get_info(&db_path).expect("database info");

        let created = create_entry_with_expiry(
            &service,
            &db_path,
            &info.root_group_id,
            Some(true),
            Some("2030-01-01T12:00:00+00:00".to_string()),
        )
        .expect("create expiring entry");

        assert!(created.expires, "created entry should report expires=true");
        assert_eq!(
            created.expiry_time.as_deref(),
            Some("2030-01-01T12:00:00+00:00"),
            "created entry should echo the UTC expiry timestamp"
        );

        // Reload from the KDBX tree and assert the flag + timestamp survive.
        let reloaded = service.get_entry(&db_path, &created.id).expect("get entry");
        assert!(reloaded.expires);
        assert_eq!(reloaded.expiry_time, created.expiry_time);
    }

    #[test]
    fn unchecking_expiry_retains_the_stored_timestamp() {
        let (service, _dir, db_path, _a, _b) = create_test_database();
        let info = service.get_info(&db_path).expect("database info");

        let created = create_entry_with_expiry(
            &service,
            &db_path,
            &info.root_group_id,
            Some(true),
            Some("2030-01-01T12:00:00+00:00".to_string()),
        )
        .expect("create expiring entry");

        // Uncheck "Expires": flip the flag off, leave the timestamp unspecified.
        let updated = service
            .update_entry(
                &db_path,
                &created.id,
                UpdateEntryData {
                    expires: Some(false),
                    ..empty_update()
                },
            )
            .expect("update entry");

        assert!(!updated.expires, "expires flag should be cleared");
        assert_eq!(
            updated.expiry_time, created.expiry_time,
            "the previously stored timestamp must be retained on uncheck"
        );
    }

    #[test]
    fn past_expiry_time_round_trips() {
        let (service, _dir, db_path, _a, _b) = create_test_database();
        let info = service.get_info(&db_path).expect("database info");

        // A past date is a valid expiry — it records an already-stale credential.
        let created = create_entry_with_expiry(
            &service,
            &db_path,
            &info.root_group_id,
            Some(true),
            Some("2000-06-15T08:30:00+00:00".to_string()),
        )
        .expect("create expiring entry");

        assert!(created.expires);
        let reloaded = service.get_entry(&db_path, &created.id).expect("get entry");
        assert!(reloaded.expires);
        assert_eq!(
            reloaded.expiry_time.as_deref(),
            Some("2000-06-15T08:30:00+00:00")
        );
    }

    #[test]
    fn updating_only_the_password_leaves_expiry_untouched() {
        let (service, _dir, db_path, _a, _b) = create_test_database();
        let info = service.get_info(&db_path).expect("database info");

        let created = create_entry_with_expiry(
            &service,
            &db_path,
            &info.root_group_id,
            Some(true),
            Some("2030-01-01T12:00:00+00:00".to_string()),
        )
        .expect("create expiring entry");

        // Rotate the password; send nothing for expiry.
        let updated = service
            .update_entry(
                &db_path,
                &created.id,
                UpdateEntryData {
                    password: Some(SecureString::from("rotated")),
                    ..empty_update()
                },
            )
            .expect("update entry");

        assert!(updated.expires, "expiry flag must be unchanged");
        assert_eq!(
            updated.expiry_time, created.expiry_time,
            "expiry timestamp must be unchanged by a password-only edit"
        );
    }

    #[test]
    fn entry_without_expiry_defaults_to_not_expiring() {
        let (service, _dir, db_path, _a, _b) = create_test_database();
        let info = service.get_info(&db_path).expect("database info");

        // Expiry is opt-in: omitting it yields a non-expiring entry.
        let created = create_entry_with_expiry(&service, &db_path, &info.root_group_id, None, None)
            .expect("create entry");

        assert!(!created.expires);
        assert_eq!(created.expiry_time, None);

        let reloaded = service.get_entry(&db_path, &created.id).expect("get entry");
        assert!(!reloaded.expires);
        assert_eq!(reloaded.expiry_time, None);
    }

    #[test]
    fn enabling_expiry_without_a_timestamp_is_rejected_on_create() {
        // expires=true with no timestamp is ambiguous: Password Health only
        // flags entries that carry a timestamp, so the backend rejects it.
        let (service, _dir, db_path, _a, _b) = create_test_database();
        let info = service.get_info(&db_path).expect("database info");

        let before = service
            .list_entries(&db_path, Some(&info.root_group_id))
            .expect("list before")
            .len();

        let result =
            create_entry_with_expiry(&service, &db_path, &info.root_group_id, Some(true), None);
        assert!(matches!(result, Err(AppError::InvalidInput(_))));

        let after = service
            .list_entries(&db_path, Some(&info.root_group_id))
            .expect("list after")
            .len();
        assert_eq!(before, after, "a rejected create must not add an entry");
    }

    #[test]
    fn enabling_expiry_without_a_timestamp_is_rejected_on_update() {
        let (service, _dir, db_path, entry_a, _b) = create_test_database();

        // Entry A has no stored expiry, so flipping the flag on with no
        // timestamp is rejected and the title change is not applied.
        let result = service.update_entry(
            &db_path,
            &entry_a,
            UpdateEntryData {
                title: Some("Renamed".to_string()),
                expires: Some(true),
                ..empty_update()
            },
        );
        assert!(matches!(result, Err(AppError::InvalidInput(_))));

        let reloaded = service.get_entry(&db_path, &entry_a).expect("get entry");
        assert!(!reloaded.expires);
        assert_eq!(reloaded.title, "Entry A");
    }

    #[test]
    fn re_enabling_expiry_uses_the_retained_timestamp() {
        // Set expiry, uncheck (timestamp retained), then re-enable with no new
        // timestamp: the retained one anchors it, so this is allowed.
        let (service, _dir, db_path, _a, _b) = create_test_database();
        let info = service.get_info(&db_path).expect("database info");

        let created = create_entry_with_expiry(
            &service,
            &db_path,
            &info.root_group_id,
            Some(true),
            Some("2030-01-01T12:00:00+00:00".to_string()),
        )
        .expect("create expiring entry");

        service
            .update_entry(
                &db_path,
                &created.id,
                UpdateEntryData {
                    expires: Some(false),
                    ..empty_update()
                },
            )
            .expect("disable expiry");

        let re_enabled = service
            .update_entry(
                &db_path,
                &created.id,
                UpdateEntryData {
                    expires: Some(true),
                    ..empty_update()
                },
            )
            .expect("re-enable expiry off the retained timestamp");

        assert!(re_enabled.expires);
        assert_eq!(re_enabled.expiry_time, created.expiry_time);
    }

    #[test]
    fn malformed_expiry_on_create_leaves_no_phantom_entry() {
        // A rejected create must be atomic: validating the timestamp before
        // inserting the entry means nothing is added to the tree on failure.
        let (service, _dir, db_path, _a, _b) = create_test_database();
        let info = service.get_info(&db_path).expect("database info");

        let before = service
            .list_entries(&db_path, Some(&info.root_group_id))
            .expect("list before")
            .len();

        let result = service.create_entry(
            &db_path,
            &info.root_group_id,
            CreateEntryData {
                title: "Phantom".to_string(),
                username: "eve".to_string(),
                password: SecureString::from("secret"),
                url: None,
                notes: None,
                icon_id: Some(0),
                tags: None,
                custom_fields: None,
                protected_custom_fields: None,
                expires: Some(true),
                expiry_time: Some("not-a-timestamp".to_string()),
            },
        );
        assert!(matches!(result, Err(AppError::InvalidInput(_))));

        let after = service
            .list_entries(&db_path, Some(&info.root_group_id))
            .expect("list after")
            .len();
        assert_eq!(before, after, "a rejected create must not add an entry");
    }

    #[test]
    fn malformed_expiry_on_update_leaves_entry_unchanged() {
        // A rejected update must be atomic: validating the timestamp before
        // applying field mutations means a bad payload changes nothing.
        let (service, _dir, db_path, entry_a, _b) = create_test_database();

        let result = service.update_entry(
            &db_path,
            &entry_a,
            UpdateEntryData {
                title: Some("Renamed".to_string()),
                expiry_time: Some("not-a-timestamp".to_string()),
                ..empty_update()
            },
        );
        assert!(matches!(result, Err(AppError::InvalidInput(_))));

        let reloaded = service.get_entry(&db_path, &entry_a).expect("get entry");
        assert_eq!(
            reloaded.title, "Entry A",
            "a rejected update must not apply the title change"
        );
    }

    #[test]
    fn get_entry_extracts_attachment_metadata_without_bytes() {
        // Seed attachments the way another KeePass client would: native KDBX
        // binaries keyed by filename, one unprotected and one protected. The
        // DTO must surface filename + byte size + a MIME hint derived from the
        // extension — and never the byte payload itself.
        let (service, _dir, db_path, entry_a, _b) = create_test_database();

        service
            .with_vault_mut(&db_path, |vault| {
                let mut entry = vault.entry_mut(&entry_a)?;
                entry.add_attachment("Notes.txt", Value::unprotected(b"hello world".to_vec()));
                entry.add_attachment("logo.png", Value::protected(vec![0u8; 2048]));
                Ok(())
            })
            .expect("seed attachments");

        let entry = service.get_entry(&db_path, &entry_a).expect("get entry");

        let by_name: HashMap<&str, &AttachmentMeta> = entry
            .attachments
            .iter()
            .map(|a| (a.filename.as_str(), a))
            .collect();

        let notes = by_name.get("Notes.txt").expect("Notes.txt metadata");
        assert_eq!(notes.size, 11, "size is the byte length of the binary");
        assert_eq!(notes.mime_type, "text/plain");

        let logo = by_name.get("logo.png").expect("logo.png metadata");
        assert_eq!(logo.size, 2048);
        assert_eq!(logo.mime_type, "image/png");
    }

    #[test]
    fn get_entry_attachment_round_trips_exact_stored_bytes() {
        // The on-demand byte fetch must return the attachment's bytes verbatim,
        // for both unprotected and protected (memory-protected) binaries — the
        // download path writes exactly these bytes to disk. Prior art for the
        // binary-in-vault round-trip shape: services/kdbx/custom_icons.rs.
        let (service, _dir, db_path, entry_a, _b) = create_test_database();

        let plain = b"recovery-code-12345".to_vec();
        let secret = vec![0xAB_u8; 4096];
        service
            .with_vault_mut(&db_path, |vault| {
                let mut entry = vault.entry_mut(&entry_a)?;
                entry.add_attachment("codes.txt", Value::unprotected(plain.clone()));
                entry.add_attachment("blob.bin", Value::protected(secret.clone()));
                Ok(())
            })
            .expect("seed attachments");

        let fetched_plain = service
            .get_entry_attachment(&db_path, &entry_a, "codes.txt")
            .expect("fetch unprotected attachment");
        assert_eq!(fetched_plain.as_bytes(), plain.as_slice());

        let fetched_secret = service
            .get_entry_attachment(&db_path, &entry_a, "blob.bin")
            .expect("fetch protected attachment");
        assert_eq!(fetched_secret.as_bytes(), secret.as_slice());
    }

    #[test]
    fn export_entry_attachment_writes_exact_bytes_to_destination() {
        // The download path resolves to a Rust-side write so decrypted bytes
        // never cross into JS. The file on disk must be byte-identical to the
        // stored Attachment.
        let (service, dir, db_path, entry_a, _b) = create_test_database();

        let payload = vec![0x01, 0x02, 0x03, 0xFF, 0x00, 0x42];
        service
            .with_vault_mut(&db_path, |vault| {
                let mut entry = vault.entry_mut(&entry_a)?;
                entry.add_attachment("export-me.bin", Value::unprotected(payload.clone()));
                Ok(())
            })
            .expect("seed attachment");

        let dest = dir.path().join("downloaded.bin");
        service
            .export_entry_attachment(&db_path, &entry_a, "export-me.bin", &dest)
            .expect("export attachment");

        let written = std::fs::read(&dest).expect("read written file");
        assert_eq!(written, payload);
    }

    #[test]
    fn export_entry_attachment_replaces_an_existing_destination_file() {
        // Downloading over an existing file must end with exactly the
        // Attachment's bytes — the atomic temp-file + rename replaces the
        // target wholesale rather than appending or leaving stale bytes.
        let (service, dir, db_path, entry_a, _b) = create_test_database();

        service
            .with_vault_mut(&db_path, |vault| {
                let mut entry = vault.entry_mut(&entry_a)?;
                entry.add_attachment("note.txt", Value::unprotected(b"new".to_vec()));
                Ok(())
            })
            .expect("seed attachment");

        let dest = dir.path().join("note.txt");
        std::fs::write(&dest, b"older longer contents").expect("seed existing file");

        service
            .export_entry_attachment(&db_path, &entry_a, "note.txt", &dest)
            .expect("export over existing file");

        assert_eq!(std::fs::read(&dest).expect("read"), b"new");
    }

    #[test]
    fn get_entry_attachment_errors_on_unknown_filename() {
        let (service, _dir, db_path, entry_a, _b) = create_test_database();
        service
            .with_vault_mut(&db_path, |vault| {
                let mut entry = vault.entry_mut(&entry_a)?;
                entry.add_attachment("present.txt", Value::unprotected(b"x".to_vec()));
                Ok(())
            })
            .expect("seed attachment");

        let result = service.get_entry_attachment(&db_path, &entry_a, "missing.txt");
        assert!(matches!(result, Err(AppError::AttachmentNotFound(name)) if name == "missing.txt"));
    }

    #[test]
    fn entry_without_attachments_reports_an_empty_list() {
        let (service, _dir, db_path, entry_a, _b) = create_test_database();
        let entry = service.get_entry(&db_path, &entry_a).expect("get entry");
        assert!(
            entry.attachments.is_empty(),
            "an entry with no binaries has no attachment metadata"
        );
    }

    #[test]
    fn delete_entry_attachment_removes_reference_and_orphaned_blob() {
        // Deleting the only Attachment referencing a pooled blob must drop both
        // the Entry's reference and the now-orphaned blob from the Vault-level
        // pool, and mark the Vault modified. Prior art for the binary-in-vault
        // round-trip shape: services/kdbx/custom_icons.rs.
        let (service, _dir, db_path, entry_a, _b) = create_test_database();
        let generation_before =
            seed_attachment(&service, &db_path, &entry_a, "codes.txt", b"secret");

        service
            .delete_entry_attachment(&db_path, &entry_a, "codes.txt")
            .expect("delete attachment");

        let entry = service.get_entry(&db_path, &entry_a).expect("get entry");
        assert!(
            entry.attachments.is_empty(),
            "the Entry's attachment reference must be gone after delete"
        );

        service
            .with_vault(&db_path, |vault| {
                assert_eq!(
                    vault.db().num_attachments(),
                    0,
                    "the orphaned blob must be cleaned from the Vault pool"
                );
                assert!(
                    vault.generation() > generation_before,
                    "a successful delete must mark the Vault modified"
                );
                Ok(())
            })
            .expect("vault scope");
    }

    #[test]
    fn delete_entry_attachment_errors_on_unknown_filename_and_leaves_vault_untouched() {
        // Deleting a filename the Entry doesn't carry must be a clean
        // AttachmentNotFound error and must not mark the Vault modified, so a
        // stale or mistargeted click never produces a phantom unsaved change.
        let (service, _dir, db_path, entry_a, _b) = create_test_database();
        let generation_before = seed_attachment(&service, &db_path, &entry_a, "present.txt", b"x");

        let result = service.delete_entry_attachment(&db_path, &entry_a, "missing.txt");
        assert!(matches!(result, Err(AppError::AttachmentNotFound(name)) if name == "missing.txt"));

        let entry = service.get_entry(&db_path, &entry_a).expect("get entry");
        assert_eq!(
            entry.attachments.len(),
            1,
            "the existing attachment must survive a failed delete"
        );
        service
            .with_vault(&db_path, |vault| {
                assert_eq!(
                    vault.generation(),
                    generation_before,
                    "a failed delete must not mark the Vault modified"
                );
                Ok(())
            })
            .expect("vault scope");
    }

    #[test]
    fn add_entry_attachment_stores_bytes_from_a_path_and_marks_modified() {
        // The primary write path: the backend reads a file off disk by path
        // (the frontend never sends bytes), stores it as a native KDBX binary,
        // and marks the Vault modified immediately — independent of the Entry
        // edit-form save cycle. The stored bytes must round-trip verbatim
        // through the on-demand fetch, and the metadata must surface on the
        // Entry. Prior art for the binary-in-vault shape: custom_icons.rs.
        let (service, dir, db_path, entry_a, _b) = create_test_database();
        let generation_before = service
            .with_vault(&db_path, |vault| Ok(vault.generation()))
            .expect("generation before add");

        let payload = b"recovery-codes-12345\nsecond-line".to_vec();
        let source = dir.path().join("codes.txt");
        std::fs::write(&source, &payload).expect("seed source file");

        let stored = service
            .add_entry_attachment(&db_path, &entry_a, &source, TEST_HARD_CAP)
            .expect("add attachment");
        assert_eq!(stored, "codes.txt", "the stored filename is the basename");

        let fetched = service
            .get_entry_attachment(&db_path, &entry_a, "codes.txt")
            .expect("fetch added attachment");
        assert_eq!(
            fetched.as_bytes(),
            payload.as_slice(),
            "the added attachment must round-trip the on-disk bytes verbatim"
        );

        let entry = service.get_entry(&db_path, &entry_a).expect("get entry");
        let meta = entry
            .attachments
            .iter()
            .find(|a| a.filename == "codes.txt")
            .expect("codes.txt metadata");
        assert_eq!(meta.size, payload.len() as u64);
        assert_eq!(meta.mime_type, "text/plain");

        service
            .with_vault(&db_path, |vault| {
                assert!(
                    vault.generation() > generation_before,
                    "a successful add must mark the Vault modified"
                );
                Ok(())
            })
            .expect("vault scope");
    }

    #[test]
    fn add_entry_attachment_rejects_a_file_over_the_hard_cap() {
        // A file larger than the hard per-file cap must be rejected with a
        // clear error and must leave the Vault completely untouched — no
        // partial binary, no modified flag — so an oversized pick never bloats
        // the database into something unusable.
        let (service, dir, db_path, entry_a, _b) = create_test_database();
        let generation_before = service
            .with_vault(&db_path, |vault| Ok(vault.generation()))
            .expect("generation before add");

        let oversized = vec![0u8; usize::try_from(TEST_HARD_CAP + 1).expect("cap fits usize")];
        let source = dir.path().join("huge.bin");
        std::fs::write(&source, &oversized).expect("seed oversized file");

        let result = service.add_entry_attachment(&db_path, &entry_a, &source, TEST_HARD_CAP);
        assert!(
            matches!(result, Err(AppError::AttachmentTooLarge { ref filename, .. }) if filename == "huge.bin"),
            "an over-cap file must be rejected with AttachmentTooLarge, got {result:?}"
        );

        let entry = service.get_entry(&db_path, &entry_a).expect("get entry");
        assert!(
            entry.attachments.is_empty(),
            "a rejected add must not store any binary"
        );
        service
            .with_vault(&db_path, |vault| {
                assert_eq!(
                    vault.generation(),
                    generation_before,
                    "a rejected add must not mark the Vault modified"
                );
                Ok(())
            })
            .expect("vault scope");
    }

    #[test]
    fn add_entry_attachment_stores_the_real_bytes_when_the_cap_is_u64_max() {
        // Regression: a hand-edited settings.json can set hardCapBytes to
        // u64::MAX (the validator only rejects 0 and soft > hard). The read
        // bound was `hard_cap + 1`, which overflows at u64::MAX — panicking in
        // overflow-checked builds, or wrapping to take(0) in release and storing
        // an empty attachment for any non-empty file. With saturating_add the
        // file's real bytes round-trip and nothing is silently truncated.
        let (service, dir, db_path, entry_a, _b) = create_test_database();

        let payload = b"real-contents-not-empty";
        let source = dir.path().join("notes.txt");
        std::fs::write(&source, payload).expect("seed file");

        let stored = service
            .add_entry_attachment(&db_path, &entry_a, &source, u64::MAX)
            .expect("add under an effectively unlimited cap");
        assert_eq!(stored, "notes.txt");

        let bytes = service
            .get_entry_attachment(&db_path, &entry_a, "notes.txt")
            .expect("read back attachment");
        assert_eq!(
            bytes.as_bytes(),
            payload,
            "the full file bytes must round-trip, not a truncated/empty blob"
        );
    }

    #[test]
    fn add_entry_attachment_auto_renames_on_filename_collision() {
        // Adding a file whose name already exists on the Entry must auto-rename
        // the newcomer (`name (1).ext`, `name (2).ext`, …) rather than
        // overwrite — so a multi-file batch never silently clobbers an existing
        // attachment or fails mid-way on a collision. Both originals stay
        // intact with their own bytes.
        let (service, dir, db_path, entry_a, _b) = create_test_database();

        let first = dir.path().join("scan.pdf");
        std::fs::write(&first, b"first-bytes").expect("seed first file");
        let stored_first = service
            .add_entry_attachment(&db_path, &entry_a, &first, TEST_HARD_CAP)
            .expect("add first");
        assert_eq!(stored_first, "scan.pdf", "the first add keeps its name");

        // A different file that happens to share the basename.
        let second = dir.path().join("subdir-stand-in");
        std::fs::create_dir_all(&second).expect("mk subdir");
        let second_file = second.join("scan.pdf");
        std::fs::write(&second_file, b"second-bytes").expect("seed second file");
        let stored_second = service
            .add_entry_attachment(&db_path, &entry_a, &second_file, TEST_HARD_CAP)
            .expect("add second");
        assert_eq!(
            stored_second, "scan (1).pdf",
            "a colliding name auto-renames the newcomer"
        );

        // The original must be untouched, and the renamed copy carries its own
        // distinct bytes — nothing was overwritten.
        let original = service
            .get_entry_attachment(&db_path, &entry_a, "scan.pdf")
            .expect("fetch original");
        assert_eq!(original.as_bytes(), b"first-bytes");
        let renamed = service
            .get_entry_attachment(&db_path, &entry_a, "scan (1).pdf")
            .expect("fetch renamed");
        assert_eq!(renamed.as_bytes(), b"second-bytes");

        let entry = service.get_entry(&db_path, &entry_a).expect("get entry");
        assert_eq!(
            entry.attachments.len(),
            2,
            "both attachments coexist after the auto-rename"
        );
    }

    #[test]
    fn add_entry_attachment_rejects_a_non_regular_file() {
        // A path that isn't a regular file (a directory here; on Unix also
        // FIFOs, /dev/zero, …) has a meaningless metadata length and an
        // unbounded read could hang or OOM, so it must be rejected before any
        // bytes are pulled in. Defends the hard cap against special files.
        let (service, dir, db_path, entry_a, _b) = create_test_database();
        let not_a_file = dir.path().join("a-directory");
        std::fs::create_dir(&not_a_file).expect("mk dir");

        let result = service.add_entry_attachment(&db_path, &entry_a, &not_a_file, TEST_HARD_CAP);
        assert!(
            matches!(result, Err(AppError::InvalidInput(_))),
            "a non-regular file must be rejected, got {result:?}"
        );

        let entry = service.get_entry(&db_path, &entry_a).expect("get entry");
        assert!(
            entry.attachments.is_empty(),
            "nothing is stored for a rejected add"
        );
    }

    #[test]
    fn add_entry_attachments_adds_every_path_in_pick_order() {
        // The batch add is the only feeder the trusted-source command uses: it
        // takes the list of OS-provided paths and stores each, returning the
        // stored names in pick order. The whole list lands and the Vault is
        // marked modified once it has run.
        let (service, dir, db_path, entry_a, _b) = create_test_database();

        let first = dir.path().join("codes.txt");
        std::fs::write(&first, b"first").expect("seed first");
        let second = dir.path().join("scan.pdf");
        std::fs::write(&second, b"second").expect("seed second");

        let outcome = service
            .add_entry_attachments(&db_path, &entry_a, &[first, second], TEST_HARD_CAP)
            .expect("batch add");

        assert_eq!(
            outcome.added,
            vec!["codes.txt".to_string(), "scan.pdf".to_string()],
            "every picked path is stored, in pick order"
        );
        assert!(outcome.failed.is_empty(), "no failures for valid files");
    }

    #[test]
    fn add_entry_attachments_keeps_going_when_one_file_is_rejected() {
        // A rejected file (here, one over the hard cap) must not abort the
        // batch: the surviving files still land, and the failure is reported
        // with its basename and the backend reason so the UI can surface a
        // per-file toast. Mirrors the single-add resilience the UI relied on.
        let (service, dir, db_path, entry_a, _b) = create_test_database();

        let ok_a = dir.path().join("ok-a.txt");
        std::fs::write(&ok_a, b"a").expect("seed ok-a");
        let huge = dir.path().join("huge.bin");
        std::fs::write(
            &huge,
            vec![0u8; usize::try_from(TEST_HARD_CAP + 1).expect("cap fits usize")],
        )
        .expect("seed huge");
        let ok_b = dir.path().join("ok-b.txt");
        std::fs::write(&ok_b, b"b").expect("seed ok-b");

        let outcome = service
            .add_entry_attachments(&db_path, &entry_a, &[ok_a, huge, ok_b], TEST_HARD_CAP)
            .expect("batch add");

        assert_eq!(
            outcome.added,
            vec!["ok-a.txt".to_string(), "ok-b.txt".to_string()],
            "the surviving files still land despite the rejected one"
        );
        assert_eq!(outcome.failed.len(), 1, "exactly the over-cap file fails");
        assert_eq!(outcome.failed[0].source_name, "huge.bin");
        assert!(
            outcome.failed[0].reason.contains("exceeding"),
            "the failure carries the backend reason, got {:?}",
            outcome.failed[0].reason
        );
    }

    /// Asserts that feeding `paths` to the add feeder is a no-op: nothing is
    /// added or fails, the Entry stays empty, and the Vault is unmodified.
    /// Shared by the picker trust-boundary test (an empty pick while a secret
    /// sits on disk) and the drop-commit no-op test (an empty buffer), since
    /// both make the same "a path never handed in is never read" guarantee.
    fn assert_empty_add_is_inert(service: &KdbxService, db_path: &str, entry_id: &str) {
        let generation_before = service
            .with_vault(db_path, |vault| Ok(vault.generation()))
            .expect("generation before");

        let outcome = service
            .add_entry_attachments(db_path, entry_id, &[], TEST_HARD_CAP)
            .expect("empty add");

        assert!(outcome.added.is_empty(), "nothing is added");
        assert!(outcome.failed.is_empty(), "and nothing fails");

        let entry = service.get_entry(db_path, entry_id).expect("get entry");
        assert!(
            entry.attachments.is_empty(),
            "a file never handed to the add path must not be readable through it"
        );
        service
            .with_vault(db_path, |vault| {
                assert_eq!(
                    vault.generation(),
                    generation_before,
                    "no read means no modification"
                );
                Ok(())
            })
            .expect("vault scope");
    }

    #[test]
    fn add_entry_attachments_reads_only_paths_handed_to_it() {
        // The trust boundary (issue #296): a file the user never selected
        // through the OS picker must not be readable through the add path. The
        // command no longer accepts a renderer-supplied `source_path` — the
        // only paths that reach the read are the ones in this slice. So a
        // sensitive file sitting on disk that is NOT in the handed-in list (the
        // empty list models a cancelled dialog) is never read, nothing is
        // stored, and the Vault is left untouched. There is structurally no
        // parameter through which a fabricated path could be injected.
        let (service, dir, db_path, entry_a, _b) = create_test_database();
        let secret = dir.path().join("id_rsa");
        std::fs::write(&secret, b"-----BEGIN PRIVATE KEY-----").expect("seed secret");

        assert_empty_add_is_inert(&service, &db_path, &entry_a);
    }

    #[test]
    fn commit_drains_the_drop_buffer_into_the_add_feeder() {
        // Drag-and-drop (#286) reuses the same trusted feeder as the picker: a
        // native drop buffers its OS-provided paths, and the commit drains them
        // into `add_entry_attachments`. This proves the drain+feed seam end to
        // end — what the drop captured lands on the Entry, and the buffer is
        // emptied so the same drop can't be replayed against a later entry.
        let (service, dir, db_path, entry_a, _b) = create_test_database();
        let dropped = dir.path().join("dropped.txt");
        std::fs::write(&dropped, b"from a drag-drop").expect("seed dropped file");

        let buffer = crate::services::drag_drop::PendingAttachmentPaths::default();
        let batch = buffer.replace(vec![dropped.clone()]);

        // The commit drains the buffer and hands only those paths to the feeder.
        let paths = buffer.take(batch);
        let outcome = service
            .add_entry_attachments(&db_path, &entry_a, &paths, TEST_HARD_CAP)
            .expect("commit drained drop");

        assert_eq!(
            outcome.added,
            vec!["dropped.txt".to_string()],
            "the dropped file lands on the Entry through the shared feeder"
        );

        let entry = service.get_entry(&db_path, &entry_a).expect("get entry");
        assert!(
            entry
                .attachments
                .iter()
                .any(|a| a.filename == "dropped.txt"),
            "the attachment is persisted on the Entry"
        );

        assert!(
            buffer.take(batch).is_empty(),
            "the buffer is drained, so a second commit reads nothing"
        );
    }

    #[test]
    fn commit_with_no_preceding_drop_is_a_noop() {
        // The drop event is window-global, so a commit can fire with nothing
        // buffered (e.g. a stale render). Draining an empty buffer hands the
        // feeder no paths, so nothing is read or stored and the Vault is
        // untouched — the same guarantee as a cancelled picker dialog.
        let (service, _dir, db_path, entry_a, _b) = create_test_database();

        let buffer = crate::services::drag_drop::PendingAttachmentPaths::default();
        assert!(buffer.take(0).is_empty(), "no drop means no buffered paths");

        assert_empty_add_is_inert(&service, &db_path, &entry_a);
    }

    #[test]
    fn list_entries_without_group_filter_hides_recycle_bin_entries() {
        // Regression: deleting an entry moves it to the recycle bin, but the
        // unfiltered list view used to keep returning it (since
        // db.iter_all_entries() walks the whole tree). The user observed:
        // entry vanishes from details, stays in list, "isn't really deleted".
        let (service, _dir, db_path, entry_a, entry_b) = create_test_database();

        let initial = service
            .list_entries(&db_path, None)
            .expect("list all entries");
        let initial_ids: Vec<&str> = initial.iter().map(|e| e.id.as_str()).collect();
        assert!(initial_ids.contains(&entry_a.as_str()));
        assert!(initial_ids.contains(&entry_b.as_str()));

        service
            .delete_entry(&db_path, &entry_a)
            .expect("delete entry A");

        let after = service
            .list_entries(&db_path, None)
            .expect("list all entries after delete");
        let after_ids: Vec<&str> = after.iter().map(|e| e.id.as_str()).collect();
        assert!(
            !after_ids.contains(&entry_a.as_str()),
            "deleted entry should not appear in the unfiltered list"
        );
        assert!(
            after_ids.contains(&entry_b.as_str()),
            "non-deleted siblings should remain"
        );

        // The entry is still in the database — just inside the recycle bin —
        // and accessible when the recycle bin group is queried directly.
        let recycle_id = service
            .get_recycle_bin_id(&db_path)
            .expect("get recycle bin id")
            .expect("recycle bin should exist after a delete");
        let in_recycle = service
            .list_entries(&db_path, Some(&recycle_id))
            .expect("list recycle bin entries");
        let recycle_ids: Vec<&str> = in_recycle.iter().map(|e| e.id.as_str()).collect();
        assert!(
            recycle_ids.contains(&entry_a.as_str()),
            "deleted entry should be visible inside the recycle bin group"
        );
    }

    #[test]
    fn list_entry_history_is_empty_for_a_freshly_created_entry() {
        // A brand-new Entry has never been edited, so its native KDBX history
        // (a fresh, empty `History`) yields no versions. The listing must report
        // that as an empty list, not an error.
        let (service, _dir, db_path, entry_a, _b) = create_test_database();

        let history = service
            .list_entry_history(&db_path, &entry_a)
            .expect("list history for a fresh entry");

        assert!(
            history.is_empty(),
            "a never-edited entry has no history versions"
        );
    }

    #[test]
    fn editing_a_field_snapshots_the_prior_state_into_history() {
        // The core chokepoint: an edit pushes the Entry's *prior* state into
        // its KDBX history before the mutation lands. Entry A starts as
        // username "alice"; after renaming it to "bob" the live entry reads
        // "bob" while the single history version preserves "alice".
        let (service, _dir, db_path, entry_a, _b) = create_test_database();

        let updated = service
            .update_entry(
                &db_path,
                &entry_a,
                UpdateEntryData {
                    username: Some("bob".to_string()),
                    ..empty_update()
                },
            )
            .expect("rename username");
        assert_eq!(updated.username, "bob", "live entry reflects the new value");

        let history = service
            .list_entry_history(&db_path, &entry_a)
            .expect("list history after edit");

        assert_eq!(history.len(), 1, "one edit captures exactly one version");
        assert_eq!(
            history[0].username, "alice",
            "the version preserves the prior username, not the new one"
        );
        assert_eq!(history[0].index, 0, "the sole version sits at index 0");
    }

    #[test]
    fn a_no_op_update_does_not_snapshot_history() {
        // The edit form submits a full update payload on every Save — even when
        // nothing is dirty — and emits a no-op update before a group move. Such
        // content-preserving updates must NOT accrue a junk history version; the
        // snapshot is gated on an actual change to stored content.
        let (service, _dir, db_path, entry_a, _b) = create_test_database();

        let current = service.get_entry(&db_path, &entry_a).expect("get entry");
        let password = service
            .get_entry_password(&db_path, &entry_a)
            .expect("get password");

        // Re-send the entry's current values unchanged.
        service
            .update_entry(
                &db_path,
                &entry_a,
                UpdateEntryData {
                    title: Some(current.title.clone()),
                    username: Some(current.username.clone()),
                    password: Some(SecureString::from(password)),
                    url: current.url.clone(),
                    notes: current.notes.clone(),
                    ..empty_update()
                },
            )
            .expect("no-op update");

        let history = service
            .list_entry_history(&db_path, &entry_a)
            .expect("list history after no-op update");

        assert!(
            history.is_empty(),
            "a content-preserving update must not create a history version"
        );
    }

    #[test]
    fn successive_edits_stack_history_newest_first() {
        // Each edit prepends the just-replaced state, so the list reads
        // newest-first: alice → bob → carol leaves the live entry as "carol",
        // with version[0] holding "bob" (the most recent prior state) and
        // version[1] holding "alice" (the original).
        let (service, _dir, db_path, entry_a, _b) = create_test_database();

        for name in ["bob", "carol"] {
            service
                .update_entry(
                    &db_path,
                    &entry_a,
                    UpdateEntryData {
                        username: Some(name.to_string()),
                        ..empty_update()
                    },
                )
                .expect("rename username");
        }

        let history = service
            .list_entry_history(&db_path, &entry_a)
            .expect("list history after two edits");

        assert_eq!(history.len(), 2, "two edits capture two versions");
        assert_eq!(history[0].index, 0);
        assert_eq!(
            history[0].username, "bob",
            "newest version holds the most recent prior state"
        );
        assert_eq!(history[1].index, 1);
        assert_eq!(
            history[1].username, "alice",
            "oldest version holds the original state"
        );
    }

    #[test]
    fn entry_with_no_history_node_lists_empty_not_error() {
        // KeePass entries imported from some apps (or malformed ones) carry
        // `history: None` rather than an empty `History`. Reading their history
        // must degrade to an empty list, never an error — the listing goes
        // through `History::get_entries()` only when the node exists.
        let (service, _dir, db_path, entry_a, _b) = create_test_database();

        // Drop the history node entirely to mimic an imported/malformed entry.
        service
            .with_vault_mut(&db_path, |vault| {
                let mut entry = vault.entry_mut(&entry_a)?;
                entry.history = None;
                Ok(())
            })
            .expect("clear history node");

        let history = service
            .list_entry_history(&db_path, &entry_a)
            .expect("listing a None-history entry must not error");

        assert!(
            history.is_empty(),
            "a None history node lists as empty, not an error"
        );
    }

    #[test]
    fn history_survives_a_save_then_reopen_round_trip() {
        // The interop proof point (ADR-0008): a captured version is real native
        // KDBX history, so it must survive being written to disk and read back.
        // Edit the entry, persist, then reopen the file with a *fresh* service
        // (nothing cached in memory) and assert the version is still there.
        let (service, _dir, db_path, entry_a, _b) = create_test_database();

        service
            .update_entry(
                &db_path,
                &entry_a,
                UpdateEntryData {
                    username: Some("bob".to_string()),
                    ..empty_update()
                },
            )
            .expect("rename username");
        service.save(&db_path).expect("save vault");
        service.close(&db_path).expect("close vault");

        let reopened = KdbxService::new();
        reopened.open(&db_path, "testpass").expect("reopen vault");

        let history = reopened
            .list_entry_history(&db_path, &entry_a)
            .expect("list history after reopen");

        assert_eq!(
            history.len(),
            1,
            "the captured version must survive the on-disk round-trip"
        );
        assert_eq!(
            history[0].username, "alice",
            "the reopened version preserves the prior username"
        );
    }

    /// Sets an Entry's tags directly through the vault, bypassing
    /// `update_entry` so the seeding itself records no history version.
    fn seed_tags(service: &KdbxService, db_path: &str, entry_id: &str, tags: &[&str]) {
        service
            .with_vault_mut(db_path, |vault| {
                let mut entry = vault.entry_mut(entry_id)?;
                entry.tags = tags.iter().map(|t| (*t).to_string()).collect();
                Ok(())
            })
            .expect("seed tags");
    }

    /// Reads a historical version's tags by reaching through the live Entry's
    /// native KDBX history — the listing DTO carries no tags, so structural
    /// assertions inspect the version directly.
    fn historical_tags(
        service: &KdbxService,
        db_path: &str,
        entry_id: &str,
        index: usize,
    ) -> Vec<String> {
        service
            .with_vault(db_path, |vault| {
                let entry = vault.find_entry(entry_id)?;
                Ok(entry
                    .historical(index)
                    .expect("history version exists")
                    .tags
                    .clone())
            })
            .expect("read historical tags")
    }

    #[test]
    fn reading_an_entry_snapshots_nothing() {
        // Access-only paths — fetching the Entry, its password, an attachment,
        // or listing its history — are not edits and must never accrue a
        // history version (#323).
        let (service, _dir, db_path, entry_a, _b) = create_test_database();
        seed_attachment(&service, &db_path, &entry_a, "codes.txt", b"data");

        service.get_entry(&db_path, &entry_a).expect("get entry");
        service
            .get_entry_password(&db_path, &entry_a)
            .expect("get password");
        service
            .get_entry_attachment(&db_path, &entry_a, "codes.txt")
            .expect("get attachment");
        service
            .list_entry_history(&db_path, &entry_a)
            .expect("list history");

        let history = service
            .list_entry_history(&db_path, &entry_a)
            .expect("list history after reads");
        assert!(
            history.is_empty(),
            "reading or accessing an Entry must not create a history version"
        );
    }

    #[test]
    fn adding_an_attachment_snapshots_the_prior_state() {
        // Adding an Attachment changes the Entry's stored content, so the
        // chokepoint captures the prior state first. The history version
        // predates the add, so it carries none of the new attachment.
        let (service, dir, db_path, entry_a, _b) = create_test_database();

        let source = dir.path().join("codes.txt");
        std::fs::write(&source, b"recovery-codes").expect("seed source file");

        service
            .add_entry_attachment(&db_path, &entry_a, &source, TEST_HARD_CAP)
            .expect("add attachment");

        let history = service
            .list_entry_history(&db_path, &entry_a)
            .expect("list history after add");
        assert_eq!(
            history.len(),
            1,
            "adding an attachment captures one version"
        );

        let prior_attachments = service
            .with_vault(&db_path, |vault| {
                let entry = vault.find_entry(&entry_a)?;
                Ok(entry
                    .historical(0)
                    .expect("history version exists")
                    .attachments()
                    .count())
            })
            .expect("inspect historical attachments");
        assert_eq!(
            prior_attachments, 0,
            "the snapshot predates the add, so it holds no attachment"
        );
    }

    #[test]
    fn bulk_rename_tag_snapshots_each_affected_entry() {
        // A vault-wide tag rename must capture a per-Entry snapshot before
        // rewriting the tag, matching KeePassXC — and only on Entries that
        // actually carried the tag.
        let (service, _dir, db_path, entry_a, entry_b) = create_test_database();
        seed_tags(&service, &db_path, &entry_a, &["work", "urgent"]);
        // entry_b never carries the tag, so it must stay untouched.

        let count = service
            .rename_tag(&db_path, "work", "office")
            .expect("rename tag across vault");
        assert_eq!(count, 1, "only the one tagged Entry is modified");

        let history_a = service
            .list_entry_history(&db_path, &entry_a)
            .expect("list history for affected entry");
        assert_eq!(history_a.len(), 1, "the affected Entry gets one snapshot");
        assert_eq!(
            historical_tags(&service, &db_path, &entry_a, 0),
            vec!["work".to_string(), "urgent".to_string()],
            "the snapshot preserves the tags as they were before the rename"
        );

        let live = service.get_entry(&db_path, &entry_a).expect("get entry");
        assert!(
            live.tags.contains(&"office".to_string()),
            "the live Entry reflects the renamed tag"
        );

        let history_b = service
            .list_entry_history(&db_path, &entry_b)
            .expect("list history for untouched entry");
        assert!(
            history_b.is_empty(),
            "an Entry without the tag accrues no snapshot"
        );
    }

    #[test]
    fn bulk_delete_tag_snapshots_each_affected_entry() {
        // Deleting a tag vault-wide likewise snapshots each Entry that loses it.
        let (service, _dir, db_path, entry_a, _b) = create_test_database();
        seed_tags(&service, &db_path, &entry_a, &["work", "urgent"]);

        let count = service
            .delete_tag(&db_path, "urgent")
            .expect("delete tag across vault");
        assert_eq!(count, 1, "the one tagged Entry is modified");

        let history = service
            .list_entry_history(&db_path, &entry_a)
            .expect("list history after tag delete");
        assert_eq!(history.len(), 1, "the affected Entry gets one snapshot");
        assert_eq!(
            historical_tags(&service, &db_path, &entry_a, 0),
            vec!["work".to_string(), "urgent".to_string()],
            "the snapshot preserves the deleted tag"
        );
    }

    #[test]
    fn a_no_op_tag_rename_snapshots_nothing() {
        // Renaming a tag no Entry carries (or renaming a tag to itself) changes
        // no stored content, so it must not accrue history versions.
        let (service, _dir, db_path, entry_a, _b) = create_test_database();
        seed_tags(&service, &db_path, &entry_a, &["work"]);

        let missing = service
            .rename_tag(&db_path, "nonexistent", "office")
            .expect("rename a tag nobody has");
        assert_eq!(missing, 0, "no Entry carries the tag");

        let identity = service
            .rename_tag(&db_path, "work", "work")
            .expect("rename a tag to itself");
        assert_eq!(identity, 0, "renaming a tag to itself is a no-op");

        let history = service
            .list_entry_history(&db_path, &entry_a)
            .expect("list history after no-op renames");
        assert!(
            history.is_empty(),
            "no-op tag renames must not create history versions"
        );
    }

    #[test]
    fn moving_between_real_groups_snapshots_prior_state() {
        // Moving an Entry between two real Groups changes its stored location,
        // so the chokepoint captures the prior state before the move lands.
        let (service, _dir, db_path, entry_a, _b) = create_test_database();
        let target = service
            .create_group(&db_path, None, "Target", None)
            .expect("create target group");

        service
            .move_entry(&db_path, &entry_a, &target.id)
            .expect("move entry to a real group");

        let history = service
            .list_entry_history(&db_path, &entry_a)
            .expect("list history after move");
        assert_eq!(
            history.len(),
            1,
            "a move between real Groups captures exactly one version"
        );
    }

    #[test]
    fn sending_to_recycle_bin_does_not_snapshot() {
        // Deletion is a Recycle-Bin transition, not a content edit: it is
        // already reversible, so it must not accrue history noise.
        let (service, _dir, db_path, entry_a, _b) = create_test_database();

        service
            .delete_entry(&db_path, &entry_a)
            .expect("send entry to recycle bin");

        let history = service
            .list_entry_history(&db_path, &entry_a)
            .expect("list history after delete");
        assert!(
            history.is_empty(),
            "sending to the Recycle Bin must not create a history version"
        );
    }

    #[test]
    fn restoring_from_recycle_bin_does_not_snapshot() {
        // Restoring a trashed Entry (a move *out* of the Recycle Bin) is the
        // mirror of deletion and is likewise excluded from history.
        let (service, _dir, db_path, entry_a, _b) = create_test_database();
        let root = service.get_info(&db_path).expect("info").root_group_id;

        service
            .delete_entry(&db_path, &entry_a)
            .expect("send entry to recycle bin");
        service
            .move_entry(&db_path, &entry_a, &root)
            .expect("restore entry to a real group");

        let history = service
            .list_entry_history(&db_path, &entry_a)
            .expect("list history after restore");
        assert!(
            history.is_empty(),
            "restoring from the Recycle Bin must not create a history version"
        );
    }

    #[test]
    fn moving_into_recycle_bin_via_move_does_not_snapshot() {
        // A move whose *target* is the Recycle Bin is a trash transition, not a
        // relocation, and shares the deletion exclusion.
        let (service, _dir, db_path, entry_a, entry_b) = create_test_database();

        // Materialise the Recycle Bin by trashing a throwaway entry, then read
        // its group id back from vault meta.
        service
            .delete_entry(&db_path, &entry_b)
            .expect("trash throwaway to create recycle bin");
        let recycle_gid = service
            .with_vault(&db_path, |vault| {
                Ok(vault
                    .db()
                    .meta
                    .recyclebin_uuid
                    .map(|u| u.to_string())
                    .expect("recycle bin exists"))
            })
            .expect("read recycle uuid");

        service
            .move_entry(&db_path, &entry_a, &recycle_gid)
            .expect("move entry into the recycle bin");

        let history = service
            .list_entry_history(&db_path, &entry_a)
            .expect("list history after move into recycle bin");
        assert!(
            history.is_empty(),
            "moving into the Recycle Bin must not create a history version"
        );
    }
}
