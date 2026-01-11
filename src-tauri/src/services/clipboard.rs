// SPDX-License-Identifier: MIT

use crate::models::error::AppError;

pub struct ClipboardService;

impl ClipboardService {
    pub fn new() -> Self {
        Self
    }

    pub fn copy(&self, _text: &str, _clear_after_secs: Option<u32>) -> Result<(), AppError> {
        Err(AppError::NotImplemented("ClipboardService::copy".into()))
    }

    pub fn clear(&self) -> Result<(), AppError> {
        Err(AppError::NotImplemented("ClipboardService::clear".into()))
    }
}

impl Default for ClipboardService {
    fn default() -> Self {
        Self::new()
    }
}
