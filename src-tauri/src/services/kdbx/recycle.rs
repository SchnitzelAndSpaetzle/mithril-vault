// SPDX-License-Identifier: MIT

//! Recycle-bin ancestry walks shared across services.
//!
//! The helper only touches `keepass` types — no `KdbxService` or
//! domain state — so callers in `password_health` can use it without
//! pulling in KDBX-internal dependencies.

use keepass::db::EntryRef;
use keepass::Database;

/// Walks `entry`'s ancestor chain looking for `recycle_uuid`. Returns
/// `true` when the Entry sits inside the Recycle Bin (or any descendant
/// of it). Walks by `GroupId` rather than holding a `GroupRef` across
/// iterations so each step has its own borrow scope.
pub(crate) fn is_in_recycle_bin(
    db: &Database,
    entry: &EntryRef<'_>,
    recycle_uuid: uuid::Uuid,
) -> bool {
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
