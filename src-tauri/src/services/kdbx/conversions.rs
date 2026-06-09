use crate::domain::secure::SecureString;
use crate::dto::entry::{AttachmentMeta, CustomFieldMeta, Entry};
use crate::dto::error::AppError;
use crate::dto::group::Group;
use chrono::DateTime;
use keepass::db::{Entry as KeepassEntry, EntryMut, EntryRef, GroupRef, Icon, Value};
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
    let attachments = collect_attachments(entry);

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
        expires: entry.times.expires.unwrap_or(false),
        // Stored as a naive UTC instant (the KDBX format carries no timezone);
        // surface it as RFC 3339 so the frontend and Password Health agree,
        // consistent with `expiry.and_utc()` in the analyzer.
        expiry_time: entry.times.expiry.map(|t| t.and_utc().to_rfc3339()),
        attachments,
    }
}

/// Collects an Entry's Attachment metadata — filename, byte size, and a MIME
/// hint derived from the extension — from its native KDBX binary references.
/// KDBX3 (XML) and KDBX4 (header) binaries are normalized to one model by the
/// `keepass` crate, so both surface here identically. The byte payload is never
/// read out; only its length is, keeping list/get responses lightweight per
/// ADR-0003.
fn collect_attachments(entry: &EntryRef<'_>) -> Vec<AttachmentMeta> {
    entry
        .attachments_named()
        .map(|(filename, attachment)| AttachmentMeta {
            filename: filename.to_string(),
            size: attachment.data.get().len() as u64,
            mime_type: derive_attachment_mime(filename),
        })
        .collect()
}

/// Derives a MIME hint from a filename's extension. Covers the formats the
/// attachments feature cares about (previewable images and plain text per
/// `CONTEXT.md`/ADR-0003) plus common document types; everything else falls
/// back to `application/octet-stream`. Extension match is case-insensitive.
fn derive_attachment_mime(filename: &str) -> String {
    let ext = filename
        .rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase())
        .unwrap_or_default();

    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "txt" | "log" | "conf" | "ini" => "text/plain",
        "md" => "text/markdown",
        "csv" => "text/csv",
        "json" => "application/json",
        "xml" => "application/xml",
        "yaml" | "yml" => "application/yaml",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    };

    mime.to_string()
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

/// Parses an optional RFC 3339 expiry timestamp into the naive UTC instant the
/// KDBX format stores (it carries no timezone), consistent with the read path
/// in `convert_entry`. `None` passes through as `None` ("leave the timestamp
/// untouched"); a value that does not parse is rejected as invalid input.
///
/// This is the *fallible* half of expiry handling and runs as up-front
/// validation, before any entry is created or mutated, so a malformed payload
/// cannot leave a partial/phantom entry behind.
pub(crate) fn parse_expiry_time(
    expiry_time: Option<&str>,
) -> Result<Option<chrono::NaiveDateTime>, AppError> {
    expiry_time
        .map(|raw| {
            DateTime::parse_from_rfc3339(raw)
                .map(|dt| dt.with_timezone(&chrono::Utc).naive_utc())
                .map_err(|e| AppError::InvalidInput(format!("invalid expiryTime: {e}")))
        })
        .transpose()
}

/// Rejects enabling expiry without any timestamp to anchor it. An entry with
/// `expires=true` but no `expiry` is an ambiguous state — Password Health only
/// flags entries that carry a timestamp, so such an entry could never report as
/// expired. Enabling expiry therefore requires a timestamp to fall back on:
/// either one supplied in this request (`new_expiry`) or one already stored on
/// the entry (`existing_expiry`, the re-enable case). Disabling or leaving the
/// flag untouched is always allowed.
pub(crate) fn validate_expiry_enabled(
    expires: Option<bool>,
    new_expiry: Option<chrono::NaiveDateTime>,
    existing_expiry: Option<chrono::NaiveDateTime>,
) -> Result<(), AppError> {
    if expires == Some(true) && new_expiry.is_none() && existing_expiry.is_none() {
        return Err(AppError::InvalidInput(
            "enabling expiry requires an expiryTime".to_string(),
        ));
    }
    Ok(())
}

/// Writes the expiry flag and an already-parsed timestamp into a keepass
/// entry's `Times`. Each field is independent: `expires` flips the flag only
/// when `Some`, and `expiry` is written only when `Some`. Unchecking expiry
/// (`expires=Some(false)`, `expiry=None`) therefore clears the flag while
/// retaining the previously stored timestamp, mirroring KeePass/KeePassXC.
///
/// Infallible by construction: the timestamp is validated earlier via
/// `parse_expiry_time`, keeping the tree mutation atomic.
pub(crate) fn apply_expiry(
    entry: &mut EntryMut<'_>,
    expires: Option<bool>,
    expiry: Option<chrono::NaiveDateTime>,
) {
    if let Some(expires) = expires {
        entry.times.expires = Some(expires);
    }
    if let Some(expiry) = expiry {
        entry.times.expiry = Some(expiry);
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
