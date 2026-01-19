use crate::dto::entry::{Entry, EntryListItem};
use crate::dto::group::Group;
use keepass::db::Node;

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

pub(crate) fn convert_entry(entry: &keepass::db::Entry, group_id: &str) -> Entry {
    let times = &entry.times;

    Entry {
        id: entry.uuid.to_string(),
        group_id: group_id.to_string(),
        title: entry.get_title().unwrap_or_default().to_string(),
        username: entry.get_username().unwrap_or_default().to_string(),
        url: entry.get_url().map(std::string::ToString::to_string),
        notes: entry.get("Notes").map(std::string::ToString::to_string),
        created_at: times
            .get_creation()
            .map(std::string::ToString::to_string)
            .unwrap_or_default(),
        modified_at: times
            .get_last_modification()
            .map(std::string::ToString::to_string)
            .unwrap_or_default(),
    }
}

pub(crate) fn convert_entry_to_list_item(
    entry: &keepass::db::Entry,
    group_id: &str,
) -> EntryListItem {
    EntryListItem {
        id: entry.uuid.to_string(),
        group_id: group_id.to_string(),
        title: entry.get_title().unwrap_or_default().to_string(),
        username: entry.get_username().unwrap_or_default().to_string(),
        url: entry.get_url().map(std::string::ToString::to_string),
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
