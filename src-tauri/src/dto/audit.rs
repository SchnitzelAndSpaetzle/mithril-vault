// SPDX-License-Identifier: MIT

//! IPC-facing audit DTOs.
//!
//! Event kinds are serialised camelCase (`vaultUnlockFailed`) so the frontend
//! never has to do string-keyed dispatch on dot-namespaced values.

use crate::services::audit::format::{AuditEvent, Reason};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AuditEventKindDto {
    VaultUnlockFailed,
    VaultOpened,
    VaultLocked,
}

/// Why a Vault transitioned from unlocked to locked, mirrored from
/// `services::audit::format::Reason` and rendered in camelCase over IPC
/// (everything else on the wire is camelCase).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReasonDto {
    Manual,
    AutoLock,
    AppQuit,
    ScreenLock,
}

impl From<Reason> for ReasonDto {
    fn from(reason: Reason) -> Self {
        match reason {
            Reason::Manual => Self::Manual,
            Reason::AutoLock => Self::AutoLock,
            Reason::AppQuit => Self::AppQuit,
            Reason::ScreenLock => Self::ScreenLock,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEventDto {
    pub kind: AuditEventKindDto,
    pub timestamp: DateTime<Utc>,
    pub attempt_count: Option<u32>,
    pub reason: Option<ReasonDto>,
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
                reason: None,
            },
            AuditEvent::VaultOpened { timestamp } => Self {
                kind: AuditEventKindDto::VaultOpened,
                timestamp,
                attempt_count: None,
                reason: None,
            },
            AuditEvent::VaultLocked { timestamp, reason } => Self {
                kind: AuditEventKindDto::VaultLocked,
                timestamp,
                attempt_count: None,
                reason: Some(reason.into()),
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
            reason: None,
        };
        let json = serde_json::to_string(&dto).expect("ser");
        assert!(json.contains("\"vaultUnlockFailed\""));
        assert!(json.contains("\"attemptCount\":2"));
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
    fn vault_opened_event_converts_to_dto_with_camel_case_kind() {
        let ts = Utc.with_ymd_and_hms(2026, 5, 15, 12, 0, 0).unwrap();
        let dto: AuditEventDto = AuditEvent::VaultOpened { timestamp: ts }.into();

        assert!(matches!(dto.kind, AuditEventKindDto::VaultOpened));
        assert_eq!(dto.timestamp, ts);
        assert!(dto.attempt_count.is_none());
        assert!(dto.reason.is_none());

        let json = serde_json::to_string(&dto).expect("ser");
        assert!(
            json.contains("\"vaultOpened\""),
            "kind must serialize as camelCase, got: {json}"
        );
    }

    #[test]
    fn vault_locked_event_converts_to_dto_with_camel_case_reason() {
        use crate::services::audit::format::Reason;
        let ts = Utc.with_ymd_and_hms(2026, 5, 15, 12, 0, 0).unwrap();

        for (reason, expected_wire) in [
            (Reason::Manual, "manual"),
            (Reason::AutoLock, "autoLock"),
            (Reason::AppQuit, "appQuit"),
            (Reason::ScreenLock, "screenLock"),
        ] {
            let dto: AuditEventDto = AuditEvent::VaultLocked {
                timestamp: ts,
                reason,
            }
            .into();
            assert!(matches!(dto.kind, AuditEventKindDto::VaultLocked));
            let json = serde_json::to_string(&dto).expect("ser");
            assert!(
                json.contains("\"vaultLocked\""),
                "kind must serialize as camelCase, got: {json}"
            );
            assert!(
                json.contains(&format!("\"reason\":\"{expected_wire}\"")),
                "reason must serialize as camelCase `{expected_wire}`, got: {json}"
            );
        }
    }

    #[test]
    fn filter_dto_defaults_to_everything() {
        let f: AuditFilterDto = serde_json::from_str("{}").expect("parse");
        assert!(f.kinds.is_none());
    }
}
