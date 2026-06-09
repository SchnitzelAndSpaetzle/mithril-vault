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
    #[serde(rename = "entry.password_revealed")]
    EntryPasswordRevealed {
        timestamp: DateTime<Utc>,
        entry_id: String,
    },
    #[serde(rename = "entry.password_copied")]
    EntryPasswordCopied {
        timestamp: DateTime<Utc>,
        entry_id: String,
    },
    #[serde(rename = "entry.protected_field_revealed")]
    EntryProtectedFieldRevealed {
        timestamp: DateTime<Utc>,
        entry_id: String,
    },
    #[serde(rename = "entry.attachment_exported")]
    EntryAttachmentExported {
        timestamp: DateTime<Utc>,
        entry_id: String,
        attachment_id: String,
    },
    #[serde(rename = "preferences.security_changed")]
    PreferencesSecurityChanged {
        timestamp: DateTime<Utc>,
        setting_name: String,
    },
    #[serde(rename = "audit.cleared")]
    AuditCleared { timestamp: DateTime<Utc> },
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

    #[test]
    fn entry_password_revealed_round_trips_with_entry_id() {
        let event = AuditEvent::EntryPasswordRevealed {
            timestamp: Utc.with_ymd_and_hms(2026, 5, 16, 10, 0, 0).unwrap(),
            entry_id: "8f1c2e3a-4b5d-6e7f-8091-a2b3c4d5e6f7".to_string(),
        };
        let parsed = AuditEvent::from_bytes(&event.to_bytes()).expect("parse");
        assert_eq!(parsed, event);
    }

    /// The on-disk JSON tag for entry-level kinds is the `entry.*`
    /// dot-namespaced string used in the PRD and ADR — not the camelCase
    /// DTO label. Pinning the literal here keeps already-written log
    /// files readable across refactors of the Rust enum.
    #[test]
    fn entry_password_revealed_serializes_with_dotted_kind() {
        let event = AuditEvent::EntryPasswordRevealed {
            timestamp: Utc.with_ymd_and_hms(2026, 5, 16, 10, 0, 0).unwrap(),
            entry_id: "abc".to_string(),
        };
        let json = String::from_utf8(event.to_bytes()).expect("utf8");
        assert!(
            json.contains("\"kind\":\"entry.password_revealed\""),
            "unexpected serialization: {json}"
        );
        assert!(json.contains("\"entry_id\":\"abc\""));
    }

    /// Entry-level kinds must carry `entry_id`; non-entry kinds must not.
    /// This pins the per-variant shape so a future refactor can't flatten
    /// the union into a free-form bag-of-optional-fields.
    #[test]
    fn entry_password_copied_round_trips_with_entry_id_and_dotted_kind() {
        let event = AuditEvent::EntryPasswordCopied {
            timestamp: Utc.with_ymd_and_hms(2026, 5, 16, 10, 0, 0).unwrap(),
            entry_id: "uuid-2".to_string(),
        };
        let parsed = AuditEvent::from_bytes(&event.to_bytes()).expect("parse");
        assert_eq!(parsed, event);
        let json = String::from_utf8(event.to_bytes()).expect("utf8");
        assert!(json.contains("\"kind\":\"entry.password_copied\""));
        assert!(json.contains("\"entry_id\":\"uuid-2\""));
    }

    #[test]
    fn entry_protected_field_revealed_round_trips_with_entry_id_and_dotted_kind() {
        let event = AuditEvent::EntryProtectedFieldRevealed {
            timestamp: Utc.with_ymd_and_hms(2026, 5, 16, 10, 0, 0).unwrap(),
            entry_id: "uuid-3".to_string(),
        };
        let parsed = AuditEvent::from_bytes(&event.to_bytes()).expect("parse");
        assert_eq!(parsed, event);
        let json = String::from_utf8(event.to_bytes()).expect("utf8");
        assert!(json.contains("\"kind\":\"entry.protected_field_revealed\""));
        assert!(json.contains("\"entry_id\":\"uuid-3\""));
    }

    /// A download writes an Attachment's bytes outside the Vault's encryption
    /// boundary, so it carries both `entry_id` and the `attachment_id` (the
    /// filename) — but never the on-disk path, per the Audit model's
    /// "no titles/paths" rule. The wire tag is pinned to the dotted form.
    #[test]
    fn entry_attachment_exported_round_trips_with_entry_id_and_attachment_id() {
        let event = AuditEvent::EntryAttachmentExported {
            timestamp: Utc.with_ymd_and_hms(2026, 5, 16, 10, 0, 0).unwrap(),
            entry_id: "uuid-4".to_string(),
            attachment_id: "recovery-codes.txt".to_string(),
        };
        let parsed = AuditEvent::from_bytes(&event.to_bytes()).expect("parse");
        assert_eq!(parsed, event);
        let json = String::from_utf8(event.to_bytes()).expect("utf8");
        assert!(json.contains("\"kind\":\"entry.attachment_exported\""));
        assert!(json.contains("\"entry_id\":\"uuid-4\""));
        assert!(json.contains("\"attachment_id\":\"recovery-codes.txt\""));
    }

    /// Manual clear of the audit log emits exactly one surviving event
    /// (`audit.cleared`) so a wipe is never silent. The wire tag is pinned
    /// to the dotted, snake-case form used by every other kind so already-
    /// written log files stay readable across refactors of the Rust enum.
    #[test]
    fn audit_cleared_round_trips_with_dotted_kind() {
        let event = AuditEvent::AuditCleared {
            timestamp: Utc.with_ymd_and_hms(2026, 5, 17, 12, 0, 0).unwrap(),
        };
        let bytes = event.to_bytes();
        let parsed = AuditEvent::from_bytes(&bytes).expect("parse");
        assert_eq!(parsed, event);

        let json = std::str::from_utf8(&bytes).expect("utf8");
        assert!(
            json.contains("\"kind\":\"audit.cleared\""),
            "kind must serialize as `audit.cleared`, got: {json}",
        );
        assert!(
            !json.contains("entry_id"),
            "audit.cleared must not carry entry_id: {json}",
        );
        assert!(
            !json.contains("attempt_count"),
            "audit.cleared must not carry attempt_count: {json}",
        );
    }

    #[test]
    fn preferences_security_changed_round_trips_with_setting_name() {
        let event = AuditEvent::PreferencesSecurityChanged {
            timestamp: Utc.with_ymd_and_hms(2026, 5, 17, 9, 30, 0).unwrap(),
            setting_name: "security.preventScreenCapture".to_string(),
        };
        let parsed = AuditEvent::from_bytes(&event.to_bytes()).expect("parse");
        assert_eq!(parsed, event);

        let json = String::from_utf8(event.to_bytes()).expect("utf8");
        assert!(
            json.contains("\"kind\":\"preferences.security_changed\""),
            "kind must serialize dot-namespaced, got: {json}"
        );
        assert!(
            json.contains("\"setting_name\":\"security.preventScreenCapture\""),
            "setting_name must serialize verbatim, got: {json}"
        );
    }

    /// The PRD is explicit: we record THAT a flip happened, not what it
    /// flipped TO. Pin the wire contract — adding an old/new field later
    /// has privacy consequences and must be a conscious decision, not a
    /// stray refactor.
    #[test]
    fn preferences_security_changed_wire_omits_old_and_new_values() {
        let event = AuditEvent::PreferencesSecurityChanged {
            timestamp: Utc.with_ymd_and_hms(2026, 5, 17, 9, 30, 0).unwrap(),
            setting_name: "security.preventScreenCapture".to_string(),
        };
        let json = String::from_utf8(event.to_bytes()).expect("utf8");
        assert!(
            !json.contains("old_value") && !json.contains("oldValue"),
            "old value must never appear on the wire, got: {json}"
        );
        assert!(
            !json.contains("new_value") && !json.contains("newValue"),
            "new value must never appear on the wire, got: {json}"
        );
    }

    #[test]
    fn vault_unlock_failed_payload_has_no_entry_id() {
        let event = AuditEvent::VaultUnlockFailed {
            timestamp: Utc.with_ymd_and_hms(2026, 5, 16, 10, 0, 0).unwrap(),
            attempt_count: 1,
        };
        let json = String::from_utf8(event.to_bytes()).expect("utf8");
        assert!(!json.contains("entry_id"), "unexpected entry_id: {json}");
    }
}
