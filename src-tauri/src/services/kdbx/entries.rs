use crate::dto::entry::{CreateEntryData, CustomFieldValue, Entry, UpdateEntryData};
use crate::dto::error::AppError;
use keepass::db::{Entry as KeepassEntry, Times, Value};

use super::conversions::{
    apply_custom_fields, convert_entry, is_standard_entry_field, replace_custom_fields,
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
            let new_eid = {
                let mut group = vault.group_mut(group_id)?;
                let mut entry = group.add_entry();
                populate_entry(&mut entry, &data);
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

            let group_uuid = {
                let mut entry = vault
                    .db_mut()
                    .entry_mut(eid)
                    .ok_or_else(|| AppError::EntryNotFound(id.to_string()))?;

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
    use crate::dto::database::DatabaseCreationOptions;
    use tempfile::TempDir;

    fn create_test_database() -> (KdbxService, TempDir, String, String, String) {
        let dir = tempfile::tempdir().expect("create temp dir");
        let db_path = dir.path().join("entries-tests.kdbx");
        let db_path_str = db_path.to_string_lossy().to_string();

        let options = DatabaseCreationOptions {
            create_default_groups: true,
            kdf_memory: Some(1024 * 1024),
            kdf_iterations: Some(1),
            kdf_parallelism: Some(1),
            description: None,
        };

        let service = KdbxService::new();
        service
            .create_database(
                &db_path_str,
                Some("testpass"),
                None,
                "Entries Tests",
                &options,
            )
            .expect("create db");
        let info = service.get_info(&db_path_str).expect("database info");

        let entry_a = service
            .create_entry(
                &db_path_str,
                &info.root_group_id,
                CreateEntryData {
                    title: "Entry A".to_string(),
                    username: "alice".to_string(),
                    password: SecureString::from("secret"),
                    url: None,
                    notes: None,
                    icon_id: Some(0),
                    tags: None,
                    custom_fields: None,
                    protected_custom_fields: None,
                },
            )
            .expect("create entry A");
        let entry_b = service
            .create_entry(
                &db_path_str,
                &info.root_group_id,
                CreateEntryData {
                    title: "Entry B".to_string(),
                    username: "bob".to_string(),
                    password: SecureString::from("secret"),
                    url: None,
                    notes: None,
                    icon_id: Some(0),
                    tags: None,
                    custom_fields: None,
                    protected_custom_fields: None,
                },
            )
            .expect("create entry B");

        (service, dir, db_path_str, entry_a.id, entry_b.id)
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
