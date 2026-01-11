// SPDX-License-Identifier: MIT

use crate::models::error::AppError;

pub struct KeychainService;

impl KeychainService {
    pub fn new() -> Self {
        Self
    }

    pub fn store(&self, _key: &str, _value: &str) -> Result<(), AppError> {
        Err(AppError::NotImplemented("KeychainService::store".into()))
    }

    pub fn retrieve(&self, _key: &str) -> Result<Option<String>, AppError> {
        Err(AppError::NotImplemented("KeychainService::retrieve".into()))
    }

    pub fn delete(&self, _key: &str) -> Result<(), AppError> {
        Err(AppError::NotImplemented("KeychainService::delete".into()))
    }
}

impl Default for KeychainService {
    fn default() -> Self {
        Self::new()
    }
}
