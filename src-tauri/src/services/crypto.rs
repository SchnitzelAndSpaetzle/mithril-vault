// SPDX-License-Identifier: MIT

use crate::dto::error::AppError;

pub struct CryptoService;

impl CryptoService {
    /// Creates a crypto service.
    pub fn new() -> Self {
        Self
    }

    /// Derives a key from a password and salt.
    pub fn derive_key(&self, _password: &str, _salt: &[u8]) -> Result<Vec<u8>, AppError> {
        Err(AppError::NotImplemented("CryptoService::derive_key".into()))
    }

    /// Generates random bytes.
    pub fn generate_random_bytes(&self, _length: usize) -> Result<Vec<u8>, AppError> {
        Err(AppError::NotImplemented(
            "CryptoService::generate_random_bytes".into(),
        ))
    }
}

impl Default for CryptoService {
    fn default() -> Self {
        Self::new()
    }
}
