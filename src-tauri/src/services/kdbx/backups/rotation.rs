// SPDX-License-Identifier: MIT

//! Snapshot rotation for the pre-save backup module.
//!
//! Pure helpers that operate on filenames (no I/O), plus a thin filesystem
//! wrapper that reads a backup directory and applies the cap. The split keeps
//! the glob/sort/exclusion logic unit-testable without touching disk.
//!
//! Rotation is keyed on the source Vault's basename, so two Vaults co-located
//! in the same backup directory rotate independently. The `.backup.manual.`
//! marker is reserved for the later manual-backup slice and is excluded here
//! defensively even though the auto-snapshot parse already rejects those
//! filenames (their timestamp slot is non-numeric).

use crate::services::kdbx::backups::filename::parse_backup_filename;
use chrono::{DateTime, Utc};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Substring that marks a manual backup. Reserved for the manual-backup slice
/// (issue parent #61); auto-snapshot rotation must never touch these.
const MANUAL_MARKER: &str = ".backup.manual.";

/// Returns auto-snapshot filenames belonging to `vault_filename`, sorted
/// **newest-first** by the timestamp parsed from the filename (not by file
/// mtime — `fs::copy` preserves the source mtime, so mtime is meaningless
/// for ordering snapshots).
///
/// Excluded:
/// - Foreign-vault backups (different basename).
/// - Files containing the `.backup.manual.` marker.
/// - Any name that doesn't match the auto-snapshot pattern.
pub(crate) fn select_auto_snapshots<'a, I>(
    names: I,
    vault_filename: &str,
) -> Vec<(DateTime<Utc>, &'a str)>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut out: Vec<(DateTime<Utc>, &str)> = names
        .into_iter()
        .filter(|name| !name.contains(MANUAL_MARKER))
        .filter_map(|name| {
            let (vault, ts) = parse_backup_filename(name)?;
            if vault == vault_filename {
                Some((ts, name))
            } else {
                None
            }
        })
        .collect();
    // Newest first. Stable sort isn't strictly required because timestamps
    // are unique-by-construction (same-ms collisions bump the snapshot's
    // chosen timestamp at write time), but stability costs nothing.
    out.sort_by(|a, b| b.0.cmp(&a.0));
    out
}

/// Walks `backup_dir` and returns auto-snapshot paths for `vault_filename`,
/// sorted newest-first by parsed timestamp. Returns `Ok(vec![])` when the
/// directory does not exist yet (first save).
pub(crate) fn list_auto_snapshots(
    backup_dir: &Path,
    vault_filename: &str,
) -> io::Result<Vec<PathBuf>> {
    let read_dir = match fs::read_dir(backup_dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let names: Vec<String> = read_dir
        .filter_map(Result::ok)
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    let selected = select_auto_snapshots(names.iter().map(String::as_str), vault_filename);
    Ok(selected
        .into_iter()
        .map(|(_, name)| backup_dir.join(name))
        .collect())
}

/// Deletes auto-snapshots beyond `max_versions` for `vault_filename`. Newest
/// `max_versions` files are retained; older ones are removed. Returns the
/// number of files deleted.
///
/// Never touches files outside the auto-snapshot glob for this Vault
/// (foreign-Vault backups, manual-marker files, unrelated files).
///
/// `max_versions` is expected to be in `1..=500` — validation is enforced on
/// the App Preferences boundary, so this function trusts the caller.
pub(crate) fn rotate(
    backup_dir: &Path,
    vault_filename: &str,
    max_versions: u32,
) -> io::Result<usize> {
    let snapshots = list_auto_snapshots(backup_dir, vault_filename)?;
    let keep = max_versions as usize;
    if snapshots.len() <= keep {
        return Ok(0);
    }
    let mut deleted = 0usize;
    for path in snapshots.into_iter().skip(keep) {
        match fs::remove_file(&path) {
            Ok(()) => deleted += 1,
            // Tolerate races (e.g. another process trimmed concurrently).
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
    }
    Ok(deleted)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::services::kdbx::backups::filename::make_backup_filename;
    use chrono::TimeZone;

    fn ts(ms: i64) -> DateTime<Utc> {
        Utc.timestamp_millis_opt(ms).single().expect("valid ts")
    }

    fn name_for(vault: &str, ms: i64) -> String {
        make_backup_filename(vault, ts(ms))
    }

    #[test]
    fn excludes_foreign_vault_backups() {
        let our_old = name_for("vault.kdbx", 1_715_000_000_000);
        let our_new = name_for("vault.kdbx", 1_715_000_001_000);
        let foreign = name_for("other.kdbx", 1_715_000_000_500);

        let names = [our_old.as_str(), foreign.as_str(), our_new.as_str()];
        let kept: Vec<&str> = select_auto_snapshots(names, "vault.kdbx")
            .into_iter()
            .map(|(_, n)| n)
            .collect();

        assert_eq!(
            kept,
            vec![our_new.as_str(), our_old.as_str()],
            "must keep only our-vault snapshots, newest-first"
        );
    }

    #[test]
    fn excludes_manual_marker_files() {
        let auto = name_for("vault.kdbx", 1_715_000_000_000);
        // Manual-backup naming convention reserved for the later slice.
        let manual = "vault.kdbx.backup.manual.20260512T143045.123Z.kdbx".to_string();

        let names = [auto.as_str(), manual.as_str()];
        let kept: Vec<&str> = select_auto_snapshots(names, "vault.kdbx")
            .into_iter()
            .map(|(_, n)| n)
            .collect();

        assert_eq!(kept, vec![auto.as_str()], "must skip manual-marker file");
    }

    #[test]
    fn excludes_unrelated_files() {
        let auto = name_for("vault.kdbx", 1_715_000_000_000);
        let names = [
            auto.as_str(),
            "README.txt",
            "vault.kdbx",
            "vault.kdbx.bak",
            ".DS_Store",
            "vault.kdbx.backup.notadate.kdbx",
        ];
        let kept: Vec<&str> = select_auto_snapshots(names, "vault.kdbx")
            .into_iter()
            .map(|(_, n)| n)
            .collect();

        assert_eq!(
            kept,
            vec![auto.as_str()],
            "must skip every name that is not an auto-snapshot for our vault"
        );
    }

    #[test]
    fn sorts_newest_first_by_parsed_timestamp_not_lex() {
        // Construct timestamps that would sort the same way lexically and
        // chronologically (the filename format guarantees that), then assert
        // newest comes first.
        let oldest = name_for("vault.kdbx", 1_715_000_000_000);
        let middle = name_for("vault.kdbx", 1_715_500_000_000);
        let newest = name_for("vault.kdbx", 1_900_000_000_000);

        // Feed in shuffled order to ensure sort is deterministic regardless of
        // directory enumeration order (which is OS-dependent).
        let names = [middle.as_str(), oldest.as_str(), newest.as_str()];
        let kept: Vec<&str> = select_auto_snapshots(names, "vault.kdbx")
            .into_iter()
            .map(|(_, n)| n)
            .collect();

        assert_eq!(
            kept,
            vec![newest.as_str(), middle.as_str(), oldest.as_str()]
        );
    }

    #[test]
    fn handles_vault_basenames_that_themselves_contain_backup() {
        // The vault file might be literally named `my.backup.kdbx`. Confirm
        // parsing and basename matching still works.
        let vault = "my.backup.kdbx";
        let ours = name_for(vault, 1_715_000_000_000);
        let foreign = name_for("other.kdbx", 1_715_000_000_000);

        let names = [ours.as_str(), foreign.as_str()];
        let kept: Vec<&str> = select_auto_snapshots(names, vault)
            .into_iter()
            .map(|(_, n)| n)
            .collect();

        assert_eq!(kept, vec![ours.as_str()]);
    }

    #[test]
    fn list_auto_snapshots_returns_empty_when_dir_missing() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let missing = tmp.path().join("does-not-exist");
        let result =
            list_auto_snapshots(&missing, "vault.kdbx").expect("missing dir should not error");
        assert!(result.is_empty());
    }
}
