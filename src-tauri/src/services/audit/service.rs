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
use crate::services::audit::format::{AuditEvent, Reason};
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

/// Errors surfaced from [`AuditService::clear`]. Like [`AuditReadError`]
/// and unlike the infallible [`AuditService::record`], clear is a user-
/// initiated action and a failure must be visible — the UI toasts it so
/// the user knows the wipe did not actually happen.
#[derive(Debug, Error)]
pub enum AuditClearError {
    #[error("audit key unavailable: {0}")]
    Key(String),
    #[error("audit clear failed: {0}")]
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
    /// Master gate. When false, [`AuditService::record`] short-circuits
    /// before any key fetch or file I/O; the existing log file is left
    /// untouched so the user can re-enable later without losing history.
    /// Pushed in from `update_app_preferences` and at startup; defaults
    /// to true so a fresh `AuditService` records out of the box.
    enabled: AtomicBool,
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
            enabled: AtomicBool::new(true),
            attempts: Mutex::new(HashMap::new()),
        }
    }

    /// Flips the master logging gate. Off => [`AuditService::record`]
    /// short-circuits without touching the key source or storage. On =>
    /// subsequent records append to the existing per-Vault file. Never
    /// modifies the on-disk log — disabling preserves history.
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
    }

    /// Reports the current state of the master logging gate. Used by the
    /// Settings panel to render the toggle and by tests to assert the
    /// short-circuit behavior.
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// Reports whether a previous `record` call failed internally. Surfaced
    /// in Settings → Audit Log as a banner; the failed user action itself is
    /// never affected.
    pub fn is_degraded(&self) -> bool {
        self.degraded.load(Ordering::SeqCst)
    }

    /// Appends `event` to the per-Vault audit log. Infallible by contract:
    /// every failure path swallows the error and flips the `degraded` flag.
    /// When the master gate is off this is a complete no-op — no key fetch,
    /// no file I/O — so a disabled audit log adds zero overhead to the
    /// vault flows it instruments.
    pub fn record(&self, vault_path: &Path, event: &AuditEvent) {
        if !self.is_enabled() {
            return;
        }
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

    /// Records a `vault.locked` event with the given reason. Called from
    /// the command layer / auto-lock service / app-quit handler whenever a
    /// Vault transitions from unlocked to locked.
    pub fn record_vault_locked(&self, vault_path: &Path, reason: Reason) {
        let event = AuditEvent::VaultLocked {
            timestamp: Utc::now(),
            reason,
        };
        self.record(vault_path, &event);
    }

    /// Convenience wrapper that records one `vault.locked` event per path
    /// in `vault_paths` with the same reason. Used by the auto-lock task
    /// and the app-quit handler, which both batch-lock multiple Vaults
    /// from a single trigger.
    pub fn record_vault_locked_batch<P: AsRef<Path>>(&self, vault_paths: &[P], reason: Reason) {
        for p in vault_paths {
            self.record_vault_locked(p.as_ref(), reason);
        }
    }

    /// Records a `vault.opened` event and resets the per-Vault
    /// failed-unlock counter so the next failure starts the count back at 1.
    /// Called from the command layer on every locked→unlocked transition.
    pub fn record_vault_opened(&self, vault_path: &Path) {
        self.reset_unlock_attempts(vault_path);
        let event = AuditEvent::VaultOpened {
            timestamp: Utc::now(),
        };
        self.record(vault_path, &event);
    }

    /// Appends one `entry.password_revealed` event for the given KDBX
    /// entry UUID against the open Vault's log. Called from the command
    /// layer immediately after a successful `get_entry_password`. Like
    /// every other `record_*` helper, this is infallible by contract.
    pub fn record_entry_password_revealed(&self, vault_path: &Path, entry_id: &str) {
        let event = AuditEvent::EntryPasswordRevealed {
            timestamp: Utc::now(),
            entry_id: entry_id.to_string(),
        };
        self.record(vault_path, &event);
    }

    /// Appends one `entry.password_copied` event after a successful
    /// clipboard write of an entry password. Infallible by contract.
    pub fn record_entry_password_copied(&self, vault_path: &Path, entry_id: &str) {
        let event = AuditEvent::EntryPasswordCopied {
            timestamp: Utc::now(),
            entry_id: entry_id.to_string(),
        };
        self.record(vault_path, &event);
    }

    /// Appends one `entry.protected_field_revealed` event after a
    /// successful `get_entry_protected_custom_field`. Infallible by
    /// contract — protected custom fields (e.g. recovery codes) get the
    /// same audit treatment as password reveals per AC #7 of the PRD.
    pub fn record_entry_protected_field_revealed(&self, vault_path: &Path, entry_id: &str) {
        let event = AuditEvent::EntryProtectedFieldRevealed {
            timestamp: Utc::now(),
            entry_id: entry_id.to_string(),
        };
        self.record(vault_path, &event);
    }

    /// Appends one `preferences.security_changed` event naming the App
    /// Preference leaf that flipped. Called by the settings command once
    /// per changed allowlisted leaf, fanned out across every currently-
    /// open Vault — the audit log is per-Vault, so a global preference
    /// change has to land in each open Vault's log to be visible from
    /// the Audit Log panel. Infallible by contract.
    pub fn record_preferences_security_changed(&self, vault_path: &Path, setting_name: &str) {
        let event = AuditEvent::PreferencesSecurityChanged {
            timestamp: Utc::now(),
            setting_name: setting_name.to_string(),
        };
        self.record(vault_path, &event);
    }

    /// Wipes the per-Vault audit log and leaves behind exactly one
    /// `audit.cleared` event so the wipe is never silent. The storage
    /// layer performs the rewrite atomically (temp file + rename under
    /// an exclusive advisory lock) so a failure mid-write leaves the
    /// original file untouched rather than producing a partial log.
    ///
    /// Unlike [`AuditService::record`], `clear` is user-initiated and
    /// surfaces hard errors so the caller can render a failure state
    /// rather than silently flipping the `degraded` flag.
    pub fn clear(&self, vault_path: &Path) -> Result<(), AuditClearError> {
        let key = self
            .key_source
            .get_or_create()
            .map_err(|e| AuditClearError::Key(e.to_string()))?;
        let event = AuditEvent::AuditCleared {
            timestamp: Utc::now(),
        };
        let plaintext = event.to_bytes();
        let frame =
            encrypt(&key, &plaintext).map_err(|e| AuditClearError::Storage(e.to_string()))?;
        let log = AuditLogFile::new(self.log_path_for(vault_path));
        log.replace_with_single(&frame)
            .map_err(|e| AuditClearError::Storage(e.to_string()))?;
        Ok(())
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
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::services::audit::key::InMemoryAuditKey;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;
    use tempfile::tempdir;

    /// Extracts `attempt_count` from a `VaultUnlockFailed` event; panics on
    /// any other variant so a leaking variant in a test is loud, not silent.
    fn unlock_failed_count(event: &AuditEvent) -> u32 {
        match event {
            AuditEvent::VaultUnlockFailed { attempt_count, .. } => *attempt_count,
            other => panic!("expected VaultUnlockFailed, got {other:?}"),
        }
    }

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
        let counts: Vec<u32> = events.iter().map(unlock_failed_count).collect();
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
        let counts: Vec<u32> = events.iter().map(unlock_failed_count).collect();
        assert_eq!(counts, vec![1, 2, 1]);
    }

    #[test]
    fn record_entry_protected_field_revealed_appends_exactly_one_event() {
        let (service, _dir, vault) = fresh_service();

        service.record_entry_protected_field_revealed(&vault, "pf-uuid");

        let events = service.read(&vault, &AuditFilter::default()).expect("read");
        assert_eq!(events.len(), 1);
        match &events[0] {
            AuditEvent::EntryProtectedFieldRevealed { entry_id, .. } => {
                assert_eq!(entry_id, "pf-uuid");
            }
            other => panic!("expected EntryProtectedFieldRevealed, got {other:?}"),
        }
        assert!(!service.is_degraded());
    }

    #[test]
    fn record_entry_password_copied_appends_exactly_one_event() {
        let (service, _dir, vault) = fresh_service();

        service.record_entry_password_copied(&vault, "copied-uuid");

        let events = service.read(&vault, &AuditFilter::default()).expect("read");
        assert_eq!(events.len(), 1);
        match &events[0] {
            AuditEvent::EntryPasswordCopied { entry_id, .. } => {
                assert_eq!(entry_id, "copied-uuid");
            }
            other => panic!("expected EntryPasswordCopied, got {other:?}"),
        }
        assert!(!service.is_degraded());
    }

    #[test]
    fn record_entry_password_revealed_appends_exactly_one_event() {
        let (service, _dir, vault) = fresh_service();

        service.record_entry_password_revealed(&vault, "uuid-1");

        let events = service.read(&vault, &AuditFilter::default()).expect("read");
        assert_eq!(events.len(), 1);
        match &events[0] {
            AuditEvent::EntryPasswordRevealed { entry_id, .. } => {
                assert_eq!(entry_id, "uuid-1");
            }
            other => panic!("expected EntryPasswordRevealed, got {other:?}"),
        }
        assert!(!service.is_degraded());
    }

    #[test]
    fn record_preferences_security_changed_appends_exactly_one_event() {
        let (service, _dir, vault) = fresh_service();

        service.record_preferences_security_changed(&vault, "security.preventScreenCapture");

        let events = service.read(&vault, &AuditFilter::default()).expect("read");
        assert_eq!(events.len(), 1);
        match &events[0] {
            AuditEvent::PreferencesSecurityChanged { setting_name, .. } => {
                assert_eq!(setting_name, "security.preventScreenCapture");
            }
            other => panic!("expected PreferencesSecurityChanged, got {other:?}"),
        }
        assert!(!service.is_degraded());
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
        let a_counts: Vec<u32> = a_events.iter().map(unlock_failed_count).collect();
        let b_counts: Vec<u32> = b_events.iter().map(unlock_failed_count).collect();
        assert_eq!(a_counts, vec![1, 2]);
        assert_eq!(b_counts, vec![1]);
    }

    /// Acceptance-criteria integration test for issue #217: a single
    /// unlock+lock cycle must leave one `vault.opened` and one
    /// `vault.locked` record in chronological order, with the lock reason
    /// preserved.
    #[test]
    fn unlock_then_lock_cycle_produces_opened_then_locked_records() {
        let (service, _dir, vault) = fresh_service();

        service.record_vault_opened(&vault);
        service.record_vault_locked(&vault, Reason::Manual);

        let events = service.read(&vault, &AuditFilter::default()).expect("read");
        assert_eq!(events.len(), 2, "one opened + one locked");

        assert!(
            matches!(events[0], AuditEvent::VaultOpened { .. }),
            "first record must be vault.opened, got {:?}",
            events[0]
        );
        match &events[1] {
            AuditEvent::VaultLocked { reason, .. } => assert_eq!(*reason, Reason::Manual),
            other => panic!("expected VaultLocked, got {other:?}"),
        }
        assert!(!service.is_degraded());
    }

    #[test]
    fn record_vault_locked_batch_writes_one_event_per_path() {
        let dir = tempdir().expect("tempdir");
        let vault_a = dir.path().join("a.kdbx");
        let vault_b = dir.path().join("b.kdbx");
        std::fs::write(&vault_a, b"a").expect("write a");
        std::fs::write(&vault_b, b"b").expect("write b");
        let service =
            AuditService::new(dir.path().join("audit"), Arc::new(InMemoryAuditKey::new()));

        service.record_vault_locked_batch(&[&vault_a, &vault_b], Reason::AppQuit);

        for vault in [&vault_a, &vault_b] {
            let events = service.read(vault, &AuditFilter::default()).expect("read");
            assert_eq!(events.len(), 1);
            match &events[0] {
                AuditEvent::VaultLocked { reason, .. } => assert_eq!(*reason, Reason::AppQuit),
                other => panic!("expected VaultLocked, got {other:?}"),
            }
        }
    }

    #[test]
    fn record_vault_locked_batch_no_op_on_empty_slice() {
        let dir = tempdir().expect("tempdir");
        let service =
            AuditService::new(dir.path().join("audit"), Arc::new(InMemoryAuditKey::new()));
        let empty: &[&Path] = &[];
        service.record_vault_locked_batch(empty, Reason::AppQuit);
        assert!(!service.is_degraded());
    }

    #[test]
    fn record_vault_locked_appends_event_with_each_reason() {
        use crate::services::audit::format::Reason;
        let (service, _dir, vault) = fresh_service();

        for reason in [
            Reason::Manual,
            Reason::AutoLock,
            Reason::AppQuit,
            Reason::ScreenLock,
        ] {
            service.record_vault_locked(&vault, reason);
        }

        let events = service.read(&vault, &AuditFilter::default()).expect("read");
        let reasons: Vec<Reason> = events
            .iter()
            .map(|e| match e {
                AuditEvent::VaultLocked { reason, .. } => *reason,
                other => panic!("expected VaultLocked, got {other:?}"),
            })
            .collect();
        assert_eq!(
            reasons,
            vec![
                Reason::Manual,
                Reason::AutoLock,
                Reason::AppQuit,
                Reason::ScreenLock
            ]
        );
    }

    #[test]
    fn record_vault_opened_appends_event_and_resets_attempt_counter() {
        let (service, _dir, vault) = fresh_service();

        // Build up some failed attempts first, then mark the open.
        service.record_vault_unlock_failed(&vault);
        service.record_vault_unlock_failed(&vault);
        service.record_vault_opened(&vault);

        // A subsequent failed unlock must start counting again from 1 — i.e.
        // record_vault_opened resets the in-memory attempts counter just
        // like the existing reset_unlock_attempts did.
        service.record_vault_unlock_failed(&vault);

        let events = service.read(&vault, &AuditFilter::default()).expect("read");
        // Expect: failed(1), failed(2), opened, failed(1)
        assert_eq!(events.len(), 4);
        assert!(matches!(events[2], AuditEvent::VaultOpened { .. }));
        match &events[3] {
            AuditEvent::VaultUnlockFailed { attempt_count, .. } => {
                assert_eq!(*attempt_count, 1, "counter must reset on vault.opened");
            }
            other => panic!("expected VaultUnlockFailed, got {other:?}"),
        }
    }

    #[test]
    fn record_short_circuits_when_disabled_no_file_no_events() {
        // AC: `AuditService::record` consults the current `enabled` flag
        // and short-circuits when disabled; no file write, no key fetch.
        let (service, _dir, vault) = fresh_service();

        service.set_enabled(false);
        service.record(&vault, &unlock_failed_event(1));

        // No log file should have been created on disk.
        let log_path = service.log_path_for(&vault);
        assert!(
            !log_path.exists(),
            "no audit file should exist when logging is disabled, found: {log_path:?}"
        );
        // And read() must report empty (file genuinely missing, not just
        // hidden by the gate — read() does not consult the enabled flag).
        let events = service.read(&vault, &AuditFilter::default()).expect("read");
        assert!(events.is_empty());
        assert!(!service.is_degraded());
    }

    #[test]
    fn re_enabling_resumes_appending_to_same_file() {
        // AC: Re-enabling logging resumes appending to the same file.
        let (service, _dir, vault) = fresh_service();

        // Record one event while enabled (default).
        service.record(&vault, &unlock_failed_event(1));

        // Disable, attempt to record — must not write.
        service.set_enabled(false);
        service.record(&vault, &unlock_failed_event(2));

        // Re-enable and record again — must append to the existing file.
        service.set_enabled(true);
        service.record(&vault, &unlock_failed_event(3));

        let events = service.read(&vault, &AuditFilter::default()).expect("read");
        let counts: Vec<u32> = events.iter().map(unlock_failed_count).collect();
        assert_eq!(counts, vec![1, 3]);
    }

    #[test]
    fn disable_preserves_existing_log_file_unchanged() {
        // AC: Disabling logging preserves the existing log file unchanged.
        // Capture the file bytes before disabling and again after; they
        // must be byte-identical (set_enabled must not touch storage).
        let (service, _dir, vault) = fresh_service();
        service.record(&vault, &unlock_failed_event(1));
        let log_path = service.log_path_for(&vault);
        let before = std::fs::read(&log_path).expect("read log before disable");

        service.set_enabled(false);
        let after = std::fs::read(&log_path).expect("read log after disable");
        assert_eq!(
            before, after,
            "set_enabled(false) must not modify the log file"
        );

        // The previously-recorded event must still come back through read().
        let events = service.read(&vault, &AuditFilter::default()).expect("read");
        assert_eq!(events.len(), 1);
    }

    /// Acceptance-criteria integration test for issue #220: populate a
    /// log with several events, `clear` it, then assert `read` returns
    /// exactly one `audit.cleared` event with no surviving history.
    #[test]
    fn clear_replaces_history_with_single_audit_cleared_event() {
        let (service, _dir, vault) = fresh_service();

        // Populate with a mix of kinds so we know clear doesn't just
        // affect one variant.
        service.record_vault_unlock_failed(&vault);
        service.record_vault_opened(&vault);
        service.record_vault_locked(&vault, Reason::Manual);
        service.record_entry_password_revealed(&vault, "uuid-x");

        service.clear(&vault).expect("clear");

        let events = service.read(&vault, &AuditFilter::default()).expect("read");
        assert_eq!(events.len(), 1, "exactly one surviving event after clear");
        assert!(
            matches!(events[0], AuditEvent::AuditCleared { .. }),
            "surviving event must be AuditCleared, got {:?}",
            events[0]
        );
    }

    /// Clearing an audit log that has never been written to still emits
    /// the surviving event — otherwise a "clear" on a fresh Vault would
    /// silently leave an empty file and the audit panel would look
    /// identical to "never logged anything", hiding the user action.
    #[test]
    fn clear_on_empty_vault_writes_audit_cleared_event() {
        let (service, _dir, vault) = fresh_service();

        service.clear(&vault).expect("clear");

        let events = service.read(&vault, &AuditFilter::default()).expect("read");
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], AuditEvent::AuditCleared { .. }));
    }

    /// `clear` is user-initiated from the Settings panel — its failure
    /// mode is a hard error, not the infallible swallow that `record`
    /// uses. A bad `base_dir` must surface as `Err(_)` so the UI can toast
    /// the failure instead of silently leaving the original log in place.
    #[test]
    fn clear_against_unwritable_dir_returns_error() {
        let dir = tempdir().expect("tempdir");
        let blocker = dir.path().join("blocker");
        std::fs::write(&blocker, b"x").expect("write blocker file");

        let base_dir = blocker.join("audit");
        let service = AuditService::new(base_dir, Arc::new(InMemoryAuditKey::new()));
        let vault = dir.path().join("vault.kdbx");
        std::fs::write(&vault, b"x").expect("write vault");

        let result = service.clear(&vault);
        assert!(result.is_err(), "clear under unwritable dir must error");
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
