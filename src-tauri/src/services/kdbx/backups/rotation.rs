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

/// Suffix segment marking a manual backup. Reserved for the manual-backup
/// slice (issue parent #61); auto-snapshot rotation must never touch these.
///
/// Anchored to the position right after the source Vault's basename so a
/// Vault literally named `foo.backup.manual.kdbx` does not accidentally
/// match itself out of rotation.
const MANUAL_SUFFIX_INFIX: &str = ".backup.manual.";

/// Returns auto-snapshot filenames belonging to `vault_filename`, sorted
/// **newest-first** by the timestamp parsed from the filename (not by file
/// mtime — `fs::copy` preserves the source mtime, so mtime is meaningless
/// for ordering snapshots).
///
/// Excluded:
/// - Foreign-vault backups (different basename).
/// - Manual-marker files for our Vault (`<vault>.backup.manual.*`).
/// - Any name that doesn't match the auto-snapshot pattern.
pub(crate) fn select_auto_snapshots<'a, I>(
    names: I,
    vault_filename: &str,
) -> Vec<(DateTime<Utc>, &'a str)>
where
    I: IntoIterator<Item = &'a str>,
{
    // Anchor the manual-marker check to *our* Vault's basename so a Vault
    // whose name itself contains `.backup.manual.` (e.g. `foo.backup.manual.kdbx`)
    // still has its auto-snapshots rotated. The marker can only appear in
    // the backup-suffix segment, never inside the source Vault basename.
    let manual_prefix = format!("{vault_filename}{MANUAL_SUFFIX_INFIX}");
    let mut out: Vec<(DateTime<Utc>, &str)> = names
        .into_iter()
        .filter(|name| !name.starts_with(&manual_prefix))
        .filter_map(|name| {
            let (vault, ts) = parse_backup_filename(name)?;
            if vault == vault_filename {
                Some((ts, name))
            } else {
                None
            }
        })
        .collect();
    // Newest first. Timestamps are unique-by-construction (same-ms collisions
    // bump the snapshot's chosen timestamp at write time) so sort stability
    // doesn't matter here.
    out.sort_by_key(|entry| std::cmp::Reverse(entry.0));
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
    // Only consider regular files. If a subdirectory happens to be named
    // like a snapshot, `fs::remove_file` would fail with `EISDIR` later and
    // a save that already wrote its new snapshot would error post-write.
    let names: Vec<String> = read_dir
        .filter_map(Result::ok)
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
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
/// the App Preferences boundary. As a defensive belt-and-braces measure for
/// hand-edited or corrupted `settings.json` files, a value of `0` is treated
/// as `1` so a single retention slot is always preserved. A misconfiguration
/// must never silently wipe every backup.
pub(crate) fn rotate(
    backup_dir: &Path,
    vault_filename: &str,
    max_versions: u32,
) -> io::Result<usize> {
    let snapshots = list_auto_snapshots(backup_dir, vault_filename)?;
    let keep = max_versions.max(1) as usize;
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

    #[test]
    fn manual_marker_check_is_anchored_to_vault_basename() {
        // A Vault literally named `foo.backup.manual.kdbx` should still
        // have its auto-snapshots rotated. The earlier broad-substring
        // check would have filtered every snapshot of this Vault out.
        let vault = "foo.backup.manual.kdbx";
        let auto = name_for(vault, 1_715_000_000_000);
        // A manual snapshot of the same Vault — must be excluded.
        let manual = format!("{vault}.backup.manual.20260512T143045.123Z.kdbx");

        let names = [auto.as_str(), manual.as_str()];
        let kept: Vec<&str> = select_auto_snapshots(names, vault)
            .into_iter()
            .map(|(_, n)| n)
            .collect();

        assert_eq!(
            kept,
            vec![auto.as_str()],
            "auto-snapshots of a vault whose name contains the marker must still rotate"
        );
    }

    #[test]
    fn rotate_clamps_max_versions_zero_to_keep_one() {
        // Defense-in-depth for a corrupt/hand-edited settings.json file
        // that smuggles `maxVersions = 0` past the App Preferences boundary.
        // Rotation must never wipe every backup.
        let tmp = tempfile::tempdir().expect("tempdir");
        for ms in [1_715_000_000_000i64, 1_715_000_001_000, 1_715_000_002_000] {
            let path = tmp.path().join(name_for("vault.kdbx", ms));
            std::fs::write(&path, b"snap").expect("write");
        }

        let deleted = rotate(tmp.path(), "vault.kdbx", 0).expect("rotate");

        let surviving: Vec<_> = std::fs::read_dir(tmp.path())
            .expect("read")
            .filter_map(Result::ok)
            .collect();
        assert_eq!(deleted, 2, "should delete all but the newest");
        assert_eq!(
            surviving.len(),
            1,
            "max_versions=0 must be coerced to keep at least one snapshot"
        );
    }

    #[test]
    fn rotate_ignores_directories_named_like_snapshots() {
        // A directory in the backup folder that happens to be named like a
        // snapshot must not propagate `EISDIR` out as a failed save.
        let tmp = tempfile::tempdir().expect("tempdir");
        for ms in [1_715_000_000_000i64, 1_715_000_001_000] {
            let path = tmp.path().join(name_for("vault.kdbx", ms));
            std::fs::write(&path, b"snap").expect("write");
        }
        // Plant a directory with a snapshot-shaped name (oldest timestamp
        // so it would be the rotation target).
        let evil_dir = tmp.path().join(name_for("vault.kdbx", 1_710_000_000_000));
        std::fs::create_dir(&evil_dir).expect("create dir");

        // cap=1 → without the file-type filter, this would attempt
        // remove_file on `evil_dir` and surface EISDIR as a save failure.
        let deleted = rotate(tmp.path(), "vault.kdbx", 1).expect("rotate must succeed");

        assert!(evil_dir.exists(), "subdirectory must be untouched");
        // One of the two real snapshots got trimmed.
        assert_eq!(deleted, 1);
    }
}
