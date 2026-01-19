use crate::dto::error::AppError;
use crate::dto::group::Group;

use super::mapping::{convert_group, find_group_by_id};
use super::KdbxService;

impl KdbxService {
    pub fn list_groups(&self) -> Result<Vec<Group>, AppError> {
        let db_lock = self.database.lock().map_err(|_| AppError::Lock)?;
        let open_db = db_lock.as_ref().ok_or(AppError::DatabaseNotOpen)?;

        let root = convert_group(&open_db.db.root, None);
        Ok(vec![root])
    }

    pub fn get_group(&self, id: &str) -> Result<Group, AppError> {
        let db_lock = self.database.lock().map_err(|_| AppError::Lock)?;
        let open_db = db_lock.as_ref().ok_or(AppError::DatabaseNotOpen)?;

        find_group_by_id(&open_db.db.root, id)
            .map(|g| convert_group(g, None))
            .ok_or_else(|| AppError::GroupNotFound(id.to_string()))
    }
}
