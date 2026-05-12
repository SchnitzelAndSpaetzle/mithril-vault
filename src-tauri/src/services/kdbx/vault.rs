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
}

/// Scoped write access to an unlocked Vault. Holds the databases lock for the
/// duration of the closure passed to `KdbxService::with_vault_mut` and
/// borrows the `is_modified` flag of the underlying `OpenDatabase` so the
/// caller can flip it via `mark_modified` after a successful mutation.
pub(crate) struct VaultMut<'a> {
    db: &'a mut Database,
    is_modified: &'a mut bool,
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
        f(Vault { db })
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
        // Split the borrow so VaultMut can hold disjoint references to the
        // database and the modified flag without alias conflicts.
        let open_db_path = open_db.path.clone();
        let db = open_db
            .db
            .as_mut()
            .ok_or(AppError::DatabaseLocked(open_db_path))?;
        let is_modified = &mut open_db.is_modified;
        let mut vault = VaultMut { db, is_modified };
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

    pub fn mark_modified(&mut self) {
        *self.is_modified = true;
    }
}
