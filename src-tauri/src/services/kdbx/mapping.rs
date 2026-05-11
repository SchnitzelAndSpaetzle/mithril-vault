use crate::domain::secure::SecureString;
use crate::dto::entry::{CustomFieldMeta, Entry};
use crate::dto::group::Group;
use keepass::db::{
    Entry as KeepassEntry, EntryId, EntryRef, GroupId, GroupRef, Icon, Times, Value,
};
use keepass::Database;
use std::collections::BTreeMap;

pub(crate) fn find_group_id(db: &Database, id: &str) -> Option<GroupId> {
    db.iter_all_groups()
        .find(|g| g.id().uuid().to_string() == id)
        .map(|g| g.id())
}

pub(crate) fn find_entry_id(db: &Database, id: &str) -> Option<EntryId> {
    db.iter_all_entries()
        .find(|e| e.id().uuid().to_string() == id)
        .map(|e| e.id())
}

pub(crate) fn find_group_by_id<'a>(db: &'a Database, id: &str) -> Option<GroupRef<'a>> {
    find_group_id(db, id).and_then(|gid| db.group(gid))
}

pub(crate) fn find_group_by_name<'a>(db: &'a Database, name: &str) -> Option<GroupRef<'a>> {
    db.iter_all_groups().find(|g| g.name == name)
}

pub(crate) fn find_parent_group_id(db: &Database, target_id: &str) -> Option<String> {
    let target = find_group_by_id(db, target_id)?;
    target.parent().map(|p| p.id().uuid().to_string())
}

pub(crate) fn is_ancestor_of(db: &Database, ancestor_id: &str, descendant_id: &str) -> bool {
    if ancestor_id == descendant_id {
        return true;
    }
    let Some(start) = find_group_by_id(db, descendant_id) else {
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

pub(crate) fn convert_entry(entry: &EntryRef<'_>, group_id: &str) -> Entry {
    let (icon_id, custom_icon_uuid) = match entry.icon() {
        Some(Icon::BuiltIn(n)) => (u32::try_from(*n).ok(), None),
        Some(Icon::Custom(cid)) => (None, Some(cid.uuid().to_string())),
        None => (None, None),
    };
    let (custom_fields, custom_field_meta) = collect_custom_fields(entry);

    Entry {
        id: entry.id().uuid().to_string(),
        group_id: group_id.to_string(),
        title: entry.get_title().unwrap_or_default().to_string(),
        username: entry.get_username().unwrap_or_default().to_string(),
        url: entry.get_url().map(std::string::ToString::to_string),
        notes: entry.get("Notes").map(std::string::ToString::to_string),
        icon_id,
        custom_icon_uuid,
        tags: entry.tags.clone(),
        custom_fields,
        custom_field_meta,
        created_at: entry
            .times
            .creation
            .map(|t| t.to_string())
            .unwrap_or_default(),
        modified_at: entry
            .times
            .last_modification
            .map(|t| t.to_string())
            .unwrap_or_default(),
        accessed_at: entry
            .times
            .last_access
            .map(|t| t.to_string())
            .unwrap_or_default(),
    }
}

pub(crate) fn convert_group(group: &GroupRef<'_>, parent_id: Option<&str>) -> Group {
    let id = group.id().uuid().to_string();
    let (icon, custom_icon_uuid) = match group.icon() {
        Some(Icon::BuiltIn(n)) => (Some(n.to_string()), None),
        Some(Icon::Custom(cid)) => (None, Some(cid.uuid().to_string())),
        None => (None, None),
    };
    let children = group
        .groups()
        .map(|child| convert_group(&child, Some(&id)))
        .collect();

    Group {
        id: id.clone(),
        parent_id: parent_id.map(std::string::ToString::to_string),
        name: group.name.clone(),
        icon,
        custom_icon_uuid,
        children,
    }
}

pub(crate) fn is_standard_entry_field(key: &str) -> bool {
    matches!(
        key,
        "Title" | "UserName" | "Password" | "URL" | "Notes" | "otp"
    )
}

pub(crate) fn insert_custom_fields(
    entry: &mut KeepassEntry,
    custom_fields: &BTreeMap<String, String>,
) {
    for (key, value) in custom_fields {
        if is_standard_entry_field(key) {
            continue;
        }
        entry
            .fields
            .insert(key.clone(), Value::Unprotected(value.clone()));
    }
}

pub(crate) fn insert_protected_custom_fields(
    entry: &mut KeepassEntry,
    protected_fields: &BTreeMap<String, SecureString>,
) {
    for (key, value) in protected_fields {
        if is_standard_entry_field(key) {
            continue;
        }
        entry
            .fields
            .insert(key.clone(), Value::protected(value.as_str().to_string()));
    }
}

pub(crate) fn apply_custom_fields(
    entry: &mut KeepassEntry,
    custom_fields: Option<&BTreeMap<String, String>>,
    protected_custom_fields: Option<&BTreeMap<String, SecureString>>,
) {
    if let Some(fields) = custom_fields {
        insert_custom_fields(entry, fields);
    }
    if let Some(fields) = protected_custom_fields {
        insert_protected_custom_fields(entry, fields);
    }
}

pub(crate) fn replace_custom_fields(
    entry: &mut KeepassEntry,
    custom_fields: Option<&BTreeMap<String, String>>,
    protected_custom_fields: Option<&BTreeMap<String, SecureString>>,
) {
    entry.fields.retain(|key, _| is_standard_entry_field(key));
    apply_custom_fields(entry, custom_fields, protected_custom_fields);
}

pub(crate) fn collect_custom_fields(
    entry: &EntryRef<'_>,
) -> (BTreeMap<String, String>, Vec<CustomFieldMeta>) {
    let mut custom_fields = BTreeMap::new();
    let mut custom_field_meta = Vec::new();

    for (key, value) in &entry.fields {
        if is_standard_entry_field(key) {
            continue;
        }

        let (rendered, is_protected) = match value {
            Value::Unprotected(text) => (Some(text.clone()), false),
            Value::Protected(_) => (None, true),
        };

        if let Some(value) = rendered {
            custom_fields.insert(key.clone(), value);
        }

        custom_field_meta.push(CustomFieldMeta {
            key: key.clone(),
            is_protected,
        });
    }

    (custom_fields, custom_field_meta)
}

/// Ensures a recycle bin exists and returns its UUID as a string.
pub(crate) fn ensure_recycle_bin(db: &mut Database) -> String {
    if let Some(uuid) = db.meta.recyclebin_uuid {
        if find_group_by_id(db, &uuid.to_string()).is_some() {
            db.meta.recyclebin_enabled = Some(true);
            db.meta.recyclebin_changed = Some(Times::now());
            return uuid.to_string();
        }
    }

    if let Some(existing) = find_group_by_name(db, "Recycle Bin") {
        let uuid = existing.id().uuid();
        db.meta.recyclebin_enabled = Some(true);
        db.meta.recyclebin_uuid = Some(uuid);
        db.meta.recyclebin_changed = Some(Times::now());
        return uuid.to_string();
    }

    let new_uuid = {
        let mut root = db.root_mut();
        let mut new_group = root.add_group();
        new_group.name = "Recycle Bin".to_string();
        new_group.id().uuid()
    };

    db.meta.recyclebin_enabled = Some(true);
    db.meta.recyclebin_uuid = Some(new_uuid);
    db.meta.recyclebin_changed = Some(Times::now());

    new_uuid.to_string()
}
