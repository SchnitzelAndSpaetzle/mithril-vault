// SPDX-License-Identifier: MIT
//! Tests for `WindowProtectionService`.
//!
//! Tauri's mock runtime does not exercise the platform `set_content_protected`
//! code paths, so these tests verify our wiring (handle lookup, error mapping,
//! support reporting) rather than the OS-level behavior.

#![allow(clippy::expect_used)]

use mithril_vault_lib::dto::error::AppError;
use mithril_vault_lib::services::window_protection::WindowProtectionService;
use tauri::test::mock_app;

#[test]
fn apply_to_all_errors_when_no_windows_registered() {
    let app = mock_app();
    let err = WindowProtectionService::apply_to_all(app.handle(), true)
        .expect_err("expected window-protection error");
    assert!(matches!(err, AppError::WindowProtection(_)));
}

#[test]
fn is_supported_matches_compile_target() {
    let expected = cfg!(any(target_os = "macos", target_os = "windows"));
    assert_eq!(WindowProtectionService::is_supported(), expected);
}
