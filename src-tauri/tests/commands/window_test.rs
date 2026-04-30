// SPDX-License-Identifier: MIT
//! Tests for window-protection command handlers.

#![allow(clippy::expect_used)]

use mithril_vault_lib::commands::window::{
    get_window_content_protection_supported, set_window_content_protected,
};
use mithril_vault_lib::dto::error::AppError;
use tauri::test::mock_app;

#[test]
fn get_supported_returns_compile_target() {
    let expected = cfg!(any(target_os = "macos", target_os = "windows"));
    let actual = tauri::async_runtime::block_on(get_window_content_protection_supported())
        .expect("get supported");
    assert_eq!(actual, expected);
}

#[test]
fn set_window_content_protected_errors_without_windows() {
    let app = mock_app();
    let err =
        tauri::async_runtime::block_on(set_window_content_protected(true, app.handle().clone()))
            .expect_err("expected window-protection error");
    assert!(matches!(err, AppError::WindowProtection(_)));
}
