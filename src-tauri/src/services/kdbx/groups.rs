use crate::dto::error::AppError;
use crate::dto::group::{Group, UpdateGroupData};
use keepass::db::{Group as KeepassGroup, Times};

use super::mapping::{
    convert_group, ensure_recycle_bin, find_group_by_id, find_group_by_id_mut,
    find_parent_group_id, group_has_children, is_ancestor_of, remove_group_by_id,
};
use super::KdbxService;

impl KdbxService {
    /// Lists groups in a hierarchy.
    pub fn list_groups(&self) -> Result<Vec<Group>, AppError> {
        let db_lock = self.database.lock().map_err(|_| AppError::Lock)?;
        let open_db = db_lock.as_ref().ok_or(AppError::DatabaseNotOpen)?;

        let root = convert_group(&open_db.db.root, None);
        Ok(vec![root])
    }

    /// Fetches a group by ID.
    pub fn get_group(&self, id: &str) -> Result<Group, AppError> {
        let db_lock = self.database.lock().map_err(|_| AppError::Lock)?;
        let open_db = db_lock.as_ref().ok_or(AppError::DatabaseNotOpen)?;

        find_group_by_id(&open_db.db.root, id)
            .map(|g| convert_group(g, None))
            .ok_or_else(|| AppError::GroupNotFound(id.to_string()))
    }

    /// Creates a new group.
    pub fn create_group(
        &self,
        parent_id: Option<&str>,
        name: &str,
        icon: Option<u32>,
    ) -> Result<Group, AppError> {
        let mut db_lock = self.database.lock().map_err(|_| AppError::Lock)?;
        let open_db = db_lock.as_mut().ok_or(AppError::DatabaseNotOpen)?;

        // Find the parent group (root if parent_id is None)
        let (parent, parent_uuid) = if let Some(pid) = parent_id {
            let parent = find_group_by_id_mut(&mut open_db.db.root, pid)
                .ok_or_else(|| AppError::GroupNotFound(pid.to_string()))?;
            let uuid = parent.uuid.to_string();
            (parent, Some(uuid))
        } else {
            let uuid = open_db.db.root.uuid.to_string();
            (&mut open_db.db.root, Some(uuid))
        };

        // Create the new group
        let mut new_group = KeepassGroup::new(name);
        if let Some(icon_id) = icon {
            new_group.icon_id = Some(icon_id as usize);
        }

        let group_model = convert_group(&new_group, parent_uuid.as_deref());
        parent.add_child(new_group);
        open_db.is_modified = true;

        Ok(group_model)
    }

    /// Updates an existing group.
    pub fn update_group(&self, id: &str, data: UpdateGroupData) -> Result<Group, AppError> {
        let mut db_lock = self.database.lock().map_err(|_| AppError::Lock)?;
        let open_db = db_lock.as_mut().ok_or(AppError::DatabaseNotOpen)?;

        // Find parent ID before mutating (for return value)
        let parent_id = find_parent_group_id(&open_db.db.root, id);

        let group = find_group_by_id_mut(&mut open_db.db.root, id)
            .ok_or_else(|| AppError::GroupNotFound(id.to_string()))?;

        if let Some(name) = data.name {
            group.name = name;
        }
        if let Some(icon) = data.icon {
            group.icon_id = icon.parse().ok().map(|i: u32| i as usize);
        }

        group.times.set_last_modification(Times::now());
        open_db.is_modified = true;

        Ok(convert_group(group, parent_id.as_deref()))
    }

    /// Deletes a group.
    /// If `recursive` is false and the group has children, returns an error.
    /// If `permanent` is true, the group is permanently deleted; otherwise moved to recycle bin.
    pub fn delete_group(&self, id: &str, recursive: bool, permanent: bool) -> Result<(), AppError> {
        let mut db_lock = self.database.lock().map_err(|_| AppError::Lock)?;
        let open_db = db_lock.as_mut().ok_or(AppError::DatabaseNotOpen)?;

        // Cannot delete root group
        if open_db.db.root.uuid.to_string() == id {
            return Err(AppError::CannotDeleteRootGroup);
        }

        // Check if group exists and whether it has children
        {
            let group = find_group_by_id(&open_db.db.root, id)
                .ok_or_else(|| AppError::GroupNotFound(id.to_string()))?;

            if !recursive && group_has_children(group) {
                return Err(AppError::GroupNotEmpty(id.to_string()));
            }
        }

        // Remove the group from its parent
        let mut removed_group = remove_group_by_id(&mut open_db.db.root, id)
            .ok_or_else(|| AppError::GroupNotFound(id.to_string()))?;

        if permanent {
            // Permanently deleted, nothing more to do
        } else {
            // Move to recycle bin
            let recycle_bin_id = ensure_recycle_bin(&mut open_db.db);
            let recycle_bin = find_group_by_id_mut(&mut open_db.db.root, &recycle_bin_id)
                .ok_or_else(|| AppError::GroupNotFound(recycle_bin_id.clone()))?;

            let now = Times::now();
            removed_group.times.set_last_modification(now);
            removed_group.times.set_location_changed(now);
            recycle_bin.add_child(removed_group);
        }

        open_db.is_modified = true;
        Ok(())
    }

    /// Moves a group to a new parent.
    /// If `target_parent_id` is None, moves to root.
    pub fn move_group(&self, id: &str, target_parent_id: Option<&str>) -> Result<Group, AppError> {
        let mut db_lock = self.database.lock().map_err(|_| AppError::Lock)?;
        let open_db = db_lock.as_mut().ok_or(AppError::DatabaseNotOpen)?;

        let root_id = open_db.db.root.uuid.to_string();

        // Cannot move root group
        if root_id == id {
            return Err(AppError::CannotMoveRootGroup);
        }

        // Verify the group exists
        if find_group_by_id(&open_db.db.root, id).is_none() {
            return Err(AppError::GroupNotFound(id.to_string()));
        }

        // Determine target parent ID (root if None)
        let target_id = target_parent_id.unwrap_or(&root_id);

        // Check for circular reference (cannot move a group into itself or its descendants)
        if is_ancestor_of(&open_db.db.root, id, target_id) {
            return Err(AppError::CircularReference);
        }

        // Verify target parent exists
        if find_group_by_id(&open_db.db.root, target_id).is_none() {
            return Err(AppError::GroupNotFound(target_id.to_string()));
        }

        // Remove the group from its current parent
        let mut group = remove_group_by_id(&mut open_db.db.root, id)
            .ok_or_else(|| AppError::GroupNotFound(id.to_string()))?;

        // Update timestamps
        let now = Times::now();
        group.times.set_last_modification(now);
        group.times.set_location_changed(now);

        // Add to new parent
        let target_parent = find_group_by_id_mut(&mut open_db.db.root, target_id)
            .ok_or_else(|| AppError::GroupNotFound(target_id.to_string()))?;

        let group_model = convert_group(&group, Some(target_id));
        target_parent.add_child(group);
        open_db.is_modified = true;

        Ok(group_model)
    }
}
