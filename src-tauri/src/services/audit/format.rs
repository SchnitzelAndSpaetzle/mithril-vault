// SPDX-License-Identifier: MIT

//! Serialize / parse audit events to the wire format used by the on-disk log
//! (one JSON object per frame plaintext).
//!
//! Kept ignorant of crypto and I/O — this module decides only the shape of an
//! `AuditEvent` and how it round-trips through bytes.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FormatError {
    #[error("malformed audit event payload: {0}")]
    Malformed(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AuditEvent {
    #[serde(rename = "vault.unlock_failed")]
    VaultUnlockFailed {
        timestamp: DateTime<Utc>,
        attempt_count: u32,
    },
}

impl AuditEvent {
    pub fn to_bytes(&self) -> Vec<u8> {
        // serde_json::to_vec on a sound enum cannot fail; if it ever does, an
        // empty buffer round-trips as a parse error rather than a panic.
        serde_json::to_vec(self).unwrap_or_default()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, FormatError> {
        serde_json::from_slice(bytes).map_err(|e| FormatError::Malformed(e.to_string()))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn vault_unlock_failed_round_trips() {
        let event = AuditEvent::VaultUnlockFailed {
            timestamp: Utc.with_ymd_and_hms(2026, 5, 15, 12, 0, 0).unwrap(),
            attempt_count: 3,
        };

        let bytes = event.to_bytes();
        let parsed = AuditEvent::from_bytes(&bytes).expect("parse");

        assert_eq!(parsed, event);
    }

    #[test]
    fn malformed_input_is_rejected() {
        let bogus = b"not-json";
        assert!(matches!(
            AuditEvent::from_bytes(bogus),
            Err(FormatError::Malformed(_))
        ));
    }
}
