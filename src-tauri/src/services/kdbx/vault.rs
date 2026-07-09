use crate::domain::kdbx::OpenDatabase;
use crate::dto::error::AppError;
use keepass::db::{
    Entry as KeepassEntry, EntryId, EntryMut, EntryRef, GroupId, GroupMut, GroupRef, Times,
};
use keepass::Database;

use super::KdbxService;

/// Scoped read access to an unlocked Vault. Holds the databases lock for the
/// duration of the closure passed to `KdbxService::with_vault`.
pub(crate) struct Vault<'a> {
    db: &'a Database,
    generation: u64,
}

/// Scoped write access to an unlocked Vault. Holds the databases lock for the
/// duration of the closure passed to `KdbxService::with_vault_mut` and
/// borrows the `is_modified` flag of the underlying `OpenDatabase` so the
/// caller can flip it via `mark_modified` after a successful mutation.
pub(crate) struct VaultMut<'a> {
    db: &'a mut Database,
    is_modified: &'a mut bool,
    generation: &'a mut u64,
}

impl KdbxService {
    pub(crate) fn with_vault<R>(
        &self,
        db_id: &str,
        f: impl FnOnce(Vault<'_>) -> Result<R, AppError>,
    ) -> Result<R, AppError> {
        let normalized_path = Self::normalize_path(db_id);
        let databases = self.lock_databases()?;
        let open_db = databases
            .get(&normalized_path)
            .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;
        let db = open_db.db_or_locked()?;
        f(Vault {
            db,
            generation: open_db.generation,
        })
    }

    pub(crate) fn with_vault_mut<R>(
        &self,
        db_id: &str,
        f: impl FnOnce(&mut VaultMut<'_>) -> Result<R, AppError>,
    ) -> Result<R, AppError> {
        let normalized_path = Self::normalize_path(db_id);
        let mut databases = self.lock_databases()?;
        let open_db = databases
            .get_mut(&normalized_path)
            .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;
        // Split the borrow into three disjoint slots so VaultMut can hold
        // references to the database, the modified flag, and the generation
        // counter at the same time without alias conflicts.
        let open_db_path = open_db.path.clone();
        let OpenDatabase {
            db: db_slot,
            is_modified,
            generation,
            ..
        } = open_db;
        let db = db_slot
            .as_mut()
            .ok_or(AppError::DatabaseLocked(open_db_path))?;
        let mut vault = VaultMut {
            db,
            is_modified,
            generation,
        };
        f(&mut vault)
    }
}

fn find_entry_id_in(db: &Database, id: &str) -> Option<EntryId> {
    db.iter_all_entries()
        .find(|e| e.id().uuid().to_string() == id)
        .map(|e| e.id())
}

fn find_group_id_in(db: &Database, id: &str) -> Option<GroupId> {
    db.iter_all_groups()
        .find(|g| g.id().uuid().to_string() == id)
        .map(|g| g.id())
}

fn find_group_by_name_in<'a>(db: &'a Database, name: &str) -> Option<GroupRef<'a>> {
    db.iter_all_groups().find(|g| g.name == name)
}

fn find_parent_group_id_in(db: &Database, target_id: &str) -> Option<String> {
    let gid = find_group_id_in(db, target_id)?;
    let target = db.group(gid)?;
    target.parent().map(|p| p.id().uuid().to_string())
}

fn is_ancestor_of_in(db: &Database, ancestor_id: &str, descendant_id: &str) -> bool {
    if ancestor_id == descendant_id {
        return true;
    }
    let Some(start_id) = find_group_id_in(db, descendant_id) else {
        return false;
    };
    let Some(start) = db.group(start_id) else {
        return false;
    };
    let mut current_id = start.parent().map(|p| p.id());
    while let Some(gid) = current_id {
        let Some(parent) = db.group(gid) else {
            return false;
        };
        if parent.id().uuid().to_string() == ancestor_id {
            return true;
        }
        current_id = parent.parent().map(|p| p.id());
    }
    false
}

pub(crate) fn group_has_children(group: &GroupRef<'_>) -> bool {
    group.groups().next().is_some() || group.entries().next().is_some()
}

impl<'a> Vault<'a> {
    pub fn db(&self) -> &'a Database {
        self.db
    }

    #[allow(dead_code)] // wired up by the upcoming password-health coordinator
    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn root(&self) -> GroupRef<'a> {
        self.db.root()
    }

    pub fn find_entry_id(&self, id: &str) -> Result<EntryId, AppError> {
        find_entry_id_in(self.db, id).ok_or_else(|| AppError::EntryNotFound(id.to_string()))
    }

    pub fn find_group_id(&self, id: &str) -> Result<GroupId, AppError> {
        find_group_id_in(self.db, id).ok_or_else(|| AppError::GroupNotFound(id.to_string()))
    }

    pub fn find_entry(&self, id: &str) -> Result<EntryRef<'a>, AppError> {
        let eid = self.find_entry_id(id)?;
        self.db
            .entry(eid)
            .ok_or_else(|| AppError::EntryNotFound(id.to_string()))
    }

    pub fn find_group(&self, id: &str) -> Result<GroupRef<'a>, AppError> {
        let gid = self.find_group_id(id)?;
        self.db
            .group(gid)
            .ok_or_else(|| AppError::GroupNotFound(id.to_string()))
    }

    pub fn try_find_group(&self, id: &str) -> Option<GroupRef<'a>> {
        let gid = find_group_id_in(self.db, id)?;
        self.db.group(gid)
    }
}

impl VaultMut<'_> {
    pub fn db(&self) -> &Database {
        self.db
    }

    pub fn db_mut(&mut self) -> &mut Database {
        self.db
    }

    pub fn root(&self) -> GroupRef<'_> {
        self.db.root()
    }

    pub fn find_entry_id(&self, id: &str) -> Result<EntryId, AppError> {
        find_entry_id_in(self.db, id).ok_or_else(|| AppError::EntryNotFound(id.to_string()))
    }

    pub fn find_group_id(&self, id: &str) -> Result<GroupId, AppError> {
        find_group_id_in(self.db, id).ok_or_else(|| AppError::GroupNotFound(id.to_string()))
    }

    pub fn find_group(&self, id: &str) -> Result<GroupRef<'_>, AppError> {
        let gid = self.find_group_id(id)?;
        self.db
            .group(gid)
            .ok_or_else(|| AppError::GroupNotFound(id.to_string()))
    }

    pub fn entry_mut(&mut self, id: &str) -> Result<EntryMut<'_>, AppError> {
        let eid = self.find_entry_id(id)?;
        self.db
            .entry_mut(eid)
            .ok_or_else(|| AppError::EntryNotFound(id.to_string()))
    }

    pub fn group_mut(&mut self, id: &str) -> Result<GroupMut<'_>, AppError> {
        let gid = self.find_group_id(id)?;
        self.db
            .group_mut(gid)
            .ok_or_else(|| AppError::GroupNotFound(id.to_string()))
    }

    pub fn find_parent_group_id(&self, target_id: &str) -> Option<String> {
        find_parent_group_id_in(self.db, target_id)
    }

    pub fn is_ancestor_of(&self, ancestor_id: &str, descendant_id: &str) -> bool {
        is_ancestor_of_in(self.db, ancestor_id, descendant_id)
    }

    /// Ensures the database has a recycle bin and returns its UUID as a
    /// string. Reuses an existing recycle bin if `meta.recyclebin_uuid` or a
    /// "Recycle Bin" group exists; otherwise creates a fresh one under root.
    pub fn ensure_recycle_bin(&mut self) -> String {
        if let Some(uuid) = self.db.meta.recyclebin_uuid {
            if find_group_id_in(self.db, &uuid.to_string()).is_some() {
                self.db.meta.recyclebin_enabled = Some(true);
                self.db.meta.recyclebin_changed = Some(Times::now());
                return uuid.to_string();
            }
        }

        if let Some(existing) = find_group_by_name_in(self.db, "Recycle Bin") {
            let uuid = existing.id().uuid();
            self.db.meta.recyclebin_enabled = Some(true);
            self.db.meta.recyclebin_uuid = Some(uuid);
            self.db.meta.recyclebin_changed = Some(Times::now());
            return uuid.to_string();
        }

        let new_uuid = {
            let mut root = self.db.root_mut();
            let mut new_group = root.add_group();
            new_group.name = "Recycle Bin".to_string();
            new_group.id().uuid()
        };

        self.db.meta.recyclebin_enabled = Some(true);
        self.db.meta.recyclebin_uuid = Some(new_uuid);
        self.db.meta.recyclebin_changed = Some(Times::now());

        new_uuid.to_string()
    }

    /// Walks every entry in the database, applying `modify_fn`. Counts
    /// entries that the closure reported as modified.
    pub fn modify_all_entries(&mut self, modify_fn: &dyn Fn(&mut KeepassEntry) -> bool) -> u32 {
        let mut count = 0u32;
        self.db.foreach_entry_mut(|mut entry| {
            if modify_fn(&mut entry) {
                entry.times.last_modification = Some(Times::now());
                count = count.saturating_add(1);
            }
        });
        count
    }

    #[allow(dead_code)] // wired up by the upcoming password-health coordinator
    pub fn generation(&self) -> u64 {
        *self.generation
    }

    pub fn mark_modified(&mut self) {
        *self.is_modified = true;
        *self.generation = self.generation.saturating_add(1);
    }

    /// The per-Vault Entry-History retention, resolved from
    /// `Meta.history_max_items` (ADR-0008). Read before taking an `entry_mut`
    /// borrow so the resolved policy (a `Copy` value) can be handed to the
    /// snapshot chokepoint without aliasing the database.
    pub fn history_retention(&self) -> super::history::HistoryRetention {
        super::history::resolve_history_retention(self.db.meta.history_max_items)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use crate::domain::kdbx::OpenDatabase;
    use crate::services::kdbx::KdbxService;
    use keepass::Database;

    /// Drops a hand-built `OpenDatabase` straight into the service's
    /// internal map so the test can drive `with_vault[_mut]` without
    /// going through disk-backed `create_database`. The path doesn't
    /// need to exist on disk — `KdbxService::normalize_path` falls back
    /// to the literal string when canonicalize fails.
    fn install_test_vault(service: &KdbxService, path: &str) {
        let db = Database::new();
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
        service
            .lock_databases()
            .expect("lock databases")
            .insert(normalized, open);
    }

    /// Every call to `mark_modified` must produce a strictly increasing
    /// generation counter. The Password Health service keys its cache
    /// on `(db_id, generation)`; if the counter ever fails to advance
    /// after a write, the next `get_password_health_report` call would
    /// return a stale report.
    #[test]
    fn mark_modified_bumps_generation() {
        let service = KdbxService::new();
        let path = "/tmp/__health_gen_test__.kdbx";
        install_test_vault(&service, path);

        let g0 = service
            .with_vault(path, |v| Ok(v.generation()))
            .expect("read g0");
        service
            .with_vault_mut(path, |v| {
                v.mark_modified();
                Ok(())
            })
            .expect("bump 1");
        let g1 = service
            .with_vault(path, |v| Ok(v.generation()))
            .expect("read g1");
        service
            .with_vault_mut(path, |v| {
                v.mark_modified();
                Ok(())
            })
            .expect("bump 2");
        let g2 = service
            .with_vault(path, |v| Ok(v.generation()))
            .expect("read g2");

        assert_eq!(g0, 0);
        assert_eq!(g1, 1);
        assert_eq!(g2, 2);
    }

    /// Entering `with_vault_mut` is only the *opportunity* to mutate —
    /// the counter does not move until the closure actually calls
    /// `mark_modified`. Read-mostly mut paths (e.g. `report_activity`
    /// touching last-access timestamps without dirtying) must not
    /// invalidate the Password Health cache.
    #[test]
    fn entering_with_vault_mut_without_mark_modified_does_not_bump_generation() {
        let service = KdbxService::new();
        let path = "/tmp/__health_gen_test_noop__.kdbx";
        install_test_vault(&service, path);

        service
            .with_vault_mut(path, |_v| Ok(()))
            .expect("enter without marking");

        let gen = service
            .with_vault(path, |v| Ok(v.generation()))
            .expect("read gen");
        assert_eq!(gen, 0);
    }
}
