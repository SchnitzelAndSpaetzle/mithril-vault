// SPDX-License-Identifier: MIT
//! Tests for audit command handlers.

#![allow(clippy::expect_used)]

use mithril_vault_lib::commands::{clear_audit_log, get_audit_events, get_audit_status};
use mithril_vault_lib::services::audit::key::InMemoryAuditKey;
use mithril_vault_lib::services::audit::AuditService;
use std::sync::Arc;
use tauri::test::mock_app;
use tauri::Manager;
use tempfile::TempDir;

fn setup() -> (
    tauri::App<tauri::test::MockRuntime>,
    Arc<AuditService>,
    TempDir,
) {
    let app = mock_app();
    let dir = tempfile::tempdir().expect("tempdir");
    let audit = Arc::new(AuditService::new(
        dir.path().join("audit"),
        Arc::new(InMemoryAuditKey::new()),
    ));
    app.manage(Arc::clone(&audit));
    (app, audit, dir)
}

/// Fresh `AuditService` reports `enabled: true` (default master gate)
/// and `degraded: false` (no failures yet). The command must reflect
/// those bits straight from the service so the Settings header renders
/// the correct initial state.
#[test]
fn get_audit_status_reports_enabled_and_not_degraded_for_fresh_service() {
    let (app, _audit, _dir) = setup();

    let status =
        tauri::async_runtime::block_on(get_audit_status(app.state())).expect("get_audit_status");
    assert!(status.enabled);
    assert!(!status.degraded);
}

/// Flipping the master gate via `AuditService::set_enabled(false)` must
/// surface as `enabled: false` over IPC — the Settings header uses this
/// to render the disabled-logging affordance.
#[test]
fn get_audit_status_reflects_disabled_master_gate() {
    let (app, audit, _dir) = setup();

    audit.set_enabled(false);
    let status =
        tauri::async_runtime::block_on(get_audit_status(app.state())).expect("get_audit_status");
    assert!(!status.enabled);
    assert!(!status.degraded);
}

/// A real read failure (directory staged at the per-Vault log path)
/// flips the session-wide `degraded` flag. The command surface must
/// expose that bit so the panel header banner can light up regardless
/// of which Vault the user is currently looking at.
#[test]
fn get_audit_status_reflects_degraded_after_internal_failure() {
    use mithril_vault_lib::services::audit::vault_id::hash_vault_path;
    use mithril_vault_lib::services::audit::AuditFilter;
    use std::path::Path;

    let app = mock_app();
    let dir = tempfile::tempdir().expect("tempdir");
    let base_dir = dir.path().join("audit");
    std::fs::create_dir_all(&base_dir).expect("base dir");
    let vault = dir.path().join("vault.kdbx");
    std::fs::write(&vault, b"x").expect("write vault");
    // Stage a directory at the per-Vault log path so the read trips an
    // IO error that flips `degraded`.
    let log_path = base_dir.join(format!("{}.jsonl", hash_vault_path(&vault)));
    std::fs::create_dir(&log_path).expect("stage dir at log path");

    let audit = Arc::new(AuditService::new(
        base_dir,
        Arc::new(InMemoryAuditKey::new()),
    ));
    app.manage(Arc::clone(&audit));

    // Trigger the failure path so `degraded` flips.
    let _ = audit.read(Path::new(&vault), &AuditFilter::default());

    let status =
        tauri::async_runtime::block_on(get_audit_status(app.state())).expect("get_audit_status");
    assert!(status.enabled);
    assert!(status.degraded);
}

/// Round-trip: `clear_audit_log` then `get_audit_events` returns the
/// surviving `auditCleared` event with `degraded: false`. Covers the
/// two thin audit command wrappers that previously had no
/// command-level coverage.
#[test]
fn clear_then_get_returns_audit_cleared_with_degraded_false() {
    let (app, _audit, dir) = setup();
    let vault = dir.path().join("vault.kdbx");
    std::fs::write(&vault, b"x").expect("write vault");
    let vault_path = vault.to_string_lossy().to_string();

    tauri::async_runtime::block_on(clear_audit_log(vault_path.clone(), app.state()))
        .expect("clear");

    let response = tauri::async_runtime::block_on(get_audit_events(vault_path, None, app.state()))
        .expect("get_audit_events");
    assert_eq!(response.events.len(), 1);
    assert!(!response.degraded);
}
