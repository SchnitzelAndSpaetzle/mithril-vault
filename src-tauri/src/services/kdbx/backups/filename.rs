// SPDX-License-Identifier: MIT

//! Pure helpers for backup snapshot filenames.
//!
//! Pattern: `<vault-filename-with-extension>.backup.<YYYYMMDDTHHMMSS.mmmZ>.kdbx`.
//! Lex sort over generated names matches chronological order by construction
//! (fixed-width fields, UTC, basic ISO 8601). Same-millisecond collisions are
//! resolved by bumping to the next free millisecond.

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};

const TIMESTAMP_FMT: &str = "%Y%m%dT%H%M%S%.3fZ";
const BACKUP_INFIX: &str = ".backup.";
const MANUAL_INFIX: &str = ".backup.manual.";
const BACKUP_SUFFIX: &str = ".kdbx";

/// Builds the snapshot filename for a Vault and an auto-snapshot timestamp.
pub fn make_backup_filename(vault_filename: &str, ts: DateTime<Utc>) -> String {
    format!(
        "{vault_filename}{BACKUP_INFIX}{}{BACKUP_SUFFIX}",
        ts.format(TIMESTAMP_FMT)
    )
}

/// Parses a snapshot filename back into the source Vault filename and its
/// auto-snapshot timestamp. Returns `None` if the name does not match the
/// auto-snapshot pattern.
pub fn parse_backup_filename(filename: &str) -> Option<(String, DateTime<Utc>)> {
    let stem = filename.strip_suffix(BACKUP_SUFFIX)?;
    let dot = stem.rfind(BACKUP_INFIX)?;
    let (vault_filename, rest) = stem.split_at(dot);
    let ts_str = &rest[BACKUP_INFIX.len()..];
    let parsed = NaiveDateTime::parse_from_str(ts_str, TIMESTAMP_FMT).ok()?;
    let ts = Utc.from_utc_datetime(&parsed);
    Some((vault_filename.to_string(), ts))
}

/// Parses a manual-snapshot filename back into the source Vault filename and
/// its timestamp. Returns `None` if the name does not match the manual-snapshot
/// pattern (`<vault>.backup.manual.<YYYYMMDDTHHMMSS.mmmZ>.kdbx`).
///
/// Mirrors `parse_backup_filename` but anchored on the `.backup.manual.` infix
/// so a vault basename that happens to contain `.backup.manual.` literally
/// (e.g. `team.backup.manual.notes.kdbx`) is NOT mistakenly classified as a
/// snapshot.
pub fn parse_manual_backup_filename(filename: &str) -> Option<(String, DateTime<Utc>)> {
    let stem = filename.strip_suffix(BACKUP_SUFFIX)?;
    let infix_pos = stem.rfind(MANUAL_INFIX)?;
    let (vault_filename, rest) = stem.split_at(infix_pos);
    let ts_str = &rest[MANUAL_INFIX.len()..];
    let parsed = NaiveDateTime::parse_from_str(ts_str, TIMESTAMP_FMT).ok()?;
    let ts = Utc.from_utc_datetime(&parsed);
    Some((vault_filename.to_string(), ts))
}

/// Returns the next timestamp at-or-after `start` whose generated filename is
/// not already taken by `existing`. Bumps in 1ms increments. Pure.
pub fn next_free_timestamp<S: std::hash::BuildHasher>(
    vault_filename: &str,
    start: DateTime<Utc>,
    existing: &std::collections::HashSet<String, S>,
) -> DateTime<Utc> {
    let mut ts = start;
    loop {
        let candidate = make_backup_filename(vault_filename, ts);
        if !existing.contains(&candidate) {
            return ts;
        }
        ts += chrono::Duration::milliseconds(1);
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::collections::HashSet;

    fn ts(ms: i64) -> DateTime<Utc> {
        Utc.timestamp_millis_opt(ms).single().expect("valid ts")
    }

    #[test]
    fn make_filename_matches_expected_pattern() {
        let t = Utc
            .with_ymd_and_hms(2026, 5, 12, 14, 30, 45)
            .single()
            .expect("valid")
            + chrono::Duration::milliseconds(123);
        let name = make_backup_filename("vault.kdbx", t);
        assert_eq!(name, "vault.kdbx.backup.20260512T143045.123Z.kdbx");
    }

    #[test]
    fn parse_roundtrip() {
        let original = ts(1_715_000_000_123);
        let name = make_backup_filename("vault.kdbx", original);
        let (vault, parsed) = parse_backup_filename(&name).expect("parses");
        assert_eq!(vault, "vault.kdbx");
        assert_eq!(parsed, original);
    }

    #[test]
    fn parse_rejects_non_backup_names() {
        assert!(parse_backup_filename("vault.kdbx").is_none());
        assert!(parse_backup_filename("vault.kdbx.bak").is_none());
        assert!(parse_backup_filename("vault.kdbx.backup.notadate.kdbx").is_none());
    }

    #[test]
    fn parse_handles_vault_names_containing_dot_backup() {
        let original = ts(1_715_000_000_456);
        let name = make_backup_filename("my.backup.notes.kdbx", original);
        let (vault, parsed) = parse_backup_filename(&name).expect("parses");
        assert_eq!(vault, "my.backup.notes.kdbx");
        assert_eq!(parsed, original);
    }

    #[test]
    fn lex_sort_matches_chronological_sort() {
        let timestamps = [
            ts(1_715_000_000_000),
            ts(1_715_000_000_001),
            ts(1_715_000_001_000),
            ts(1_715_000_500_000),
            ts(1_900_000_000_000),
        ];
        let names: Vec<String> = timestamps
            .iter()
            .map(|t| make_backup_filename("vault.kdbx", *t))
            .collect();

        let mut by_chrono = names.clone();
        // Already in chrono order because the source list is.
        let mut by_lex = names.clone();
        by_lex.sort();
        // by_chrono is already chronological, but make it explicit.
        by_chrono.sort_by_key(|n| {
            parse_backup_filename(n)
                .expect("parses")
                .1
                .timestamp_millis()
        });

        assert_eq!(by_lex, by_chrono);
    }

    #[test]
    fn next_free_returns_start_when_no_collision() {
        let start = ts(1_715_000_000_000);
        let chosen = next_free_timestamp("vault.kdbx", start, &HashSet::new());
        assert_eq!(chosen, start);
    }

    #[test]
    fn next_free_bumps_until_unused() {
        let start = ts(1_715_000_000_000);
        let mut taken: HashSet<String> = HashSet::new();
        taken.insert(make_backup_filename("vault.kdbx", start));
        taken.insert(make_backup_filename(
            "vault.kdbx",
            start + chrono::Duration::milliseconds(1),
        ));

        let chosen = next_free_timestamp("vault.kdbx", start, &taken);
        assert_eq!(chosen, start + chrono::Duration::milliseconds(2));
    }
}
