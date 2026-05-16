// SPDX-License-Identifier: MIT

//! Retention policy for the audit log.
//!
//! Stub for the tracer-bullet slice. The real age-based + size-cap policy
//! lands in a follow-up issue; for now [`apply_retention`] is a no-op so
//! `AuditService::record` can call it without conditionals.

use std::path::Path;

/// Applies retention to the audit log file at `_path`. No-op until the
/// follow-up implementation lands.
pub fn apply_retention(_path: &Path) {
    // intentionally empty
}
