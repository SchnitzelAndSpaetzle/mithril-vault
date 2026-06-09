use crate::domain::secure::SecureBytes;
use crate::dto::entry::{CreateEntryData, CustomFieldValue, Entry, UpdateEntryData};
use crate::dto::error::AppError;
use keepass::db::{Entry as KeepassEntry, Times, Value};

use super::conversions::{
    apply_custom_fields, apply_expiry, convert_entry, is_standard_entry_field, parse_expiry_time,
    replace_custom_fields, validate_expiry_enabled,
};
use super::recycle::is_in_recycle_bin;
use super::KdbxService;

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
        std::fs::write(dest, bytes.as_bytes())?;
        Ok(())
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

            let count =
                vault.modify_all_entries(&|entry| rename_tag_in_entry(entry, old_name, new_name));

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
            let count = vault.modify_all_entries(&|entry| delete_tag_in_entry(entry, tag_name));

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

            let now = Times::now();
            {
                let mut entry = vault
                    .db_mut()
                    .entry_mut(eid)
                    .ok_or_else(|| AppError::EntryNotFound(id.to_string()))?;
                entry.times.last_modification = Some(now);
                entry.times.location_changed = Some(now);
                entry
                    .move_to(target_gid)
                    .map_err(|e| AppError::Kdbx(e.to_string()))?;
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

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::domain::secure::SecureString;
    use crate::dto::entry::AttachmentMeta;
    use crate::services::kdbx::test_support::create_test_database;
    use keepass::db::Value;
    use std::collections::HashMap;

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
}
