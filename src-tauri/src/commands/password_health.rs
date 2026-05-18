// SPDX-License-Identifier: MIT

//! Tauri commands for the Password Health report.
//!
//! In this slice the command surface is the single
//! [`get_password_health_report`] entry point. It returns a complete
//! report synchronously per call; progressive analysis via Tauri
//! events ships in the follow-up cycle. Cancellation will surface
//! through a separate command then.

use std::sync::Arc;

use chrono::Utc;
use tauri::State;

use crate::dto::error::AppError;
use crate::dto::password_health::PasswordHealthReportDto;
use crate::services::kdbx::KdbxService;
use crate::services::password_health::service::PasswordHealthService;

/// Returns the Password Health report for the Vault at `db_id`.
///
/// Cache-keyed on `(db_id, generation)` — repeat calls against an
/// unchanged Vault return the cached report; a call after a write
/// (one that flipped `VaultMut::mark_modified`) returns a freshly-
/// computed report. The clock is `Utc::now()` here so callers in
/// tests pin time at the service-level entry point rather than at
/// the IPC boundary.
#[tauri::command]
pub async fn get_password_health_report(
    db_id: String,
    kdbx: State<'_, Arc<KdbxService>>,
    health: State<'_, Arc<PasswordHealthService>>,
) -> Result<PasswordHealthReportDto, AppError> {
    let report = health.generate_report(&kdbx, &db_id, Utc::now())?;
    Ok(report.into())
}
