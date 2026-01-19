use crate::dto::entry::{Entry, EntryListItem};
use crate::dto::error::AppError;
use keepass::db::Node;

use super::mapping::{convert_entry, convert_entry_to_list_item, find_group_by_id};
use super::KdbxService;

impl KdbxService {
    pub fn list_entries(&self, group_id: Option<&str>) -> Result<Vec<EntryListItem>, AppError> {
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

fn collect_entries_from_group(group: &keepass::db::Group, entries: &mut Vec<EntryListItem>) {
    let group_id = group.uuid.to_string();
    for node in &group.children {
        if let Node::Entry(entry) = node {
            entries.push(convert_entry_to_list_item(entry, &group_id));
        }
    }
}

fn collect_all_entries(group: &keepass::db::Group, entries: &mut Vec<EntryListItem>) {
    let group_id = group.uuid.to_string();
    for node in &group.children {
        match node {
            Node::Entry(entry) => {
                entries.push(convert_entry_to_list_item(entry, &group_id));
            }
            Node::Group(child) => {
                collect_all_entries(child, entries);
            }
        }
    }
}
