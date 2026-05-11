use crate::dto::entry::{CreateEntryData, CustomFieldValue, Entry, UpdateEntryData};
use crate::dto::error::AppError;
use keepass::db::{Entry as KeepassEntry, Times, Value};
use keepass::Database;

use super::mapping::{
    apply_custom_fields, convert_entry, ensure_recycle_bin, find_entry_id, find_group_by_id,
    find_group_id, is_standard_entry_field, replace_custom_fields,
};
use super::KdbxService;

impl KdbxService {
    /// Lists entries, optionally filtered by group.
    pub fn list_entries(
        &self,
        db_id: &str,
        group_id: Option<&str>,
    ) -> Result<Vec<Entry>, AppError> {
        let normalized_path = Self::normalize_path(db_id);
        let databases = self.lock_databases()?;
        let open_db = databases
            .get(&normalized_path)
            .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;
        let db = open_db.db_or_locked()?;

        let mut entries = Vec::new();

        if let Some(gid) = group_id {
            let group = find_group_by_id(db, gid)
                .ok_or_else(|| AppError::GroupNotFound(gid.to_string()))?;
            let group_uuid = group.id().uuid().to_string();
            for entry in group.entries() {
                entries.push(convert_entry(&entry, &group_uuid));
            }
        } else {
            for entry in db.iter_all_entries() {
                let group_uuid = entry.parent().id().uuid().to_string();
                entries.push(convert_entry(&entry, &group_uuid));
            }
        }

        Ok(entries)
    }

    /// Fetches an entry by ID.
    pub fn get_entry(&self, db_id: &str, id: &str) -> Result<Entry, AppError> {
        let normalized_path = Self::normalize_path(db_id);
        let databases = self.lock_databases()?;
        let open_db = databases
            .get(&normalized_path)
            .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;
        let db = open_db.db_or_locked()?;

        let eid = find_entry_id(db, id).ok_or_else(|| AppError::EntryNotFound(id.to_string()))?;
        let entry = db
            .entry(eid)
            .ok_or_else(|| AppError::EntryNotFound(id.to_string()))?;
        let group_uuid = entry.parent().id().uuid().to_string();
        Ok(convert_entry(&entry, &group_uuid))
    }

    /// Fetches an entry password.
    pub fn get_entry_password(&self, db_id: &str, id: &str) -> Result<String, AppError> {
        let normalized_path = Self::normalize_path(db_id);
        let databases = self.lock_databases()?;
        let open_db = databases
            .get(&normalized_path)
            .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;
        let db = open_db.db_or_locked()?;

        let eid = find_entry_id(db, id).ok_or_else(|| AppError::EntryNotFound(id.to_string()))?;
        let entry = db
            .entry(eid)
            .ok_or_else(|| AppError::EntryNotFound(id.to_string()))?;
        Ok(entry
            .get_password()
            .map(std::string::ToString::to_string)
            .unwrap_or_default())
    }

    /// Fetches a protected custom field value.
    pub fn get_entry_protected_custom_field(
        &self,
        db_id: &str,
        entry_id: &str,
        key: &str,
    ) -> Result<CustomFieldValue, AppError> {
        let normalized_path = Self::normalize_path(db_id);
        let databases = self.lock_databases()?;
        let open_db = databases
            .get(&normalized_path)
            .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;
        let db = open_db.db_or_locked()?;

        if is_standard_entry_field(key) {
            return Err(AppError::CustomFieldNotFound(key.to_string()));
        }

        let eid = find_entry_id(db, entry_id)
            .ok_or_else(|| AppError::EntryNotFound(entry_id.to_string()))?;
        let entry = db
            .entry(eid)
            .ok_or_else(|| AppError::EntryNotFound(entry_id.to_string()))?;

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
    }

    /// Creates a new entry in a group.
    #[allow(clippy::needless_pass_by_value)]
    pub fn create_entry(
        &self,
        db_id: &str,
        group_id: &str,
        data: CreateEntryData,
    ) -> Result<Entry, AppError> {
        let normalized_path = Self::normalize_path(db_id);
        let mut databases = self.lock_databases()?;
        let open_db = databases
            .get_mut(&normalized_path)
            .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;
        let db = open_db.db_mut_or_locked()?;

        let gid = find_group_id(db, group_id)
            .ok_or_else(|| AppError::GroupNotFound(group_id.to_string()))?;

        let new_eid = {
            let mut group = db
                .group_mut(gid)
                .ok_or_else(|| AppError::GroupNotFound(group_id.to_string()))?;
            let mut entry = group.add_entry();
            populate_entry(&mut entry, &data);
            entry.id()
        };

        let entry_ref = db
            .entry(new_eid)
            .ok_or_else(|| AppError::EntryNotFound(new_eid.uuid().to_string()))?;
        let entry_model = convert_entry(&entry_ref, group_id);
        open_db.is_modified = true;

        Ok(entry_model)
    }

    /// Updates an existing entry.
    pub fn update_entry(
        &self,
        db_id: &str,
        id: &str,
        data: UpdateEntryData,
    ) -> Result<Entry, AppError> {
        let normalized_path = Self::normalize_path(db_id);
        let mut databases = self.lock_databases()?;
        let open_db = databases
            .get_mut(&normalized_path)
            .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;

        let db = open_db.db_mut_or_locked()?;
        let eid = find_entry_id(db, id).ok_or_else(|| AppError::EntryNotFound(id.to_string()))?;

        let group_uuid = {
            let mut entry = db
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

        let entry_ref = db
            .entry(eid)
            .ok_or_else(|| AppError::EntryNotFound(id.to_string()))?;
        let result = convert_entry(&entry_ref, &group_uuid);
        open_db.is_modified = true;

        Ok(result)
    }

    /// Deletes an entry by moving it to recycle bin.
    pub fn delete_entry(&self, db_id: &str, id: &str) -> Result<(), AppError> {
        let normalized_path = Self::normalize_path(db_id);
        let mut databases = self.lock_databases()?;
        let open_db = databases
            .get_mut(&normalized_path)
            .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;

        let db = open_db.db_mut_or_locked()?;
        let eid = find_entry_id(db, id).ok_or_else(|| AppError::EntryNotFound(id.to_string()))?;

        let recycle_uuid = ensure_recycle_bin(db);
        let recycle_gid = find_group_id(db, &recycle_uuid)
            .ok_or_else(|| AppError::GroupNotFound(recycle_uuid.clone()))?;

        let now = Times::now();
        {
            let mut entry = db
                .entry_mut(eid)
                .ok_or_else(|| AppError::EntryNotFound(id.to_string()))?;
            entry.times.last_modification = Some(now);
            entry.times.location_changed = Some(now);
            entry
                .move_to(recycle_gid)
                .map_err(|e| AppError::Kdbx(e.to_string()))?;
        }

        open_db.is_modified = true;
        Ok(())
    }

    /// Renames a tag across all entries in the database.
    /// Returns the number of entries that were modified.
    pub fn rename_tag(&self, db_id: &str, old_name: &str, new_name: &str) -> Result<u32, AppError> {
        let normalized_path = Self::normalize_path(db_id);
        let mut databases = self.lock_databases()?;
        let open_db = databases
            .get_mut(&normalized_path)
            .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;

        if old_name == new_name {
            return Ok(0);
        }

        let db = open_db.db_mut_or_locked()?;
        let count = modify_all_entries(db, &|entry| rename_tag_in_entry(entry, old_name, new_name));

        if count > 0 {
            open_db.is_modified = true;
        }

        Ok(count)
    }

    /// Deletes a tag from all entries in the database.
    /// Returns the number of entries that were modified.
    pub fn delete_tag(&self, db_id: &str, tag_name: &str) -> Result<u32, AppError> {
        let normalized_path = Self::normalize_path(db_id);
        let mut databases = self.lock_databases()?;
        let open_db = databases
            .get_mut(&normalized_path)
            .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;

        let db = open_db.db_mut_or_locked()?;
        let count = modify_all_entries(db, &|entry| delete_tag_in_entry(entry, tag_name));

        if count > 0 {
            open_db.is_modified = true;
        }

        Ok(count)
    }

    /// Moves an entry to another group.
    pub fn move_entry(
        &self,
        db_id: &str,
        id: &str,
        target_group_id: &str,
    ) -> Result<Entry, AppError> {
        let normalized_path = Self::normalize_path(db_id);
        let mut databases = self.lock_databases()?;
        let open_db = databases
            .get_mut(&normalized_path)
            .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;

        let db = open_db.db_mut_or_locked()?;
        let eid = find_entry_id(db, id).ok_or_else(|| AppError::EntryNotFound(id.to_string()))?;
        let target_gid = find_group_id(db, target_group_id)
            .ok_or_else(|| AppError::GroupNotFound(target_group_id.to_string()))?;

        let now = Times::now();
        {
            let mut entry = db
                .entry_mut(eid)
                .ok_or_else(|| AppError::EntryNotFound(id.to_string()))?;
            entry.times.last_modification = Some(now);
            entry.times.location_changed = Some(now);
            entry
                .move_to(target_gid)
                .map_err(|e| AppError::Kdbx(e.to_string()))?;
        }

        let entry_ref = db
            .entry(eid)
            .ok_or_else(|| AppError::EntryNotFound(id.to_string()))?;
        let entry_model = convert_entry(&entry_ref, target_group_id);
        open_db.is_modified = true;

        Ok(entry_model)
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

fn modify_all_entries(db: &mut Database, modify_fn: &dyn Fn(&mut KeepassEntry) -> bool) -> u32 {
    let mut count = 0u32;
    db.foreach_entry_mut(|mut entry| {
        if modify_fn(&mut entry) {
            entry.times.last_modification = Some(Times::now());
            count += 1;
        }
    });
    count
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
