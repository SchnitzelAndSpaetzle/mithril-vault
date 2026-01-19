pub mod create;
pub mod entries;
pub mod groups;
pub mod key;
pub mod mapping;
pub mod open;
pub mod save;

use crate::domain::kdbx::OpenDatabase;
use std::sync::Mutex;

pub struct KdbxService {
    database: Mutex<Option<OpenDatabase>>,
}

impl KdbxService {
    pub fn new() -> Self {
        Self {
            database: Mutex::new(None),
        }
    }
}

impl Default for KdbxService {
    fn default() -> Self {
        Self::new()
    }
}
