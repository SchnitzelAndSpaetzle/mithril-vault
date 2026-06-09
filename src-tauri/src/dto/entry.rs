// SPDX-License-Identifier: MIT

use crate::domain::secure::SecureString;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomFieldMeta {
    pub key: String,
    pub is_protected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomFieldValue {
    pub key: String,
    pub value: String,
}

/// Metadata for an Entry's Attachment — never its byte payload.
///
/// Attachments are presented as per-Entry and private (see `CONTEXT.md` →
/// "Attachment"); the KDBX Vault-level binary pool and its dedup stay
/// invisible. Bytes are fetched per-file on demand, so list/get responses
/// carry only the filename, byte size, and a MIME hint derived from the
/// filename extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentMeta {
    pub filename: String,
    pub size: u64,
    pub mime_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    pub group_id: String,
    pub title: String,
    pub username: String,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub icon_id: Option<u32>,
    pub custom_icon_uuid: Option<String>,
    pub tags: Vec<String>,
    pub custom_fields: BTreeMap<String, String>,
    pub custom_field_meta: Vec<CustomFieldMeta>,
    pub created_at: String,
    pub modified_at: String,
    pub accessed_at: String,
    pub expires: bool,
    pub expiry_time: Option<String>,
    pub attachments: Vec<AttachmentMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryListItem {
    pub id: String,
    pub group_id: String,
    pub title: String,
    pub username: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateEntryData {
    pub title: String,
    pub username: String,
    pub password: SecureString,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub icon_id: Option<u32>,
    pub tags: Option<Vec<String>>,
    pub custom_fields: Option<BTreeMap<String, String>>,
    pub protected_custom_fields: Option<BTreeMap<String, SecureString>>,
    pub expires: Option<bool>,
    pub expiry_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEntryData {
    pub title: Option<String>,
    pub username: Option<String>,
    pub password: Option<SecureString>,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub icon_id: Option<u32>,
    pub tags: Option<Vec<String>>,
    pub custom_fields: Option<BTreeMap<String, String>>,
    pub protected_custom_fields: Option<BTreeMap<String, SecureString>>,
    pub expires: Option<bool>,
    pub expiry_time: Option<String>,
}
