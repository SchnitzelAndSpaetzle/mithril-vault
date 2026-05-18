// SPDX-License-Identifier: MIT

//! Pure-function Password Health analyzer.
//!
//! Takes an iterator of [`EntryInput`] — one per in-scope Entry — and
//! returns a [`PasswordHealthReport`]. Owns the score formula and the
//! finding-emission rules; owns no I/O, no Tauri, no `keepass-rs`. The
//! service layer is responsible for walking the unlocked Vault, filtering
//! Recycle Bin descendants and `password: None` Entries, and handing the
//! remaining cleartext over.
//!
//! The clock is injected (`now: DateTime<Utc>`) so the policy stays a
//! pure function — tests pin the clock; the service supplies
//! `Utc::now()`.

use chrono::{DateTime, Utc};

/// Per-Entry input the analyzer consumes. The service layer assembles
/// one of these for every in-scope Entry before invoking [`analyze`].
///
/// `expires` mirrors the KDBX `Times.expires` flag; `expiry_time` is the
/// associated timestamp. Both come straight from the Entry record — the
/// analyzer does not resolve "expired" until [`analyze`] compares them
/// against the injected `now`.
#[derive(Debug, Clone)]
pub struct EntryInput {
    pub id: String,
    pub expires: bool,
    pub expiry_time: Option<DateTime<Utc>>,
}

/// The namespaced enum of recordable Password Health findings. Only
/// `PasswordExpired` is emitted in this slice; the remaining kinds
/// (`PasswordVeryWeak`, `PasswordWeak`, `PasswordReused`) ship in
/// follow-up slices. See CONTEXT.md → "Password Health Finding Kind".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FindingKind {
    PasswordExpired,
}

/// A single recordable Password Health Finding. Each Finding is scoped
/// to exactly one Entry; an Entry that hits multiple checks produces
/// multiple Findings rather than one merged Finding because the
/// remediations differ (see ADR 0002 → consequences).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub entry_id: String,
    pub kind: FindingKind,
}

/// The output of [`analyze`] — a single periodic snapshot of every
/// in-scope Entry's health.
///
/// `score` is `None` for an empty Vault (no in-scope Entries to assess)
/// and `Some(0..=100)` otherwise. The score is computed as
/// `round(100 × healthy / total_in_scope)` — see ADR 0002 for why the
/// healthy-ratio formula was chosen over weighted finding deficits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasswordHealthReport {
    pub score: Option<u32>,
    pub findings: Vec<Finding>,
}

pub fn analyze(
    entries: impl IntoIterator<Item = EntryInput>,
    now: DateTime<Utc>,
) -> PasswordHealthReport {
    let entries: Vec<EntryInput> = entries.into_iter().collect();
    let total = entries.len();
    if total == 0 {
        return PasswordHealthReport {
            score: None,
            findings: Vec::new(),
        };
    }

    let findings: Vec<Finding> = entries
        .iter()
        .filter(|e| is_expired(e, now))
        .map(|e| Finding {
            entry_id: e.id.clone(),
            kind: FindingKind::PasswordExpired,
        })
        .collect();

    // In this slice, every emitted Finding is scoped to a distinct
    // Entry — there is only one Finding Kind — so `findings.len()`
    // equals the un-healthy Entry count. Once additional Finding Kinds
    // land, this collapse must move to a distinct-entry-id count.
    let unhealthy = findings.len();
    let healthy = total - unhealthy;
    // `total <= u32::MAX` in any realistic Vault; cast is safe.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let score = ((healthy as f64 / total as f64) * 100.0).round() as u32;

    PasswordHealthReport {
        score: Some(score),
        findings,
    }
}

fn is_expired(entry: &EntryInput, now: DateTime<Utc>) -> bool {
    entry.expires && entry.expiry_time.is_some_and(|t| t < now)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn now_fixed() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 18, 12, 0, 0).unwrap()
    }

    fn healthy(id: &str) -> EntryInput {
        EntryInput {
            id: id.to_string(),
            expires: false,
            expiry_time: None,
        }
    }

    fn expired(id: &str, now: DateTime<Utc>) -> EntryInput {
        EntryInput {
            id: id.to_string(),
            expires: true,
            expiry_time: Some(now - chrono::Duration::days(1)),
        }
    }

    /// An empty Vault (no in-scope Entries) cannot have a healthy ratio
    /// — division by zero — so the analyzer reports `score: None`. The
    /// `/dashboard/security/$dbId` view renders this as an em-dash and
    /// the "No passwords to analyze" empty state.
    #[test]
    fn empty_input_produces_score_none() {
        let report = analyze(std::iter::empty::<EntryInput>(), now_fixed());
        assert_eq!(report.score, None);
        assert!(report.findings.is_empty());
    }

    /// A Vault whose Entries all clear every check produces the maximum
    /// score, and no Findings are emitted. The sidebar badge is hidden
    /// at this score because the un-healthy count is zero.
    #[test]
    fn all_healthy_input_produces_score_100_and_no_findings() {
        let entries = vec![healthy("a"), healthy("b"), healthy("c"), healthy("d")];
        let report = analyze(entries, now_fixed());
        assert_eq!(report.score, Some(100));
        assert!(report.findings.is_empty());
    }

    /// One expired Entry in a four-Entry Vault: 3 healthy / 4 total =
    /// 75. Exactly one `PasswordExpired` Finding, scoped to the
    /// expired Entry's id. Pinned in the issue's acceptance criteria.
    #[test]
    fn single_expired_entry_in_four_yields_score_75_and_one_finding() {
        let now = now_fixed();
        let entries = vec![healthy("a"), healthy("b"), healthy("c"), expired("d", now)];
        let report = analyze(entries, now);
        assert_eq!(report.score, Some(75));
        assert_eq!(
            report.findings,
            vec![Finding {
                entry_id: "d".to_string(),
                kind: FindingKind::PasswordExpired,
            }]
        );
    }
}
