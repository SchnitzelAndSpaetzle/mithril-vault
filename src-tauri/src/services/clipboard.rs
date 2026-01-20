// SPDX-License-Identifier: MIT

use crate::dto::error::AppError;

pub struct ClipboardService;

impl ClipboardService {
    /// Creates a clipboard service.
    pub fn new() -> Self {
        Self
    }

    /// Copies text to the clipboard.
    pub fn copy(&self, _text: &str, _clear_after_secs: Option<u32>) -> Result<(), AppError> {
        Err(AppError::NotImplemented("ClipboardService::copy".into()))
    }

    /// Clears the clipboard.
    pub fn clear(&self) -> Result<(), AppError> {
        Err(AppError::NotImplemented("ClipboardService::clear".into()))
    }
}

impl Default for ClipboardService {
    fn default() -> Self {
        Self::new()
    }
}
