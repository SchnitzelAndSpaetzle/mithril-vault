// SPDX-License-Identifier: MIT

//! `(db_id, generation)`-keyed cache for Password Health reports.
//!
//! The cache is a small in-memory map: each open Vault gets at most
//! one slot, identified by its normalized path. A slot binds the
//! report to the generation counter that produced it; reads that pass
//! a different generation miss the cache. Lock drops the slot via
//! [`CacheStore::evict`].
//!
//! Each slot also carries a `relevant_until` instant — the soonest
//! future expiry that was still in the future when the report was
//! computed. Reads after that instant miss the cache so a follow-up
//! recompute can surface the newly-elapsed `PasswordExpired` Finding
//! without needing a Vault mutation to bump the generation.
//!
//! Kept as a standalone struct so its semantics (which are the
//! load-bearing invariant of the freshness story) can be exercised
//! without spinning up the coordinator, Tauri, or `keepass-rs`.

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use super::analyzer::PasswordHealthReport;

#[derive(Debug, Clone, PartialEq, Eq)]
struct CacheEntry {
    generation: u64,
    report: PasswordHealthReport,
    /// Earliest in-scope expiry that hadn't yet elapsed at compute
    /// time. `None` means no time-dependent Finding source exists for
    /// this Vault; the slot is good until the generation changes.
    relevant_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Default)]
pub struct CacheStore {
    entries: HashMap<String, CacheEntry>,
}

impl CacheStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the cached report only when the slot exists, its
    /// generation matches, **and** `now` has not yet reached the
    /// recorded `relevant_until`. A mismatch on any of those is a
    /// miss — the caller is expected to recompute and `insert` the
    /// fresh report. Including `now` in the freshness check is what
    /// keeps the cache from serving a "healthy" snapshot after an
    /// entry's expiry moment has silently passed.
    pub fn get(
        &self,
        db_id: &str,
        generation: u64,
        now: DateTime<Utc>,
    ) -> Option<&PasswordHealthReport> {
        let entry = self.entries.get(db_id)?;
        if entry.generation != generation {
            return None;
        }
        if let Some(boundary) = entry.relevant_until {
            if now >= boundary {
                return None;
            }
        }
        Some(&entry.report)
    }

    /// Inserts or replaces the slot for `db_id`. Each Vault has at
    /// most one cached report; a fresh insert against an existing
    /// `db_id` overwrites whatever was there. This is intentional —
    /// the Password Health cache exists to surface the *current*
    /// report, not a history of past ones.
    pub fn insert(
        &mut self,
        db_id: String,
        generation: u64,
        report: PasswordHealthReport,
        relevant_until: Option<DateTime<Utc>>,
    ) {
        self.entries.insert(
            db_id,
            CacheEntry {
                generation,
                report,
                relevant_until,
            },
        );
    }

    /// Drops the slot for `db_id`. Called on Vault lock so a re-unlock
    /// triggers a fresh analysis instead of returning a stale report
    /// from the previous session.
    pub fn evict(&mut self, db_id: &str) {
        self.entries.remove(db_id);
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn report_with_score(score: Option<u32>) -> PasswordHealthReport {
        PasswordHealthReport {
            score,
            findings: Vec::new(),
            totals: super::super::analyzer::HealthTotals::default(),
        }
    }

    fn now_fixed() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 18, 12, 0, 0).unwrap()
    }

    /// The cache only returns a report when the `(db_id, generation)`
    /// pair matches. Other combinations — empty cache, wrong
    /// generation, wrong db — are misses. Pinning all four together
    /// captures the load-bearing freshness invariant in a single read.
    #[test]
    fn cache_returns_only_on_matching_db_and_generation() {
        let mut store = CacheStore::new();
        let now = now_fixed();
        assert!(store.get("db1", 0, now).is_none(), "empty cache misses");

        let r = report_with_score(Some(75));
        store.insert("db1".into(), 1, r.clone(), None);

        assert_eq!(store.get("db1", 1, now), Some(&r), "matching key hits");
        assert!(
            store.get("db1", 2, now).is_none(),
            "newer generation misses — Vault has mutated, report is stale"
        );
        assert!(
            store.get("db1", 0, now).is_none(),
            "older generation misses — caller has a stale handle"
        );
        assert!(
            store.get("db2", 1, now).is_none(),
            "different Vault id misses"
        );
    }

    /// `evict` drops the slot so the next `get` is a miss regardless
    /// of the generation passed in. This is the lock-time path — the
    /// service drops the cached report on Vault lock so the re-unlock
    /// path is forced to recompute.
    #[test]
    fn evict_drops_slot_for_db_id() {
        let mut store = CacheStore::new();
        let now = now_fixed();
        store.insert("db1".into(), 1, report_with_score(Some(100)), None);
        store.evict("db1");
        assert!(store.get("db1", 1, now).is_none());
    }

    /// Insert against an existing `db_id` overwrites the previous
    /// slot rather than accumulating. The cache surfaces the current
    /// report, not a history.
    #[test]
    fn insert_replaces_existing_slot() {
        let mut store = CacheStore::new();
        let now = now_fixed();
        store.insert("db1".into(), 1, report_with_score(Some(75)), None);
        store.insert("db1".into(), 2, report_with_score(Some(100)), None);

        assert!(
            store.get("db1", 1, now).is_none(),
            "old generation must be gone"
        );
        assert_eq!(
            store.get("db1", 2, now),
            Some(&report_with_score(Some(100)))
        );
    }

    /// A slot with a `relevant_until` boundary hits while `now` is
    /// before it and misses once `now` reaches it. This is the
    /// time-component freshness check that prevents serving a stale
    /// "healthy" snapshot after an entry's expiry has silently
    /// elapsed between two reads on an otherwise-unchanged Vault.
    #[test]
    fn cache_misses_once_now_reaches_relevant_until() {
        let mut store = CacheStore::new();
        let computed_at = now_fixed();
        let boundary = computed_at + chrono::Duration::hours(1);
        let r = report_with_score(Some(100));

        store.insert("db1".into(), 1, r.clone(), Some(boundary));

        assert_eq!(
            store.get("db1", 1, computed_at),
            Some(&r),
            "before the boundary the slot still hits"
        );
        assert!(
            store.get("db1", 1, boundary).is_none(),
            "at the boundary the slot misses — the expiry instant is reached"
        );
        assert!(
            store
                .get("db1", 1, boundary + chrono::Duration::seconds(1))
                .is_none(),
            "past the boundary the slot misses"
        );
    }
}
