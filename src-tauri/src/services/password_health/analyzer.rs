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

use std::collections::HashSet;

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
    /// Cleartext password for this Entry. Empty string is a valid
    /// in-scope value and triggers the empty-string Very-Weak path
    /// without consulting zxcvbn. The cleartext lives only inside the
    /// analyzer's stack frame and is dropped when `analyze` returns.
    pub password: String,
    pub expires: bool,
    pub expiry_time: Option<DateTime<Utc>>,
}

/// The namespaced enum of recordable Password Health findings. The
/// reuse check (`PasswordReused`) lands in a follow-up slice. See
/// CONTEXT.md → "Password Health Finding Kind".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FindingKind {
    PasswordVeryWeak,
    PasswordWeak,
    PasswordExpired,
}

/// Two-bucket severity used to populate [`HealthTotals`] and decide
/// which Section of the Security report an Entry surfaces in. Mirrors
/// the wording in ADR 0002: `Very Weak` is the only Critical Finding
/// Kind; `Weak`, `Reused`, and `Expired` are High. Healthy Entries
/// have no Findings and therefore no severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Critical,
    High,
}

impl FindingKind {
    pub fn severity(&self) -> Severity {
        match self {
            FindingKind::PasswordVeryWeak => Severity::Critical,
            FindingKind::PasswordWeak | FindingKind::PasswordExpired => Severity::High,
        }
    }
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

/// Severity-bucketed counts the report renders in the Security
/// dashboard's totals strip. `critical` and `high` count *distinct
/// Entries* with at least one Finding of that bucket — an Entry that
/// is both Reused and Very Weak is counted once in `critical`, not
/// twice. `healthy` counts Entries with zero Findings. `total` is the
/// in-scope denominator the score is computed against. All four are
/// zero for an empty Vault.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HealthTotals {
    pub critical: u32,
    pub high: u32,
    pub healthy: u32,
    pub total: u32,
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
    pub totals: HealthTotals,
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
            totals: HealthTotals::default(),
        };
    }

    let mut findings: Vec<Finding> = Vec::new();
    for entry in &entries {
        if let Some(kind) = classify_strength(&entry.password) {
            findings.push(Finding {
                entry_id: entry.id.clone(),
                kind,
            });
        }
        if is_expired(entry, now) {
            findings.push(Finding {
                entry_id: entry.id.clone(),
                kind: FindingKind::PasswordExpired,
            });
        }
    }

    // Bucket each Entry by its highest-severity Finding so
    // `critical + high + healthy == total` holds (the Security view
    // adds the three numbers; an Entry double-counted between buckets
    // would break the breakdown).
    let mut critical_ids: HashSet<&str> = HashSet::new();
    let mut high_ids: HashSet<&str> = HashSet::new();
    for f in &findings {
        match f.kind.severity() {
            Severity::Critical => {
                critical_ids.insert(f.entry_id.as_str());
            }
            Severity::High => {
                high_ids.insert(f.entry_id.as_str());
            }
        }
    }
    // Critical wins over High when an Entry has Findings in both
    // buckets — the more severe label is the one we surface.
    for id in &critical_ids {
        high_ids.remove(id);
    }
    let unhealthy = critical_ids.len() + high_ids.len();
    let healthy = total - unhealthy;

    // `total <= u32::MAX` in any realistic Vault; casts are safe.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let score = ((healthy as f64 / total as f64) * 100.0).round() as u32;
    #[allow(clippy::cast_possible_truncation)]
    let totals = HealthTotals {
        critical: critical_ids.len() as u32,
        high: high_ids.len() as u32,
        healthy: healthy as u32,
        total: total as u32,
    };

    PasswordHealthReport {
        score: Some(score),
        findings,
        totals,
    }
}

/// Classify a password's strength into a Finding Kind, or return
/// `None` if it does not warrant a Weak/Very-Weak Finding.
///
/// Empty strings are special-cased to Very Weak without invoking
/// zxcvbn — the crate's behaviour on empty input is not part of the
/// contract this analyzer wants to depend on. Non-empty passwords go
/// through `zxcvbn::zxcvbn` with no user-context inputs; score 0 maps
/// to Very Weak (Critical), score 1 to Weak (High), and everything
/// 2-or-higher is left un-flagged.
fn classify_strength(password: &str) -> Option<FindingKind> {
    if password.is_empty() {
        return Some(FindingKind::PasswordVeryWeak);
    }
    let score = u8::from(zxcvbn::zxcvbn(password, &[]).score());
    match score {
        0 => Some(FindingKind::PasswordVeryWeak),
        1 => Some(FindingKind::PasswordWeak),
        _ => None,
    }
}

fn is_expired(entry: &EntryInput, now: DateTime<Utc>) -> bool {
    // `KeePass` expiry semantics are "not in the future": an entry
    // whose expiry instant equals `now` is already expired. Using `<`
    // would leave a one-tick window where an exactly-expired entry
    // counts as healthy.
    entry.expires && entry.expiry_time.is_some_and(|t| t <= now)
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
            password: "correct horse battery staple".to_string(),
            expires: false,
            expiry_time: None,
        }
    }

    fn expired(id: &str, now: DateTime<Utc>) -> EntryInput {
        EntryInput {
            id: id.to_string(),
            password: "correct horse battery staple".to_string(),
            expires: true,
            expiry_time: Some(now - chrono::Duration::days(1)),
        }
    }

    fn with_password(id: &str, password: &str) -> EntryInput {
        EntryInput {
            id: id.to_string(),
            password: password.to_string(),
            expires: false,
            expiry_time: None,
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

    /// An empty Vault has no Entries in any bucket; every total is
    /// zero. Pairs with `empty_input_produces_score_none` — both
    /// behaviors share the same early-return path inside `analyze`.
    #[test]
    fn empty_input_totals_are_all_zero() {
        let report = analyze(std::iter::empty::<EntryInput>(), now_fixed());
        assert_eq!(report.totals, HealthTotals::default());
    }

    /// Severity-bucketed counts on a mixed Vault: two healthy + two
    /// expired Entries. Expired is a High Finding (per
    /// `FindingKind::severity`), so both expired Entries land in
    /// `high` and the two healthy land in `healthy`. `critical` stays
    /// zero — no Critical Findings exist in this slice. The sum
    /// `critical + high + healthy` must equal `total`.
    #[test]
    fn totals_bucket_distinct_entries_by_highest_severity() {
        let now = now_fixed();
        let entries = vec![
            healthy("a"),
            healthy("b"),
            expired("c", now),
            expired("d", now),
        ];
        let report = analyze(entries, now);
        assert_eq!(
            report.totals,
            HealthTotals {
                critical: 0,
                high: 2,
                healthy: 2,
                total: 4,
            }
        );
    }

    /// Entries whose expiry instant equals `now` must already count as
    /// expired — `KeePass` semantics are "not in the future", not "in
    /// the past". Pinning the boundary prevents a regression to the
    /// strict `<` comparison.
    #[test]
    fn entry_with_expiry_equal_to_now_is_expired() {
        let now = now_fixed();
        let entry = EntryInput {
            id: "boundary".to_string(),
            password: "correct horse battery staple".to_string(),
            expires: true,
            expiry_time: Some(now),
        };
        let report = analyze(vec![entry], now);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].kind, FindingKind::PasswordExpired);
    }

    /// A password the zxcvbn dictionary rates score 0 (the top hits in
    /// the leaked-credentials corpora) must emit Very Weak. "password"
    /// is the canonical example — it is the most common password ever
    /// observed in breach dumps, and zxcvbn scores it 0.
    #[test]
    fn zxcvbn_score_zero_password_emits_very_weak() {
        let report = analyze(vec![with_password("a", "password")], now_fixed());
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].kind, FindingKind::PasswordVeryWeak);
        assert_eq!(report.totals.critical, 1);
    }

    /// A password that zxcvbn rates score 1 (a common pattern with a
    /// small substitution — still trivially guessable within an hour
    /// of offline attack) emits Weak (High severity). The fixture
    /// string is chosen to be deterministic against the bundled
    /// dictionary; if a future zxcvbn release shifts the score this
    /// pin will fail loudly and we re-calibrate.
    #[test]
    fn zxcvbn_score_one_password_emits_weak_high() {
        let report = analyze(vec![with_password("a", "P@ssword1")], now_fixed());
        let score = u8::from(zxcvbn::zxcvbn("P@ssword1", &[]).score());
        assert_eq!(
            score, 1,
            "fixture password must score exactly 1 in the bundled zxcvbn dictionary; got {score}"
        );
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].kind, FindingKind::PasswordWeak);
        assert_eq!(FindingKind::PasswordWeak.severity(), Severity::High);
        assert_eq!(report.totals.high, 1);
        assert_eq!(report.totals.critical, 0);
    }

    /// Passwords zxcvbn rates ≥ 2 do not produce a Weak or Very-Weak
    /// Finding. The xkcd-style passphrase is the canonical
    /// "high-entropy memorable" example and scores 3; the analyzer
    /// must let it through. Pins the boundary against an over-eager
    /// future tweak.
    #[test]
    fn zxcvbn_score_two_or_higher_emits_no_strength_finding() {
        let report = analyze(vec![with_password("a", "Tr0ub4dor!")], now_fixed());
        let score = u8::from(zxcvbn::zxcvbn("Tr0ub4dor!", &[]).score());
        assert!(
            score >= 2,
            "fixture password must score ≥2 in zxcvbn; got {score}"
        );
        assert!(report.findings.is_empty());
        assert_eq!(report.totals.healthy, 1);
    }

    /// An Entry that is both Expired and zxcvbn-score-0 emits **two**
    /// independent Findings (the remediations differ — fix expiry vs.
    /// regenerate the password) but counts as **one** un-healthy
    /// contribution in the totals breakdown. The Entry lands in
    /// `critical` (the more severe of its two Findings), not in
    /// `high`; the un-healthy denominator is one, not two.
    #[test]
    fn weak_and_expired_on_same_entry_emits_two_findings_one_unhealthy() {
        let now = now_fixed();
        let past = now - chrono::Duration::days(1);
        let entry = EntryInput {
            id: "a".to_string(),
            password: "password".to_string(),
            expires: true,
            expiry_time: Some(past),
        };
        let report = analyze(vec![entry], now);
        assert_eq!(report.findings.len(), 2);
        let kinds: Vec<&FindingKind> = report.findings.iter().map(|f| &f.kind).collect();
        assert!(kinds.contains(&&FindingKind::PasswordVeryWeak));
        assert!(kinds.contains(&&FindingKind::PasswordExpired));
        assert_eq!(
            report.totals,
            HealthTotals {
                critical: 1,
                high: 0,
                healthy: 0,
                total: 1,
            }
        );
    }

    /// Empty-string passwords are special-cased to Very Weak without
    /// invoking zxcvbn (which has ambiguous behaviour on empty input).
    /// The Entry stays in scope — empty is what the analyzer is here
    /// to call out — and the resulting Finding is Critical, so the
    /// totals breakdown puts the Entry in `critical`, not `high`.
    #[test]
    fn empty_password_emits_one_very_weak_finding() {
        let report = analyze(vec![with_password("a", "")], now_fixed());
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].entry_id, "a");
        assert_eq!(report.findings[0].kind, FindingKind::PasswordVeryWeak);
        assert_eq!(FindingKind::PasswordVeryWeak.severity(), Severity::Critical);
        assert_eq!(report.totals.critical, 1);
        assert_eq!(report.totals.high, 0);
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
