use keepass::config::DatabaseVersion;
use keepass::Database;

use super::secure::SecureString;

#[derive(Debug)]
pub struct OpenDatabase {
    pub db: Database,
    pub path: String,
    pub is_modified: bool,
    pub password: Option<SecureString>,
    pub keyfile_path: Option<String>,
    pub version: String,
}

/// Formats a database version for display.
pub fn format_database_version(version: &DatabaseVersion) -> String {
    match version {
        DatabaseVersion::KDB(minor) => format!("KDB 1.{minor}"),
        DatabaseVersion::KDB2(minor) => format!("KDB 2.{minor}"),
        DatabaseVersion::KDB3(minor) => format!("KDBX 3.{minor}"),
        DatabaseVersion::KDB4(minor) => format!("KDBX 4.{minor}"),
    }
}
