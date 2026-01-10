// SPDX-License-Identifier: GPL-3.0-or-later

use crate::models::database::DatabaseInfo;
use crate::models::entry::{Entry, EntryListItem};
use crate::models::error::AppError;
use crate::models::group::Group;
use keepass::config::DatabaseConfig;
use keepass::db::Node;
use keepass::error::{
    BlockStreamError,
    CryptographyError,
    DatabaseIntegrityError,
    DatabaseKeyError,
    DatabaseOpenError,
};
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
    /// Password used to open/create the database (needed for saving)
    /// TODO: Use zeroize crate for secure memory handling
    password: String,
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
        let db = Database::open(&mut file, key).map_err(map_open_error)?;

        let root_group_id = db.root.uuid.to_string();
        let name = db.root.name.clone();

        *db_lock = Some(OpenDatabase {
            db,
            path: path.to_string(),
            is_modified: false,
            password: password.to_string(),
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

        let db = Database::open(&mut file, key).map_err(map_open_error)?;

        let root_group_id = db.root.uuid.to_string();
        let name = db.root.name.clone();

        *db_lock = Some(OpenDatabase {
            db,
            path: path.to_string(),
            is_modified: false,
            password: password.to_string(),
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
    /// Returns empty string if entry exists but has no password field
    pub fn get_entry_password(&self, id: &str) -> Result<String, AppError> {
        let db_lock = self.database.lock().map_err(|_| AppError::Lock)?;
        let open_db = db_lock.as_ref().ok_or(AppError::DatabaseNotOpen)?;

        match find_entry_password(&open_db.db.root, id) {
            PasswordSearchResult::Found(password) => Ok(password),
            PasswordSearchResult::NotFound => Err(AppError::EntryNotFound(id.to_string())),
        }
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

    /// Create a new KDBX4 database
    pub fn create(&self, path: &str, password: &str, name: &str) -> Result<DatabaseInfo, AppError> {
        let mut db_lock = self.database.lock().map_err(|_| AppError::Lock)?;

        if db_lock.is_some() {
            return Err(AppError::DatabaseAlreadyOpen);
        }

        // Create a new database with default settings (KDBX4)
        let mut db = Database::new(DatabaseConfig::default());
        db.root.name = name.to_string();

        // Save the database to disk
        let key = DatabaseKey::new().with_password(password);
        let mut file = File::create(path).map_err(|e| AppError::Io(e.to_string()))?;
        db.save(&mut file, key)
            .map_err(|e| AppError::Kdbx(e.to_string()))?;

        let root_group_id = db.root.uuid.to_string();

        *db_lock = Some(OpenDatabase {
            db,
            path: path.to_string(),
            is_modified: false,
            password: password.to_string(),
        });

        Ok(DatabaseInfo {
            name: name.to_string(),
            path: path.to_string(),
            is_modified: false,
            is_locked: false,
            root_group_id,
        })
    }

    /// Save the currently open database
    pub fn save(&self) -> Result<(), AppError> {
        let mut db_lock = self.database.lock().map_err(|_| AppError::Lock)?;
        let open_db = db_lock.as_mut().ok_or(AppError::DatabaseNotOpen)?;

        let key = DatabaseKey::new().with_password(&open_db.password);
        let mut file = File::create(&open_db.path).map_err(|e| AppError::Io(e.to_string()))?;

        open_db
            .db
            .save(&mut file, key)
            .map_err(|e| AppError::Kdbx(e.to_string()))?;

        open_db.is_modified = false;
        Ok(())
    }

    /// Save the database to a new path (Save As)
    pub fn save_as(&self, new_path: &str, new_password: Option<&str>) -> Result<(), AppError> {
        let mut db_lock = self.database.lock().map_err(|_| AppError::Lock)?;
        let open_db = db_lock.as_mut().ok_or(AppError::DatabaseNotOpen)?;

        // Use new password if provided, otherwise use existing
        let password = new_password.unwrap_or(&open_db.password);
        let key = DatabaseKey::new().with_password(password);
        let mut file = File::create(new_path).map_err(|e| AppError::Io(e.to_string()))?;

        open_db
            .db
            .save(&mut file, key)
            .map_err(|e| AppError::Kdbx(e.to_string()))?;

        // Update path and password if changed
        open_db.path = new_path.to_string();
        if new_password.is_some() {
            open_db.password = password.to_string();
        }
        open_db.is_modified = false;

        Ok(())
    }
}

fn map_open_error(err: DatabaseOpenError) -> AppError {
    match err {
        // All of these errors indicate incorrect password/key
        DatabaseOpenError::Key(DatabaseKeyError::IncorrectKey)
        | DatabaseOpenError::DatabaseIntegrity(
            DatabaseIntegrityError::BlockStream(BlockStreamError::BlockHashMismatch { .. })
            | DatabaseIntegrityError::HeaderHashMismatch
            | DatabaseIntegrityError::Cryptography(
                CryptographyError::Unpadding(_) | CryptographyError::Padding(_),
            ),
        ) => AppError::InvalidPassword,
        other => AppError::Kdbx(other.to_string()),
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

    for node in &group.children {
        if let Node::Group(child) = node {
            if let Some(found) = find_group_by_id(child, id) {
                return Some(found);
            }
        }
    }

    None
}

/// Find an entry by its UUID string and return it converted to our Entry type
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

/// Result of searching for an entry's password
enum PasswordSearchResult {
    /// Entry not found
    NotFound,
    /// Entry found, password returned (empty string if no password field)
    Found(String),
}

/// Find an entry's password by its UUID string
fn find_entry_password(group: &keepass::db::Group, id: &str) -> PasswordSearchResult {
    for node in &group.children {
        match node {
            Node::Entry(entry) => {
                if entry.uuid.to_string() == id {
                    // Entry found - return password or empty string if field is missing
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

/// Collect entries from a specific group (non-recursive)
fn collect_entries_from_group(group: &keepass::db::Group, entries: &mut Vec<EntryListItem>) {
    let group_id = group.uuid.to_string();
    for node in &group.children {
        if let Node::Entry(entry) = node {
            entries.push(convert_entry_to_list_item(entry, &group_id));
        }
    }
}

/// Collect all entries recursively from a group
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

    for node in &group.children {
        if let Node::Group(child) = node {
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

// Tests live in src-tauri/tests/kdbx_integration.rs
