// SPDX-License-Identifier: MIT

//! Composing facade for the audit subsystem.
//!
//! Wires the five deep modules (`format`, `crypto`, `storage`, `key`,
//! `vault_id`) plus the retention stub into the public surface used by the
//! rest of the app:
//!
//! * [`AuditService::record`] — append an event. Infallible by contract:
//!   internal errors are swallowed and flip a `degraded` flag so a broken
//!   audit log cannot become a `DoS` vector against the user's own Vault flows.
//! * [`AuditService::read`] — list events for a Vault path.

use crate::services::audit::crypto::{decrypt, encrypt, KEY_LEN};
use crate::services::audit::format::AuditEvent;
use crate::services::audit::key::AuditKey;
use crate::services::audit::retention::apply_retention;
use crate::services::audit::storage::AuditLogFile;
use crate::services::audit::vault_id::hash_vault_path;
use chrono::Utc;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use thiserror::Error;

/// Errors surfaced from [`AuditService::read`]. Distinct from the
/// internally-swallowed errors of [`AuditService::record`]: read is called
/// from the Settings → Audit Log panel, where "failed to load" is a real
/// state the UI must render differently from "no events yet".
#[derive(Debug, Error)]
pub enum AuditReadError {
    #[error("audit key unavailable: {0}")]
    Key(String),
    #[error("audit log read failed: {0}")]
    Storage(String),
}

/// Permissive filter applied by [`AuditService::read`]. Shape is in place so
/// future issues can plug in `kinds` / time-range filtering without changing
/// callers; today it returns everything.
#[derive(Debug, Default, Clone)]
pub struct AuditFilter {}

pub struct AuditService {
    base_dir: PathBuf,
    key_source: Arc<dyn AuditKey>,
    degraded: AtomicBool,
    /// Per-Vault consecutive-failed-unlock counters, keyed by canonicalized
    /// path. Lives in memory only — resets on process restart per the AC's
    /// "per session" wording.
    attempts: Mutex<HashMap<PathBuf, u32>>,
}

impl AuditService {
    pub fn new(base_dir: PathBuf, key_source: Arc<dyn AuditKey>) -> Self {
        Self {
            base_dir,
            key_source,
            degraded: AtomicBool::new(false),
            attempts: Mutex::new(HashMap::new()),
        }
    }

    /// Reports whether a previous `record` call failed internally. Surfaced
    /// in Settings → Audit Log as a banner; the failed user action itself is
    /// never affected.
    pub fn is_degraded(&self) -> bool {
        self.degraded.load(Ordering::SeqCst)
    }

    /// Appends `event` to the per-Vault audit log. Infallible by contract:
    /// every failure path swallows the error and flips the `degraded` flag.
    pub fn record(&self, vault_path: &Path, event: &AuditEvent) {
        if let Err(()) = self.try_record(vault_path, event) {
            self.degraded.store(true, Ordering::SeqCst);
        }
    }

    fn try_record(&self, vault_path: &Path, event: &AuditEvent) -> Result<(), ()> {
        let key = self.key_source.get_or_create().map_err(|_| ())?;
        let plaintext = event.to_bytes();
        let frame = encrypt(&key, &plaintext).map_err(|_| ())?;
        let log_path = self.log_path_for(vault_path);
        let log = AuditLogFile::new(log_path.clone());
        log.append(&frame).map_err(|_| ())?;
        apply_retention(&log_path);
        Ok(())
    }

    /// Returns every recorded event for the Vault at `vault_path` that
    /// matches `_filter`. Frames that fail to decrypt or parse are skipped
    /// so one corrupt record cannot hide the rest — but skipping is no
    /// longer silent: any malformed-base64 lines from storage or any
    /// frames that fail AEAD auth / JSON parse flip the session-wide
    /// `degraded` flag so the UI banner appears.
    ///
    /// Hard failures (key source unavailable, log file unreadable) bubble
    /// up as [`AuditReadError`] and also flip `degraded`. This is the
    /// intentional asymmetry with [`AuditService::record`]: read is called
    /// from a Settings panel where a swallowed error would silently look
    /// identical to "no events yet" and hide a real problem.
    pub fn read(
        &self,
        vault_path: &Path,
        _filter: &AuditFilter,
    ) -> Result<Vec<AuditEvent>, AuditReadError> {
        let key = self.key_source.get_or_create().map_err(|e| {
            self.degraded.store(true, Ordering::SeqCst);
            AuditReadError::Key(e.to_string())
        })?;
        let log = AuditLogFile::new(self.log_path_for(vault_path));
        let outcome = log.read_all().map_err(|e| {
            self.degraded.store(true, Ordering::SeqCst);
            AuditReadError::Storage(e.to_string())
        })?;

        let mut events = Vec::with_capacity(outcome.frames.len());
        let mut undecryptable_frames: usize = 0;
        for frame in &outcome.frames {
            if let Some(event) = decrypt_and_parse(&key, frame) {
                events.push(event);
            } else {
                undecryptable_frames = undecryptable_frames.saturating_add(1);
            }
        }

        if outcome.malformed_lines > 0 || undecryptable_frames > 0 {
            self.degraded.store(true, Ordering::SeqCst);
        }

        Ok(events)
    }

    fn log_path_for(&self, vault_path: &Path) -> PathBuf {
        let name = hash_vault_path(vault_path);
        self.base_dir.join(format!("{name}.jsonl"))
    }

    /// Increments the per-Vault consecutive-failed-unlock counter and appends
    /// one `vault.unlock_failed` event carrying the new count. Called from
    /// the command layer whenever an open/unlock returns `InvalidPassword`.
    pub fn record_vault_unlock_failed(&self, vault_path: &Path) {
        let Ok(mut map) = self.attempts.lock() else {
            self.degraded.store(true, Ordering::SeqCst);
            return;
        };
        let entry = map.entry(attempts_key(vault_path)).or_insert(0);
        *entry = entry.saturating_add(1);
        let count = *entry;
        drop(map);

        let event = AuditEvent::VaultUnlockFailed {
            timestamp: Utc::now(),
            attempt_count: count,
        };
        self.record(vault_path, &event);
    }

    /// Resets the per-Vault failed-unlock counter — called on successful
    /// open/unlock so the next failure starts the count back at 1.
    pub fn reset_unlock_attempts(&self, vault_path: &Path) {
        if let Ok(mut map) = self.attempts.lock() {
            map.remove(&attempts_key(vault_path));
        }
    }
}

fn attempts_key(vault_path: &Path) -> PathBuf {
    vault_path
        .canonicalize()
        .unwrap_or_else(|_| vault_path.to_path_buf())
}

fn decrypt_and_parse(key: &[u8; KEY_LEN], frame: &[u8]) -> Option<AuditEvent> {
    let plaintext = decrypt(key, frame).ok()?;
    AuditEvent::from_bytes(&plaintext).ok()
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::services::audit::key::InMemoryAuditKey;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;
    use tempfile::tempdir;

    fn unlock_failed_event(count: u32) -> AuditEvent {
        AuditEvent::VaultUnlockFailed {
            timestamp: Utc.with_ymd_and_hms(2026, 5, 15, 12, 0, 0).unwrap(),
            attempt_count: count,
        }
    }

    fn fresh_service() -> (AuditService, tempfile::TempDir, PathBuf) {
        let dir = tempdir().expect("tempdir");
        let vault = dir.path().join("vault.kdbx");
        std::fs::write(&vault, b"x").expect("write vault");
        let service =
            AuditService::new(dir.path().join("audit"), Arc::new(InMemoryAuditKey::new()));
        (service, dir, vault)
    }

    #[test]
    fn tracer_bullet_one_record_appears_in_read() {
        let (service, _dir, vault) = fresh_service();

        service.record(&vault, &unlock_failed_event(1));
        let events = service.read(&vault, &AuditFilter::default()).expect("read");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0], unlock_failed_event(1));
        assert!(!service.is_degraded());
    }

    #[test]
    fn multiple_records_returned_in_order() {
        let (service, _dir, vault) = fresh_service();

        for count in 1..=3 {
            service.record(&vault, &unlock_failed_event(count));
        }

        let events = service.read(&vault, &AuditFilter::default()).expect("read");
        assert_eq!(events.len(), 3);
        for (i, evt) in events.iter().enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            let want = unlock_failed_event(u32::try_from(i + 1).unwrap());
            assert_eq!(evt, &want);
        }
    }

    #[test]
    fn read_returns_empty_for_vault_with_no_events() {
        let (service, _dir, vault) = fresh_service();
        assert!(service
            .read(&vault, &AuditFilter::default())
            .expect("read")
            .is_empty());
    }

    /// A frame that AEAD-auth-fails (tampered, or written under a stale
    /// key) must NOT be silently dropped — drop is fine for `DoS` safety,
    /// but the user has to be told via the degraded banner that the log
    /// has gaps.
    #[test]
    fn undecryptable_frame_in_log_flips_degraded_without_hiding_good_frames() {
        let (service, _dir, vault) = fresh_service();

        // Record one valid event.
        service.record(&vault, &unlock_failed_event(1));
        assert!(!service.is_degraded());

        // Append a frame that won't decrypt — base64 of plain ASCII so
        // storage decodes it cleanly but the AEAD check fails.
        let log_path = service.log_path_for(&vault);
        let log = AuditLogFile::new(log_path);
        log.append(b"bogus-non-aead-bytes").expect("append junk");

        let events = service.read(&vault, &AuditFilter::default()).expect("read");
        assert_eq!(events.len(), 1, "the good event must still surface");
        assert!(
            service.is_degraded(),
            "an undecryptable frame must flip degraded"
        );
    }

    /// A non-base64 line in the log file flips degraded for the same
    /// reason: storage continues (`DoS` safety) but the caller is told.
    #[test]
    fn malformed_base64_line_in_log_flips_degraded() {
        use std::io::Write;

        let (service, _dir, vault) = fresh_service();

        service.record(&vault, &unlock_failed_event(1));

        // Splice a raw non-base64 line into the log file.
        let log_path = service.log_path_for(&vault);
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&log_path)
            .expect("open log for splice");
        f.write_all(b"!!!not-base64!!!\n").expect("splice");
        drop(f);

        let events = service.read(&vault, &AuditFilter::default()).expect("read");
        assert_eq!(events.len(), 1);
        assert!(service.is_degraded());
    }

    /// A non-NotFound storage error must NOT be reported back as "no events
    /// yet" — that would mean a real subsystem failure looks identical to
    /// the empty state in the Settings panel. Stage a directory at the
    /// per-Vault log path so `fs::read` fails with `IsADirectory` /
    /// non-NotFound; `read` must return an error AND flip `degraded`.
    #[test]
    fn hard_read_failure_surfaces_as_error_and_flags_degraded() {
        use crate::services::audit::vault_id::hash_vault_path;

        let dir = tempdir().expect("tempdir");
        let base_dir = dir.path().join("audit");
        std::fs::create_dir_all(&base_dir).expect("base dir");
        let vault = dir.path().join("vault.kdbx");
        std::fs::write(&vault, b"x").expect("write vault");

        // Pre-compute the expected log file path and stage a directory
        // there so the storage read trips a non-NotFound error.
        let log_path = base_dir.join(format!("{}.jsonl", hash_vault_path(&vault)));
        std::fs::create_dir(&log_path).expect("stage dir at log path");

        let service = AuditService::new(base_dir, Arc::new(InMemoryAuditKey::new()));

        let result = service.read(&vault, &AuditFilter::default());
        assert!(matches!(result, Err(AuditReadError::Storage(_))));
        assert!(service.is_degraded());
    }

    #[test]
    fn consecutive_failed_unlocks_increment_attempt_count() {
        let (service, _dir, vault) = fresh_service();

        service.record_vault_unlock_failed(&vault);
        service.record_vault_unlock_failed(&vault);
        service.record_vault_unlock_failed(&vault);

        let events = service.read(&vault, &AuditFilter::default()).expect("read");
        let counts: Vec<u32> = events
            .iter()
            .map(|e| match e {
                AuditEvent::VaultUnlockFailed { attempt_count, .. } => *attempt_count,
            })
            .collect();
        assert_eq!(counts, vec![1, 2, 3]);
    }

    #[test]
    fn reset_starts_counter_from_one_again() {
        let (service, _dir, vault) = fresh_service();

        service.record_vault_unlock_failed(&vault);
        service.record_vault_unlock_failed(&vault);
        service.reset_unlock_attempts(&vault);
        service.record_vault_unlock_failed(&vault);

        let events = service.read(&vault, &AuditFilter::default()).expect("read");
        let counts: Vec<u32> = events
            .iter()
            .map(|e| match e {
                AuditEvent::VaultUnlockFailed { attempt_count, .. } => *attempt_count,
            })
            .collect();
        assert_eq!(counts, vec![1, 2, 1]);
    }

    #[test]
    fn counters_are_per_vault() {
        let dir = tempdir().expect("tempdir");
        let vault_a = dir.path().join("a.kdbx");
        let vault_b = dir.path().join("b.kdbx");
        std::fs::write(&vault_a, b"a").expect("write a");
        std::fs::write(&vault_b, b"b").expect("write b");
        let service =
            AuditService::new(dir.path().join("audit"), Arc::new(InMemoryAuditKey::new()));

        service.record_vault_unlock_failed(&vault_a);
        service.record_vault_unlock_failed(&vault_a);
        service.record_vault_unlock_failed(&vault_b);

        let a_events = service
            .read(&vault_a, &AuditFilter::default())
            .expect("read a");
        let b_events = service
            .read(&vault_b, &AuditFilter::default())
            .expect("read b");
        let a_counts: Vec<u32> = a_events
            .iter()
            .map(|e| match e {
                AuditEvent::VaultUnlockFailed { attempt_count, .. } => *attempt_count,
            })
            .collect();
        let b_counts: Vec<u32> = b_events
            .iter()
            .map(|e| match e {
                AuditEvent::VaultUnlockFailed { attempt_count, .. } => *attempt_count,
            })
            .collect();
        assert_eq!(a_counts, vec![1, 2]);
        assert_eq!(b_counts, vec![1]);
    }

    #[test]
    fn record_against_unwritable_dir_does_not_panic_and_flags_degraded() {
        // Point the base_dir at a path that cannot be created (a regular
        // file masquerading as a directory). `record` must swallow the
        // error and flip `degraded`.
        let dir = tempdir().expect("tempdir");
        let blocker = dir.path().join("blocker");
        std::fs::write(&blocker, b"x").expect("write blocker file");

        let base_dir = blocker.join("audit"); // can't mkdir under a regular file
        let service = AuditService::new(base_dir, Arc::new(InMemoryAuditKey::new()));

        let vault = dir.path().join("vault.kdbx");
        std::fs::write(&vault, b"x").expect("write vault");

        service.record(&vault, &unlock_failed_event(1));
        assert!(service.is_degraded());
    }
}
