use crate::domain::secure::SecureString;
use crate::dto::entry::{CustomFieldMeta, Entry};
use crate::dto::group::Group;
use keepass::db::{Entry as KeepassEntry, EntryRef, GroupRef, Icon, Value};
use std::collections::BTreeMap;

pub(crate) fn convert_entry(entry: &EntryRef<'_>, group_id: &str) -> Entry {
    // keepass 0.12 collapsed builtin/custom icons into a single Icon enum:
    // an entry has either Icon::BuiltIn(n) or Icon::Custom(cid) or no icon.
    // Mirror that shape faithfully to the frontend so update_entry can use
    // `data.icon_id.is_some()` as "user explicitly picked a builtin" — a
    // synthesized echo would silently overwrite the entry's custom icon on
    // the round-trip through the update form.
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
    // keepass 0.12 stores child groups in a HashSet, so iteration order is
    // nondeterministic. Sort by (name, id) so the sidebar renders stably
    // across reloads.
    let mut children: Vec<Group> = group
        .groups()
        .map(|child| convert_group(&child, Some(&id)))
        .collect();
    children.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));

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
