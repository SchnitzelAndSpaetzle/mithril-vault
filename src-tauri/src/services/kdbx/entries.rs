use crate::domain::secure::SecureBytes;
use crate::dto::entry::{
    AddAttachmentsOutcome, AttachmentAddFailure, AttachmentAddPlan, AttachmentPlanItem,
    AttachmentSizeStatus, CreateEntryData, CustomFieldValue, Entry, EntryHistoryItem,
    UpdateEntryData,
};
use crate::dto::error::AppError;
use crate::utils::atomic_write::{atomic_write, AtomicWriteOptions};
use keepass::db::{Entry as KeepassEntry, EntryRef, History, Icon, Times, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};

use super::conversions::{
    apply_custom_fields, apply_expiry, convert_entry, is_standard_entry_field, parse_expiry_time,
    replace_custom_fields, validate_expiry_enabled,
};
use super::history::HistoryRetention;
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
/// S6 enforces the per-Vault [`HistoryRetention`] here — the single place every
/// mutation funnels through, so the limit applies uniformly:
/// - [`HistoryRetention::Disabled`]: no new snapshot, and the Entry's existing
///   history is dropped — the "pruned to zero lazily on the next content edit"
///   rule. Because only snapshot-producing content edits reach this chokepoint,
///   reversible Recycle-Bin transitions leave a disabled Entry's history intact.
/// - [`HistoryRetention::Unlimited`]: append, never prune.
/// - [`HistoryRetention::Limited`]`(n)`: append, then prune to the newest `n`.
///
/// Takes `&mut KeepassEntry` rather than an `EntryMut` so both the `EntryMut`
/// call sites (via deref coercion) and the raw-`Entry` closures handed to
/// [`Vault::modify_all_entries`] can funnel through the one chokepoint.
///
/// Attachment retention (#332): a pushed pre-image clones the live Entry's
/// `attachments` map, sharing the same binary-pool `AttachmentId`s. The patched
/// `keepass` fork never garbage-collects a binary on attachment removal (it
/// drops only the exact live back-reference, mirroring `set_icon_none`), so a
/// blob referenced by *any* snapshot survives a later `delete_entry_attachment`
/// and a save/reopen round-trip, and its id is never reused. The fork also
/// prunes genuinely-orphaned binaries and re-indexes the pool at save time.
/// Pruning the oldest versions drops their references the same way, so a blob
/// only an evicted version held becomes a genuine orphan and is reclaimed.
pub(crate) fn snapshot_entry_history(
    entry: &mut KeepassEntry,
    mut pre_image: KeepassEntry,
    retention: HistoryRetention,
) {
    match retention {
        HistoryRetention::Disabled => {
            // No new snapshots; drop any existing history lazily on this
            // content edit (Recycle-Bin transitions never reach here).
            entry.history = None;
        }
        HistoryRetention::Unlimited => {
            // `History::add_entry` also strips nested history, but clearing it
            // here makes the intent explicit and keeps the snapshot minimal.
            pre_image.history = None;
            entry.history.get_or_insert_default().add_entry(pre_image);
        }
        HistoryRetention::Limited(max) => {
            pre_image.history = None;
            entry.history.get_or_insert_default().add_entry(pre_image);
            prune_history_to_newest(entry, max);
        }
    }
}

/// Keeps the newest `max` history versions of `entry`, dropping the rest. The
/// native [`History`] exposes no remove/truncate (its `entries` vec is
/// crate-private), so the surviving newest-first prefix is rebuilt through the
/// public `add_entry` API — re-added oldest-first so the result stays
/// newest-first. A no-op when history already fits within `max`.
fn prune_history_to_newest(entry: &mut KeepassEntry, max: usize) {
    let Some(history) = entry.history.as_ref() else {
        return;
    };
    if history.get_entries().len() <= max {
        return;
    }
    let kept: Vec<KeepassEntry> = history.get_entries().iter().take(max).cloned().collect();
    let mut rebuilt = History::default();
    for version in kept.into_iter().rev() {
        rebuilt.add_entry(version);
    }
    entry.history = Some(rebuilt);
}

/// Runs `mutate` against a clone-guarded `entry`, pushing a pre-mutation
/// history snapshot through the chokepoint only when the closure reports a
/// real change. The shared shape behind the bulk tag mutators (rename/delete),
/// where one snapshot must land per touched Entry and nothing on the rest.
fn snapshot_on_change(
    entry: &mut KeepassEntry,
    retention: HistoryRetention,
    mutate: impl FnOnce(&mut KeepassEntry) -> bool,
) -> bool {
    let before = entry.clone();
    if mutate(entry) {
        snapshot_entry_history(entry, before, retention);
        true
    } else {
        false
    }
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

/// Names the fields that differ between two versions of an Entry, for the
/// history "Changed: …" line. `before` is the older version; `after` is the
/// version immediately newer than it (the live Entry, for the newest
/// snapshot). Field *values* — including the password and protected custom
/// fields — are compared in-process so a rotation registers, but only the
/// names are returned: no secret crosses IPC (ADR-0008).
fn changed_field_names(before: &EntryRef<'_>, after: &EntryRef<'_>) -> Vec<String> {
    let mut changed = Vec::new();

    // Text fields: the union of keys present in either version. Compare the
    // stored `Value`, not just the resolved string, so a value change *and* a
    // protected/unprotected toggle of the same text both register. `Value`'s
    // `PartialEq` compares protected secrets by content, so a password change
    // is detected while only the field *name* is emitted.
    let mut keys: BTreeSet<&str> = BTreeSet::new();
    keys.extend(before.fields.keys().map(String::as_str));
    keys.extend(after.fields.keys().map(String::as_str));
    for key in keys {
        if before.fields.get(key) != after.fields.get(key) {
            changed.push(field_display_name(key));
        }
    }

    // Tags: order-insensitive so a reorder isn't reported as a change.
    let mut before_tags = before.tags.clone();
    let mut after_tags = after.tags.clone();
    before_tags.sort();
    after_tags.sort();
    if before_tags != after_tags {
        changed.push("tags".to_string());
    }

    // Icon: builtin id or custom-icon reference (`Icon` is `PartialEq`).
    if before.icon() != after.icon() {
        changed.push("icon".to_string());
    }

    // Expiry: both the flag and the timestamp, so enabling/disabling or moving
    // the date registers.
    if (before.times.expires, before.times.expiry) != (after.times.expires, after.times.expiry) {
        changed.push("expiry".to_string());
    }

    // Attachments: compare the set of filenames only. An add/delete/rename
    // registers; the binary payloads are never read (ADR-0003).
    let before_atts: BTreeSet<&str> = before.attachments_named().map(|(name, _)| name).collect();
    let after_atts: BTreeSet<&str> = after.attachments_named().map(|(name, _)| name).collect();
    if before_atts != after_atts {
        changed.push("attachments".to_string());
    }

    // Location: a move between Groups bumps `location_changed` (but leaves the
    // content untouched), so a move-only version would otherwise have a blank
    // changed line. Second-resolution timestamps mean two moves within the same
    // second don't register — an acceptable, non-crashing degradation versus
    // resolving the (possibly deleted) parent group, which would panic.
    if before.times.location_changed != after.times.location_changed {
        changed.push("location".to_string());
    }

    changed
}

/// A stable per-version content fingerprint: a hex **keyed BLAKE3 MAC** over the
/// snapshot's `modified_at`, every field (key + protected flag + resolved value,
/// so a password rotation registers even when it shares a second with its
/// predecessor), tags, icon, expiry and attachment names + bytes. This is the addressing
/// guard for per-version reveal/restore (ADR-0008): a version is addressed by
/// `index` but only acted on when its fingerprint still matches, so a concurrent
/// edit that shifts the list can't silently retarget. `modified_at` alone is
/// insufficient — `keepass::Times` stores it at second precision, so two
/// snapshots made within the same second share it.
///
/// Secret *content* is fed in to disambiguate same-second rotations, but the
/// MAC is **keyed** by a per-process backend key (`key`), so although the
/// fingerprint is returned by `list_entry_history`, it is not an unsalted
/// brute-force oracle over historical passwords/PINs: confirming a guess would
/// require the key, which never leaves the backend. Inputs are streamed directly
/// into the hasher rather than collected into an owned buffer, so no extra
/// plaintext copy of the secrets lingers in allocator memory after the call.
/// One restored attachment: its filename and the original `Value` (carrying the
/// protected/unprotected flag, so restore preserves it).
type RestoredAttachment = (String, Value<Vec<u8>>);

/// Whether restoring `source` (with its `restored_attachments`) onto `live`
/// would actually change any restorable content. Restore copies content but
/// never identity or location, so a version that differs from the live Entry
/// only in where it lived (a move-only snapshot) restores to a no-op. Comparing
/// only the restorable subset — and attachments by name→`Value`, not pool id —
/// lets the caller reject that no-op instead of bumping mtime, snapshotting,
/// auditing, and reporting a phantom success (#328).
fn restore_changes_content(
    live: &EntryRef<'_>,
    source: &KeepassEntry,
    restored_attachments: &[RestoredAttachment],
) -> bool {
    if live.fields != source.fields
        || live.tags != source.tags
        || live.custom_data != source.custom_data
        || live.autotype != source.autotype
        || live.foreground_color != source.foreground_color
        || live.background_color != source.background_color
        || live.override_url != source.override_url
        || live.quality_check != source.quality_check
        || live.icon() != source.icon()
        || live.times.expires != source.times.expires
        || live.times.expiry != source.times.expiry
    {
        return true;
    }
    // Attachments by content (name → bytes + protection flag), not by the
    // pool ids the rebuild would churn.
    let live_attachments: BTreeMap<String, Value<Vec<u8>>> = live
        .attachments_named()
        .map(|(name, att)| (name.to_string(), att.data.clone()))
        .collect();
    let restored: BTreeMap<String, Value<Vec<u8>>> = restored_attachments.iter().cloned().collect();
    live_attachments != restored
}

fn history_fingerprint(snapshot: &EntryRef<'_>, key: &[u8; blake3::KEY_LEN]) -> String {
    // Length-prefix every segment so distinct field boundaries can't collide by
    // concatenation (e.g. key "ab"+value "c" vs key "a"+value "bc").
    fn feed(hasher: &mut blake3::Hasher, bytes: &[u8]) {
        hasher.update(&(bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }

    let mut hasher = blake3::Hasher::new_keyed(key);

    feed(
        &mut hasher,
        snapshot
            .times
            .last_modification
            .map(|t| t.to_string())
            .unwrap_or_default()
            .as_bytes(),
    );

    // Fields in key order so the fingerprint is independent of map iteration.
    let mut keys: Vec<&str> = snapshot.fields.keys().map(String::as_str).collect();
    keys.sort_unstable();
    for field_key in keys {
        feed(&mut hasher, field_key.as_bytes());
        if let Some(value) = snapshot.fields.get(field_key) {
            feed(
                &mut hasher,
                &[u8::from(matches!(value, Value::Protected(_)))],
            );
            feed(&mut hasher, value.get().as_bytes());
        }
    }

    let mut tags = snapshot.tags.clone();
    tags.sort();
    for tag in &tags {
        feed(&mut hasher, tag.as_bytes());
    }

    feed(&mut hasher, format!("{:?}", snapshot.icon()).as_bytes());
    feed(
        &mut hasher,
        &[u8::from(snapshot.times.expires.unwrap_or(false))],
    );
    feed(
        &mut hasher,
        snapshot
            .times
            .expiry
            .map(|t| t.to_string())
            .unwrap_or_default()
            .as_bytes(),
    );

    // Attachment name, protection flag, *and* bytes, in name order. Restore
    // copies the whole `Value` — bytes and protected/unprotected state — so the
    // guard must cover all three: a same-second delete + re-add of a different
    // file (or the same file with a flipped protection flag) under the same name
    // would otherwise share a fingerprint and let a stale index shift restore
    // onto the wrong content. Bytes are streamed straight into the hasher (no
    // owned copy lingers); `AttachmentRef` borrows the snapshot's Database, which
    // outlives this call.
    let mut attachments: Vec<_> = snapshot.attachments_named().collect();
    attachments.sort_by(|a, b| a.0.cmp(b.0));
    for (name, attachment) in &attachments {
        feed(&mut hasher, name.as_bytes());
        feed(
            &mut hasher,
            &[u8::from(matches!(attachment.data, Value::Protected(_)))],
        );
        feed(&mut hasher, attachment.get());
    }

    hasher.finalize().to_hex().to_string()
}

/// The keys of a snapshot's *protected custom* fields (excludes the standard
/// Password etc.), sorted. Names only — values never cross IPC (ADR-0008); they
/// let the view offer a per-version reveal action for each protected field.
/// The filenames of a snapshot's attachments — names only, never the bytes
/// (ADR-0008). Sorted so the listing is deterministic and the frontend's
/// filename-set diff is stable.
fn attachment_names(snapshot: &EntryRef<'_>) -> Vec<String> {
    let mut names: Vec<String> = snapshot
        .attachments_named()
        .map(|(name, _)| name.to_string())
        .collect();
    names.sort();
    names
}

fn protected_field_keys(snapshot: &EntryRef<'_>) -> Vec<String> {
    let mut keys: Vec<String> = snapshot
        .fields
        .iter()
        .filter(|(key, value)| {
            matches!(value, Value::Protected(_)) && !is_standard_entry_field(key)
        })
        .map(|(key, _)| key.clone())
        .collect();
    keys.sort();
    keys
}

/// Maps a KDBX field key to the name surfaced in `changed_fields`. Standard
/// fields read as lowercase domain names (`password`, `title`, …) so the view
/// can localize them; custom fields keep their user-defined key verbatim.
fn field_display_name(key: &str) -> String {
    match key {
        "Title" => "title",
        "UserName" => "username",
        "Password" => "password",
        "URL" => "url",
        "Notes" => "notes",
        other => other,
    }
    .to_string()
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
            let count = entry.history.as_ref().map_or(0, |h| h.get_entries().len());
            let mut items = Vec::with_capacity(count);
            for index in 0..count {
                let snapshot = entry
                    .historical(index)
                    .ok_or_else(|| AppError::EntryNotFound(id.to_string()))?;
                // Diff this snapshot against the version immediately newer than
                // it: the previous (newer) snapshot, or the live Entry when this
                // is the newest snapshot (index 0).
                let changed_fields = if index == 0 {
                    changed_field_names(&snapshot, &entry)
                } else {
                    let newer = entry
                        .historical(index - 1)
                        .ok_or_else(|| AppError::EntryNotFound(id.to_string()))?;
                    changed_field_names(&snapshot, &newer)
                };
                // The oldest version is the original "Created" snapshot only
                // when its timestamp still matches the Entry's creation time; if
                // retention pruned the original away, the oldest survivor is an
                // "Earliest kept version" instead.
                let is_creation =
                    index + 1 == count && snapshot.times.last_modification == entry.times.creation;
                items.push(EntryHistoryItem {
                    index,
                    modified_at: snapshot
                        .times
                        .last_modification
                        .map(|t| t.to_string())
                        .unwrap_or_default(),
                    title: snapshot.get_title().unwrap_or_default().to_string(),
                    username: snapshot.get_username().unwrap_or_default().to_string(),
                    url: snapshot.get_url().map(std::string::ToString::to_string),
                    changed_fields,
                    is_creation,
                    fingerprint: history_fingerprint(&snapshot, &self.history_fingerprint_key),
                    protected_fields: protected_field_keys(&snapshot),
                    attachment_names: attachment_names(&snapshot),
                });
            }
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

    /// Resolves the snapshot at `index` in the newest-first history list and
    /// verifies its content fingerprint still matches `expected_fingerprint`
    /// before handing it to `read` (ADR-0008). The guard re-reads the live list
    /// inside the same Vault lock, so a concurrent edit that prepended a new
    /// version (shifting indices) or rewrote this one is caught: either the
    /// index no longer exists or its fingerprint differs, both
    /// [`AppError::HistoryVersionMismatch`]. This is the shared addressing
    /// mechanism reused by Restore (#328). `read` extracts the secret from the
    /// verified snapshot; the listing itself never carries secrets.
    fn with_history_version<T>(
        &self,
        db_id: &str,
        id: &str,
        index: usize,
        expected_fingerprint: &str,
        read: impl FnOnce(&EntryRef<'_>) -> Result<T, AppError>,
    ) -> Result<T, AppError> {
        self.with_vault(db_id, |vault| {
            let entry = vault.find_entry(id)?;
            let snapshot = entry
                .historical(index)
                .ok_or(AppError::HistoryVersionMismatch(index))?;
            if history_fingerprint(&snapshot, &self.history_fingerprint_key) != expected_fingerprint
            {
                return Err(AppError::HistoryVersionMismatch(index));
            }
            read(&snapshot)
        })
    }

    /// Fetches a historical version's password on demand, mirroring
    /// [`get_entry_password`] for the live Entry (ADR-0008). The version is
    /// addressed by `index` in the newest-first list and guarded by
    /// `fingerprint`; a stale/mismatched fingerprint errors rather than
    /// returning the wrong version's secret. The history listing never carries
    /// this value.
    ///
    /// [`get_entry_password`]: Self::get_entry_password
    pub fn get_history_entry_password(
        &self,
        db_id: &str,
        id: &str,
        index: usize,
        fingerprint: &str,
    ) -> Result<String, AppError> {
        self.with_history_version(db_id, id, index, fingerprint, |snapshot| {
            Ok(snapshot
                .get_password()
                .map(std::string::ToString::to_string)
                .unwrap_or_default())
        })
    }

    /// Fetches a historical version's protected custom field on demand,
    /// mirroring [`get_entry_protected_custom_field`] for the live Entry. Same
    /// index+fingerprint guard as [`get_history_entry_password`]; a non-existent
    /// or unprotected key errors exactly as the live path does.
    ///
    /// [`get_entry_protected_custom_field`]: Self::get_entry_protected_custom_field
    pub fn get_history_protected_field(
        &self,
        db_id: &str,
        id: &str,
        index: usize,
        fingerprint: &str,
        key: &str,
    ) -> Result<CustomFieldValue, AppError> {
        if is_standard_entry_field(key) {
            return Err(AppError::CustomFieldNotFound(key.to_string()));
        }
        self.with_history_version(db_id, id, index, fingerprint, |snapshot| {
            let value = snapshot
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

    /// Restores an Entry to a past version, addressed by `index` in the
    /// newest-first history list and guarded by `fingerprint` — the same
    /// addressing mechanism as the per-version reveal (ADR-0008); a
    /// stale/mismatched guard errors [`AppError::HistoryVersionMismatch`]
    /// rather than restoring the wrong version. The current state is first
    /// snapshotted into history (so the restore is itself undoable), then the
    /// live Entry's content — all fields including the password — is overwritten
    /// from the chosen version and `last_modification` bumped to now. The
    /// Entry's UUID and parent Group are left untouched: a version carries
    /// content, not location. The restored secret is read in-process and never
    /// crosses IPC.
    pub fn restore_entry_history(
        &self,
        db_id: &str,
        id: &str,
        index: usize,
        fingerprint: &str,
    ) -> Result<Entry, AppError> {
        self.with_vault_mut(db_id, |vault| {
            let eid = vault.find_entry_id(id)?;
            let retention = vault.history_retention();
            let group_uuid = {
                let mut entry = vault.entry_mut(id)?;

                // Resolve + guard the target version, then clone its content out
                // before any mutation. The fingerprint is re-checked against the
                // live snapshot inside the same Vault lock, so a concurrent edit
                // that shifted indices is caught here. A full owned clone of the
                // snapshot Entry is taken so every content field — text fields
                // (incl. the password), tags, icon, expiry, attachments, and the
                // remaining metadata — can be reapplied without holding the
                // immutable borrow across the mutation.
                let (source, restored_attachments): (KeepassEntry, Vec<RestoredAttachment>) = {
                    let live = entry.as_ref();
                    let snapshot = live
                        .historical(index)
                        .ok_or(AppError::HistoryVersionMismatch(index))?;
                    if history_fingerprint(&snapshot, &self.history_fingerprint_key) != fingerprint
                    {
                        return Err(AppError::HistoryVersionMismatch(index));
                    }
                    // Attachments must be read here, while the snapshot's
                    // `EntryRef` still has Database access — the owned clone
                    // below can't resolve binaries on its own (the map is
                    // crate-private and keyed into the pool). The whole `Value`
                    // is cloned (not just the bytes) so a protected attachment
                    // from an imported vault stays protected after restore.
                    let attachments = snapshot
                        .attachments_named()
                        .map(|(name, att)| (name.to_string(), att.data.clone()))
                        .collect();
                    ((*snapshot).clone(), attachments)
                };

                // Reject a no-op restore: a version that differs from the live
                // Entry only in where it lived (a move-only snapshot) would
                // otherwise bump mtime, append a junk history version, audit,
                // save, and report a phantom success even though nothing
                // restorable changed. Checked before any mutation.
                if !restore_changes_content(&entry.as_ref(), &source, &restored_attachments) {
                    return Err(AppError::HistoryVersionUnchanged);
                }

                // Snapshot the pre-restore state so the restore is undoable: it
                // becomes the newest history version. Attachment binaries that
                // the live Entry still references are kept alive by this
                // snapshot's reference even though the live map is overwritten
                // below (the fork's retention rule).
                let before: KeepassEntry = (*entry.as_ref()).clone();

                // Overwrite content from the chosen version. Identity (UUID),
                // location (parent Group), and the history list are deliberately
                // left untouched — a version carries content, not where the
                // Entry lives.
                entry.fields.clone_from(&source.fields);
                entry.tags.clone_from(&source.tags);
                entry.custom_data.clone_from(&source.custom_data);
                entry.autotype.clone_from(&source.autotype);
                entry.foreground_color.clone_from(&source.foreground_color);
                entry.background_color.clone_from(&source.background_color);
                entry.override_url.clone_from(&source.override_url);
                entry.quality_check = source.quality_check;

                // Icon: the field is crate-private, so go through the setters.
                match source.icon() {
                    None => entry.set_icon_none(),
                    Some(Icon::BuiltIn(n)) => entry.set_icon_builtin(*n),
                    Some(Icon::Custom(cid)) => entry
                        .set_icon_custom(*cid)
                        .map_err(|e| AppError::Kdbx(e.to_string()))?,
                }

                // Attachments: the map is crate-private and shares binary-pool
                // ids, so rebuild it through the public add/remove API. Re-adding
                // by value restores the same bytes (and protection flag) under
                // the same names; ids the live Entry no longer references survive
                // via `before` and are pruned at save time.
                let live_names: Vec<String> = entry
                    .as_ref()
                    .attachments_named()
                    .map(|(name, _)| name.to_string())
                    .collect();
                for name in live_names {
                    entry.remove_attachment_by_name(&name);
                }
                for (name, value) in restored_attachments {
                    entry.add_attachment(name, value);
                }

                // Expiry (flag + timestamp), then bump last_modification to now.
                entry.times.expires = source.times.expires;
                entry.times.expiry = source.times.expiry;
                entry.times.last_modification = Some(Times::now());

                snapshot_entry_history(&mut entry, before, retention);

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

    /// Removes a single Attachment from an Entry, keyed by its filename, after
    /// snapshotting the pre-delete state into Entry History so the removed file
    /// stays recoverable (#332). The patched `keepass` fork drops only the live
    /// reference and retains the binary in the Vault pool for as long as any
    /// history Version references it (mirroring the custom-icon path), so the
    /// snapshot — and any earlier snapshot referencing the same blob — survives
    /// a save/reopen round-trip without the freed id being reused. The Entry's
    /// modification time is bumped and the Vault is marked modified (the caller
    /// persists). Deleting an unknown filename is an
    /// [`AppError::AttachmentNotFound`] that leaves the Vault untouched and
    /// snapshots nothing.
    pub fn delete_entry_attachment(
        &self,
        db_id: &str,
        entry_id: &str,
        filename: &str,
    ) -> Result<(), AppError> {
        self.with_vault_mut(db_id, |vault| {
            let retention = vault.history_retention();
            {
                let mut entry = vault.entry_mut(entry_id)?;
                if entry.attachment_by_name_mut(filename).is_none() {
                    return Err(AppError::AttachmentNotFound(filename.to_string()));
                }
                // Snapshot the pre-delete state (still referencing the blob)
                // before dropping the live reference, mirroring the add path.
                let before: KeepassEntry = (*entry.as_ref()).clone();
                entry.remove_attachment_by_name(filename);
                entry.times.last_modification = Some(Times::now());
                snapshot_entry_history(&mut entry, before, retention);
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
            let retention = vault.history_retention();
            let stored_name = {
                let mut entry = vault.entry_mut(entry_id)?;
                // Snapshot the pre-add state before the new binary lands (#323).
                // The pre-image predates the add, so it never references the new
                // blob.
                let before: KeepassEntry = (*entry.as_ref()).clone();
                let stored_name = unique_attachment_name(&entry.as_ref(), &filename);
                entry.add_attachment(stored_name.clone(), Value::unprotected(bytes));
                entry.times.last_modification = Some(Times::now());
                snapshot_entry_history(&mut entry, before, retention);
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
            let retention = vault.history_retention();
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
                    snapshot_entry_history(&mut entry, before, retention);
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
            // rewritten (#323), kept only when the rename actually changed it.
            let retention = vault.history_retention();
            let count = vault.modify_all_entries(&|entry| {
                snapshot_on_change(entry, retention, |e| {
                    rename_tag_in_entry(e, old_name, new_name)
                })
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
            let retention = vault.history_retention();
            let count = vault.modify_all_entries(&|entry| {
                snapshot_on_change(entry, retention, |e| delete_tag_in_entry(e, tag_name))
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
            let retention = vault.history_retention();

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
                    snapshot_entry_history(&mut entry, before, retention);
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
    use std::collections::{BTreeMap, HashMap};

    /// The hard cap the attachment add/round-trip tests inject. A generous
    /// fixed value so the round-trip cases never trip the cap; the over-cap
    /// cases seed a file one byte past it.
    const TEST_HARD_CAP: u64 = 25 * 1024 * 1024;

    #[test]
    fn history_fingerprint_is_keyed_and_deterministic() {
        // The fingerprint is exposed by `list_entry_history`, so it must be a
        // keyed MAC rather than an unsalted hash of the snapshot's (secret-
        // bearing) content — otherwise it would be an offline brute-force oracle
        // over historical passwords/PINs. This pins that property: fixed key →
        // stable output, different key → different output.
        let (service, _dir, db, entry_a, _entry_b) = create_test_database();
        let key1 = [1u8; blake3::KEY_LEN];
        let key2 = [2u8; blake3::KEY_LEN];
        service
            .with_vault(&db, |vault| {
                let entry = vault.find_entry(&entry_a)?;
                let under_key1 = history_fingerprint(&entry, &key1);
                assert_eq!(
                    under_key1,
                    history_fingerprint(&entry, &key1),
                    "same key + same content must be deterministic"
                );
                assert_ne!(
                    under_key1,
                    history_fingerprint(&entry, &key2),
                    "a different key must yield a different fingerprint (keyed MAC)"
                );
                Ok(())
            })
            .expect("with_vault");
    }

    #[test]
    fn history_fingerprint_distinguishes_attachment_protection_state() {
        // Restore preserves an attachment's protected/unprotected `Value`, so the
        // guard must too: two same-second versions whose attachment differs only
        // by that flag (same name, same bytes) must not share a fingerprint, or a
        // stale index could shift restore onto the wrong protection state. The
        // app's own add path only stores unprotected, so this is constructed
        // directly via the keepass API (imported vaults can carry protected
        // binaries).
        let (service, _dir, db, entry_a, _entry_b) = create_test_database();
        let key = [7u8; blake3::KEY_LEN];
        service
            .with_vault_mut(&db, |vault| {
                let mut entry = vault.entry_mut(&entry_a)?;
                entry.add_attachment("f.txt", Value::protected(vec![1u8, 2, 3]));
                let protected_fp = history_fingerprint(&entry.as_ref(), &key);

                entry.remove_attachment_by_name("f.txt");
                entry.add_attachment("f.txt", Value::unprotected(vec![1u8, 2, 3]));
                let unprotected_fp = history_fingerprint(&entry.as_ref(), &key);

                assert_ne!(
                    protected_fp, unprotected_fp,
                    "fingerprint must reflect an attachment's protection state, not just its bytes"
                );
                Ok(())
            })
            .expect("with_vault_mut");
    }

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
    fn delete_entry_attachment_snapshots_and_retains_blob_for_history() {
        // Deleting an Attachment captures a pre-delete history version and
        // retains the binary in the Vault pool for as long as that version
        // references it (#332) — it is NOT GC'd on the last *live* reference,
        // mirroring how custom icons are kept. The Vault is marked modified.
        let (service, _dir, db_path, entry_a, _b) = create_test_database();
        let generation_before =
            seed_attachment(&service, &db_path, &entry_a, "codes.txt", b"secret");

        service
            .delete_entry_attachment(&db_path, &entry_a, "codes.txt")
            .expect("delete attachment");

        // The live Entry no longer references the attachment...
        let entry = service.get_entry(&db_path, &entry_a).expect("get entry");
        assert!(
            entry.attachments.is_empty(),
            "the live Entry's attachment reference must be gone after delete"
        );

        // ...exactly one pre-delete version was captured...
        let history = service
            .list_entry_history(&db_path, &entry_a)
            .expect("list history");
        assert_eq!(
            history.len(),
            1,
            "deleting an attachment captures one history version"
        );

        let expected: &[u8] = b"secret";
        service
            .with_vault(&db_path, |vault| {
                // ...the blob is retained, referenced only by that version...
                assert_eq!(
                    vault.db().num_attachments(),
                    1,
                    "the blob must be retained while a history version references it"
                );
                // ...and that version still resolves the original bytes.
                let entry = vault.find_entry(&entry_a)?;
                let hist = entry.historical(0).expect("history version exists");
                let att = hist
                    .attachment_by_name("codes.txt")
                    .expect("history retains the attachment binary");
                assert_eq!(att.data.get().as_slice(), expected);
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
    fn the_newest_version_lists_the_fields_changed_against_the_live_entry() {
        // `changed_fields` on a version names what differs from the version
        // immediately newer than it — and the *newest* snapshot's newer
        // neighbour is the live Entry. Entry A starts as username "alice";
        // renaming it to "bob" leaves the sole snapshot (still "alice")
        // reporting that username changed relative to the live "bob".
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

        let history = service
            .list_entry_history(&db_path, &entry_a)
            .expect("list history after edit");

        assert_eq!(
            history[0].changed_fields,
            vec!["username".to_string()],
            "the newest snapshot diffs against the live Entry; only username changed"
        );
    }

    #[test]
    fn each_version_diffs_against_the_next_newer_version_not_the_live_entry() {
        // A multi-edit sequence: rename the username (alice → bob), then the
        // title (Entry A → Renamed). Each snapshot's `changed_fields` reflects
        // the *single* field that changed when it was superseded — proving the
        // diff walks neighbour-to-neighbour, not every snapshot against live.
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
        service
            .update_entry(
                &db_path,
                &entry_a,
                UpdateEntryData {
                    title: Some("Renamed".to_string()),
                    ..empty_update()
                },
            )
            .expect("rename title");

        let history = service
            .list_entry_history(&db_path, &entry_a)
            .expect("list history after two edits");

        assert_eq!(
            history[0].changed_fields,
            vec!["title".to_string()],
            "newest snapshot vs live Entry: only the title changed"
        );
        assert_eq!(
            history[1].changed_fields,
            vec!["username".to_string()],
            "older snapshot vs its newer neighbour: only the username changed"
        );
    }

    #[test]
    fn a_password_rotation_surfaces_the_name_password_with_no_value() {
        // The protected Password field is compared in-process (so a rotation is
        // detected), but `changed_fields` carries only the name `password`. The
        // DTO has no password field at all, so the secret structurally cannot
        // cross IPC — names only (ADR-0008).
        let (service, _dir, db_path, entry_a, _b) = create_test_database();

        service
            .update_entry(
                &db_path,
                &entry_a,
                UpdateEntryData {
                    password: Some(SecureString::from("rotated-secret")),
                    ..empty_update()
                },
            )
            .expect("rotate password");

        let history = service
            .list_entry_history(&db_path, &entry_a)
            .expect("list history after rotation");

        assert_eq!(
            history[0].changed_fields,
            vec!["password".to_string()],
            "a changed password surfaces by name only"
        );
    }

    #[test]
    fn a_tags_only_edit_surfaces_the_tags_attribute() {
        // Snapshots are captured for tag edits too (PRD #321), so a version
        // created by a tags-only change must carry a meaningful changed line.
        // Entry A starts untagged; applying a tag surfaces `tags`.
        let (service, _dir, db_path, entry_a, _b) = create_test_database();

        service
            .update_entry(
                &db_path,
                &entry_a,
                UpdateEntryData {
                    tags: Some(vec!["work".to_string()]),
                    ..empty_update()
                },
            )
            .expect("apply a tag");

        let history = service
            .list_entry_history(&db_path, &entry_a)
            .expect("list history after tag edit");

        assert_eq!(
            history[0].changed_fields,
            vec!["tags".to_string()],
            "a tags-only edit surfaces the `tags` attribute"
        );
    }

    #[test]
    fn an_icon_change_surfaces_the_icon_attribute() {
        // Entry A is seeded with builtin icon 0; switching it to icon 1 is a
        // content change that snapshots, and the diff names `icon`.
        let (service, _dir, db_path, entry_a, _b) = create_test_database();

        service
            .update_entry(
                &db_path,
                &entry_a,
                UpdateEntryData {
                    icon_id: Some(1),
                    ..empty_update()
                },
            )
            .expect("change icon");

        let history = service
            .list_entry_history(&db_path, &entry_a)
            .expect("list history after icon change");

        assert_eq!(
            history[0].changed_fields,
            vec!["icon".to_string()],
            "an icon change surfaces the `icon` attribute"
        );
    }

    #[test]
    fn enabling_expiry_surfaces_the_expiry_attribute() {
        // Entry A has no expiry; enabling it with a timestamp is a content
        // change that snapshots, and the diff names `expiry`.
        let (service, _dir, db_path, entry_a, _b) = create_test_database();

        service
            .update_entry(
                &db_path,
                &entry_a,
                UpdateEntryData {
                    expires: Some(true),
                    expiry_time: Some("2030-01-01T12:00:00+00:00".to_string()),
                    ..empty_update()
                },
            )
            .expect("enable expiry");

        let history = service
            .list_entry_history(&db_path, &entry_a)
            .expect("list history after enabling expiry");

        assert_eq!(
            history[0].changed_fields,
            vec!["expiry".to_string()],
            "enabling expiry surfaces the `expiry` attribute"
        );
    }

    #[test]
    fn deleting_an_attachment_surfaces_the_attachments_attribute() {
        // Attachment add/delete snapshots too (PRD #321). Seed an attachment,
        // then delete it: the captured version still holds the file while the
        // live Entry has none, so the diff names `attachments` — by name, never
        // the blob.
        let (service, _dir, db_path, entry_a, _b) = create_test_database();
        seed_attachment(&service, &db_path, &entry_a, "codes.txt", b"secret");

        service
            .delete_entry_attachment(&db_path, &entry_a, "codes.txt")
            .expect("delete attachment");

        let history = service
            .list_entry_history(&db_path, &entry_a)
            .expect("list history after attachment delete");

        assert_eq!(
            history[0].changed_fields,
            vec!["attachments".to_string()],
            "an attachment delete surfaces the `attachments` attribute"
        );
    }

    #[test]
    fn toggling_a_custom_field_protection_surfaces_its_name() {
        // Toggling a custom field between unprotected and protected without
        // changing its text still snapshots (the stored `Value` variant
        // changed), so the diff must report the field by name even though the
        // resolved plaintext is identical on both sides.
        let (service, _dir, db_path, entry_a, _b) = create_test_database();

        service
            .update_entry(
                &db_path,
                &entry_a,
                UpdateEntryData {
                    custom_fields: Some(BTreeMap::from([("API".to_string(), "xyz".to_string())])),
                    ..empty_update()
                },
            )
            .expect("add an unprotected custom field");
        service
            .update_entry(
                &db_path,
                &entry_a,
                UpdateEntryData {
                    protected_custom_fields: Some(BTreeMap::from([(
                        "API".to_string(),
                        SecureString::from("xyz"),
                    )])),
                    ..empty_update()
                },
            )
            .expect("re-store the same value as protected");

        let history = service
            .list_entry_history(&db_path, &entry_a)
            .expect("list history after protection toggle");

        assert_eq!(
            history[0].changed_fields,
            vec!["API".to_string()],
            "a protection toggle with unchanged text surfaces the field by name"
        );
    }

    #[test]
    fn moving_an_entry_between_groups_surfaces_the_location_attribute() {
        // A move between real Groups snapshots (#321/#323) but changes none of
        // the text fields, tags, icon, expiry, or attachments — only the
        // entry's location. The diff reports `location` so the version isn't a
        // blank "Changed:" row.
        let (service, _dir, db_path, entry_a, _b) = create_test_database();
        let target = service
            .create_group(&db_path, None, "Target", None)
            .expect("create target group");

        service
            .move_entry(&db_path, &entry_a, &target.id)
            .expect("move the entry");

        // The pre-move snapshot keeps the entry's creation-time
        // `location_changed`; the live entry's is bumped to the move instant.
        // In a fast test both land in the same second, so force them apart to
        // model the realistic create-then-move-later case.
        service
            .with_vault_mut(&db_path, |vault| {
                let mut entry = vault.entry_mut(&entry_a)?;
                entry.times.location_changed = chrono::NaiveDate::from_ymd_opt(2099, 1, 1)
                    .and_then(|d| d.and_hms_opt(0, 0, 0));
                Ok(())
            })
            .expect("distinguish the move timestamp");

        let history = service
            .list_entry_history(&db_path, &entry_a)
            .expect("list history after move");

        assert_eq!(
            history[0].changed_fields,
            vec!["location".to_string()],
            "a move between groups surfaces the `location` attribute"
        );
    }

    #[test]
    fn the_oldest_version_is_the_creation_when_its_timestamp_matches_creation() {
        // The original snapshot's `last_modification` is the Entry's creation
        // instant (it was never edited while live). So after editing, the oldest
        // kept version is flagged `is_creation` — the view labels it "Created".
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

        let history = service
            .list_entry_history(&db_path, &entry_a)
            .expect("list history after edit");

        let oldest = history.last().expect("a version exists");
        assert!(
            oldest.is_creation,
            "the original snapshot's timestamp matches the Entry's creation time"
        );
    }

    #[test]
    fn the_oldest_version_is_not_the_creation_after_the_original_was_evicted() {
        // Once retention prunes the original snapshot away, the oldest survivor
        // postdates the Entry's creation, so it is NOT flagged `is_creation` —
        // the view labels it "Earliest kept version". Pruning isn't enforced
        // yet, so simulate eviction by dropping the oldest (creation) snapshot.
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

        // Rebuild history keeping only the newest snapshot, mimicking eviction
        // of the original creation version. Stamp the survivor with a clearly
        // later timestamp, since a real evicted-original survivor postdates
        // creation (KDBX timestamps are second-resolution, so same-second test
        // edits would otherwise all collide with the creation instant).
        service
            .with_vault_mut(&db_path, |vault| {
                let mut entry = vault.entry_mut(&entry_a)?;
                if let Some(history) = entry.history.as_mut() {
                    let mut newest = history.get_entries().first().cloned();
                    if let Some(snapshot) = newest.as_mut() {
                        snapshot.times.last_modification =
                            chrono::NaiveDate::from_ymd_opt(2099, 1, 1)
                                .and_then(|d| d.and_hms_opt(0, 0, 0));
                    }
                    let mut rebuilt = keepass::db::History::default();
                    if let Some(snapshot) = newest {
                        rebuilt.add_entry(snapshot);
                    }
                    entry.history = Some(rebuilt);
                }
                Ok(())
            })
            .expect("evict the original snapshot");

        let history = service
            .list_entry_history(&db_path, &entry_a)
            .expect("list history after eviction");

        assert_eq!(history.len(), 1, "only the newest snapshot survives");
        let oldest = history.last().expect("a version exists");
        assert!(
            !oldest.is_creation,
            "an earliest-kept survivor postdates creation and is not the original"
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

        let reopened = save_close_reopen(&service, &db_path);

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

    /// Applies a single username edit — one content change, so one snapshot
    /// under any non-disabled limit.
    fn edit_username(service: &KdbxService, db_path: &str, entry_id: &str, username: &str) {
        service
            .update_entry(
                db_path,
                entry_id,
                UpdateEntryData {
                    username: Some(username.to_string()),
                    ..empty_update()
                },
            )
            .expect("edit username");
    }

    /// Renames the Entry's username `count` times, yielding `count` history
    /// snapshots (each edit changes content, so each is kept). Returns nothing;
    /// callers inspect via [`history_len`].
    fn edit_username_n_times(service: &KdbxService, db_path: &str, entry_id: &str, count: usize) {
        for i in 0..count {
            edit_username(service, db_path, entry_id, &format!("user{i}"));
        }
    }

    /// The number of history versions currently recorded for an Entry.
    fn history_len(service: &KdbxService, db_path: &str, entry_id: &str) -> usize {
        service
            .list_entry_history(db_path, entry_id)
            .expect("list history")
            .len()
    }

    #[test]
    fn positive_limit_prunes_oldest_keeping_newest_n() {
        // With a positive History Limit, appending a snapshot beyond the cap
        // prunes the oldest, keeping exactly the newest N (ADR-0008).
        let (service, _dir, db_path, entry_a, _b) = create_test_database();
        service
            .update_vault_history_settings(&db_path, Some(3))
            .expect("set limit to 3");

        // Seed username is "alice"; five edits would otherwise yield five
        // snapshots (pre-images: alice, user0, user1, user2, user3).
        edit_username_n_times(&service, &db_path, &entry_a, 5);

        let history = service
            .list_entry_history(&db_path, &entry_a)
            .expect("list history");
        assert_eq!(history.len(), 3, "history is capped at the limit of 3");
        assert_eq!(
            history[0].username, "user3",
            "newest kept snapshot is the most recent pre-image"
        );
        assert_eq!(
            history[2].username, "user1",
            "oldest kept snapshot; 'alice' and 'user0' were pruned"
        );
    }

    #[test]
    fn negative_limit_lets_history_grow_unbounded() {
        // A negative History Limit means unlimited: every edit accrues a
        // version, past the default cap of 10.
        let (service, _dir, db_path, entry_a, _b) = create_test_database();
        service
            .update_vault_history_settings(&db_path, Some(-1))
            .expect("set unlimited");

        edit_username_n_times(&service, &db_path, &entry_a, 12);

        assert_eq!(
            history_len(&service, &db_path, &entry_a),
            12,
            "unlimited history is never pruned"
        );
    }

    #[test]
    fn absent_limit_caps_history_at_the_default_of_ten() {
        // A brand-new Vault never sets the field; history must still be capped
        // at the default of 10, NOT left unbounded (ADR-0008).
        let (service, _dir, db_path, entry_a, _b) = create_test_database();

        edit_username_n_times(&service, &db_path, &entry_a, 15);

        assert_eq!(
            history_len(&service, &db_path, &entry_a),
            10,
            "absent History Limit resolves to the bounded default of 10"
        );
    }

    #[test]
    fn disabled_limit_adds_no_snapshot_and_prunes_existing_lazily() {
        // `0` disables history: no new snapshots, and existing history is pruned
        // to zero lazily on the Entry's *next* edit — not wiped the instant the
        // limit is set.
        let (service, _dir, db_path, entry_a, _b) = create_test_database();

        // Accrue some history under the default limit first.
        edit_username_n_times(&service, &db_path, &entry_a, 3);
        assert_eq!(
            history_len(&service, &db_path, &entry_a),
            3,
            "precondition: three versions exist"
        );

        // Disabling must not wipe the existing history instantly.
        service
            .update_vault_history_settings(&db_path, Some(0))
            .expect("disable history");
        assert_eq!(
            history_len(&service, &db_path, &entry_a),
            3,
            "setting the limit to 0 does not wipe history instantly"
        );

        // The next content edit adds no snapshot and prunes existing history to zero.
        edit_username(&service, &db_path, &entry_a, "after-disable");
        assert_eq!(
            history_len(&service, &db_path, &entry_a),
            0,
            "disabled history is pruned to zero on the next content edit"
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

    /// Reads a historical version's attachment bytes by reaching through the
    /// live Entry's native KDBX history. Returns `None` when that version does
    /// not carry the named attachment. The listing DTO carries no attachment
    /// bytes, so retention assertions inspect the version directly.
    fn historical_attachment_bytes(
        service: &KdbxService,
        db_path: &str,
        entry_id: &str,
        index: usize,
        filename: &str,
    ) -> Option<Vec<u8>> {
        service
            .with_vault(db_path, |vault| {
                let entry = vault.find_entry(entry_id)?;
                Ok(entry
                    .historical(index)
                    .and_then(|h| h.attachment_by_name(filename).map(|a| a.data.get().clone())))
            })
            .expect("read historical attachment")
    }

    /// Persists the vault, drops it, and reopens it with a *fresh* service so a
    /// test can assert on-disk durability with nothing cached in memory. Returns
    /// the reopened service.
    fn save_close_reopen(service: &KdbxService, db_path: &str) -> KdbxService {
        service.save(db_path).expect("save vault");
        service.close(db_path).expect("close vault");
        let reopened = KdbxService::new();
        reopened.open(db_path, "testpass").expect("reopen vault");
        reopened
    }

    #[test]
    fn attachment_blob_is_retrievable_from_history_after_delete_and_reopen() {
        // The interop proof point (#332): after deleting an Attachment, the
        // captured version's binary must survive being written to disk and read
        // back with a *fresh* service — the blob is not GC'd at delete time and
        // the on-disk pool round-trips it.
        let (service, _dir, db_path, entry_a, _b) = create_test_database();
        seed_attachment(&service, &db_path, &entry_a, "codes.txt", b"secret");

        service
            .delete_entry_attachment(&db_path, &entry_a, "codes.txt")
            .expect("delete attachment");

        let reopened = save_close_reopen(&service, &db_path);

        // The live Entry still has no attachment after the round-trip...
        let entry = reopened.get_entry(&db_path, &entry_a).expect("get entry");
        assert!(
            entry.attachments.is_empty(),
            "the live Entry must not regain the deleted attachment on reopen"
        );

        // ...the version is still listed...
        let history = reopened
            .list_entry_history(&db_path, &entry_a)
            .expect("list history after reopen");
        assert_eq!(
            history.len(),
            1,
            "the pre-delete version must survive the on-disk round-trip"
        );

        // ...and its blob is still retrievable verbatim.
        let bytes = historical_attachment_bytes(&reopened, &db_path, &entry_a, 0, "codes.txt");
        assert_eq!(
            bytes.as_deref(),
            Some(b"secret".as_slice()),
            "the deleted attachment's bytes must round-trip through history"
        );
    }

    #[test]
    fn a_blob_from_an_earlier_snapshot_survives_a_later_attachment_delete() {
        // Broadened retention (#332): the stock binary-pool GC was keyed by entry
        // id only, so deleting an attachment could reclaim a blob that an
        // *earlier* snapshot (from any trigger) still referenced. Here a
        // field-edit snapshot captures the attachment-bearing state; a subsequent
        // delete must not strip that earlier version's blob, across save/reopen.
        let (service, _dir, db_path, entry_a, _b) = create_test_database();
        seed_attachment(&service, &db_path, &entry_a, "codes.txt", b"secret");

        // Field edit snapshots the attachment-bearing state at index 0.
        service
            .update_entry(
                &db_path,
                &entry_a,
                UpdateEntryData {
                    title: Some("renamed".to_string()),
                    ..empty_update()
                },
            )
            .expect("field edit");

        // Deleting the attachment pushes a second snapshot (now at index 0) and
        // drops the live reference; the earlier field-edit version (index 1)
        // still references the blob.
        service
            .delete_entry_attachment(&db_path, &entry_a, "codes.txt")
            .expect("delete attachment");

        let reopened = save_close_reopen(&service, &db_path);

        let history = reopened
            .list_entry_history(&db_path, &entry_a)
            .expect("list history after reopen");
        assert_eq!(
            history.len(),
            2,
            "both the field-edit and the delete snapshots must survive reopen"
        );

        // Both the delete snapshot (index 0) and the earlier field-edit snapshot
        // (index 1) must still resolve the original bytes — neither was aliased
        // or GC'd by the delete.
        for index in [0, 1] {
            let bytes =
                historical_attachment_bytes(&reopened, &db_path, &entry_a, index, "codes.txt");
            assert_eq!(
                bytes.as_deref(),
                Some(b"secret".as_slice()),
                "history version {index} must retain the original attachment bytes"
            );
        }
    }

    #[test]
    fn history_listing_exposes_a_versions_attachment_filenames_across_reopen() {
        // The compare view names which attachments a version carried (#356), so
        // the listing DTO must surface each snapshot's attachment filenames
        // (names only — never bytes) and they must survive the on-disk
        // round-trip. Seeding then deleting an attachment captures a pre-delete
        // version that still references it.
        let (service, _dir, db_path, entry_a, _b) = create_test_database();
        seed_attachment(&service, &db_path, &entry_a, "codes.txt", b"secret");

        service
            .delete_entry_attachment(&db_path, &entry_a, "codes.txt")
            .expect("delete attachment");

        let reopened = save_close_reopen(&service, &db_path);

        let history = reopened
            .list_entry_history(&db_path, &entry_a)
            .expect("list history after reopen");
        let version = history.first().expect("one pre-delete version");
        assert_eq!(
            version.attachment_names,
            vec!["codes.txt".to_string()],
            "the version's attachment filename must round-trip into the listing"
        );
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
    fn disabled_history_survives_trash_and_restore_then_prunes_on_next_content_edit() {
        // Disabling history must not destroy existing versions through a
        // reversible trash/restore round-trip — only a genuine content edit
        // prunes them to zero. This pins the "next *content* edit" invariant:
        // Recycle-Bin transitions are excluded from the snapshot chokepoint
        // (#323 / ADR-0008), so they must neither snapshot nor prune.
        let (service, _dir, db_path, entry_a, _b) = create_test_database();
        let root = service.get_info(&db_path).expect("info").root_group_id;

        // Accrue history under the default limit, then disable.
        edit_username_n_times(&service, &db_path, &entry_a, 3);
        service
            .update_vault_history_settings(&db_path, Some(0))
            .expect("disable history");

        // Trash then restore: a reversible location round-trip must preserve
        // the (already-frozen) history untouched.
        service
            .delete_entry(&db_path, &entry_a)
            .expect("send to recycle bin");
        service
            .move_entry(&db_path, &entry_a, &root)
            .expect("restore from recycle bin");
        assert_eq!(
            history_len(&service, &db_path, &entry_a),
            3,
            "history survives trash/restore while disabled"
        );

        // The next genuine content edit prunes the disabled history to zero.
        edit_username(&service, &db_path, &entry_a, "after-restore");
        assert_eq!(
            history_len(&service, &db_path, &entry_a),
            0,
            "the next content edit prunes disabled history to zero"
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
