// SPDX-License-Identifier: MIT

//! Password Health service-layer wiring.
//!
//! This module owns the bridge between the unlocked KDBX tree and the
//! pure analyzer in [`super::analyzer`]. It walks the Vault, enforces
//! the scope rules from ADR 0002 ("exclude Recycle Bin, skip Entries
//! with `password: None`, include Entries with empty-string password"),
//! and hands the resulting [`EntryInput`] iterator to the analyzer.
//!
//! [`PasswordHealthService`] is the coordinator the rest of the app
//! talks to. Right now it is a thin wrapper that materializes a fresh
//! report on every call; the `(db_id, generation)` cache, cancellation
//! handles, debounce, and progressive Tauri events layer on top in
//! subsequent cycles.

use std::sync::Mutex;

use chrono::{DateTime, Utc};
use keepass::db::EntryRef;
use keepass::Database;

use super::analyzer::{analyze, EntryInput, PasswordHealthReport};
use super::cache::CacheStore;
use crate::dto::error::AppError;
use crate::services::kdbx::KdbxService;

/// Walks the unlocked Vault and produces one [`EntryInput`] per
/// in-scope Entry.
///
/// Scope rules:
/// - Entries inside the Recycle Bin group (or any descendant of it)
///   are excluded — a deleted Entry should not contribute to the
///   score or surface a warning icon.
/// - Entries with no `Password` field at all (TOTP-only,
///   attachment-only) are excluded — there is nothing for the
///   analyzer to score.
/// - Entries with an empty-string `Password` are **included**; they
///   contribute to the total-in-scope denominator even though no
///   Finding Kind in this slice would emit for them. Once the
///   Very-Weak check lands they will start emitting that Finding.
pub fn collect_entry_inputs(db: &Database) -> Vec<EntryInput> {
    let recycle_uuid = db.meta.recyclebin_uuid;
    db.iter_all_entries()
        .filter(|entry| {
            if let Some(rid) = recycle_uuid {
                if is_in_recycle_bin(db, entry, rid) {
                    return false;
                }
            }
            entry.get_password().is_some()
        })
        .map(|entry| EntryInput {
            id: entry.id().uuid().to_string(),
            expires: entry.times.expires.unwrap_or(false),
            expiry_time: entry.times.expiry.map(|naive| naive.and_utc()),
        })
        .collect()
}

/// Coordinator for Password Health analysis across every open Vault.
///
/// Holds the `(db_id, generation)`-keyed [`CacheStore`] so repeat
/// calls against an unchanged Vault are free. Cancellation, debounce,
/// and event-emission concerns slot onto this type in the cycles that
/// follow.
pub struct PasswordHealthService {
    cache: Mutex<CacheStore>,
}

impl PasswordHealthService {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(CacheStore::new()),
        }
    }

    /// Returns the report for the Vault at `db_id`, computing it only
    /// if the cache slot is missing or stale.
    ///
    /// The generation read and the cache probe happen inside the
    /// `with_vault` callback so the `(generation, snapshot)` pair is
    /// taken under a coherent lock. On a miss we drop the Vault lock
    /// before running the analyzer (which is pure and CPU-only) and
    /// re-acquire only the cache lock to insert. The clock is
    /// injected so the analyzer stays a pure function downstream.
    pub fn generate_report(
        &self,
        kdbx: &KdbxService,
        db_id: &str,
        now: DateTime<Utc>,
    ) -> Result<PasswordHealthReport, AppError> {
        enum Outcome {
            Hit(PasswordHealthReport),
            Compute {
                generation: u64,
                inputs: Vec<EntryInput>,
            },
        }

        let outcome = kdbx.with_vault(db_id, |vault| {
            let generation = vault.generation();
            let cache = self.cache.lock().map_err(|_| AppError::Lock)?;
            if let Some(cached) = cache.get(db_id, generation) {
                return Ok(Outcome::Hit(cached.clone()));
            }
            drop(cache);
            Ok(Outcome::Compute {
                generation,
                inputs: collect_entry_inputs(vault.db()),
            })
        })?;

        match outcome {
            Outcome::Hit(report) => Ok(report),
            Outcome::Compute { generation, inputs } => {
                let report = analyze(inputs, now);
                self.cache
                    .lock()
                    .map_err(|_| AppError::Lock)?
                    .insert(db_id.to_string(), generation, report.clone());
                Ok(report)
            }
        }
    }
}

impl Default for PasswordHealthService {
    fn default() -> Self {
        Self::new()
    }
}

/// Walks the Entry's ancestor chain looking for `recycle_uuid`. Matches
/// the equivalent helper in `services::kdbx::entries` — we duplicate
/// here to keep `password_health` free of dependencies on KDBX
/// internals beyond the `keepass` crate itself.
fn is_in_recycle_bin(db: &Database, entry: &EntryRef<'_>, recycle_uuid: uuid::Uuid) -> bool {
    let mut current_id = Some(entry.parent().id());
    while let Some(gid) = current_id {
        if gid.uuid() == recycle_uuid {
            return true;
        }
        let Some(group) = db.group(gid) else {
            return false;
        };
        current_id = group.parent().map(|p| p.id());
    }
    false
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::domain::kdbx::OpenDatabase;
    use crate::services::password_health::analyzer::FindingKind;
    use chrono::TimeZone;
    use keepass::db::Value;
    use keepass::Database;

    fn now_fixed() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 18, 12, 0, 0).unwrap()
    }

    /// Drops a pre-built `keepass::Database` into the `KdbxService`
    /// open-databases map so the test can drive `with_vault` without
    /// going through disk-backed `create_database`.
    fn install_vault(kdbx: &KdbxService, path: &str, db: Database) {
        let root_id = db.root().id().uuid().to_string();
        let open = OpenDatabase {
            db: Some(db),
            path: path.to_string(),
            is_modified: false,
            password: None,
            keyfile_path: None,
            version: "test".into(),
            name: "test".into(),
            root_group_id: root_id,
            generation: 0,
        };
        let normalized = KdbxService::normalize_path(path);
        kdbx.lock_databases()
            .expect("lock databases")
            .insert(normalized, open);
    }

    /// The scope filter must drop Recycle Bin descendants and Entries
    /// with no Password field, and keep everything else. Empty-string
    /// passwords stay in scope (they will start emitting Very Weak in
    /// the follow-up slice).
    #[test]
    fn excludes_recycle_bin_and_password_none() {
        let mut db = Database::new();

        // Add a Recycle Bin group under root and wire it into meta so
        // the scope filter can find it.
        let recycle_uuid = {
            let mut root = db.root_mut();
            let recycle = root.add_group();
            recycle.id().uuid()
        };
        db.meta.recyclebin_uuid = Some(recycle_uuid);

        // In-scope: regular Entry with a Password field.
        let in_scope_uuid = {
            let mut root = db.root_mut();
            let mut entry = root.add_entry();
            entry.set("Password", Value::protected("secret"));
            entry.id().uuid()
        };

        // Out-of-scope: no Password field at all.
        let _no_password_uuid = {
            let mut root = db.root_mut();
            let entry = root.add_entry();
            entry.id().uuid()
        };

        // Out-of-scope: lives inside the Recycle Bin.
        let _recycled_uuid = {
            let recycle_id = db
                .iter_all_groups()
                .find(|g| g.id().uuid() == recycle_uuid)
                .expect("recycle bin must exist")
                .id();
            let mut recycle = db.group_mut(recycle_id).expect("recycle bin must exist");
            let mut entry = recycle.add_entry();
            entry.set("Password", Value::protected("also-secret"));
            entry.id().uuid()
        };

        let inputs = collect_entry_inputs(&db);

        let ids: Vec<&str> = inputs.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(
            ids,
            vec![in_scope_uuid.to_string().as_str()],
            "only the in-scope Entry should reach the analyzer"
        );
    }

    /// End-to-end tracer through the synchronous coordinator: install
    /// a Vault containing one healthy Entry and one Entry whose
    /// `times.expires` is set with `times.expiry` in the past, then
    /// call `generate_report`. The returned report must carry exactly
    /// one `password.expired` Finding scoped to the expired Entry's
    /// id, and the score must be below 100. This proves that the
    /// `KdbxService::with_vault` ↔ `collect_entry_inputs` ↔ `analyze`
    /// chain plugs together correctly.
    #[test]
    fn generate_report_emits_password_expired_for_expired_entry() {
        let now = now_fixed();
        let past = (now - chrono::Duration::days(1)).naive_utc();

        let mut db = Database::new();
        let _healthy_uuid = {
            let mut root = db.root_mut();
            let mut entry = root.add_entry();
            entry.set("Password", Value::protected("ok"));
            entry.id().uuid()
        };
        let expired_uuid = {
            let mut root = db.root_mut();
            let mut entry = root.add_entry();
            entry.set("Password", Value::protected("ok-too"));
            entry.times.expires = Some(true);
            entry.times.expiry = Some(past);
            entry.id().uuid()
        };

        let kdbx = KdbxService::new();
        let path = "/tmp/__health_generate_report_test__.kdbx";
        install_vault(&kdbx, path, db);

        let service = PasswordHealthService::new();
        let report = service
            .generate_report(&kdbx, path, now)
            .expect("generate_report");

        assert!(
            report.score.is_some_and(|s| s < 100),
            "score must be below 100 when an Entry is expired (got {:?})",
            report.score
        );
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].entry_id, expired_uuid.to_string());
        assert_eq!(report.findings[0].kind, FindingKind::PasswordExpired);
    }

    /// A mutation followed by `VaultMut::mark_modified()` advances the
    /// generation counter; the next `generate_report` call must
    /// recompute against the new tree state instead of returning the
    /// stale cached report. This is the user-visible freshness
    /// guarantee — saving a fix in the entry editor and immediately
    /// re-opening the Security report must reflect the fix.
    #[test]
    fn generate_report_busts_cache_when_mark_modified_advances_generation() {
        let now = now_fixed();
        let past = (now - chrono::Duration::days(1)).naive_utc();

        let mut db = Database::new();
        {
            let mut root = db.root_mut();
            let mut e = root.add_entry();
            e.set("Password", Value::protected("ok"));
        }

        let kdbx = KdbxService::new();
        let path = "/tmp/__health_cache_bust_test__.kdbx";
        install_vault(&kdbx, path, db);
        let service = PasswordHealthService::new();

        let r1 = service.generate_report(&kdbx, path, now).expect("first");
        assert!(r1.findings.is_empty());

        kdbx.with_vault_mut(path, |v| {
            let mut root = v.db_mut().root_mut();
            let mut e = root.add_entry();
            e.set("Password", Value::protected("x"));
            e.times.expires = Some(true);
            e.times.expiry = Some(past);
            v.mark_modified();
            Ok(())
        })
        .expect("mutate + mark");

        let r2 = service.generate_report(&kdbx, path, now).expect("second");
        assert_eq!(
            r2.findings.len(),
            1,
            "report must reflect the newly-added expired Entry"
        );
        assert_eq!(r2.findings[0].kind, FindingKind::PasswordExpired);
    }

    /// Mutations that **don't** call `mark_modified` (e.g. a no-op
    /// write path, or `report_activity` touching access timestamps)
    /// must not invalidate the cache — the generation counter is the
    /// sole freshness signal. This pin doubles as proof that the
    /// cache actually serves repeat reads: if it weren't caching, the
    /// second call would walk the freshly-mutated tree and disagree
    /// with the first.
    #[test]
    fn generate_report_hits_cache_when_generation_does_not_advance() {
        let now = now_fixed();
        let past = (now - chrono::Duration::days(1)).naive_utc();

        let mut db = Database::new();
        {
            let mut root = db.root_mut();
            let mut e = root.add_entry();
            e.set("Password", Value::protected("ok"));
        }

        let kdbx = KdbxService::new();
        let path = "/tmp/__health_cache_hit_test__.kdbx";
        install_vault(&kdbx, path, db);
        let service = PasswordHealthService::new();

        let r1 = service.generate_report(&kdbx, path, now).expect("first");
        assert!(r1.findings.is_empty());

        kdbx.with_vault_mut(path, |v| {
            let mut root = v.db_mut().root_mut();
            let mut e = root.add_entry();
            e.set("Password", Value::protected("y"));
            e.times.expires = Some(true);
            e.times.expiry = Some(past);
            // NOTE: deliberately not calling v.mark_modified(). The
            // cache is keyed on generation, and generation only moves
            // through that one call site.
            Ok(())
        })
        .expect("mutate without marking");

        let r2 = service.generate_report(&kdbx, path, now).expect("second");
        assert_eq!(
            r1, r2,
            "cache must return the previous report when generation hasn't advanced"
        );
    }
}
