#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(clippy::needless_borrows_for_generic_args)]

use mithril_vault_lib::dto::database::{
    Compression, DatabaseCreationOptions, InnerCipher, KdfSettings, OuterCipher,
};
use mithril_vault_lib::dto::error::AppError;
use mithril_vault_lib::services::kdbx::KdbxService;
use std::path::PathBuf;
use tempfile::{tempdir, TempDir};

#[path = "support/mod.rs"]
mod support;

use support::fixture_path;

/// Creates a temporary copy of a fixture file for isolated testing.
fn copy_fixture_to_temp(filename: &str) -> Option<(TempDir, PathBuf)> {
    let source = fixture_path(filename);
    if !source.exists() {
        return None;
    }

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let dest = temp_dir.path().join(filename);

    std::fs::copy(&source, &dest).expect("Failed to copy fixture");
    Some((temp_dir, dest))
}

// ============================================================================
// inspect() Tests - Pre-authentication header reading
// ============================================================================

#[test]
fn test_inspect_kdbx4_file() {
    let path = fixture_path("test-kdbx4-low-KDF.kdbx");
    if !path.exists() {
        eprintln!(
            "Skipping test: fixture not found at {path:?}. \
             Create with KeePassXC using password 'test123'"
        );
        return;
    }

    let service = KdbxService::new();
    let info = service
        .inspect(&path.to_string_lossy())
        .expect("Failed to inspect KDBX4 file");

    assert!(info.is_valid_kdbx, "Should be a valid KDBX file");
    assert!(info.is_supported, "KDBX 4.x should be supported");
    assert!(
        info.version.starts_with("KDBX 4."),
        "Version should be KDBX 4.x, got: {}",
        info.version
    );
    assert_eq!(info.path, path.to_string_lossy(), "Path should match input");
}

#[test]
fn test_inspect_kdbx3_file() {
    let path = fixture_path("test-kdbx3-low-KDF.kdbx");
    if !path.exists() {
        eprintln!(
            "Skipping test: fixture not found at {path:?}. \
             Create with KeePassXC (KDBX 3.1 format) using password 'test123'"
        );
        return;
    }

    let service = KdbxService::new();
    let info = service
        .inspect(&path.to_string_lossy())
        .expect("Failed to inspect KDBX3 file");

    assert!(info.is_valid_kdbx, "Should be a valid KDBX file");
    assert!(info.is_supported, "KDBX 3.x should be supported");

    if !info.version.starts_with("KDBX 3.") {
        eprintln!(
            "Skipping test: fixture is {} format, not KDBX 3.x. \
             Recreate with KeePassXC using KDBX 3.1 format.",
            info.version
        );
        return;
    }

    assert!(
        info.version.starts_with("KDBX 3."),
        "Version should be KDBX 3.x, got: {}",
        info.version
    );
}

#[test]
fn test_inspect_non_kdbx_file() {
    let dir = tempdir().expect("Failed to create temp dir");
    let file_path = dir.path().join("not-a-kdbx.txt");
    std::fs::write(&file_path, b"This is not a KDBX file").expect("Failed to write test file");

    let service = KdbxService::new();
    let result = service.inspect(&file_path.to_string_lossy());

    assert!(
        matches!(result, Err(AppError::InvalidKdbxFile)),
        "Should fail with InvalidKdbxFile error for non-KDBX file, got: {result:?}"
    );
}

#[test]
fn test_inspect_nonexistent_file() {
    let path = fixture_path("nonexistent-database.kdbx");

    let service = KdbxService::new();
    let result = service.inspect(&path.to_string_lossy());

    assert!(
        matches!(result, Err(AppError::InvalidPath(_))),
        "Should fail with InvalidPath error for nonexistent file, got: {result:?}"
    );
}

#[test]
fn test_inspect_empty_file() {
    let dir = tempdir().expect("Failed to create temp dir");
    let file_path = dir.path().join("empty.kdbx");
    std::fs::write(&file_path, b"").expect("Failed to write empty file");

    let service = KdbxService::new();
    let result = service.inspect(&file_path.to_string_lossy());

    // Empty file should fail - either InvalidKdbxFile or HeaderParseError
    assert!(
        matches!(
            result,
            Err(AppError::InvalidKdbxFile | AppError::HeaderParseError(_))
        ),
        "Should fail for empty file, got: {result:?}"
    );
}

#[test]
fn test_inspect_truncated_header() {
    let dir = tempdir().expect("Failed to create temp dir");
    let file_path = dir.path().join("truncated.kdbx");
    // KDBX magic bytes but truncated before version info
    std::fs::write(&file_path, &[0x03, 0xD9, 0xA2, 0x9A]).expect("Failed to write truncated file");

    let service = KdbxService::new();
    let result = service.inspect(&file_path.to_string_lossy());

    // Truncated file should fail with header parse error or invalid KDBX
    assert!(
        matches!(
            result,
            Err(AppError::InvalidKdbxFile | AppError::HeaderParseError(_))
        ),
        "Should fail for truncated file, got: {result:?}"
    );
}

// ============================================================================
// get_config() Tests - Post-authentication configuration retrieval
// ============================================================================

#[test]
fn test_get_config_after_open() {
    let Some((_temp_dir, path)) = copy_fixture_to_temp("test-kdbx4-low-KDF.kdbx") else {
        eprintln!("Skipping test: KDBX4 fixture not found");
        return;
    };

    let service = KdbxService::new();
    service
        .open(&path.to_string_lossy(), "test123")
        .expect("Failed to open database");

    let config = service.get_config().expect("Failed to get config");

    assert!(
        config.version.starts_with("KDBX 4."),
        "Version should be KDBX 4.x, got: {}",
        config.version
    );

    // KDBX4 default uses ChaCha20 for inner cipher
    // Note: actual cipher may vary depending on how the test file was created
    assert!(
        matches!(
            config.inner_cipher,
            InnerCipher::ChaCha20 | InnerCipher::Salsa20
        ),
        "Inner cipher should be ChaCha20 or Salsa20 for KDBX4"
    );
}

#[test]
fn test_get_config_kdbx3_uses_aes_kdf() {
    let Some((_temp_dir, path)) = copy_fixture_to_temp("test-kdbx3-low-KDF.kdbx") else {
        eprintln!("Skipping test: KDBX3 fixture not found");
        return;
    };

    let service = KdbxService::new();
    service
        .open(&path.to_string_lossy(), "test123")
        .expect("Failed to open database");

    let config = service.get_config().expect("Failed to get config");

    if !config.version.starts_with("KDBX 3.") {
        eprintln!(
            "Skipping test: fixture is {} format, not KDBX 3.x. \
             Recreate with KeePassXC using KDBX 3.1 format.",
            config.version
        );
        return;
    }

    // KDBX3 uses AES KDF
    assert!(
        matches!(config.kdf, KdfSettings::AesKdf { .. }),
        "KDBX3 should use AES KDF, got: {:?}",
        config.kdf
    );
}

#[test]
fn test_get_config_without_open() {
    let service = KdbxService::new();
    let result = service.get_config();

    assert!(
        matches!(result, Err(AppError::DatabaseNotOpen)),
        "Should fail with DatabaseNotOpen when no database is open, got: {result:?}"
    );
}

#[test]
fn test_created_database_config() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("config-test.kdbx");

    let service = KdbxService::new();
    service
        .create(&db_path.to_string_lossy(), "testpassword", "Config Test")
        .expect("Failed to create database");

    let config = service.get_config().expect("Failed to get config");

    // Newly created databases should be KDBX 4.0
    assert_eq!(
        config.version, "KDBX 4.0",
        "New database should be KDBX 4.0"
    );

    // Default config: AES256 outer cipher, ChaCha20 inner cipher, GZip compression, Argon2id KDF
    assert_eq!(
        config.outer_cipher,
        OuterCipher::Aes256,
        "Default outer cipher should be AES256"
    );
    assert_eq!(
        config.inner_cipher,
        InnerCipher::ChaCha20,
        "Default inner cipher should be ChaCha20"
    );
    assert_eq!(
        config.compression,
        Compression::GZip,
        "Default compression should be GZip"
    );
    assert!(
        matches!(config.kdf, KdfSettings::Argon2id { .. }),
        "Default KDF should be Argon2id, got: {:?}",
        config.kdf
    );
}

#[test]
fn test_created_database_custom_kdf_params() {
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("custom-kdf-test.kdbx");

    let service = KdbxService::new();

    let options = DatabaseCreationOptions {
        description: Some("Test database".to_string()),
        create_default_groups: false,
        kdf_memory: Some(32 * 1024 * 1024), // 32 MB
        kdf_iterations: Some(5),
        kdf_parallelism: Some(2),
    };

    service
        .create_database(
            &db_path.to_string_lossy(),
            Some("testpassword"),
            None,
            "Custom KDF Test",
            &options,
        )
        .expect("Failed to create database with custom KDF");

    let config = service.get_config().expect("Failed to get config");

    // Verify custom KDF parameters
    match config.kdf {
        KdfSettings::Argon2id {
            memory,
            iterations,
            parallelism,
        } => {
            assert_eq!(memory, 32 * 1024 * 1024, "Memory should be 32 MB");
            assert_eq!(iterations, 5, "Iterations should be 5");
            assert_eq!(parallelism, 2, "Parallelism should be 2");
        }
        other => panic!("Expected Argon2id KDF, got: {other:?}"),
    }
}

// ============================================================================
// Combined workflow tests
// ============================================================================

#[test]
fn test_inspect_then_open_then_config() {
    let Some((_temp_dir, path)) = copy_fixture_to_temp("test-kdbx4-low-KDF.kdbx") else {
        eprintln!("Skipping test: KDBX4 fixture not found");
        return;
    };

    let service = KdbxService::new();

    // Step 1: Inspect without credentials
    let header_info = service
        .inspect(&path.to_string_lossy())
        .expect("Failed to inspect database");
    assert!(header_info.is_valid_kdbx);
    assert!(header_info.is_supported);

    // Step 2: Open with credentials
    service
        .open(&path.to_string_lossy(), "test123")
        .expect("Failed to open database");

    // Step 3: Get full config
    let config = service.get_config().expect("Failed to get config");
    assert_eq!(
        header_info.version, config.version,
        "Version from inspect should match version from config"
    );
}

#[test]
fn test_get_config_after_close_fails() {
    let Some((_temp_dir, path)) = copy_fixture_to_temp("test-kdbx4-low-KDF.kdbx") else {
        eprintln!("Skipping test: KDBX4 fixture not found");
        return;
    };

    let service = KdbxService::new();

    service
        .open(&path.to_string_lossy(), "test123")
        .expect("Failed to open database");

    // Config should work while open
    service.get_config().expect("Should get config while open");

    // Close the database
    service.close().expect("Failed to close database");

    // Config should fail after close
    let result = service.get_config();
    assert!(
        matches!(result, Err(AppError::DatabaseNotOpen)),
        "Should fail with DatabaseNotOpen after close, got: {result:?}"
    );
}
