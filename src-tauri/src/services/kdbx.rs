// SPDX-License-Identifier: GPL-3.0-or-later

use crate::models::database::DatabaseInfo;
use crate::models::entry::{Entry, EntryListItem};
use crate::models::error::AppError;
use crate::models::group::Group;
use keepass::db::NodeRef;
use keepass::{Database, DatabaseKey};
use std::fs::File;
use std::sync::Mutex;

/// Internal state for an open database
struct OpenDatabase {
    /// The keepass-rs database instance
    db: Database,
    /// Path to the database file
    path: String,
    /// Whether the database has unsaved modifications
    is_modified: bool,
}

/// Service for KDBX database operations
pub struct KdbxService {
    /// Currently open database (if any)
    database: Mutex<Option<OpenDatabase>>,
}

impl KdbxService {
    pub fn new() -> Self {
        Self {
            database: Mutex::new(None),
        }
    }

    /// Open a KDBX database with password authentication
    pub fn open(&self, path: &str, password: &str) -> Result<DatabaseInfo, AppError> {
        let mut db_lock = self.database.lock().map_err(|_| AppError::Lock)?;

        if db_lock.is_some() {
            return Err(AppError::DatabaseAlreadyOpen);
        }

        let mut file = File::open(path).map_err(|e| AppError::InvalidPath(e.to_string()))?;

        let key = DatabaseKey::new().with_password(password);
        let db = Database::open(&mut file, key).map_err(|e| {
            let err_str = e.to_string();
            if err_str.contains("Invalid credentials")
                || err_str.contains("decryption")
                || err_str.contains("HMAC")
            {
                AppError::InvalidPassword
            } else {
                AppError::Kdbx(err_str)
            }
        })?;

        let root_group_id = db.root.uuid.to_string();
        let name = db.root.name.clone();

        *db_lock = Some(OpenDatabase {
            db,
            path: path.to_string(),
            is_modified: false,
        });

        Ok(DatabaseInfo {
            name,
            path: path.to_string(),
            is_modified: false,
            is_locked: false,
            root_group_id,
        })
    }

    /// Open a KDBX database with password and keyfile authentication
    pub fn open_with_keyfile(
        &self,
        path: &str,
        password: &str,
        keyfile_path: &str,
    ) -> Result<DatabaseInfo, AppError> {
        let mut db_lock = self.database.lock().map_err(|_| AppError::Lock)?;

        if db_lock.is_some() {
            return Err(AppError::DatabaseAlreadyOpen);
        }

        let mut file = File::open(path).map_err(|e| AppError::InvalidPath(e.to_string()))?;
        let mut keyfile =
            File::open(keyfile_path).map_err(|e| AppError::InvalidPath(e.to_string()))?;

        let key = DatabaseKey::new()
            .with_password(password)
            .with_keyfile(&mut keyfile)
            .map_err(|e| AppError::Kdbx(e.to_string()))?;

        let db = Database::open(&mut file, key).map_err(|e| {
            let err_str = e.to_string();
            if err_str.contains("Invalid credentials")
                || err_str.contains("decryption")
                || err_str.contains("HMAC")
            {
                AppError::InvalidPassword
            } else {
                AppError::Kdbx(err_str)
            }
        })?;

        let root_group_id = db.root.uuid.to_string();
        let name = db.root.name.clone();

        *db_lock = Some(OpenDatabase {
            db,
            path: path.to_string(),
            is_modified: false,
        });

        Ok(DatabaseInfo {
            name,
            path: path.to_string(),
            is_modified: false,
            is_locked: false,
            root_group_id,
        })
    }

    /// Close the currently open database
    pub fn close(&self) -> Result<(), AppError> {
        let mut db_lock = self.database.lock().map_err(|_| AppError::Lock)?;

        if db_lock.is_none() {
            return Err(AppError::DatabaseNotOpen);
        }

        *db_lock = None;
        Ok(())
    }

    /// Get information about the currently open database
    pub fn get_info(&self) -> Result<DatabaseInfo, AppError> {
        let db_lock = self.database.lock().map_err(|_| AppError::Lock)?;
        let open_db = db_lock.as_ref().ok_or(AppError::DatabaseNotOpen)?;

        Ok(DatabaseInfo {
            name: open_db.db.root.name.clone(),
            path: open_db.path.clone(),
            is_modified: open_db.is_modified,
            is_locked: false,
            root_group_id: open_db.db.root.uuid.to_string(),
        })
    }

    /// List all entries, optionally filtered by group
    pub fn list_entries(&self, group_id: Option<&str>) -> Result<Vec<EntryListItem>, AppError> {
        let db_lock = self.database.lock().map_err(|_| AppError::Lock)?;
        let open_db = db_lock.as_ref().ok_or(AppError::DatabaseNotOpen)?;

        let mut entries = Vec::new();

        if let Some(gid) = group_id {
            // Find the specific group and list its entries
            if let Some(group) = find_group_by_id(&open_db.db.root, gid) {
                collect_entries_from_group(group, &mut entries);
            }
        } else {
            // List all entries in the database
            collect_all_entries(&open_db.db.root, &mut entries);
        }

        Ok(entries)
    }

    /// Get a specific entry by ID
    pub fn get_entry(&self, id: &str) -> Result<Entry, AppError> {
        let db_lock = self.database.lock().map_err(|_| AppError::Lock)?;
        let open_db = db_lock.as_ref().ok_or(AppError::DatabaseNotOpen)?;

        find_entry_by_id(&open_db.db.root, id)
            .ok_or_else(|| AppError::EntryNotFound(id.to_string()))
    }

    /// Get the password for a specific entry
    pub fn get_entry_password(&self, id: &str) -> Result<String, AppError> {
        let db_lock = self.database.lock().map_err(|_| AppError::Lock)?;
        let open_db = db_lock.as_ref().ok_or(AppError::DatabaseNotOpen)?;

        find_entry_password(&open_db.db.root, id)
            .ok_or_else(|| AppError::EntryNotFound(id.to_string()))
    }

    /// List all groups as a hierarchical structure
    pub fn list_groups(&self) -> Result<Vec<Group>, AppError> {
        let db_lock = self.database.lock().map_err(|_| AppError::Lock)?;
        let open_db = db_lock.as_ref().ok_or(AppError::DatabaseNotOpen)?;

        // Return the root group with its children
        let root = convert_group(&open_db.db.root, None);
        Ok(vec![root])
    }

    /// Get a specific group by ID
    pub fn get_group(&self, id: &str) -> Result<Group, AppError> {
        let db_lock = self.database.lock().map_err(|_| AppError::Lock)?;
        let open_db = db_lock.as_ref().ok_or(AppError::DatabaseNotOpen)?;

        find_group_by_id(&open_db.db.root, id)
            .map(|g| convert_group(g, None))
            .ok_or_else(|| AppError::GroupNotFound(id.to_string()))
    }

    /// Create a new database (experimental - keepass-rs write support)
    pub fn create(&self, _path: &str, _password: &str, _name: &str) -> Result<(), AppError> {
        // Write support is experimental in keepass-rs
        // For now, return not implemented
        Err(AppError::NotImplemented(
            "Database creation not yet implemented".into(),
        ))
    }

    /// Save the currently open database
    pub fn save(&self) -> Result<(), AppError> {
        // Write support is experimental in keepass-rs
        Err(AppError::NotImplemented(
            "Database saving not yet implemented".into(),
        ))
    }
}

impl Default for KdbxService {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Helper functions for converting keepass-rs types to our models
// ============================================================================

/// Find a group by its UUID string
fn find_group_by_id<'a>(group: &'a keepass::db::Group, id: &str) -> Option<&'a keepass::db::Group> {
    if group.uuid.to_string() == id {
        return Some(group);
    }

    for node in group {
        if let NodeRef::Group(child) = node {
            if let Some(found) = find_group_by_id(child, id) {
                return Some(found);
            }
        }
    }

    None
}

/// Find an entry by its UUID string and return it converted to our Entry type
fn find_entry_by_id(group: &keepass::db::Group, id: &str) -> Option<Entry> {
    for node in group {
        match node {
            NodeRef::Entry(entry) => {
                if entry.uuid.to_string() == id {
                    return Some(convert_entry(entry, &group.uuid.to_string()));
                }
            }
            NodeRef::Group(child) => {
                if let Some(found) = find_entry_by_id(child, id) {
                    return Some(found);
                }
            }
        }
    }
    None
}

/// Find an entry's password by its UUID string
fn find_entry_password(group: &keepass::db::Group, id: &str) -> Option<String> {
    for node in group {
        match node {
            NodeRef::Entry(entry) => {
                if entry.uuid.to_string() == id {
                    return entry.get_password().map(std::string::ToString::to_string);
                }
            }
            NodeRef::Group(child) => {
                if let Some(found) = find_entry_password(child, id) {
                    return Some(found);
                }
            }
        }
    }
    None
}

/// Collect entries from a specific group (non-recursive)
fn collect_entries_from_group(group: &keepass::db::Group, entries: &mut Vec<EntryListItem>) {
    let group_id = group.uuid.to_string();
    for node in group {
        if let NodeRef::Entry(entry) = node {
            entries.push(convert_entry_to_list_item(entry, &group_id));
        }
    }
}

/// Collect all entries recursively from a group
fn collect_all_entries(group: &keepass::db::Group, entries: &mut Vec<EntryListItem>) {
    let group_id = group.uuid.to_string();
    for node in group {
        match node {
            NodeRef::Entry(entry) => {
                entries.push(convert_entry_to_list_item(entry, &group_id));
            }
            NodeRef::Group(child) => {
                collect_all_entries(child, entries);
            }
        }
    }
}

/// Convert a keepass-rs Entry to our Entry model
fn convert_entry(entry: &keepass::db::Entry, group_id: &str) -> Entry {
    let times = &entry.times;

    Entry {
        id: entry.uuid.to_string(),
        group_id: group_id.to_string(),
        title: entry.get_title().unwrap_or_default().to_string(),
        username: entry.get_username().unwrap_or_default().to_string(),
        url: entry.get_url().map(std::string::ToString::to_string),
        notes: entry.get("Notes").map(std::string::ToString::to_string),
        created_at: times
            .get_creation()
            .map(std::string::ToString::to_string)
            .unwrap_or_default(),
        modified_at: times
            .get_last_modification()
            .map(std::string::ToString::to_string)
            .unwrap_or_default(),
    }
}

/// Convert a keepass-rs Entry to our `EntryListItem` model (without password)
fn convert_entry_to_list_item(entry: &keepass::db::Entry, group_id: &str) -> EntryListItem {
    EntryListItem {
        id: entry.uuid.to_string(),
        group_id: group_id.to_string(),
        title: entry.get_title().unwrap_or_default().to_string(),
        username: entry.get_username().unwrap_or_default().to_string(),
        url: entry.get_url().map(std::string::ToString::to_string),
    }
}

/// Convert a keepass-rs Group to our Group model (recursive)
fn convert_group(group: &keepass::db::Group, parent_id: Option<&str>) -> Group {
    let id = group.uuid.to_string();
    let mut children = Vec::new();

    for node in group {
        if let NodeRef::Group(child) = node {
            children.push(convert_group(child, Some(&id)));
        }
    }

    Group {
        id: id.clone(),
        parent_id: parent_id.map(std::string::ToString::to_string),
        name: group.name.clone(),
        icon: group.icon_id.map(|i| i.to_string()),
        children,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_creation() {
        let service = KdbxService::new();
        // Service should be created without an open database
        assert!(service.get_info().is_err());
    }

    #[test]
    fn test_close_without_open() {
        let service = KdbxService::new();
        let result = service.close();
        assert!(result.is_err());
    }

    #[test]
    fn test_list_entries_without_open() {
        let service = KdbxService::new();
        let result = service.list_entries(None);
        assert!(result.is_err());
    }
}
