// SPDX-License-Identifier: MIT

//! IPC-facing types for the Password Health report.
//!
//! Wire shape is camelCase to match the rest of `dto::*`. Finding kinds
//! serialize as namespaced strings (`"password.expired"`, etc.) — same
//! convention as Audit Event Kinds. `reuseGroups` is on the wire from
//! day one even though the reuse check ships in slice 2; freezing the
//! shape now means the frontend's `usePasswordHealthReport` hook does
//! not have to migrate when reuse detection lands.

use serde::{Deserialize, Serialize};

use crate::services::password_health::analyzer::{
    Finding, FindingKind, HealthTotals, PasswordHealthReport,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PasswordHealthReportDto {
    pub score: Option<u32>,
    pub findings: Vec<FindingDto>,
    pub totals: HealthTotalsDto,
    /// Empty in this slice — the reuse check ships in slice 2.
    pub reuse_groups: Vec<ReuseGroupDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FindingDto {
    pub entry_id: String,
    pub kind: FindingKindDto,
}

/// Namespaced enum of Finding kinds. Serialized as the dotted string
/// the frontend pattern-matches on (`"password.expired"`, etc.). Adding
/// a new variant in a follow-up slice is wire-additive.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FindingKindDto {
    #[serde(rename = "password.very_weak")]
    PasswordVeryWeak,
    #[serde(rename = "password.weak")]
    PasswordWeak,
    #[serde(rename = "password.expired")]
    PasswordExpired,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HealthTotalsDto {
    pub critical: u32,
    pub high: u32,
    pub healthy: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReuseGroupDto {
    pub password_hash: String,
    pub entry_ids: Vec<String>,
}

impl From<PasswordHealthReport> for PasswordHealthReportDto {
    fn from(report: PasswordHealthReport) -> Self {
        Self {
            score: report.score,
            findings: report.findings.into_iter().map(FindingDto::from).collect(),
            totals: report.totals.into(),
            reuse_groups: Vec::new(),
        }
    }
}

impl From<Finding> for FindingDto {
    fn from(finding: Finding) -> Self {
        Self {
            entry_id: finding.entry_id,
            kind: finding.kind.into(),
        }
    }
}

impl From<FindingKind> for FindingKindDto {
    fn from(kind: FindingKind) -> Self {
        match kind {
            FindingKind::PasswordVeryWeak => FindingKindDto::PasswordVeryWeak,
            FindingKind::PasswordWeak => FindingKindDto::PasswordWeak,
            FindingKind::PasswordExpired => FindingKindDto::PasswordExpired,
        }
    }
}

impl From<HealthTotals> for HealthTotalsDto {
    fn from(totals: HealthTotals) -> Self {
        Self {
            critical: totals.critical,
            high: totals.high,
            healthy: totals.healthy,
            total: totals.total,
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    /// The frontend pattern-matches on the namespaced string. Pinning
    /// the JSON shape prevents a refactor inside `FindingKindDto` from
    /// silently moving the variant to `"PasswordExpired"` or
    /// `"password_expired"` and breaking the icon-and-section
    /// rendering in `EntryListItem` / `PasswordHealthReportView`.
    #[test]
    fn finding_kind_serializes_as_namespaced_string() {
        let json = serde_json::to_string(&FindingKindDto::PasswordExpired).expect("serialize");
        assert_eq!(json, r#""password.expired""#);
    }

    /// The strength-based Finding Kinds serialize on the same
    /// `password.<kind>` shape. The frontend `FindingKindSchema` is
    /// keyed off these exact strings — if a future refactor renamed
    /// them, the schema parse would reject every report.
    #[test]
    fn strength_finding_kinds_serialize_as_namespaced_strings() {
        assert_eq!(
            serde_json::to_string(&FindingKindDto::PasswordVeryWeak).expect("very_weak"),
            r#""password.very_weak""#
        );
        assert_eq!(
            serde_json::to_string(&FindingKindDto::PasswordWeak).expect("weak"),
            r#""password.weak""#
        );
    }

    /// End-to-end conversion from the domain `FindingKind` enum to
    /// the DTO must preserve the new strength variants. Pins the
    /// `From` impl so the two enums cannot drift.
    #[test]
    fn domain_to_dto_preserves_strength_variants() {
        assert_eq!(
            FindingKindDto::from(FindingKind::PasswordVeryWeak),
            FindingKindDto::PasswordVeryWeak
        );
        assert_eq!(
            FindingKindDto::from(FindingKind::PasswordWeak),
            FindingKindDto::PasswordWeak
        );
    }

    /// End-to-end conversion: a domain report with one expired Finding
    /// and a 1-high-of-4 totals breakdown maps to the wire-shape DTO
    /// with the corresponding fields and an empty `reuseGroups`
    /// vector. Pinning the conversion (not just the analyzer) catches
    /// drift between the analyzer and the IPC contract.
    #[test]
    fn report_dto_carries_score_findings_totals_and_empty_reuse_groups() {
        let domain = PasswordHealthReport {
            score: Some(75),
            findings: vec![Finding {
                entry_id: "abc-123".into(),
                kind: FindingKind::PasswordExpired,
            }],
            totals: HealthTotals {
                critical: 0,
                high: 1,
                healthy: 3,
                total: 4,
            },
        };

        let dto: PasswordHealthReportDto = domain.into();

        assert_eq!(dto.score, Some(75));
        assert_eq!(
            dto.findings,
            vec![FindingDto {
                entry_id: "abc-123".into(),
                kind: FindingKindDto::PasswordExpired,
            }]
        );
        assert_eq!(
            dto.totals,
            HealthTotalsDto {
                critical: 0,
                high: 1,
                healthy: 3,
                total: 4,
            }
        );
        assert!(dto.reuse_groups.is_empty());
    }

    /// The whole `PasswordHealthReportDto` serializes as a camelCase
    /// object. Pinning the JSON shape protects the frontend from a
    /// silent rename to `snake_case` — a recurring footgun on the
    /// boundary between Rust types and TypeScript consumers.
    #[test]
    fn report_dto_serializes_with_camelcase_keys() {
        let dto = PasswordHealthReportDto {
            score: Some(100),
            findings: Vec::new(),
            totals: HealthTotalsDto::default(),
            reuse_groups: Vec::new(),
        };
        let value: serde_json::Value = serde_json::to_value(&dto).expect("serialize");
        let obj = value.as_object().expect("object");
        assert!(obj.contains_key("score"));
        assert!(obj.contains_key("findings"));
        assert!(obj.contains_key("totals"));
        assert!(obj.contains_key("reuseGroups"));
        assert!(!obj.contains_key("reuse_groups"));
    }
}
