use crate::dto::entry::{CreateEntryData, CustomFieldValue, Entry, UpdateEntryData};
use crate::dto::error::AppError;
use keepass::db::{Entry as KeepassEntry, Node, Times, Value};
use secstr::SecStr;

use super::mapping::{
    apply_custom_fields, convert_entry, ensure_recycle_bin, find_group_by_id, find_group_by_id_mut,
    is_standard_entry_field, replace_custom_fields,
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
            let group = find_group_by_id(&db.root, gid)
                .ok_or_else(|| AppError::GroupNotFound(gid.to_string()))?;
            collect_entries_from_group(group, &mut entries);
        } else {
            collect_all_entries(&db.root, &mut entries);
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

        find_entry_by_id(&db.root, id).ok_or_else(|| AppError::EntryNotFound(id.to_string()))
    }

    /// Fetches an entry password.
    pub fn get_entry_password(&self, db_id: &str, id: &str) -> Result<String, AppError> {
        let normalized_path = Self::normalize_path(db_id);
        let databases = self.lock_databases()?;
        let open_db = databases
            .get(&normalized_path)
            .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;
        let db = open_db.db_or_locked()?;

        match find_entry_password(&db.root, id) {
            PasswordSearchResult::Found(password) => Ok(password),
            PasswordSearchResult::NotFound => Err(AppError::EntryNotFound(id.to_string())),
        }
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

        let entry = find_entry_by_id_ref(&db.root, entry_id)
            .ok_or_else(|| AppError::EntryNotFound(entry_id.to_string()))?;

        if is_standard_entry_field(key) {
            return Err(AppError::CustomFieldNotFound(key.to_string()));
        }

        let value = entry
            .fields
            .get(key)
            .ok_or_else(|| AppError::CustomFieldNotFound(key.to_string()))?;

        match value {
            Value::Protected(secret) => Ok(CustomFieldValue {
                key: key.to_string(),
                value: String::from_utf8_lossy(secret.unsecure()).to_string(),
            }),
            _ => Err(AppError::CustomFieldNotProtected(key.to_string())),
        }
    }

    /// Creates a new entry in a group.
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

        let group = find_group_by_id_mut(&mut db.root, group_id)
            .ok_or_else(|| AppError::GroupNotFound(group_id.to_string()))?;

        let mut entry = KeepassEntry::new();
        entry
            .fields
            .insert("Title".to_string(), Value::Unprotected(data.title));
        entry
            .fields
            .insert("UserName".to_string(), Value::Unprotected(data.username));
        entry.fields.insert(
            "Password".to_string(),
            Value::Protected(SecStr::new(data.password.as_str().as_bytes().to_vec())),
        );

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
            entry.icon_id = Some(icon_id as usize);
        }
        if let Some(tags) = data.tags {
            entry.tags = tags;
        }
        apply_custom_fields(
            &mut entry,
            data.custom_fields.as_ref(),
            data.protected_custom_fields.as_ref(),
        );

        let entry_model = convert_entry(&entry, group_id);
        group.add_child(entry);
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
        let (entry, group_id) = find_entry_by_id_mut(&mut db.root, id)
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
                Value::Protected(SecStr::new(password.as_str().as_bytes().to_vec())),
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
            entry.icon_id = Some(icon_id as usize);
        }
        if let Some(tags) = data.tags {
            entry.tags = tags;
        }
        if data.custom_fields.is_some() || data.protected_custom_fields.is_some() {
            replace_custom_fields(
                entry,
                data.custom_fields.as_ref(),
                data.protected_custom_fields.as_ref(),
            );
        }

        entry.times.set_last_modification(Times::now());
        let result = convert_entry(entry, &group_id);
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

        let mut entry = remove_entry_by_id(&mut db.root, id)
            .ok_or_else(|| AppError::EntryNotFound(id.to_string()))?;

        let recycle_bin_id = ensure_recycle_bin(db);
        let recycle_bin = find_group_by_id_mut(&mut db.root, &recycle_bin_id)
            .ok_or_else(|| AppError::GroupNotFound(recycle_bin_id.clone()))?;

        let now = Times::now();
        entry.times.set_last_modification(now);
        entry.times.set_location_changed(now);
        recycle_bin.add_child(entry);

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
        let count = modify_tags_in_group(&mut db.root, &|entry| {
            rename_tag_in_entry(entry, old_name, new_name)
        });

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
        let count =
            modify_tags_in_group(&mut db.root, &|entry| delete_tag_in_entry(entry, tag_name));

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

        let mut entry = remove_entry_by_id(&mut db.root, id)
            .ok_or_else(|| AppError::EntryNotFound(id.to_string()))?;

        let target_group = find_group_by_id_mut(&mut db.root, target_group_id)
            .ok_or_else(|| AppError::GroupNotFound(target_group_id.to_string()))?;

        let now = Times::now();
        entry.times.set_last_modification(now);
        entry.times.set_location_changed(now);

        let entry_model = convert_entry(&entry, target_group_id);
        target_group.add_child(entry);
        open_db.is_modified = true;

        Ok(entry_model)
    }
}

fn find_entry_by_id(group: &keepass::db::Group, id: &str) -> Option<Entry> {
    for node in &group.children {
        match node {
            Node::Entry(entry) => {
                if entry.uuid.to_string() == id {
                    return Some(convert_entry(entry, &group.uuid.to_string()));
                }
            }
            Node::Group(child) => {
                if let Some(found) = find_entry_by_id(child, id) {
                    return Some(found);
                }
            }
        }
    }
    None
}

fn find_entry_by_id_ref<'a>(group: &'a keepass::db::Group, id: &str) -> Option<&'a KeepassEntry> {
    for node in &group.children {
        match node {
            Node::Entry(entry) => {
                if entry.uuid.to_string() == id {
                    return Some(entry);
                }
            }
            Node::Group(child) => {
                if let Some(found) = find_entry_by_id_ref(child, id) {
                    return Some(found);
                }
            }
        }
    }
    None
}

fn find_entry_by_id_mut<'a>(
    group: &'a mut keepass::db::Group,
    id: &str,
) -> Option<(&'a mut KeepassEntry, String)> {
    let group_id = group.uuid.to_string();

    for node in &mut group.children {
        match node {
            Node::Entry(entry) => {
                if entry.uuid.to_string() == id {
                    return Some((entry, group_id));
                }
            }
            Node::Group(child) => {
                if let Some(found) = find_entry_by_id_mut(child, id) {
                    return Some(found);
                }
            }
        }
    }

    None
}

enum PasswordSearchResult {
    NotFound,
    Found(String),
}

fn find_entry_password(group: &keepass::db::Group, id: &str) -> PasswordSearchResult {
    for node in &group.children {
        match node {
            Node::Entry(entry) => {
                if entry.uuid.to_string() == id {
                    let password = entry
                        .get_password()
                        .map(std::string::ToString::to_string)
                        .unwrap_or_default();
                    return PasswordSearchResult::Found(password);
                }
            }
            Node::Group(child) => {
                if let PasswordSearchResult::Found(pw) = find_entry_password(child, id) {
                    return PasswordSearchResult::Found(pw);
                }
            }
        }
    }
    PasswordSearchResult::NotFound
}

fn collect_entries_from_group(group: &keepass::db::Group, entries: &mut Vec<Entry>) {
    let group_id = group.uuid.to_string();
    for node in &group.children {
        if let Node::Entry(entry) = node {
            entries.push(convert_entry(entry, &group_id));
        }
    }
}

fn collect_all_entries(group: &keepass::db::Group, entries: &mut Vec<Entry>) {
    let group_id = group.uuid.to_string();
    for node in &group.children {
        match node {
            Node::Entry(entry) => {
                entries.push(convert_entry(entry, &group_id));
            }
            Node::Group(child) => {
                collect_all_entries(child, entries);
            }
        }
    }
}

fn modify_tags_in_group(
    group: &mut keepass::db::Group,
    modify_fn: &dyn Fn(&mut KeepassEntry) -> bool,
) -> u32 {
    let mut count = 0u32;
    for node in &mut group.children {
        match node {
            Node::Entry(entry) => {
                if modify_fn(entry) {
                    entry.times.set_last_modification(Times::now());
                    count += 1;
                }
            }
            Node::Group(child) => {
                count += modify_tags_in_group(child, modify_fn);
            }
        }
    }
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
        Value::Protected(secret) => {
            let mut text = String::from_utf8_lossy(secret.unsecure()).to_string();
            if !rename_tag_in_tag_text(&mut text, old_name, new_name) {
                return false;
            }
            *secret = SecStr::new(text.into_bytes());
            true
        }
        Value::Bytes(_) => false,
    }
}

fn delete_tag_in_custom_field(entry: &mut KeepassEntry, key: &str, tag_name: &str) -> bool {
    let Some(value) = entry.fields.get_mut(key) else {
        return false;
    };

    match value {
        Value::Unprotected(text) => delete_tag_in_tag_text(text, tag_name),
        Value::Protected(secret) => {
            let mut text = String::from_utf8_lossy(secret.unsecure()).to_string();
            if !delete_tag_in_tag_text(&mut text, tag_name) {
                return false;
            }
            *secret = SecStr::new(text.into_bytes());
            true
        }
        Value::Bytes(_) => false,
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

fn remove_entry_by_id(group: &mut keepass::db::Group, id: &str) -> Option<KeepassEntry> {
    let mut index = 0;
    while index < group.children.len() {
        match &mut group.children[index] {
            Node::Entry(entry) => {
                if entry.uuid.to_string() == id {
                    return match group.children.remove(index) {
                        Node::Entry(removed) => Some(removed),
                        Node::Group(_) => None,
                    };
                }
                index += 1;
            }
            Node::Group(child) => {
                if let Some(found) = remove_entry_by_id(child, id) {
                    return Some(found);
                }
                index += 1;
            }
        }
    }
    None
}
