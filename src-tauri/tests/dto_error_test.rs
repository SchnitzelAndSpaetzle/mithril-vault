// SPDX-License-Identifier: MIT
//! Tests for DTO error serialization

#![allow(clippy::expect_used)]

use mithril_vault_lib::dto::error::AppError;

#[test]
fn app_error_serializes_to_string() {
    let err = AppError::InvalidPassword;
    let json = serde_json::to_string(&err).expect("serialize error");
    assert_eq!(json, "\"Invalid password\"");
}

#[test]
fn app_error_from_io_converts_to_io_variant() {
    let err = std::io::Error::other("disk full");
    let app_err: AppError = err.into();
    assert!(matches!(app_err, AppError::Io(_)));
}
