// SPDX-License-Identifier: MIT

use crate::dto::audit::{AuditEventDto, AuditFilterDto};
use crate::dto::error::AppError;
use crate::services::audit::{AuditFilter, AuditService};
use std::path::Path;
use std::sync::Arc;
use tauri::State;

/// Returns the audit events recorded on this device for the Vault at
/// `vault_path`, newest-first.
///
/// The `_filter` argument is accepted but ignored in this tracer-bullet slice
/// (kinds-based and time-range filtering land in a follow-up). The wire shape
/// is in place so callers do not need to migrate later.
#[tauri::command]
pub async fn get_audit_events(
    vault_path: String,
    filter: Option<AuditFilterDto>,
    audit: State<'_, Arc<AuditService>>,
) -> Result<Vec<AuditEventDto>, AppError> {
    let _ = filter; // permissive default in this slice
    let events = audit.read(Path::new(&vault_path), &AuditFilter::default());
    let mut dtos: Vec<AuditEventDto> = events.into_iter().map(AuditEventDto::from).collect();
    dtos.sort_by_key(|e| std::cmp::Reverse(e.timestamp));
    Ok(dtos)
}
