// SPDX-License-Identifier: MIT

use crate::dto::error::AppError;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub struct ClipboardService {
    /// Counter incremented on each copy; used to cancel pending clears.
    copy_generation: Arc<AtomicU64>,
}

impl ClipboardService {
    /// Creates a clipboard service.
    pub fn new() -> Self {
        Self {
            copy_generation: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Copies text to the clipboard. If `clear_after_secs` is `Some`, spawns
    /// an async task that clears the clipboard after the specified duration.
    /// A subsequent copy cancels any pending clear.
    pub fn copy(&self, text: &str, clear_after_secs: Option<u32>) -> Result<(), AppError> {
        let mut cb = arboard::Clipboard::new()
            .map_err(|e| AppError::Io(format!("Failed to access clipboard: {e}")))?;
        cb.set_text(text)
            .map_err(|e| AppError::Io(format!("Failed to copy to clipboard: {e}")))?;

        // Bump generation so any in-flight clear task becomes a no-op.
        let gen = self.copy_generation.fetch_add(1, Ordering::SeqCst) + 1;

        if let Some(secs) = clear_after_secs {
            let generation = Arc::clone(&self.copy_generation);
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(u64::from(secs))).await;
                // Only clear if no newer copy happened.
                if generation.load(Ordering::SeqCst) == gen {
                    if let Ok(mut cb) = arboard::Clipboard::new() {
                        let _ = cb.set_text("");
                    }
                }
            });
        }

        Ok(())
    }

    /// Clears the clipboard.
    pub fn clear(&self) -> Result<(), AppError> {
        // Bump generation to cancel any pending auto-clear.
        self.copy_generation.fetch_add(1, Ordering::SeqCst);

        let mut cb = arboard::Clipboard::new()
            .map_err(|e| AppError::Io(format!("Failed to access clipboard: {e}")))?;
        cb.set_text("")
            .map_err(|e| AppError::Io(format!("Failed to clear clipboard: {e}")))?;
        Ok(())
    }
}

impl Default for ClipboardService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_creates_service() {
        let service = ClipboardService::new();
        assert_eq!(service.copy_generation.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_default_creates_service() {
        let service = ClipboardService::default();
        assert_eq!(service.copy_generation.load(Ordering::SeqCst), 0);
    }

    // Note: copy/clear tests that access the real clipboard are integration tests
    // and may fail in headless CI environments. The generation-based cancellation
    // logic is tested via the atomic counter assertions below.

    #[test]
    fn test_generation_increments_on_clear() {
        let service = ClipboardService::new();
        assert_eq!(service.copy_generation.load(Ordering::SeqCst), 0);
        // clear bumps generation even if clipboard access fails in CI
        let _ = service.clear();
        assert_eq!(service.copy_generation.load(Ordering::SeqCst), 1);
    }
}
