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
    Finding, FindingKind, HealthTotals, PasswordHealthReport, ReuseGroup,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PasswordHealthReportDto {
    pub score: Option<u32>,
    pub findings: Vec<FindingDto>,
    pub totals: HealthTotalsDto,
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
    #[serde(rename = "password.reused")]
    PasswordReused,
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

/// Wire shape per the PRD: only the member Entry ids — the per-analysis
/// hash bytes stay backend-internal. Hash values are key-randomized per
/// run, so leaking them would be a "what was in your Vault" signal even
/// across re-analyses; the frontend doesn't need them to render the
/// inline-expanded member list.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReuseGroupDto {
    pub entry_ids: Vec<String>,
}

impl From<PasswordHealthReport> for PasswordHealthReportDto {
    fn from(report: PasswordHealthReport) -> Self {
        Self {
            score: report.score,
            findings: report.findings.into_iter().map(FindingDto::from).collect(),
            totals: report.totals.into(),
            reuse_groups: report
                .reuse_groups
                .into_iter()
                .map(ReuseGroupDto::from)
                .collect(),
        }
    }
}

impl From<ReuseGroup> for ReuseGroupDto {
    fn from(group: ReuseGroup) -> Self {
        Self {
            entry_ids: group.entry_ids,
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
            FindingKind::PasswordReused => FindingKindDto::PasswordReused,
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

    /// `password.reused` is the wire identifier the frontend pattern
    /// matches on (matches the dotted convention shared by the other
    /// Finding Kinds). Pinning the exact JSON catches a refactor that
    /// silently renames the variant to `"PasswordReused"` or
    /// `"password_reused"` and breaks the report view.
    #[test]
    fn reused_finding_kind_serializes_as_namespaced_string() {
        assert_eq!(
            serde_json::to_string(&FindingKindDto::PasswordReused).expect("reused"),
            r#""password.reused""#
        );
    }

    /// The domain `FindingKind::PasswordReused` maps to the matching
    /// DTO variant. Pins the From impl so adding the new variant on
    /// either side doesn't silently lose information at the IPC edge.
    #[test]
    fn domain_to_dto_preserves_reused_variant() {
        assert_eq!(
            FindingKindDto::from(FindingKind::PasswordReused),
            FindingKindDto::PasswordReused,
        );
    }

    /// `ReuseGroupDto` carries only `entryIds` on the wire — the
    /// per-analysis hash bytes never leave Rust. Pinning the JSON
    /// shape catches a refactor that resurrects the previous
    /// `passwordHash` field.
    #[test]
    fn reuse_group_dto_serializes_with_entry_ids_only() {
        let dto = ReuseGroupDto {
            entry_ids: vec!["a".into(), "b".into()],
        };
        let value: serde_json::Value = serde_json::to_value(&dto).expect("serialize");
        let obj = value.as_object().expect("object");
        assert!(obj.contains_key("entryIds"));
        assert!(!obj.contains_key("passwordHash"));
        assert_eq!(obj.len(), 1, "ReuseGroupDto must expose only entryIds");
    }

    /// End-to-end conversion: a domain report carrying a `ReuseGroup`
    /// maps to a DTO whose `reuseGroups` Vec has one matching entry.
    /// Pins the new `From<PasswordHealthReport>` arm.
    #[test]
    fn report_dto_carries_reuse_groups_from_domain() {
        use crate::services::password_health::analyzer::ReuseGroup;

        let domain = PasswordHealthReport {
            score: Some(100),
            findings: Vec::new(),
            totals: HealthTotals::default(),
            reuse_groups: vec![ReuseGroup {
                entry_ids: vec!["a".into(), "b".into()],
            }],
        };

        let dto: PasswordHealthReportDto = domain.into();
        assert_eq!(dto.reuse_groups.len(), 1);
        assert_eq!(dto.reuse_groups[0].entry_ids, vec!["a", "b"]);
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
            reuse_groups: Vec::new(),
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
