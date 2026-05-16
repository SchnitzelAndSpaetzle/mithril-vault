// SPDX-License-Identifier: MIT

//! IPC-facing audit DTOs.
//!
//! Event kinds are serialised camelCase (`vaultUnlockFailed`) so the frontend
//! never has to do string-keyed dispatch on dot-namespaced values.

use crate::services::audit::format::AuditEvent;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AuditEventKindDto {
    VaultUnlockFailed,
    EntryPasswordRevealed,
    EntryPasswordCopied,
    EntryProtectedFieldRevealed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEventDto {
    pub kind: AuditEventKindDto,
    pub timestamp: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attempt_count: Option<u32>,
    /// KDBX UUID for entry-level kinds. Titles are deliberately resolved
    /// at render time from the open Vault's React Query cache so the
    /// on-disk log can never carry entry titles outside the Vault.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AuditFilterDto {
    /// Optional list of kinds to include. Today's command accepts but does
    /// not enforce this — kept on the wire so the UI can stop sending all
    /// events back to itself once #8 lands.
    pub kinds: Option<Vec<AuditEventKindDto>>,
}

/// IPC response from `get_audit_events`. Carries the list of events plus a
/// session-wide `degraded` flag set by the backend whenever an audit
/// `record` or `read` has failed internally. The frontend uses `degraded`
/// to render a banner — without it, a soft failure would be visually
/// indistinguishable from "no events yet" and a real audit-subsystem
/// problem would never reach the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEventsResponseDto {
    pub events: Vec<AuditEventDto>,
    pub degraded: bool,
}

impl From<AuditEvent> for AuditEventDto {
    fn from(event: AuditEvent) -> Self {
        match event {
            AuditEvent::VaultUnlockFailed {
                timestamp,
                attempt_count,
            } => Self {
                kind: AuditEventKindDto::VaultUnlockFailed,
                timestamp,
                attempt_count: Some(attempt_count),
                entry_id: None,
            },
            AuditEvent::EntryPasswordRevealed {
                timestamp,
                entry_id,
            } => Self {
                kind: AuditEventKindDto::EntryPasswordRevealed,
                timestamp,
                attempt_count: None,
                entry_id: Some(entry_id),
            },
            AuditEvent::EntryPasswordCopied {
                timestamp,
                entry_id,
            } => Self {
                kind: AuditEventKindDto::EntryPasswordCopied,
                timestamp,
                attempt_count: None,
                entry_id: Some(entry_id),
            },
            AuditEvent::EntryProtectedFieldRevealed {
                timestamp,
                entry_id,
            } => Self {
                kind: AuditEventKindDto::EntryProtectedFieldRevealed,
                timestamp,
                attempt_count: None,
                entry_id: Some(entry_id),
            },
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn dto_uses_camel_case_kind_over_the_wire() {
        let dto = AuditEventDto {
            kind: AuditEventKindDto::VaultUnlockFailed,
            timestamp: Utc.with_ymd_and_hms(2026, 5, 15, 12, 0, 0).unwrap(),
            attempt_count: Some(2),
            entry_id: None,
        };
        let json = serde_json::to_string(&dto).expect("ser");
        assert!(json.contains("\"vaultUnlockFailed\""));
        assert!(json.contains("\"attemptCount\":2"));
        assert!(
            !json.contains("\"entryId\""),
            "entryId must be omitted when None"
        );
    }

    #[test]
    fn event_converts_to_dto() {
        let ts = Utc.with_ymd_and_hms(2026, 5, 15, 12, 0, 0).unwrap();
        let dto: AuditEventDto = AuditEvent::VaultUnlockFailed {
            timestamp: ts,
            attempt_count: 4,
        }
        .into();
        assert!(matches!(dto.kind, AuditEventKindDto::VaultUnlockFailed));
        assert_eq!(dto.timestamp, ts);
        assert_eq!(dto.attempt_count, Some(4));
    }

    #[test]
    fn entry_password_revealed_event_converts_to_dto_with_camel_case_kind_and_entry_id() {
        let ts = Utc.with_ymd_and_hms(2026, 5, 16, 10, 0, 0).unwrap();
        let dto: AuditEventDto = AuditEvent::EntryPasswordRevealed {
            timestamp: ts,
            entry_id: "uuid-abc".to_string(),
        }
        .into();

        assert!(matches!(dto.kind, AuditEventKindDto::EntryPasswordRevealed));
        assert_eq!(dto.timestamp, ts);
        assert_eq!(dto.entry_id.as_deref(), Some("uuid-abc"));
        assert!(dto.attempt_count.is_none());

        let json = serde_json::to_string(&dto).expect("ser");
        assert!(json.contains("\"entryPasswordRevealed\""));
        assert!(json.contains("\"entryId\":\"uuid-abc\""));
        assert!(
            !json.contains("attemptCount"),
            "attemptCount must be omitted for entry kinds: {json}"
        );
    }

    #[test]
    fn filter_dto_defaults_to_everything() {
        let f: AuditFilterDto = serde_json::from_str("{}").expect("parse");
        assert!(f.kinds.is_none());
    }
}
