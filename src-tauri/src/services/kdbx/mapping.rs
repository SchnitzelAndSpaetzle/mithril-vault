use crate::dto::entry::{CustomFieldMeta, Entry};
use crate::dto::group::Group;
use keepass::db::{Entry as KeepassEntry, Node, Value};
use secstr::SecStr;
use std::collections::BTreeMap;

pub(crate) fn find_group_by_id<'a>(
    group: &'a keepass::db::Group,
    id: &str,
) -> Option<&'a keepass::db::Group> {
    if group.uuid.to_string() == id {
        return Some(group);
    }

    for node in &group.children {
        if let Node::Group(child) = node {
            if let Some(found) = find_group_by_id(child, id) {
                return Some(found);
            }
        }
    }

    None
}

pub(crate) fn find_group_by_name<'a>(
    group: &'a keepass::db::Group,
    name: &str,
) -> Option<&'a keepass::db::Group> {
    if group.name == name {
        return Some(group);
    }

    for node in &group.children {
        if let Node::Group(child) = node {
            if let Some(found) = find_group_by_name(child, name) {
                return Some(found);
            }
        }
    }

    None
}

pub(crate) fn convert_entry(entry: &keepass::db::Entry, group_id: &str) -> Entry {
    let times = &entry.times;
    let (custom_fields, custom_field_meta) = collect_custom_fields(entry);

    Entry {
        id: entry.uuid.to_string(),
        group_id: group_id.to_string(),
        title: entry.get_title().unwrap_or_default().to_string(),
        username: entry.get_username().unwrap_or_default().to_string(),
        url: entry.get_url().map(std::string::ToString::to_string),
        notes: entry.get("Notes").map(std::string::ToString::to_string),
        icon_id: entry.icon_id.and_then(|id| u32::try_from(id).ok()),
        tags: entry.tags.clone(),
        custom_fields,
        custom_field_meta,
        created_at: times
            .get_creation()
            .map(std::string::ToString::to_string)
            .unwrap_or_default(),
        modified_at: times
            .get_last_modification()
            .map(std::string::ToString::to_string)
            .unwrap_or_default(),
        accessed_at: times
            .get_last_access()
            .map(std::string::ToString::to_string)
            .unwrap_or_default(),
    }
}

pub(crate) fn convert_group(group: &keepass::db::Group, parent_id: Option<&str>) -> Group {
    let id = group.uuid.to_string();
    let mut children = Vec::new();

    for node in &group.children {
        if let Node::Group(child) = node {
            children.push(convert_group(child, Some(&id)));
        }
    }

    Group {
        id: id.clone(),
        parent_id: parent_id.map(std::string::ToString::to_string),
        name: group.name.clone(),
        icon: group.icon_id.map(|i| i.to_string()),
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
    protect_values: bool,
) {
    for (key, value) in custom_fields {
        if is_standard_entry_field(key) {
            continue;
        }
        let field_value = if protect_values {
            Value::Protected(SecStr::new(value.as_bytes().to_vec()))
        } else {
            Value::Unprotected(value.clone())
        };
        entry.fields.insert(key.clone(), field_value);
    }
}

pub(crate) fn apply_custom_fields(
    entry: &mut KeepassEntry,
    custom_fields: Option<&BTreeMap<String, String>>,
    protected_custom_fields: Option<&BTreeMap<String, String>>,
) {
    if let Some(fields) = custom_fields {
        insert_custom_fields(entry, fields, false);
    }
    if let Some(fields) = protected_custom_fields {
        insert_custom_fields(entry, fields, true);
    }
}

pub(crate) fn replace_custom_fields(
    entry: &mut KeepassEntry,
    custom_fields: Option<&BTreeMap<String, String>>,
    protected_custom_fields: Option<&BTreeMap<String, String>>,
) {
    entry.fields.retain(|key, _| is_standard_entry_field(key));
    apply_custom_fields(entry, custom_fields, protected_custom_fields);
}

pub(crate) fn collect_custom_fields(
    entry: &keepass::db::Entry,
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
            Value::Bytes(_) => continue,
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
