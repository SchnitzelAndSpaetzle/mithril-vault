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

/// Where a candidate attachment's size falls relative to the configured
/// guardrails. `Ok` is under (or at) the soft threshold — add silently.
/// `OverSoft` is above the soft threshold but within the hard cap — the add
/// flow prompts for confirmation. `OverHard` is above the hard cap — rejected
/// outright. Serialized in camelCase for the IPC plan the frontend reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AttachmentSizeStatus {
    Ok,
    OverSoft,
    OverHard,
}

/// One candidate file in an attachment-add plan: the basename it was picked
/// under, its on-disk size, and where that size falls relative to the
/// thresholds. Returned by the prepare step so the frontend can decide whether
/// to prompt before committing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentPlanItem {
    pub source_name: String,
    pub size: u64,
    pub status: AttachmentSizeStatus,
}

/// The classification of a batch of candidate files against the configured
/// guardrails, returned by the prepare step. `requires_confirmation` is `true`
/// iff at least one item is `OverSoft` — the single signal the frontend uses to
/// decide whether to show the soft-warning prompt before committing. Files over
/// the hard cap do not gate the prompt; they surface as per-file failures at
/// commit time instead.
///
/// `batch_id` is the generation of the buffered batch this plan describes. The
/// frontend echoes it back to `commit_prepared_attachments`, which only stores
/// the batch when the id still matches — so a later pick/drop that supersedes the
/// buffer turns a stale (e.g. post-confirmation) commit into a no-op rather than
/// attaching the wrong file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentAddPlan {
    pub items: Vec<AttachmentPlanItem>,
    pub requires_confirmation: bool,
    pub batch_id: u64,
}

/// One file that failed to add in a batch, paired with the basename it was
/// picked under and the backend's reason (the `AppError` display string, e.g.
/// "…exceeds the 25-byte limit"). The frontend raises one toast per failure so
/// the user can tell which file failed and whether a retry could ever work.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentAddFailure {
    pub source_name: String,
    pub reason: String,
}

/// Result of adding a batch of picked files to an Entry. `added` holds the
/// stored filenames in pick order (which may differ from the source basename
/// after an auto-rename); `failed` holds the per-file failures. A failure on
/// one file never aborts the rest, mirroring the single-add resilience.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddAttachmentsOutcome {
    pub added: Vec<String>,
    pub failed: Vec<AttachmentAddFailure>,
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
