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

use std::collections::{BTreeMap, HashSet};

use chrono::{DateTime, Utc};
use rand::rand_core::TryRng;
use rand::rngs::SysRng;

use crate::domain::secure::SecureString;

/// Per-Entry input the analyzer consumes. The service layer assembles
/// one of these for every in-scope Entry before invoking [`analyze`].
///
/// `expires` mirrors the KDBX `Times.expires` flag; `expiry_time` is the
/// associated timestamp. Both come straight from the Entry record — the
/// analyzer does not resolve "expired" until [`analyze`] compares them
/// against the injected `now`.
/// `Debug` is derived but the [`SecureString`] password field renders
/// as `[REDACTED]`, so panic-printing or accidental logging cannot
/// leak the cleartext. `Clone` is intentionally **not** derived —
/// every additional copy is another heap region the analyzer has to
/// zeroize, and the pipeline never needs more than one copy per
/// Entry (the analyzer borrows). Consumers that need another copy
/// should clone the inner `SecureString` explicitly.
#[derive(Debug)]
pub struct EntryInput {
    pub id: String,
    /// Cleartext password for this Entry, held in a zeroizing
    /// wrapper. Empty string is a valid in-scope value and triggers
    /// the empty-string Very-Weak path without consulting zxcvbn.
    /// The cleartext lives only inside the analyzer's stack frame
    /// and is wiped from memory when `analyze` drops the `Vec`.
    pub password: SecureString,
    pub expires: bool,
    pub expiry_time: Option<DateTime<Utc>>,
}

/// The namespaced enum of recordable Password Health findings. See
/// CONTEXT.md → "Password Health Finding Kind".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FindingKind {
    PasswordVeryWeak,
    PasswordWeak,
    PasswordReused,
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
            FindingKind::PasswordWeak
            | FindingKind::PasswordReused
            | FindingKind::PasswordExpired => Severity::High,
        }
    }
}

/// A set of in-scope Entries in this Vault whose passwords are byte-equal.
/// Each member is also emitted as an individual `PasswordReused` Finding;
/// the group exists so the UI can render one expandable row per shared
/// password instead of N independent rows. `entry_ids` always has size ≥ 2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReuseGroup {
    pub entry_ids: Vec<String>,
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
    /// One entry per byte-equal-password cluster of size ≥ 2.
    /// Empty-string passwords are excluded — they reach `findings` as
    /// `PasswordVeryWeak` but never group together.
    pub reuse_groups: Vec<ReuseGroup>,
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
            reuse_groups: Vec::new(),
        };
    }

    let reuse_groups = compute_reuse_groups(&entries);
    let reused_ids: HashSet<&str> = reuse_groups
        .iter()
        .flat_map(|g| g.entry_ids.iter().map(String::as_str))
        .collect();

    let mut findings: Vec<Finding> = Vec::new();
    for entry in &entries {
        if let Some(kind) = classify_strength(entry.password.as_str()) {
            findings.push(Finding {
                entry_id: entry.id.clone(),
                kind,
            });
        }
        if reused_ids.contains(entry.id.as_str()) {
            findings.push(Finding {
                entry_id: entry.id.clone(),
                kind: FindingKind::PasswordReused,
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
        reuse_groups,
    }
}

/// Cluster Entries by byte-equal password and return one
/// [`ReuseGroup`] per cluster of size ≥ 2.
///
/// Passwords are hashed with keyed BLAKE3 using a fresh 32-byte key
/// generated per call so the in-memory hash bytes are unlinkable
/// across analysis runs. The cluster membership is independent of
/// the key — same inputs always yield the same partition; only the
/// hash bytes (used internally as map keys) shift between runs.
/// Empty-string passwords are skipped (per ADR 0002): the Very-Weak
/// Finding from [`classify_strength`] already names the remediation,
/// and an "all empty" group isn't a meaningful "shared secret".
fn compute_reuse_groups(entries: &[EntryInput]) -> Vec<ReuseGroup> {
    let mut hash_key = [0u8; blake3::KEY_LEN];
    // `SysRng::try_fill_bytes` can only fail if the OS RNG itself
    // fails — at that point the process can't safely produce any
    // randomness and reuse grouping is the least of our worries. Fall
    // back to an all-zero key: the partition is still correct (the
    // hash is deterministic for that run) and the only loss is the
    // per-run unlinkability of the in-memory bytes, which we treat
    // as best-effort rather than a soundness guarantee.
    if SysRng.try_fill_bytes(&mut hash_key).is_err() {
        hash_key = [0u8; blake3::KEY_LEN];
    }
    // `BTreeMap` keeps iteration order stable across runs (only the
    // hash bytes — used as keys — shift). We reorder by first-seen
    // Entry index below so the wire ordering doesn't leak the
    // per-run randomness.
    let mut by_hash: BTreeMap<[u8; blake3::OUT_LEN], Vec<&str>> = BTreeMap::new();
    for entry in entries {
        let password = entry.password.as_str();
        if password.is_empty() {
            continue;
        }
        let hash = blake3::keyed_hash(&hash_key, password.as_bytes());
        by_hash
            .entry(*hash.as_bytes())
            .or_default()
            .push(entry.id.as_str());
    }

    let order: std::collections::HashMap<&str, usize> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| (e.id.as_str(), i))
        .collect();
    let mut groups: Vec<ReuseGroup> = by_hash
        .into_values()
        .filter(|ids| ids.len() >= 2)
        .map(|ids| ReuseGroup {
            entry_ids: ids.into_iter().map(String::from).collect(),
        })
        .collect();
    for g in &mut groups {
        g.entry_ids
            .sort_by_key(|id| *order.get(id.as_str()).unwrap_or(&usize::MAX));
    }
    groups.sort_by_key(|g| {
        g.entry_ids
            .first()
            .and_then(|id| order.get(id.as_str()).copied())
            .unwrap_or(usize::MAX)
    });
    groups
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
            // Per-id suffix so the helper Entries don't reuse-group
            // with one another. The string still scores ≥ 2 on zxcvbn
            // because the diceware prefix dominates the entropy.
            password: SecureString::from(format!("correct horse battery staple {id}")),
            expires: false,
            expiry_time: None,
        }
    }

    fn expired(id: &str, now: DateTime<Utc>) -> EntryInput {
        EntryInput {
            id: id.to_string(),
            password: SecureString::from(format!("correct horse battery staple {id}")),
            expires: true,
            expiry_time: Some(now - chrono::Duration::days(1)),
        }
    }

    fn with_password(id: &str, password: &str) -> EntryInput {
        EntryInput {
            id: id.to_string(),
            password: SecureString::from(password),
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
            password: SecureString::from("correct horse battery staple"),
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
            password: SecureString::from("password"),
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

    /// The cleartext password on `EntryInput` is held in a
    /// [`SecureString`] so a stray `{:?}` or `panic!` never leaks it.
    /// If a future refactor swaps the field back to plain `String`,
    /// the assertion below would print the actual password instead of
    /// `[REDACTED]` and this test would fail loudly.
    #[test]
    fn entry_input_debug_redacts_cleartext_password() {
        let entry = with_password("a", "hunter2-leak-canary");
        let debug = format!("{entry:?}");
        assert!(
            !debug.contains("hunter2-leak-canary"),
            "EntryInput Debug must redact the cleartext password; got: {debug}"
        );
        assert!(
            debug.contains("[REDACTED]"),
            "EntryInput Debug must show [REDACTED] for the password; got: {debug}"
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

    /// Two in-scope Entries sharing a byte-equal non-empty password
    /// must each emit a `PasswordReused` Finding, and the report must
    /// carry exactly one `ReuseGroup` whose `entry_ids` contain both
    /// Entry ids. Pinned in the issue's acceptance criteria.
    #[test]
    fn two_entries_with_same_password_emit_reused_findings_and_one_group() {
        let report = analyze(
            vec![
                with_password("a", "shared-passw0rd-Tr0ub4dor!"),
                with_password("b", "shared-passw0rd-Tr0ub4dor!"),
            ],
            now_fixed(),
        );

        let reused: Vec<&Finding> = report
            .findings
            .iter()
            .filter(|f| f.kind == FindingKind::PasswordReused)
            .collect();
        let reused_ids: HashSet<&str> = reused.iter().map(|f| f.entry_id.as_str()).collect();
        assert_eq!(reused_ids, HashSet::from(["a", "b"]));

        assert_eq!(report.reuse_groups.len(), 1);
        let group_ids: HashSet<&str> = report.reuse_groups[0]
            .entry_ids
            .iter()
            .map(String::as_str)
            .collect();
        assert_eq!(group_ids, HashSet::from(["a", "b"]));
    }

    /// Determinism: the per-analysis hash-key randomness must not
    /// influence grouping outcomes. Two runs with the same input
    /// Entries must produce `reuse_groups` whose memberships and
    /// order match exactly. (The raw hash bytes underlying the
    /// partition shift between runs, but `reuse_groups` only carries
    /// `entry_ids`.) Pinned in the issue's acceptance criteria.
    #[test]
    fn reuse_groups_deterministic_across_runs() {
        let inputs = || {
            vec![
                with_password("a", "Tr0ub4dor-staple-horse-d1"),
                with_password("b", "Tr0ub4dor-staple-horse-d2"),
                with_password("c", "Tr0ub4dor-staple-horse-d1"),
                with_password("d", "Tr0ub4dor-staple-horse-d2"),
                with_password("e", "Tr0ub4dor-staple-horse-d1"),
            ]
        };
        let r1 = analyze(inputs(), now_fixed());
        let r2 = analyze(inputs(), now_fixed());
        assert_eq!(
            r1.reuse_groups, r2.reuse_groups,
            "reuse_groups must be deterministic across runs"
        );
        // Two groups expected: {a, c, e} and {b, d}, in first-seen
        // input order. Pin the exact ordering to catch a regression
        // that lets the hash-key randomness leak through.
        assert_eq!(r1.reuse_groups.len(), 2);
        assert_eq!(
            r1.reuse_groups[0].entry_ids,
            vec!["a".to_string(), "c".to_string(), "e".to_string()]
        );
        assert_eq!(
            r1.reuse_groups[1].entry_ids,
            vec!["b".to_string(), "d".to_string()]
        );
    }

    /// An Entry that is both Reused (with one other Entry) and
    /// zxcvbn-score-0 (Very Weak) emits two independent Findings —
    /// remediations differ — but lands in the `critical` totals
    /// bucket once. The reused partner is the only other Entry in
    /// scope and emits its own Reused Finding; the unhealthy
    /// denominator is two, not three. Pinned in the issue's
    /// acceptance criteria.
    #[test]
    fn reused_and_very_weak_on_same_entry_emits_two_findings_one_unhealthy() {
        // "password" is the canonical zxcvbn-score-0 string. Both
        // Entries share it, so both also reuse-flag.
        let report = analyze(
            vec![
                with_password("a", "password"),
                with_password("b", "password"),
            ],
            now_fixed(),
        );

        // Both Entries get the Reused Finding (group of two).
        let reused_count = report
            .findings
            .iter()
            .filter(|f| f.kind == FindingKind::PasswordReused)
            .count();
        assert_eq!(reused_count, 2, "each member of the group must emit Reused");

        // Both Entries also get Very Weak (zxcvbn score 0).
        let very_weak_count = report
            .findings
            .iter()
            .filter(|f| f.kind == FindingKind::PasswordVeryWeak)
            .count();
        assert_eq!(
            very_weak_count, 2,
            "each Entry must independently emit Very Weak"
        );

        // Critical wins over High when an Entry has Findings in both
        // buckets. Both Entries land in `critical`.
        assert_eq!(
            report.totals,
            HealthTotals {
                critical: 2,
                high: 0,
                healthy: 0,
                total: 2,
            }
        );
    }

    /// Three in-scope Entries sharing one byte-equal password produce
    /// three `PasswordReused` Findings (one per Entry) and a single
    /// `ReuseGroup` whose `entry_ids` enumerate all three members.
    /// Pinned in the issue's acceptance criteria — proves the
    /// grouping logic scales beyond the pair-only case.
    #[test]
    fn three_entries_with_same_password_emit_three_findings_one_group_with_three() {
        let report = analyze(
            vec![
                with_password("a", "Tr0ub4dor-staple-horse-3"),
                with_password("b", "Tr0ub4dor-staple-horse-3"),
                with_password("c", "Tr0ub4dor-staple-horse-3"),
            ],
            now_fixed(),
        );

        let reused_ids: HashSet<&str> = report
            .findings
            .iter()
            .filter(|f| f.kind == FindingKind::PasswordReused)
            .map(|f| f.entry_id.as_str())
            .collect();
        assert_eq!(reused_ids, HashSet::from(["a", "b", "c"]));

        assert_eq!(report.reuse_groups.len(), 1);
        let group_ids: HashSet<&str> = report.reuse_groups[0]
            .entry_ids
            .iter()
            .map(String::as_str)
            .collect();
        assert_eq!(group_ids, HashSet::from(["a", "b", "c"]));
    }

    /// Empty-string passwords must NOT reuse-group with one another:
    /// per ADR 0002 they are "each has no secret" rather than "sharing
    /// the same secret", and the Very-Weak finding carries the
    /// remediation. Two empty-password Entries produce two
    /// `PasswordVeryWeak` Findings, zero `PasswordReused` Findings,
    /// and no `ReuseGroup`.
    #[test]
    fn empty_passwords_do_not_reuse_group_only_very_weak() {
        let report = analyze(
            vec![with_password("a", ""), with_password("b", "")],
            now_fixed(),
        );
        let kinds: Vec<&FindingKind> = report.findings.iter().map(|f| &f.kind).collect();
        assert!(
            !kinds.contains(&&FindingKind::PasswordReused),
            "empty passwords must not emit PasswordReused; got {kinds:?}"
        );
        assert_eq!(
            kinds
                .iter()
                .filter(|k| **k == &FindingKind::PasswordVeryWeak)
                .count(),
            2,
            "both empty-password Entries must each emit one PasswordVeryWeak; got {kinds:?}"
        );
        assert!(
            report.reuse_groups.is_empty(),
            "empty-password Entries must not produce a ReuseGroup; got {:?}",
            report.reuse_groups
        );
    }

    /// `PasswordReused` is a High-severity Finding Kind (per ADR 0002
    /// → "Password Health Finding Kind"). Pinning the mapping protects
    /// the totals-bucket math: an Entry that is only Reused must land
    /// in `high`, not `critical`.
    #[test]
    fn password_reused_is_high_severity() {
        assert_eq!(FindingKind::PasswordReused.severity(), Severity::High);
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
