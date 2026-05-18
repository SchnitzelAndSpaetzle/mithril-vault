// SPDX-License-Identifier: MIT

//! Password Health service-layer wiring.
//!
//! This module owns the bridge between the unlocked KDBX tree and the
//! pure analyzer in [`super::analyzer`]. It walks the Vault, enforces
//! the scope rules from ADR 0002 ("exclude Recycle Bin, skip Entries
//! with `password: None`, include Entries with empty-string password"),
//! and hands the resulting [`EntryInput`] iterator to the analyzer.
//!
//! The eager-on-unlock coordinator, the `(db_id, generation)` cache,
//! cancellation handles, and the Tauri-event stream layer on top of
//! this collection step in subsequent cycles.

use keepass::db::EntryRef;
use keepass::Database;

use super::analyzer::EntryInput;

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
    use keepass::db::Value;
    use keepass::Database;

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
}
