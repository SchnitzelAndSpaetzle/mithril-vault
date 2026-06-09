use crate::domain::kdbx::{format_database_version, OpenDatabase};
use crate::domain::secure::SecureString;
use crate::dto::database::{DatabaseCreationOptions, DatabaseInfo};
use crate::dto::error::AppError;
use crate::services::kdbx::key::build_database_key;
use crate::utils::atomic_write::{atomic_write, AtomicWriteOptions};
use keepass::config::{
    CompressionConfig, DatabaseConfig, DatabaseVersion, InnerCipherConfig, KdfConfig,
    OuterCipherConfig,
};
use keepass::Database;

use super::KdbxService;

const DEFAULT_GROUP_NAMES: &[&str] = &["General", "Email", "Banking", "Social"];

impl KdbxService {
    /// Creates a new database with a password.
    pub fn create(&self, path: &str, password: &str, name: &str) -> Result<DatabaseInfo, AppError> {
        self.create_database(
            path,
            Some(password),
            None,
            name,
            &DatabaseCreationOptions::default(),
        )
    }

    /// Creates a new database with the provided options.
    /// The new database is automatically opened and added to the list of open databases.
    pub fn create_database(
        &self,
        path: &str,
        password: Option<&str>,
        keyfile_path: Option<&str>,
        name: &str,
        options: &DatabaseCreationOptions,
    ) -> Result<DatabaseInfo, AppError> {
        let normalized_path = Self::normalize_path(path);
        let mut databases = self.lock_databases()?;

        // Check if this specific path is already open
        if databases.contains_key(&normalized_path) {
            return Err(AppError::DatabaseAlreadyOpen(path.to_string()));
        }

        if password.is_none() && keyfile_path.is_none() {
            return Err(AppError::NoCredentials);
        }

        let mut config = DatabaseConfig::default();
        // KDBX 4.1 — keepass 0.13 no longer supports writing KDBX 4.0.
        config.version = DatabaseVersion::KDB4(1);
        config.outer_cipher_config = OuterCipherConfig::AES256;
        config.compression_config = CompressionConfig::GZip;
        config.inner_cipher_config = InnerCipherConfig::ChaCha20;
        config.kdf_config = KdfConfig::Argon2id {
            iterations: options.iterations(),
            memory: options.memory_bytes(),
            parallelism: options.parallelism(),
            version: argon2::Version::Version13,
        };

        let mut db = Database::with_config(config);
        db.meta.database_name = Some(name.to_string());
        db.meta.generator = Some(String::from("MithrilVault"));
        if let Some(description) = &options.description {
            db.meta.database_description = Some(description.clone());
        }

        let root_group_id = db.root().id().uuid().to_string();
        {
            let mut root = db.root_mut();
            root.name = name.to_string();
            if options.create_default_groups {
                for group_name in DEFAULT_GROUP_NAMES {
                    root.add_group().name = (*group_name).to_string();
                }
            }
        }
        let password_owned = password.map(String::from);
        let keyfile_path_owned = keyfile_path.map(String::from);

        atomic_write(
            path,
            &AtomicWriteOptions {
                preserve_permissions: false,
            },
            |file| {
                let key =
                    build_database_key(password_owned.as_deref(), keyfile_path_owned.as_deref())?;
                db.save(file, key)
                    .map_err(|e| AppError::Kdbx(e.to_string()))
            },
        )?;

        let version = format_database_version(&db.config.version);

        let normalized_path = Self::normalize_path(path);
        databases.insert(
            normalized_path,
            OpenDatabase {
                db: Some(db),
                path: path.to_string(),
                is_modified: false,
                password: password.map(SecureString::from),
                keyfile_path: keyfile_path.map(String::from),
                version: version.clone(),
                name: name.to_string(),
                root_group_id: root_group_id.clone(),
                generation: 0,
            },
        );

        Ok(DatabaseInfo {
            name: name.to_string(),
            path: path.to_string(),
            is_modified: false,
            is_locked: false,
            root_group_id,
            version,
        })
    }
}
