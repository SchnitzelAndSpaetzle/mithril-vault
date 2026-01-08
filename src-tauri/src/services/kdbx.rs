// SPDX-License-Identifier: GPL-3.0-or-later

use crate::models::error::AppError;

pub struct KdbxService;

impl KdbxService {
    pub fn new() -> Self {
        Self
    }

    pub fn open(&self, _path: &str, _password: &str) -> Result<(), AppError> {
        Err(AppError::NotImplemented("KdbxService::open".into()))
    }

    pub fn create(&self, _path: &str, _password: &str, _name: &str) -> Result<(), AppError> {
        Err(AppError::NotImplemented("KdbxService::create".into()))
    }

    pub fn save(&self) -> Result<(), AppError> {
        Err(AppError::NotImplemented("KdbxService::save".into()))
    }

    pub fn close(&self) -> Result<(), AppError> {
        Err(AppError::NotImplemented("KdbxService::close".into()))
    }
}

impl Default for KdbxService {
    fn default() -> Self {
        Self::new()
    }
}
