use crate::dto::error::AppError;
use crate::dto::group::{Group, UpdateGroupData};
use keepass::db::{GroupId, GroupRef, Times};

use std::collections::HashMap;

use super::mapping::{
    convert_group, ensure_recycle_bin, find_group_by_id, find_group_id, find_parent_group_id,
    group_has_children, is_ancestor_of,
};
use super::KdbxService;

impl KdbxService {
    /// Lists groups in a hierarchy.
    pub fn list_groups(&self, db_id: &str) -> Result<Vec<Group>, AppError> {
        let normalized_path = Self::normalize_path(db_id);
        let databases = self.lock_databases()?;
        let open_db = databases
            .get(&normalized_path)
            .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;

        let db = open_db.db_or_locked()?;
        let root = convert_group(&db.root(), None);
        Ok(vec![root])
    }

    /// Fetches a group by ID.
    pub fn get_group(&self, db_id: &str, id: &str) -> Result<Group, AppError> {
        let normalized_path = Self::normalize_path(db_id);
        let databases = self.lock_databases()?;
        let open_db = databases
            .get(&normalized_path)
            .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;

        let db = open_db.db_or_locked()?;
        find_group_by_id(db, id)
            .map(|g| convert_group(&g, None))
            .ok_or_else(|| AppError::GroupNotFound(id.to_string()))
    }

    /// Creates a new group.
    pub fn create_group(
        &self,
        db_id: &str,
        parent_id: Option<&str>,
        name: &str,
        icon: Option<u32>,
    ) -> Result<Group, AppError> {
        let normalized_path = Self::normalize_path(db_id);
        let mut databases = self.lock_databases()?;
        let open_db = databases
            .get_mut(&normalized_path)
            .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;

        let db = open_db.db_mut_or_locked()?;

        // Resolve the parent group id (root if parent_id is None)
        let resolved_parent: GroupId = if let Some(pid) = parent_id {
            find_group_id(db, pid).ok_or_else(|| AppError::GroupNotFound(pid.to_string()))?
        } else {
            db.root().id()
        };
        let parent_uuid = resolved_parent.uuid().to_string();

        let new_gid = {
            let mut parent = db
                .group_mut(resolved_parent)
                .ok_or_else(|| AppError::GroupNotFound(parent_uuid.clone()))?;
            let mut new_group = parent.add_group();
            new_group.name = name.to_string();
            if let Some(icon_id) = icon {
                new_group.set_icon_builtin(icon_id as usize);
            }
            new_group.id()
        };

        let group_model = db
            .group(new_gid)
            .map(|g| convert_group(&g, Some(&parent_uuid)))
            .ok_or_else(|| AppError::GroupNotFound(new_gid.uuid().to_string()))?;
        open_db.is_modified = true;

        Ok(group_model)
    }

    /// Updates an existing group.
    pub fn update_group(
        &self,
        db_id: &str,
        id: &str,
        data: UpdateGroupData,
    ) -> Result<Group, AppError> {
        let normalized_path = Self::normalize_path(db_id);
        let mut databases = self.lock_databases()?;
        let open_db = databases
            .get_mut(&normalized_path)
            .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;

        let db = open_db.db_mut_or_locked()?;

        let parent_id = find_parent_group_id(db, id);
        let gid = find_group_id(db, id).ok_or_else(|| AppError::GroupNotFound(id.to_string()))?;

        {
            let mut group = db
                .group_mut(gid)
                .ok_or_else(|| AppError::GroupNotFound(id.to_string()))?;
            if let Some(name) = data.name {
                group.name = name;
            }
            if let Some(icon) = data.icon {
                if let Some(idx) = icon.parse::<u32>().ok().map(|i| i as usize) {
                    group.set_icon_builtin(idx);
                } else {
                    group.set_icon_none();
                }
            }
            group.times.last_modification = Some(Times::now());
        }

        let result = db
            .group(gid)
            .map(|g| convert_group(&g, parent_id.as_deref()))
            .ok_or_else(|| AppError::GroupNotFound(id.to_string()))?;
        open_db.is_modified = true;

        Ok(result)
    }

    /// Deletes a group.
    /// If `recursive` is false and the group has children, returns an error.
    /// If `permanent` is true, the group is permanently deleted; otherwise moved to recycle bin.
    pub fn delete_group(
        &self,
        db_id: &str,
        id: &str,
        recursive: bool,
        permanent: bool,
    ) -> Result<(), AppError> {
        let normalized_path = Self::normalize_path(db_id);
        let mut databases = self.lock_databases()?;
        let open_db = databases
            .get_mut(&normalized_path)
            .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;

        let db = open_db.db_mut_or_locked()?;

        // Cannot delete root group
        if db.root().id().uuid().to_string() == id {
            return Err(AppError::CannotDeleteRootGroup);
        }

        // Check if group exists and whether it has children
        let gid = {
            let group =
                find_group_by_id(db, id).ok_or_else(|| AppError::GroupNotFound(id.to_string()))?;
            if !recursive && group_has_children(&group) {
                return Err(AppError::GroupNotEmpty(id.to_string()));
            }
            group.id()
        };

        if permanent {
            db.group_mut(gid)
                .ok_or_else(|| AppError::GroupNotFound(id.to_string()))?
                .remove();
        } else {
            // If the user soft-deletes the recycle bin itself, create a fresh
            // replacement recycle bin under root and move the old one into it.
            // We can't go through ensure_recycle_bin here because both its
            // lookups (recyclebin_uuid and find-by-name) would resolve to the
            // group being deleted; move_to would then fail with WouldCreateCycle.
            let is_self_recycle = db
                .meta
                .recyclebin_uuid
                .is_some_and(|uuid| uuid == gid.uuid());

            let recycle_gid = if is_self_recycle {
                let new_id = {
                    let mut root = db.root_mut();
                    let mut new_group = root.add_group();
                    new_group.name = "Recycle Bin".to_string();
                    new_group.id()
                };
                db.meta.recyclebin_enabled = Some(true);
                db.meta.recyclebin_uuid = Some(new_id.uuid());
                db.meta.recyclebin_changed = Some(Times::now());
                new_id
            } else {
                let recycle_bin_uuid = ensure_recycle_bin(db);
                find_group_id(db, &recycle_bin_uuid)
                    .ok_or_else(|| AppError::GroupNotFound(recycle_bin_uuid.clone()))?
            };

            let now = Times::now();
            {
                let mut group = db
                    .group_mut(gid)
                    .ok_or_else(|| AppError::GroupNotFound(id.to_string()))?;
                group.times.last_modification = Some(now);
                group.times.location_changed = Some(now);
                group
                    .move_to(recycle_gid)
                    .map_err(|e| AppError::Kdbx(e.to_string()))?;
            }
        }

        open_db.is_modified = true;
        Ok(())
    }

    /// Moves a group to a new parent.
    /// If `target_parent_id` is None, moves to root.
    pub fn move_group(
        &self,
        db_id: &str,
        id: &str,
        target_parent_id: Option<&str>,
    ) -> Result<Group, AppError> {
        let normalized_path = Self::normalize_path(db_id);
        let mut databases = self.lock_databases()?;
        let open_db = databases
            .get_mut(&normalized_path)
            .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;

        let db = open_db.db_mut_or_locked()?;
        let root_id = db.root().id().uuid().to_string();

        // Cannot move root group
        if root_id == id {
            return Err(AppError::CannotMoveRootGroup);
        }

        let gid = find_group_id(db, id).ok_or_else(|| AppError::GroupNotFound(id.to_string()))?;

        let target_id_str = target_parent_id.unwrap_or(&root_id).to_string();

        // Check for circular reference (cannot move a group into itself or its descendants)
        if is_ancestor_of(db, id, &target_id_str) {
            return Err(AppError::CircularReference);
        }

        let target_gid = find_group_id(db, &target_id_str)
            .ok_or_else(|| AppError::GroupNotFound(target_id_str.clone()))?;

        let now = Times::now();
        {
            let mut group = db
                .group_mut(gid)
                .ok_or_else(|| AppError::GroupNotFound(id.to_string()))?;
            group.times.last_modification = Some(now);
            group.times.location_changed = Some(now);
            group
                .move_to(target_gid)
                .map_err(|e| AppError::Kdbx(e.to_string()))?;
        }

        let group_model = db
            .group(gid)
            .map(|g| convert_group(&g, Some(&target_id_str)))
            .ok_or_else(|| AppError::GroupNotFound(id.to_string()))?;
        open_db.is_modified = true;

        Ok(group_model)
    }

    /// Returns entry counts per group (direct entries only, not recursive).
    pub fn get_group_entry_counts(&self, db_id: &str) -> Result<HashMap<String, u32>, AppError> {
        let normalized_path = Self::normalize_path(db_id);
        let databases = self.lock_databases()?;
        let open_db = databases
            .get(&normalized_path)
            .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;

        let db = open_db.db_or_locked()?;
        let mut counts = HashMap::new();
        for group in db.iter_all_groups() {
            counts.insert(
                group.id().uuid().to_string(),
                collect_direct_entry_count(&group),
            );
        }
        Ok(counts)
    }

    /// Returns the recycle bin group ID if it exists and is set in metadata.
    pub fn get_recycle_bin_id(&self, db_id: &str) -> Result<Option<String>, AppError> {
        let normalized_path = Self::normalize_path(db_id);
        let databases = self.lock_databases()?;
        let open_db = databases
            .get(&normalized_path)
            .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;

        let db = open_db.db_or_locked()?;
        // Check if recycle bin is set in metadata and the group actually exists
        if let Some(recycle_uuid) = db.meta.recyclebin_uuid {
            let recycle_id = recycle_uuid.to_string();
            if find_group_by_id(db, &recycle_id).is_some() {
                return Ok(Some(recycle_id));
            }
        }

        Ok(None)
    }
}

fn collect_direct_entry_count(group: &GroupRef<'_>) -> u32 {
    u32::try_from(group.entries().count()).unwrap_or(u32::MAX)
}
