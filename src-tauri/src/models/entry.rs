// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    pub tags: Vec<String>,
    pub custom_fields: BTreeMap<String, String>,
    pub created_at: String,
    pub modified_at: String,
    pub accessed_at: String,
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
    pub password: String,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub icon_id: Option<u32>,
    pub tags: Option<Vec<String>>,
    pub custom_fields: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEntryData {
    pub title: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub icon_id: Option<u32>,
    pub tags: Option<Vec<String>>,
    pub custom_fields: Option<BTreeMap<String, String>>,
}
