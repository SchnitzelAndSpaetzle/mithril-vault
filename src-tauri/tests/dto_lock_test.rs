// SPDX-License-Identifier: MIT
//! Tests for lock DTO conversions

#![allow(clippy::expect_used)]

use chrono::Utc;
use mithril_vault_lib::dto::lock::{LockFileInfoDto, LockStatusDto};
use mithril_vault_lib::services::file_lock::{LockFileInfo, LockStatus};

#[test]
fn lock_file_info_converts_to_dto() {
    let info = LockFileInfo {
        pid: 1234,
        application: "MithrilVault".to_string(),
        version: "0.1.0".to_string(),
        opened_at: Utc::now(),
        hostname: "host".to_string(),
    };

    let dto: LockFileInfoDto = info.clone().into();
    assert_eq!(dto.pid, 1234);
    assert_eq!(dto.application, "MithrilVault");
    assert_eq!(dto.version, "0.1.0");
    assert_eq!(dto.hostname, "host");
    assert!(!dto.opened_at.is_empty());
}

#[test]
fn lock_status_converts_to_dto() {
    let info = LockFileInfo {
        pid: 99,
        application: "App".to_string(),
        version: "1.2.3".to_string(),
        opened_at: Utc::now(),
        hostname: "box".to_string(),
    };

    let available: LockStatusDto = LockStatus::Available.into();
    let current: LockStatusDto = LockStatus::LockedByCurrentProcess.into();
    let other: LockStatusDto = LockStatus::LockedByOtherProcess(info.clone()).into();
    let stale: LockStatusDto = LockStatus::StaleLock(info).into();

    let available_json = serde_json::to_string(&available).expect("serialize available");
    assert!(available_json.contains("\"status\":\"available\""));

    let current_json = serde_json::to_string(&current).expect("serialize current");
    assert!(current_json.contains("\"status\":\"lockedByCurrentProcess\""));

    let other_json = serde_json::to_string(&other).expect("serialize other");
    assert!(other_json.contains("\"status\":\"lockedByOtherProcess\""));

    let stale_json = serde_json::to_string(&stale).expect("serialize stale");
    assert!(stale_json.contains("\"status\":\"staleLock\""));
}
