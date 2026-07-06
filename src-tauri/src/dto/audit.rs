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
    EntryPasswordRevealed,
    EntryPasswordCopied,
    EntryProtectedFieldRevealed,
    EntryAttachmentExported,
    EntryHistoryRestored,
    PreferencesSecurityChanged,
    AuditCleared,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attempt_count: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<ReasonDto>,
    /// KDBX UUID for entry-level kinds. Titles are deliberately resolved
    /// at render time from the open Vault's React Query cache so the
    /// on-disk log can never carry entry titles outside the Vault.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_id: Option<String>,
    /// Dot-pathed App Preference leaf (e.g. `security.preventScreenCapture`)
    /// for `preferences.security_changed`. Old/new values are deliberately
    /// not carried — the on-disk audit log records that a flip happened,
    /// not what it flipped to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setting_name: Option<String>,
    /// Filename (the per-Entry Attachment identifier) for
    /// `entry.attachment_exported`. The on-disk save path is deliberately
    /// not carried — the audit log records *which* Attachment left the
    /// Vault, never *where* it landed, per the "no titles/paths" rule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachment_id: Option<String>,
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

/// Snapshot of the audit subsystem's runtime state. Separate from
/// `AuditEventsResponseDto.degraded` because the Settings panel
/// renders a header indicator that must survive across Vault picks
/// without needing to (re)fetch a Vault-specific event list. The
/// header indicator clears on app restart since `degraded` is a
/// session-wide in-memory flag.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditStatusDto {
    pub enabled: bool,
    pub degraded: bool,
}

impl From<AuditEvent> for AuditEventDto {
    #[allow(clippy::too_many_lines)] // flat per-variant match; splitting hurts readability
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
                entry_id: None,
                setting_name: None,
                attachment_id: None,
            },
            AuditEvent::VaultOpened { timestamp } => Self {
                kind: AuditEventKindDto::VaultOpened,
                timestamp,
                attempt_count: None,
                reason: None,
                entry_id: None,
                setting_name: None,
                attachment_id: None,
            },
            AuditEvent::VaultLocked { timestamp, reason } => Self {
                kind: AuditEventKindDto::VaultLocked,
                timestamp,
                attempt_count: None,
                reason: Some(reason.into()),
                entry_id: None,
                setting_name: None,
                attachment_id: None,
            },
            AuditEvent::PreferencesSecurityChanged {
                timestamp,
                setting_name,
            } => Self {
                kind: AuditEventKindDto::PreferencesSecurityChanged,
                timestamp,
                attempt_count: None,
                reason: None,
                entry_id: None,
                setting_name: Some(setting_name),
                attachment_id: None,
            },
            AuditEvent::EntryPasswordRevealed {
                timestamp,
                entry_id,
            } => Self {
                kind: AuditEventKindDto::EntryPasswordRevealed,
                timestamp,
                attempt_count: None,
                reason: None,
                entry_id: Some(entry_id),
                setting_name: None,
                attachment_id: None,
            },
            AuditEvent::EntryPasswordCopied {
                timestamp,
                entry_id,
            } => Self {
                kind: AuditEventKindDto::EntryPasswordCopied,
                timestamp,
                attempt_count: None,
                reason: None,
                entry_id: Some(entry_id),
                setting_name: None,
                attachment_id: None,
            },
            AuditEvent::EntryProtectedFieldRevealed {
                timestamp,
                entry_id,
            } => Self {
                kind: AuditEventKindDto::EntryProtectedFieldRevealed,
                timestamp,
                attempt_count: None,
                reason: None,
                entry_id: Some(entry_id),
                setting_name: None,
                attachment_id: None,
            },
            AuditEvent::EntryAttachmentExported {
                timestamp,
                entry_id,
                attachment_id,
            } => Self {
                kind: AuditEventKindDto::EntryAttachmentExported,
                timestamp,
                attempt_count: None,
                reason: None,
                entry_id: Some(entry_id),
                setting_name: None,
                attachment_id: Some(attachment_id),
            },
            AuditEvent::EntryHistoryRestored {
                timestamp,
                entry_id,
            } => Self {
                kind: AuditEventKindDto::EntryHistoryRestored,
                timestamp,
                attempt_count: None,
                reason: None,
                entry_id: Some(entry_id),
                setting_name: None,
                attachment_id: None,
            },
            AuditEvent::AuditCleared { timestamp } => Self {
                kind: AuditEventKindDto::AuditCleared,
                timestamp,
                attempt_count: None,
                reason: None,
                entry_id: None,
                setting_name: None,
                attachment_id: None,
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
            entry_id: None,
            setting_name: None,
            attachment_id: None,
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
    fn entry_attachment_exported_event_converts_to_dto_with_entry_id_and_attachment_id() {
        let ts = Utc.with_ymd_and_hms(2026, 6, 9, 10, 0, 0).unwrap();
        let dto: AuditEventDto = AuditEvent::EntryAttachmentExported {
            timestamp: ts,
            entry_id: "uuid-att".to_string(),
            attachment_id: "recovery-codes.txt".to_string(),
        }
        .into();

        assert!(matches!(
            dto.kind,
            AuditEventKindDto::EntryAttachmentExported
        ));
        assert_eq!(dto.timestamp, ts);
        assert_eq!(dto.entry_id.as_deref(), Some("uuid-att"));
        assert_eq!(dto.attachment_id.as_deref(), Some("recovery-codes.txt"));
        assert!(dto.attempt_count.is_none());
        assert!(dto.reason.is_none());
        assert!(dto.setting_name.is_none());

        let json = serde_json::to_string(&dto).expect("ser");
        assert!(
            json.contains("\"entryAttachmentExported\""),
            "kind must serialize as camelCase, got: {json}"
        );
        assert!(json.contains("\"entryId\":\"uuid-att\""));
        assert!(
            json.contains("\"attachmentId\":\"recovery-codes.txt\""),
            "attachmentId must be camelCase on the wire, got: {json}"
        );
    }

    #[test]
    fn entry_history_restored_event_converts_to_dto_with_camel_case_kind_and_entry_id() {
        let ts = Utc.with_ymd_and_hms(2026, 6, 18, 10, 0, 0).unwrap();
        let dto: AuditEventDto = AuditEvent::EntryHistoryRestored {
            timestamp: ts,
            entry_id: "uuid-restore".to_string(),
        }
        .into();

        assert!(matches!(dto.kind, AuditEventKindDto::EntryHistoryRestored));
        assert_eq!(dto.timestamp, ts);
        assert_eq!(dto.entry_id.as_deref(), Some("uuid-restore"));
        assert!(dto.attempt_count.is_none());
        assert!(dto.reason.is_none());
        assert!(dto.attachment_id.is_none());

        let json = serde_json::to_string(&dto).expect("ser");
        assert!(
            json.contains("\"entryHistoryRestored\""),
            "kind must serialize as camelCase, got: {json}"
        );
        assert!(json.contains("\"entryId\":\"uuid-restore\""));
    }

    /// The clear-log surviving event reaches the UI as a camelCase
    /// `auditCleared` kind with no payload beyond the timestamp. The DTO
    /// translation has to mirror the wire-format rename done in
    /// `services::audit::format` so the row renderer can dispatch on a
    /// clean camelCase variant.
    #[test]
    fn audit_cleared_event_converts_to_dto_with_camel_case_kind() {
        let ts = Utc.with_ymd_and_hms(2026, 5, 17, 12, 0, 0).unwrap();
        let dto: AuditEventDto = AuditEvent::AuditCleared { timestamp: ts }.into();

        assert!(matches!(dto.kind, AuditEventKindDto::AuditCleared));
        assert_eq!(dto.timestamp, ts);
        assert!(dto.attempt_count.is_none());
        assert!(dto.reason.is_none());
        assert!(dto.entry_id.is_none());

        let json = serde_json::to_string(&dto).expect("ser");
        assert!(
            json.contains("\"auditCleared\""),
            "kind must serialize as camelCase, got: {json}",
        );
    }

    #[test]
    fn preferences_security_changed_event_converts_to_dto_with_camel_case_kind_and_setting_name() {
        let ts = Utc.with_ymd_and_hms(2026, 5, 17, 10, 0, 0).unwrap();
        let dto: AuditEventDto = AuditEvent::PreferencesSecurityChanged {
            timestamp: ts,
            setting_name: "security.preventScreenCapture".to_string(),
        }
        .into();

        assert!(matches!(
            dto.kind,
            AuditEventKindDto::PreferencesSecurityChanged
        ));
        assert_eq!(dto.timestamp, ts);
        assert_eq!(
            dto.setting_name.as_deref(),
            Some("security.preventScreenCapture")
        );
        assert!(dto.entry_id.is_none());
        assert!(dto.attempt_count.is_none());
        assert!(dto.reason.is_none());

        let json = serde_json::to_string(&dto).expect("ser");
        assert!(
            json.contains("\"preferencesSecurityChanged\""),
            "kind must serialize as camelCase, got: {json}"
        );
        assert!(
            json.contains("\"settingName\":\"security.preventScreenCapture\""),
            "settingName must be camelCase on the wire, got: {json}"
        );
        assert!(
            !json.contains("oldValue") && !json.contains("newValue"),
            "no old/new values must reach the wire, got: {json}"
        );
    }

    #[test]
    fn filter_dto_defaults_to_everything() {
        let f: AuditFilterDto = serde_json::from_str("{}").expect("parse");
        assert!(f.kinds.is_none());
    }

    /// The Settings → Audit Log panel header renders a degraded indicator
    /// from a session-wide flag — distinct from the response-level
    /// `degraded` returned by `get_audit_events`. The indicator must clear
    /// on app restart, so the wire shape carries `enabled` (the master gate
    /// state from preferences) plus `degraded` (session-wide, set whenever
    /// any audit operation has failed internally this session).
    #[test]
    fn status_dto_serializes_enabled_and_degraded_as_camel_case() {
        let dto = AuditStatusDto {
            enabled: true,
            degraded: false,
        };
        let json = serde_json::to_string(&dto).expect("ser");
        assert!(json.contains("\"enabled\":true"), "got: {json}");
        assert!(json.contains("\"degraded\":false"), "got: {json}");
    }
}
