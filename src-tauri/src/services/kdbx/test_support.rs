// SPDX-License-Identifier: MIT
//! Shared test fixtures for the KDBX service modules.
//!
//! Several `kdbx` submodules (`entries`, `favicons`, `custom_icons`, …) need a
//! freshly created database seeded with a couple of entries. Keeping a single
//! fixture here avoids near-identical `create_test_database` copies drifting
//! apart across those test modules.
#![allow(clippy::expect_used)]

use super::KdbxService;
use crate::domain::secure::SecureString;
use crate::dto::database::DatabaseCreationOptions;
use crate::dto::entry::CreateEntryData;
use tempfile::TempDir;

/// Creates a temp KDBX database seeded with two entries ("Entry A" / "Entry B")
/// and returns `(service, tempdir, db_path, entry_a_id, entry_b_id)`.
///
/// The `TempDir` is returned so the caller keeps it alive for the duration of
/// the test; dropping it deletes the on-disk database.
pub(crate) fn create_test_database() -> (KdbxService, TempDir, String, String, String) {
    let dir = tempfile::tempdir().expect("create temp dir");
    let db_path = dir.path().join("kdbx-tests.kdbx");
    let db_path_str = db_path.to_string_lossy().to_string();

    let options = DatabaseCreationOptions {
        create_default_groups: true,
        kdf_memory: Some(1024 * 1024),
        kdf_iterations: Some(1),
        kdf_parallelism: Some(1),
        description: None,
    };

    let service = KdbxService::new();
    service
        .create_database(&db_path_str, Some("testpass"), None, "KDBX Tests", &options)
        .expect("create db");
    let info = service.get_info(&db_path_str).expect("database info");

    let entry_a = seed_entry(
        &service,
        &db_path_str,
        &info.root_group_id,
        "Entry A",
        "alice",
    );
    let entry_b = seed_entry(
        &service,
        &db_path_str,
        &info.root_group_id,
        "Entry B",
        "bob",
    );

    (service, dir, db_path_str, entry_a, entry_b)
}

/// Creates one minimal entry and returns its id.
fn seed_entry(
    service: &KdbxService,
    db_path: &str,
    group_id: &str,
    title: &str,
    username: &str,
) -> String {
    service
        .create_entry(
            db_path,
            group_id,
            CreateEntryData {
                title: title.to_string(),
                username: username.to_string(),
                password: SecureString::from("secret"),
                url: None,
                notes: None,
                icon_id: Some(0),
                tags: None,
                custom_fields: None,
                protected_custom_fields: None,
                expires: None,
                expiry_time: None,
            },
        )
        .expect("create entry")
        .id
}
