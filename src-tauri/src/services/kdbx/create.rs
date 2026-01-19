use crate::domain::kdbx::OpenDatabase;
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
    pub fn create(&self, path: &str, password: &str, name: &str) -> Result<DatabaseInfo, AppError> {
        self.create_database(
            path,
            Some(password),
            None,
            name,
            &DatabaseCreationOptions::default(),
        )
    }

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

        if password.is_none() && keyfile_path.is_none() {
            return Err(AppError::NoCredentials);
        }

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
        db.root.name = name.to_string();
        db.meta.database_name = Some(name.to_string());
        db.meta.generator = Some(String::from("MithrilVault"));
        if let Some(description) = &options.description {
            db.meta.database_description = Some(description.clone());
        }

        if options.create_default_groups {
            for group_name in DEFAULT_GROUP_NAMES {
                let group = keepass::db::Group::new(group_name);
                db.root.add_child(group);
            }
        }

        let root_group_id = db.root.uuid.to_string();
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
}
