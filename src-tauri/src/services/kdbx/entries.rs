use crate::dto::entry::{CreateEntryData, CustomFieldValue, Entry, UpdateEntryData};
use crate::dto::error::AppError;
use keepass::db::{Entry as KeepassEntry, Node, Times, Value};
use keepass::Database;
use secstr::SecStr;

use super::mapping::{
    apply_custom_fields, convert_entry, find_group_by_id, find_group_by_name,
    is_standard_entry_field, replace_custom_fields,
};
use super::KdbxService;

impl KdbxService {
    pub fn list_entries(&self, group_id: Option<&str>) -> Result<Vec<Entry>, AppError> {
        let db_lock = self.database.lock().map_err(|_| AppError::Lock)?;
        let open_db = db_lock.as_ref().ok_or(AppError::DatabaseNotOpen)?;

        let mut entries = Vec::new();

        if let Some(gid) = group_id {
            let group = find_group_by_id(&open_db.db.root, gid)
                .ok_or_else(|| AppError::GroupNotFound(gid.to_string()))?;
            collect_entries_from_group(group, &mut entries);
        } else {
            collect_all_entries(&open_db.db.root, &mut entries);
        }

        Ok(entries)
    }

    pub fn get_entry(&self, id: &str) -> Result<Entry, AppError> {
        let db_lock = self.database.lock().map_err(|_| AppError::Lock)?;
        let open_db = db_lock.as_ref().ok_or(AppError::DatabaseNotOpen)?;

        find_entry_by_id(&open_db.db.root, id)
            .ok_or_else(|| AppError::EntryNotFound(id.to_string()))
    }

    pub fn get_entry_password(&self, id: &str) -> Result<String, AppError> {
        let db_lock = self.database.lock().map_err(|_| AppError::Lock)?;
        let open_db = db_lock.as_ref().ok_or(AppError::DatabaseNotOpen)?;

        match find_entry_password(&open_db.db.root, id) {
            PasswordSearchResult::Found(password) => Ok(password),
            PasswordSearchResult::NotFound => Err(AppError::EntryNotFound(id.to_string())),
        }
    }

    pub fn get_entry_protected_custom_field(
        &self,
        entry_id: &str,
        key: &str,
    ) -> Result<CustomFieldValue, AppError> {
        let db_lock = self.database.lock().map_err(|_| AppError::Lock)?;
        let open_db = db_lock.as_ref().ok_or(AppError::DatabaseNotOpen)?;

        let entry = find_entry_by_id_ref(&open_db.db.root, entry_id)
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

    pub fn create_entry(&self, group_id: &str, data: CreateEntryData) -> Result<Entry, AppError> {
        let mut db_lock = self.database.lock().map_err(|_| AppError::Lock)?;
        let open_db = db_lock.as_mut().ok_or(AppError::DatabaseNotOpen)?;

        let group = find_group_by_id_mut(&mut open_db.db.root, group_id)
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
            Value::Protected(SecStr::new(data.password.into_bytes())),
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

    pub fn update_entry(&self, id: &str, data: UpdateEntryData) -> Result<Entry, AppError> {
        let mut db_lock = self.database.lock().map_err(|_| AppError::Lock)?;
        let open_db = db_lock.as_mut().ok_or(AppError::DatabaseNotOpen)?;

        let (entry, group_id) = find_entry_by_id_mut(&mut open_db.db.root, id)
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
        if let Some(password) = data.password {
            entry.fields.insert(
                "Password".to_string(),
                Value::Protected(SecStr::new(password.into_bytes())),
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
        open_db.is_modified = true;

        Ok(convert_entry(entry, &group_id))
    }

    pub fn delete_entry(&self, id: &str) -> Result<(), AppError> {
        let mut db_lock = self.database.lock().map_err(|_| AppError::Lock)?;
        let open_db = db_lock.as_mut().ok_or(AppError::DatabaseNotOpen)?;

        let mut entry = {
            let root = &mut open_db.db.root;
            remove_entry_by_id(root, id).ok_or_else(|| AppError::EntryNotFound(id.to_string()))?
        };

        let recycle_bin_id = ensure_recycle_bin(&mut open_db.db);
        let recycle_bin = find_group_by_id_mut(&mut open_db.db.root, &recycle_bin_id)
            .ok_or_else(|| AppError::GroupNotFound(recycle_bin_id.clone()))?;

        let now = Times::now();
        entry.times.set_last_modification(now);
        entry.times.set_location_changed(now);
        recycle_bin.add_child(entry);

        open_db.is_modified = true;
        Ok(())
    }

    pub fn move_entry(&self, id: &str, target_group_id: &str) -> Result<Entry, AppError> {
        let mut db_lock = self.database.lock().map_err(|_| AppError::Lock)?;
        let open_db = db_lock.as_mut().ok_or(AppError::DatabaseNotOpen)?;

        let mut entry = {
            let root = &mut open_db.db.root;
            remove_entry_by_id(root, id).ok_or_else(|| AppError::EntryNotFound(id.to_string()))?
        };

        let target_group = find_group_by_id_mut(&mut open_db.db.root, target_group_id)
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

fn find_group_by_id_mut<'a>(
    group: &'a mut keepass::db::Group,
    id: &str,
) -> Option<&'a mut keepass::db::Group> {
    if group.uuid.to_string() == id {
        return Some(group);
    }

    for node in &mut group.children {
        if let Node::Group(child) = node {
            if let Some(found) = find_group_by_id_mut(child, id) {
                return Some(found);
            }
        }
    }

    None
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

fn ensure_recycle_bin(db: &mut Database) -> String {
    if let Some(recycle_uuid) = db.meta.recyclebin_uuid {
        if find_group_by_id(&db.root, &recycle_uuid.to_string()).is_some() {
            db.meta.recyclebin_enabled = Some(true);
            db.meta.recyclebin_changed = Some(Times::now());
            return recycle_uuid.to_string();
        }
    }

    if let Some(group) = find_group_by_name(&db.root, "Recycle Bin") {
        db.meta.recyclebin_enabled = Some(true);
        db.meta.recyclebin_uuid = Some(group.uuid);
        db.meta.recyclebin_changed = Some(Times::now());
        return group.uuid.to_string();
    }

    let recycle_bin = keepass::db::Group::new("Recycle Bin");
    let recycle_uuid = recycle_bin.uuid;
    db.root.add_child(recycle_bin);

    db.meta.recyclebin_enabled = Some(true);
    db.meta.recyclebin_uuid = Some(recycle_uuid);
    db.meta.recyclebin_changed = Some(Times::now());

    recycle_uuid.to_string()
}
