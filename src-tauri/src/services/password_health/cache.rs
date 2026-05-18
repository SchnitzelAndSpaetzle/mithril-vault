// SPDX-License-Identifier: MIT

//! `(db_id, generation)`-keyed cache for Password Health reports.
//!
//! The cache is a small in-memory map: each open Vault gets at most
//! one slot, identified by its normalized path. A slot binds the
//! report to the generation counter that produced it; reads that pass
//! a different generation miss the cache. Lock drops the slot via
//! [`CacheStore::evict`].
//!
//! Kept as a standalone struct so its semantics (which are the
//! load-bearing invariant of the freshness story) can be exercised
//! without spinning up the coordinator, Tauri, or `keepass-rs`.

use std::collections::HashMap;

use super::analyzer::PasswordHealthReport;

#[derive(Debug, Clone, PartialEq, Eq)]
struct CacheEntry {
    generation: u64,
    report: PasswordHealthReport,
}

#[derive(Debug, Default)]
pub struct CacheStore {
    entries: HashMap<String, CacheEntry>,
}

impl CacheStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the cached report only when the slot exists **and** its
    /// generation matches. A mismatch is a miss — the caller is
    /// expected to recompute and `insert` the fresh report.
    pub fn get(&self, db_id: &str, generation: u64) -> Option<&PasswordHealthReport> {
        let entry = self.entries.get(db_id)?;
        if entry.generation == generation {
            Some(&entry.report)
        } else {
            None
        }
    }

    /// Inserts or replaces the slot for `db_id`. Each Vault has at
    /// most one cached report; a fresh insert against an existing
    /// `db_id` overwrites whatever was there. This is intentional —
    /// the Password Health cache exists to surface the *current*
    /// report, not a history of past ones.
    pub fn insert(&mut self, db_id: String, generation: u64, report: PasswordHealthReport) {
        self.entries
            .insert(db_id, CacheEntry { generation, report });
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

    fn report_with_score(score: Option<u32>) -> PasswordHealthReport {
        PasswordHealthReport {
            score,
            findings: Vec::new(),
        }
    }

    /// The cache only returns a report when the `(db_id, generation)`
    /// pair matches. Other combinations — empty cache, wrong
    /// generation, wrong db — are misses. Pinning all four together
    /// captures the load-bearing freshness invariant in a single read.
    #[test]
    fn cache_returns_only_on_matching_db_and_generation() {
        let mut store = CacheStore::new();
        assert!(
            store.get("db1", 0).is_none(),
            "empty cache misses"
        );

        let r = report_with_score(Some(75));
        store.insert("db1".into(), 1, r.clone());

        assert_eq!(store.get("db1", 1), Some(&r), "matching key hits");
        assert!(
            store.get("db1", 2).is_none(),
            "newer generation misses — Vault has mutated, report is stale"
        );
        assert!(
            store.get("db1", 0).is_none(),
            "older generation misses — caller has a stale handle"
        );
        assert!(
            store.get("db2", 1).is_none(),
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
        store.insert("db1".into(), 1, report_with_score(Some(100)));
        store.evict("db1");
        assert!(store.get("db1", 1).is_none());
    }

    /// Insert against an existing `db_id` overwrites the previous
    /// slot rather than accumulating. The cache surfaces the current
    /// report, not a history.
    #[test]
    fn insert_replaces_existing_slot() {
        let mut store = CacheStore::new();
        store.insert("db1".into(), 1, report_with_score(Some(75)));
        store.insert("db1".into(), 2, report_with_score(Some(100)));

        assert!(store.get("db1", 1).is_none(), "old generation must be gone");
        assert_eq!(store.get("db1", 2), Some(&report_with_score(Some(100))));
    }
}
