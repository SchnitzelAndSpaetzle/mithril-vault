// SPDX-License-Identifier: MIT

use crate::dto::error::AppError;
use tauri::{AppHandle, Manager, Runtime};

pub struct WindowProtectionService;

impl WindowProtectionService {
    /// Applies content-protection to every webview window owned by the app.
    ///
    /// On macOS this maps to `NSWindow.sharingType = NSWindowSharingNone`, on
    /// Windows to `SetWindowDisplayAffinity(WDA_EXCLUDEFROMCAPTURE)` (with a
    /// `WDA_MONITOR` fallback below Win10 2004), and on Linux it is a no-op.
    pub fn apply_to_all<R: Runtime>(app: &AppHandle<R>, enabled: bool) -> Result<(), AppError> {
        let windows = app.webview_windows();
        if windows.is_empty() {
            return Err(AppError::WindowProtection(
                "no webview windows registered".into(),
            ));
        }
        for (label, window) in windows {
            window
                .set_content_protected(enabled)
                .map_err(|e| AppError::WindowProtection(format!("window {label}: {e}")))?;
        }
        Ok(())
    }

    /// Reports whether the underlying platform actually enforces content
    /// protection. Linux returns `false` because Tauri's implementation is a
    /// no-op there.
    pub const fn is_supported() -> bool {
        cfg!(any(target_os = "macos", target_os = "windows"))
    }
}
