// SPDX-License-Identifier: MIT

use crate::models::database::{DatabaseCreationOptions, DatabaseInfo};
use crate::models::entry::{Entry, EntryListItem};
use crate::models::error::AppError;
use crate::models::group::Group;
use crate::utils::atomic_write::{atomic_write, AtomicWriteOptions};
use keepass::config::{
    CompressionConfig, DatabaseConfig, DatabaseVersion, InnerCipherConfig, KdfConfig,
    OuterCipherConfig,
};
use keepass::db::Node;
use keepass::error::{
    BlockStreamError, CryptographyError, DatabaseIntegrityError, DatabaseKeyError,
    DatabaseOpenError,
};
use keepass::{Database, DatabaseKey};
use std::fs::File;
use std::sync::Mutex;

/// Default groups to create when `create_default_groups` option is enabled
const DEFAULT_GROUP_NAMES: &[&str] = &["General", "Email", "Banking", "Social"];

/// Internal state for an open database
struct OpenDatabase {
    /// The keepass-rs database instance
    db: Database,
    /// Path to the database file
    path: String,
    /// Whether the database has unsaved modifications
    is_modified: bool,
    /// Password used to open/create the database (needed for saving).
    /// Note: keepass-rs already uses `secstr`/`zeroize` internally for key handling.
    /// This field is `None` for keyfile-only authentication.
    password: Option<String>,
    /// Optional keyfile path for databases using keyfile authentication.
    /// When saving, the keyfile is re-read from this path to preserve authentication.
    keyfile_path: Option<String>,
    /// Database format version (e.g., "KDBX 3.1", "KDBX 4.0")
    version: String,
}

/// Format the database version as a user-friendly string
fn format_database_version(version: &DatabaseVersion) -> String {
    match version {
        DatabaseVersion::KDB(minor) => format!("KDB 1.{minor}"),
        DatabaseVersion::KDB2(minor) => format!("KDB 2.{minor}"),
        DatabaseVersion::KDB3(minor) => format!("KDBX 3.{minor}"),
        DatabaseVersion::KDB4(minor) => format!("KDBX 4.{minor}"),
    }
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
        let version = format_database_version(&db.config.version);

        *db_lock = Some(OpenDatabase {
            db,
            path: path.to_string(),
            is_modified: false,
            password: Some(password.to_string()),
            keyfile_path: None,
            version: version.clone(),
        });

        Ok(DatabaseInfo {
            name,
            path: path.to_string(),
            is_modified: false,
            is_locked: false,
            root_group_id,
            version,
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
        let version = format_database_version(&db.config.version);

        *db_lock = Some(OpenDatabase {
            db,
            path: path.to_string(),
            is_modified: false,
            password: Some(password.to_string()),
            keyfile_path: Some(keyfile_path.to_string()),
            version: version.clone(),
        });

        Ok(DatabaseInfo {
            name,
            path: path.to_string(),
            is_modified: false,
            is_locked: false,
            root_group_id,
            version,
        })
    }

    /// Open a KDBX database with keyfile-only authentication (no password)
    pub fn open_with_keyfile_only(
        &self,
        path: &str,
        keyfile_path: &str,
    ) -> Result<DatabaseInfo, AppError> {
        let mut db_lock = self.database.lock().map_err(|_| AppError::Lock)?;

        if db_lock.is_some() {
            return Err(AppError::DatabaseAlreadyOpen);
        }

        let mut file = File::open(path).map_err(|e| AppError::InvalidPath(e.to_string()))?;
        let mut keyfile = File::open(keyfile_path).map_err(|_| AppError::KeyfileNotFound)?;

        let key = DatabaseKey::new()
            .with_keyfile(&mut keyfile)
            .map_err(|_| AppError::KeyfileInvalid)?;

        let db = Database::open(&mut file, key).map_err(map_open_error)?;

        let root_group_id = db.root.uuid.to_string();
        let name = db.root.name.clone();
        let version = format_database_version(&db.config.version);

        *db_lock = Some(OpenDatabase {
            db,
            path: path.to_string(),
            is_modified: false,
            password: None,
            keyfile_path: Some(keyfile_path.to_string()),
            version: version.clone(),
        });

        Ok(DatabaseInfo {
            name,
            path: path.to_string(),
            is_modified: false,
            is_locked: false,
            root_group_id,
            version,
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
            version: open_db.version.clone(),
        })
    }

    /// List all entries, optionally filtered by group
    pub fn list_entries(&self, group_id: Option<&str>) -> Result<Vec<EntryListItem>, AppError> {
        let db_lock = self.database.lock().map_err(|_| AppError::Lock)?;
        let open_db = db_lock.as_ref().ok_or(AppError::DatabaseNotOpen)?;

        let mut entries = Vec::new();

        if let Some(gid) = group_id {
            // Find the specific group and list its entries
            let group = find_group_by_id(&open_db.db.root, gid)
                .ok_or_else(|| AppError::GroupNotFound(gid.to_string()))?;
            collect_entries_from_group(group, &mut entries);
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

    /// Create a new KDBX4 database with password-only authentication (legacy API)
    pub fn create(&self, path: &str, password: &str, name: &str) -> Result<DatabaseInfo, AppError> {
        self.create_database(
            path,
            Some(password),
            None,
            name,
            &DatabaseCreationOptions::default(),
        )
    }

    /// Create a new KDBX4 database with full options
    ///
    /// Supports:
    /// - Password-only, keyfile-only, or password+keyfile authentication
    /// - Configurable Argon2id KDF parameters (memory, iterations, parallelism)
    /// - Optional database description metadata
    /// - Optional default groups (General, Email, Banking, Social)
    pub fn create_database(
        &self,
        path: &str,
        password: Option<&str>,
        keyfile_path: Option<&str>,
        name: &str,
        options: &DatabaseCreationOptions,
    ) -> Result<DatabaseInfo, AppError> {
        let mut db_lock = self.database.lock().map_err(|_| AppError::Lock)?;

        if db_lock.is_some() {
            return Err(AppError::DatabaseAlreadyOpen);
        }

        // Require at least password or keyfile
        if password.is_none() && keyfile_path.is_none() {
            return Err(AppError::NoCredentials);
        }

        // Create custom database configuration with Argon2id KDF
        let config = DatabaseConfig {
            version: DatabaseVersion::KDB4(0),
            outer_cipher_config: OuterCipherConfig::AES256,
            compression_config: CompressionConfig::GZip,
            inner_cipher_config: InnerCipherConfig::ChaCha20,
            kdf_config: KdfConfig::Argon2id {
                iterations: options.iterations(),
                memory: options.memory_bytes(),
                parallelism: options.parallelism(),
                version: argon2::Version::Version13,
            },
            public_custom_data: None,
        };

        let mut db = Database::new(config);

        // Set root group name
        db.root.name = name.to_string();

        // Set database metadata
        db.meta.database_name = Some(name.to_string());
        db.meta.generator = Some(String::from("MithrilVault"));
        if let Some(description) = &options.description {
            db.meta.database_description = Some(description.clone());
        }

        // Create default groups if requested
        if options.create_default_groups {
            for group_name in DEFAULT_GROUP_NAMES {
                let group = keepass::db::Group::new(group_name);
                db.root.add_child(group);
            }
        }

        // Get root group ID before moving db into closure
        let root_group_id = db.root.uuid.to_string();

        // Copy values for use in closure
        let password_owned = password.map(String::from);
        let keyfile_path_owned = keyfile_path.map(String::from);

        // Save the database to disk using atomic write
        atomic_write(
            path,
            &AtomicWriteOptions {
                preserve_permissions: false, // New file gets secure 0600 permissions
            },
            |file| {
                // Build key inside closure (DatabaseKey doesn't implement Clone)
                let key =
                    build_database_key(password_owned.as_deref(), keyfile_path_owned.as_deref())?;
                db.save(file, key)
                    .map_err(|e| AppError::Kdbx(e.to_string()))
            },
        )?;
        // New databases are always KDBX 4.0
        let version = String::from("KDBX 4.0");

        *db_lock = Some(OpenDatabase {
            db,
            path: path.to_string(),
            is_modified: false,
            password: password.map(String::from),
            keyfile_path: keyfile_path.map(String::from),
            version: version.clone(),
        });

        Ok(DatabaseInfo {
            name: name.to_string(),
            path: path.to_string(),
            is_modified: false,
            is_locked: false,
            root_group_id,
            version,
        })
    }

    /// Save the currently open database using atomic write
    ///
    /// Uses the temp file + rename pattern to ensure system interruptions
    /// cannot corrupt existing database files.
    pub fn save(&self) -> Result<(), AppError> {
        let mut db_lock = self.database.lock().map_err(|_| AppError::Lock)?;
        let open_db = db_lock.as_mut().ok_or(AppError::DatabaseNotOpen)?;

        // Require at least password or keyfile for saving
        if open_db.password.is_none() && open_db.keyfile_path.is_none() {
            return Err(AppError::NoCredentials);
        }

        // Clone values needed for the closure (to avoid borrow issues)
        let path = open_db.path.clone();
        let password = open_db.password.clone();
        let keyfile_path = open_db.keyfile_path.clone();

        // Use atomic write with preserve_permissions for existing files
        atomic_write(
            &path,
            &AtomicWriteOptions {
                preserve_permissions: true,
            },
            |file| {
                // Build key inside closure (DatabaseKey doesn't implement Clone)
                let key = build_database_key(password.as_deref(), keyfile_path.as_deref())?;
                open_db
                    .db
                    .save(file, key)
                    .map_err(|e| AppError::Kdbx(e.to_string()))
            },
        )?;

        open_db.is_modified = false;
        Ok(())
    }

    /// Save the database to a new path (Save As) using atomic write
    ///
    /// Uses the temp file + rename pattern to ensure system interruptions
    /// cannot corrupt existing database files.
    pub fn save_as(&self, new_path: &str, new_password: Option<&str>) -> Result<(), AppError> {
        let mut db_lock = self.database.lock().map_err(|_| AppError::Lock)?;
        let open_db = db_lock.as_mut().ok_or(AppError::DatabaseNotOpen)?;

        // Determine the password to use (new_password takes precedence, then existing)
        let effective_password: Option<String> = new_password
            .map(String::from)
            .or_else(|| open_db.password.clone());

        // Require at least password or keyfile for saving
        if effective_password.is_none() && open_db.keyfile_path.is_none() {
            return Err(AppError::NoCredentials);
        }

        // Clone keyfile path for the closure
        let keyfile_path = open_db.keyfile_path.clone();

        // Use atomic write with preserve_permissions=false (new file gets secure permissions)
        atomic_write(
            new_path,
            &AtomicWriteOptions {
                preserve_permissions: false,
            },
            |file| {
                // Build key inside closure (DatabaseKey doesn't implement Clone)
                let key =
                    build_database_key(effective_password.as_deref(), keyfile_path.as_deref())?;
                open_db
                    .db
                    .save(file, key)
                    .map_err(|e| AppError::Kdbx(e.to_string()))
            },
        )?;

        // Update path and password after successful save
        open_db.path = new_path.to_string();
        if new_password.is_some() {
            open_db.password = new_password.map(String::from);
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

/// Build a `DatabaseKey` from optional password and keyfile path.
///
/// This helper avoids code duplication in save operations.
fn build_database_key(
    password: Option<&str>,
    keyfile_path: Option<&str>,
) -> Result<DatabaseKey, AppError> {
    let mut key = DatabaseKey::new();

    if let Some(pw) = password {
        key = key.with_password(pw);
    }

    if let Some(kf_path) = keyfile_path {
        let mut keyfile = File::open(kf_path)
            .map_err(|e| AppError::InvalidPath(format!("Keyfile not found: {e}")))?;
        key = key
            .with_keyfile(&mut keyfile)
            .map_err(|e| AppError::Kdbx(e.to_string()))?;
    }

    Ok(key)
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
