// SPDX-License-Identifier: MIT

use crate::domain::kdbx::format_database_version;
use crate::dto::database::{
    Compression, DatabaseConfigDto, DatabaseHeaderInfo, InnerCipher, KdfSettings, OuterCipher,
};
use crate::dto::error::AppError;
use keepass::config::{
    CompressionConfig, DatabaseVersion, InnerCipherConfig, KdfConfig, OuterCipherConfig,
};
use keepass::error::DatabaseIntegrityError;
use keepass::Database;
use std::fs::File;

use super::KdbxService;

impl KdbxService {
    /// Inspects a KDBX file without requiring credentials.
    /// Returns version and validity information that can be read from the file header.
    pub fn inspect(&self, path: &str) -> Result<DatabaseHeaderInfo, AppError> {
        let mut file = File::open(path).map_err(|e| AppError::InvalidPath(e.to_string()))?;

        match Database::get_version(&mut file) {
            Ok(version) => {
                let version_str = format_database_version(&version);
                let is_supported = is_version_supported(&version);

                Ok(DatabaseHeaderInfo {
                    version: version_str,
                    is_valid_kdbx: true,
                    is_supported,
                    path: path.to_string(),
                })
            }
            Err(err) => map_version_error(err, path),
        }
    }

    /// Returns the cryptographic configuration of the currently open database.
    /// Requires the database to be open (authenticated).
    pub fn get_config(&self) -> Result<DatabaseConfigDto, AppError> {
        let db_lock = self.database.lock().map_err(|_| AppError::Lock)?;
        let open_db = db_lock.as_ref().ok_or(AppError::DatabaseNotOpen)?;

        let config = &open_db.db.config;

        Ok(DatabaseConfigDto {
            version: open_db.version.clone(),
            outer_cipher: convert_outer_cipher(&config.outer_cipher_config),
            inner_cipher: convert_inner_cipher(&config.inner_cipher_config),
            compression: convert_compression(&config.compression_config),
            kdf: convert_kdf(&config.kdf_config),
        })
    }
}

/// Checks if a database version is supported by this application.
fn is_version_supported(version: &DatabaseVersion) -> bool {
    matches!(version, DatabaseVersion::KDB3(_) | DatabaseVersion::KDB4(_))
}

/// Maps version retrieval errors to appropriate `AppError` variants.
fn map_version_error(
    err: DatabaseIntegrityError,
    path: &str,
) -> Result<DatabaseHeaderInfo, AppError> {
    match err {
        DatabaseIntegrityError::InvalidKDBXIdentifier => Err(AppError::InvalidKdbxFile),
        DatabaseIntegrityError::InvalidKDBXVersion {
            version,
            file_major_version,
            file_minor_version,
        } => {
            // File has valid KDBX magic bytes but unsupported version
            let version_str =
                format!("KDBX {file_major_version}.{file_minor_version} (internal: {version})");
            Ok(DatabaseHeaderInfo {
                version: version_str.clone(),
                is_valid_kdbx: true,
                is_supported: false,
                path: path.to_string(),
            })
        }
        other => Err(AppError::HeaderParseError(other.to_string())),
    }
}

/// Converts keepass `OuterCipherConfig` to our `OuterCipher` DTO.
fn convert_outer_cipher(config: &OuterCipherConfig) -> OuterCipher {
    match config {
        OuterCipherConfig::AES256 => OuterCipher::Aes256,
        OuterCipherConfig::Twofish => OuterCipher::Twofish,
        OuterCipherConfig::ChaCha20 => OuterCipher::ChaCha20,
    }
}

/// Converts keepass `InnerCipherConfig` to our `InnerCipher` DTO.
fn convert_inner_cipher(config: &InnerCipherConfig) -> InnerCipher {
    match config {
        InnerCipherConfig::Plain => InnerCipher::Plain,
        InnerCipherConfig::Salsa20 => InnerCipher::Salsa20,
        InnerCipherConfig::ChaCha20 => InnerCipher::ChaCha20,
    }
}

/// Converts keepass `CompressionConfig` to our `Compression` DTO.
fn convert_compression(config: &CompressionConfig) -> Compression {
    match config {
        CompressionConfig::None => Compression::None,
        CompressionConfig::GZip => Compression::GZip,
    }
}

/// Converts keepass `KdfConfig` to our `KdfSettings` DTO.
fn convert_kdf(config: &KdfConfig) -> KdfSettings {
    match config {
        KdfConfig::Aes { rounds } => KdfSettings::AesKdf { rounds: *rounds },
        KdfConfig::Argon2 {
            memory,
            iterations,
            parallelism,
            ..
        } => KdfSettings::Argon2d {
            memory: *memory,
            iterations: *iterations,
            parallelism: *parallelism,
        },
        KdfConfig::Argon2id {
            memory,
            iterations,
            parallelism,
            ..
        } => KdfSettings::Argon2id {
            memory: *memory,
            iterations: *iterations,
            parallelism: *parallelism,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_outer_cipher_aes256() {
        assert_eq!(
            convert_outer_cipher(&OuterCipherConfig::AES256),
            OuterCipher::Aes256
        );
    }

    #[test]
    fn test_convert_outer_cipher_twofish() {
        assert_eq!(
            convert_outer_cipher(&OuterCipherConfig::Twofish),
            OuterCipher::Twofish
        );
    }

    #[test]
    fn test_convert_outer_cipher_chacha20() {
        assert_eq!(
            convert_outer_cipher(&OuterCipherConfig::ChaCha20),
            OuterCipher::ChaCha20
        );
    }

    #[test]
    fn test_convert_inner_cipher_plain() {
        assert_eq!(
            convert_inner_cipher(&InnerCipherConfig::Plain),
            InnerCipher::Plain
        );
    }

    #[test]
    fn test_convert_inner_cipher_salsa20() {
        assert_eq!(
            convert_inner_cipher(&InnerCipherConfig::Salsa20),
            InnerCipher::Salsa20
        );
    }

    #[test]
    fn test_convert_inner_cipher_chacha20() {
        assert_eq!(
            convert_inner_cipher(&InnerCipherConfig::ChaCha20),
            InnerCipher::ChaCha20
        );
    }

    #[test]
    fn test_convert_compression_none() {
        assert_eq!(
            convert_compression(&CompressionConfig::None),
            Compression::None
        );
    }

    #[test]
    fn test_convert_compression_gzip() {
        assert_eq!(
            convert_compression(&CompressionConfig::GZip),
            Compression::GZip
        );
    }

    #[test]
    fn test_convert_kdf_aes() {
        let kdf = KdfConfig::Aes { rounds: 60000 };
        assert_eq!(convert_kdf(&kdf), KdfSettings::AesKdf { rounds: 60000 });
    }

    #[test]
    fn test_convert_kdf_argon2d() {
        let kdf = KdfConfig::Argon2 {
            memory: 65536,
            iterations: 3,
            parallelism: 4,
            version: argon2::Version::Version13,
        };
        assert_eq!(
            convert_kdf(&kdf),
            KdfSettings::Argon2d {
                memory: 65536,
                iterations: 3,
                parallelism: 4
            }
        );
    }

    #[test]
    fn test_convert_kdf_argon2id() {
        let kdf = KdfConfig::Argon2id {
            memory: 65536,
            iterations: 3,
            parallelism: 4,
            version: argon2::Version::Version13,
        };
        assert_eq!(
            convert_kdf(&kdf),
            KdfSettings::Argon2id {
                memory: 65536,
                iterations: 3,
                parallelism: 4
            }
        );
    }

    #[test]
    fn test_is_version_supported_kdbx3() {
        assert!(is_version_supported(&DatabaseVersion::KDB3(1)));
    }

    #[test]
    fn test_is_version_supported_kdbx4() {
        assert!(is_version_supported(&DatabaseVersion::KDB4(0)));
    }

    #[test]
    fn test_is_version_not_supported_kdb() {
        assert!(!is_version_supported(&DatabaseVersion::KDB(0)));
    }

    #[test]
    fn test_is_version_not_supported_kdb2() {
        assert!(!is_version_supported(&DatabaseVersion::KDB2(0)));
    }
}
