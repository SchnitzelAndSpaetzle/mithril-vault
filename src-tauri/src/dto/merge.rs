// SPDX-License-Identifier: MIT
//! IPC structures for the Merge engine (ADR-0005).

use serde::Serialize;

/// Structured report of what a Merge combined and what conflicted.
///
/// Returned alongside the merged database by the pure merge engine and
/// surfaced to the frontend as the post-merge summary toast (a full
/// review surface is a later slice).
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeSummary {
    /// Entries that existed only on the incoming side and were added.
    pub entries_added: u32,
    /// Entries whose content changed through a clean (non-conflicting)
    /// update — edited on only one side.
    pub entries_updated: u32,
    /// Entries removed because the other side deleted them (propagated
    /// via the KDBX `DeletedObjects` list).
    pub entries_deleted: u32,
    /// Entries edited on both sides, resolved newest-wins with the losing
    /// version preserved in that Entry's KDBX history.
    pub conflicts: Vec<MergeConflict>,
    /// Security-posture differences between the two sides. These are
    /// reported for explicit user confirmation and never auto-applied —
    /// the merged Vault always keeps the local configuration.
    pub security_posture_changes: Vec<SecurityPostureChange>,
}

/// One aspect of the Vault's security posture that differs between the
/// local and incoming copies (ADR-0006 carve-out: never applied silently).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SecurityPostureChange {
    /// Key-derivation function or its parameters differ.
    Kdf,
    /// Outer encryption cipher differs.
    OuterCipher,
    /// Inner stream cipher differs.
    InnerCipher,
    /// Compression setting differs.
    Compression,
}

/// One same-entry conflict the Merge resolved newest-wins.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeConflict {
    /// UUID of the conflicted Entry.
    pub entry_id: String,
    /// Entry title at the winning version, for display in the summary.
    pub title: String,
}
