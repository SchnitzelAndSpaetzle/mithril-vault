// SPDX-License-Identifier: MIT

use crate::dto::audit::{AuditEventDto, AuditEventsResponseDto, AuditFilterDto, AuditStatusDto};
use crate::dto::error::AppError;
use crate::services::audit::{AuditFilter, AuditService};
use std::path::Path;
use std::sync::Arc;
use tauri::State;

/// Returns the audit events recorded on this device for the Vault at
/// `vault_path`, newest-first, alongside a session-wide `degraded` flag.
///
/// Hard read failures (key source down, log file unreadable) surface as
/// `AppError::AuditRead` so the UI renders a load-error state distinct
/// from "no events yet". The `degraded` flag separately signals "earlier
/// record calls failed; some history may be missing" — a state the read
/// itself may have succeeded around.
///
/// The `filter` argument is accepted but ignored in this tracer-bullet
/// slice (kinds-based and time-range filtering land in a follow-up); the
/// wire shape is in place so callers do not need to migrate later.
#[tauri::command]
pub async fn get_audit_events(
    vault_path: String,
    filter: Option<AuditFilterDto>,
    audit: State<'_, Arc<AuditService>>,
) -> Result<AuditEventsResponseDto, AppError> {
    let _ = filter; // permissive default in this slice
    let events = audit
        .read(Path::new(&vault_path), &AuditFilter::default())
        .map_err(|e| AppError::AuditRead(e.to_string()))?;
    let mut dtos: Vec<AuditEventDto> = events.into_iter().map(AuditEventDto::from).collect();
    dtos.sort_by_key(|e| std::cmp::Reverse(e.timestamp));
    Ok(AuditEventsResponseDto {
        events: dtos,
        degraded: audit.is_degraded(),
    })
}

/// Reports the audit subsystem's runtime state for the Settings panel
/// header: whether logging is on (the master gate) and whether the
/// session-wide `degraded` flag has tripped from any prior record/read
/// failure. The header indicator clears on app restart because
/// `degraded` lives in process memory — disabling-then-re-enabling
/// alone does not clear it.
#[tauri::command]
pub async fn get_audit_status(
    audit: State<'_, Arc<AuditService>>,
) -> Result<AuditStatusDto, AppError> {
    Ok(AuditStatusDto {
        enabled: audit.is_enabled(),
        degraded: audit.is_degraded(),
    })
}

/// Truncates the per-Vault audit log and leaves behind a single
/// `audit.cleared` event so the wipe is visible in the panel. The
/// underlying service performs the rewrite atomically so a mid-write
/// failure leaves the original file untouched; the caller sees that as
/// `AppError::AuditClear`.
#[tauri::command]
pub async fn clear_audit_log(
    vault_path: String,
    audit: State<'_, Arc<AuditService>>,
) -> Result<(), AppError> {
    audit
        .clear(Path::new(&vault_path))
        .map_err(|e| AppError::AuditClear(e.to_string()))
}
