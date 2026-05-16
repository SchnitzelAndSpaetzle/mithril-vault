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
    #[serde(rename = "vault.opened")]
    VaultOpened { timestamp: DateTime<Utc> },
    #[serde(rename = "vault.locked")]
    VaultLocked {
        timestamp: DateTime<Utc>,
        reason: Reason,
    },
}

/// Why a Vault transitioned from unlocked to locked. Serialised as
/// `snake_case` to match the rest of the on-disk audit wire format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// Explicit lock from the UI.
    Manual,
    /// Auto-lock inactivity timer fired.
    AutoLock,
    /// App is quitting while the Vault was unlocked.
    AppQuit,
    /// OS screen-lock hook fired (kept on the wire even though no
    /// emitter is wired yet — see issue #217).
    ScreenLock,
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
    fn vault_opened_round_trips() {
        let event = AuditEvent::VaultOpened {
            timestamp: Utc.with_ymd_and_hms(2026, 5, 15, 12, 0, 0).unwrap(),
        };

        let bytes = event.to_bytes();
        let parsed = AuditEvent::from_bytes(&bytes).expect("parse");

        assert_eq!(parsed, event);

        // Wire shape: kind stays dot-namespaced (snake_case rename rule
        // keeps the literal we set on the variant).
        let json = std::str::from_utf8(&bytes).expect("utf8");
        assert!(
            json.contains("\"kind\":\"vault.opened\""),
            "kind must serialize as `vault.opened`, got: {json}",
        );
    }

    #[test]
    fn vault_locked_round_trips_each_reason() {
        for reason in [
            Reason::Manual,
            Reason::AutoLock,
            Reason::AppQuit,
            Reason::ScreenLock,
        ] {
            let event = AuditEvent::VaultLocked {
                timestamp: Utc.with_ymd_and_hms(2026, 5, 15, 12, 0, 0).unwrap(),
                reason,
            };
            let bytes = event.to_bytes();
            let parsed = AuditEvent::from_bytes(&bytes).expect("parse");
            assert_eq!(parsed, event);
        }
    }

    #[test]
    fn vault_locked_serialises_reason_as_snake_case() {
        // The wire format keeps the AuditEvent enum in snake_case; Reason
        // is rendered consistently with that so log files stay greppable.
        let event = AuditEvent::VaultLocked {
            timestamp: Utc.with_ymd_and_hms(2026, 5, 15, 12, 0, 0).unwrap(),
            reason: Reason::AutoLock,
        };
        let json = String::from_utf8(event.to_bytes()).expect("utf8");
        assert!(
            json.contains("\"kind\":\"vault.locked\""),
            "kind must be `vault.locked`, got: {json}"
        );
        assert!(
            json.contains("\"reason\":\"auto_lock\""),
            "reason must be snake_case `auto_lock`, got: {json}"
        );
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
