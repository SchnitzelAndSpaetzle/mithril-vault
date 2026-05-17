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

use crate::services::audit::crypto::{decrypt, encode_frame, encrypt, KEY_LEN};
use crate::services::audit::format::{AuditEvent, Reason};
use crate::services::audit::key::AuditKey;
use crate::services::audit::retention::{partition_by_retention, SizedEvent};
use crate::services::audit::storage::{AuditLogFile, CompactStats};
use crate::services::audit::vault_id::hash_vault_path;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use thiserror::Error;

/// Hard ceiling on the on-disk audit log size, in bytes. Defense-in-depth
/// against the age policy running away — even at the maximum allowed
/// `retentionDays`, a single Vault's log cannot grow past this.
pub(crate) const MAX_LOG_BYTES: u64 = 10 * 1024 * 1024;

/// Default `retentionDays` used when nothing has pushed a value in yet —
/// matches the public default in [`crate::commands::settings::AuditSettings`].
/// Duplicated here (rather than imported) so the audit service stays free
/// of a dependency on the commands layer.
const DEFAULT_RETENTION_DAYS: u32 = 90;

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

/// Errors surfaced from [`AuditService::compact`]. Compaction is a
/// best-effort retention operation that the audit subsystem runs lazily
/// from `record`; it is also directly callable for tests. Failures are
/// distinct from the infallible `record` path because a test or settings-
/// triggered compaction wants the failure to be observable.
#[derive(Debug, Error)]
pub enum AuditCompactError {
    #[error("audit key unavailable: {0}")]
    Key(String),
    #[error("audit compact failed: {0}")]
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
    /// Configured age cap pushed in from `AuditSettings.retentionDays`.
    /// Validated to `1..=365` at the settings boundary, so any value
    /// stored here is already in range; we still saturate at `u32` here
    /// for cheap atomic reads from the `record` hot path.
    retention_days: AtomicU32,
    /// On-disk size ceiling beyond which the lazy trigger fires
    /// compaction. Defaults to [`MAX_LOG_BYTES`]; overridable via
    /// [`AuditService::set_size_cap_for_test`] so tests can exercise
    /// the trigger without writing 10 MiB of fake data.
    size_cap: AtomicU64,
    /// Whether the lazy compaction trigger in `record` fires at all.
    /// `true` in production; tests that want to inspect the explicit
    /// `compact` pass in isolation flip this to `false` so setup
    /// `record` calls don't pre-compact behind the assertion.
    lazy_trigger_enabled: AtomicBool,
    /// Per-Vault consecutive-failed-unlock counters, keyed by canonicalized
    /// path. Lives in memory only — resets on process restart per the AC's
    /// "per session" wording.
    attempts: Mutex<HashMap<PathBuf, u32>>,
    /// Per-Vault cached oldest event timestamp, populated lazily on the
    /// first `record` for a Vault after process start (and after each
    /// successful compaction). The lazy-trigger check reads from this
    /// cache so `record` doesn't have to re-decode the whole log on
    /// every append — only the first append for a Vault, and again
    /// after a compaction shrinks the log.
    oldest: Mutex<HashMap<PathBuf, DateTime<Utc>>>,
}

impl AuditService {
    pub fn new(base_dir: PathBuf, key_source: Arc<dyn AuditKey>) -> Self {
        Self {
            base_dir,
            key_source,
            degraded: AtomicBool::new(false),
            enabled: AtomicBool::new(true),
            retention_days: AtomicU32::new(DEFAULT_RETENTION_DAYS),
            size_cap: AtomicU64::new(MAX_LOG_BYTES),
            lazy_trigger_enabled: AtomicBool::new(true),
            attempts: Mutex::new(HashMap::new()),
            oldest: Mutex::new(HashMap::new()),
        }
    }

    /// Test-only hook: override the on-disk size cap so the lazy trigger
    /// can be exercised with a tiny synthetic ceiling instead of the
    /// production 10 MiB. Production paths leave this at
    /// [`MAX_LOG_BYTES`].
    pub fn set_size_cap_for_test(&self, n: u64) {
        self.size_cap.store(n, Ordering::SeqCst);
    }

    /// Test-only hook: suspend the lazy compaction trigger in `record`
    /// so setup-phase appends don't silently pre-compact the log before
    /// an assertion runs. Production paths leave the trigger enabled.
    pub fn set_lazy_trigger_enabled_for_test(&self, enabled: bool) {
        self.lazy_trigger_enabled.store(enabled, Ordering::SeqCst);
    }

    fn size_cap(&self) -> u64 {
        self.size_cap.load(Ordering::SeqCst)
    }

    /// Pushes the configured `retentionDays` in from settings. Called from
    /// `update_app_preferences` / `reset_app_preferences` / startup, the
    /// same plumbing that drives [`AuditService::set_enabled`]. Stored as
    /// an atomic so `record` can read it on the hot path without a lock.
    pub fn set_retention_days(&self, days: u32) {
        self.retention_days.store(days, Ordering::SeqCst);
    }

    fn retention_days(&self) -> u32 {
        self.retention_days.load(Ordering::SeqCst)
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

        // Update the cached oldest-timestamp for this vault: the
        // just-appended event is the smallest known timestamp if the
        // cache was empty, or stays unchanged if an older event was
        // already recorded earlier in this session. Failures swallowed —
        // a poisoned mutex must not cause an audit record to silently
        // re-error after the storage append already succeeded.
        let event_ts = match event {
            AuditEvent::VaultUnlockFailed { timestamp, .. }
            | AuditEvent::VaultOpened { timestamp }
            | AuditEvent::VaultLocked { timestamp, .. }
            | AuditEvent::EntryPasswordRevealed { timestamp, .. }
            | AuditEvent::EntryPasswordCopied { timestamp, .. }
            | AuditEvent::EntryProtectedFieldRevealed { timestamp, .. }
            | AuditEvent::AuditCleared { timestamp } => *timestamp,
        };
        if let Ok(mut map) = self.oldest.lock() {
            let key = attempts_key(vault_path);
            map.entry(key)
                .and_modify(|t| {
                    if event_ts < *t {
                        *t = event_ts;
                    }
                })
                .or_insert(event_ts);
        }

        // Lazy compaction trigger: synchronous because `record` itself is
        // already best-effort and called *after* the triggering user
        // action has returned, so latency here cannot regress the
        // user-facing flow. Errors swallowed: compaction is a
        // housekeeping nicety, not a correctness requirement.
        self.maybe_trigger_compaction(vault_path);
        Ok(())
    }

    fn maybe_trigger_compaction(&self, vault_path: &Path) {
        if !self.lazy_trigger_enabled.load(Ordering::SeqCst) {
            return;
        }
        let path = self.log_path_for(vault_path);
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let now = Utc::now();
        let cutoff = now - Duration::days(i64::from(self.retention_days()));
        let oldest = self
            .oldest
            .lock()
            .ok()
            .and_then(|m| m.get(&attempts_key(vault_path)).copied());
        let size_trigger = size > self.size_cap();
        let age_trigger = oldest.is_some_and(|ts| ts < cutoff);
        if size_trigger || age_trigger {
            let _ = self.compact(vault_path);
        }
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

    /// Forces a retention compaction pass on the per-Vault log: reads
    /// every encrypted frame under the exclusive storage lock, decrypts
    /// to [`AuditEvent`]s, partitions by the configured `retentionDays`
    /// and the [`MAX_LOG_BYTES`] hard cap via
    /// [`partition_by_retention`], re-encrypts the keepers with fresh
    /// XChaCha20-Poly1305 nonces, and atomically replaces the file.
    ///
    /// Public so the lazy trigger in `record` and tests can invoke it
    /// directly. Returns the kept / dropped counts so callers can verify
    /// idempotency (a second run on an already-in-window log drops 0).
    ///
    /// Skipped (returns `CompactStats::default()`) when the master gate
    /// is off — disabled audit logging shouldn't trigger background
    /// rewrites against an existing log. Frames that fail to decrypt
    /// (auth failure, wrong key) flip `degraded` and are *dropped* from
    /// the rewrite: keeping unreadable frames around forever would let
    /// a corrupt log bloat the file with bytes the user can never see.
    pub fn compact(&self, vault_path: &Path) -> Result<CompactStats, AuditCompactError> {
        if !self.is_enabled() {
            return Ok(CompactStats::default());
        }
        let key = self
            .key_source
            .get_or_create()
            .map_err(|e| AuditCompactError::Key(e.to_string()))?;

        let now = Utc::now();
        let max_age = Duration::days(i64::from(self.retention_days()));
        let max_bytes = usize::try_from(self.size_cap()).unwrap_or(usize::MAX);

        let log = AuditLogFile::new(self.log_path_for(vault_path));
        let mut undecryptable: usize = 0;
        let degraded_flag = &self.degraded;

        let stats = log
            .compact(|frames| {
                let mut sized: Vec<SizedEvent> = Vec::with_capacity(frames.len());
                for frame in frames {
                    match decrypt_and_parse(&key, &frame) {
                        Some(event) => {
                            // Approximate encoded size: base64 of the
                            // frame, plus newline. This is the same
                            // shape the writer will produce, so the
                            // size-cap accounting is exact.
                            let encoded_len = encode_frame(&frame).len() + 1;
                            sized.push(SizedEvent { event, encoded_len });
                        }
                        None => {
                            undecryptable = undecryptable.saturating_add(1);
                        }
                    }
                }

                let (keep, _drop) = partition_by_retention(sized, now, max_age, max_bytes);

                // Re-encrypt keepers with fresh nonces. If any individual
                // re-encryption fails (vanishingly unlikely with a valid
                // key) we drop that frame rather than abort — the
                // retention pass should always converge, and dropping a
                // single frame is preferable to refusing to compact a
                // bloated log at all.
                keep.into_iter()
                    .filter_map(|s| encrypt(&key, &s.event.to_bytes()).ok())
                    .collect()
            })
            .map_err(|e| {
                degraded_flag.store(true, Ordering::SeqCst);
                AuditCompactError::Storage(e.to_string())
            })?;

        if undecryptable > 0 {
            self.degraded.store(true, Ordering::SeqCst);
        }

        // Refresh the cached oldest timestamp so the next lazy-trigger
        // check uses post-compaction state. Re-read the file rather than
        // tracking it through the closure: the file is now smaller and
        // the re-read is cheap.
        self.refresh_oldest(vault_path, &key);

        Ok(stats)
    }

    /// Recomputes and caches the oldest timestamp on disk for
    /// `vault_path`. Best-effort: failures (key unavailable, file
    /// unreadable, no decryptable frames) clear the cache entry rather
    /// than poisoning it with stale data.
    fn refresh_oldest(&self, vault_path: &Path, key: &[u8; KEY_LEN]) {
        let log = AuditLogFile::new(self.log_path_for(vault_path));
        let Ok(outcome) = log.read_all() else {
            if let Ok(mut map) = self.oldest.lock() {
                map.remove(&attempts_key(vault_path));
            }
            return;
        };
        let oldest_ts = outcome
            .frames
            .iter()
            .filter_map(|f| decrypt_and_parse(key, f))
            .map(|e| event_timestamp(&e))
            .min();
        if let Ok(mut map) = self.oldest.lock() {
            match oldest_ts {
                Some(ts) => {
                    map.insert(attempts_key(vault_path), ts);
                }
                None => {
                    map.remove(&attempts_key(vault_path));
                }
            }
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

fn event_timestamp(event: &AuditEvent) -> DateTime<Utc> {
    match event {
        AuditEvent::VaultUnlockFailed { timestamp, .. }
        | AuditEvent::VaultOpened { timestamp }
        | AuditEvent::VaultLocked { timestamp, .. }
        | AuditEvent::EntryPasswordRevealed { timestamp, .. }
        | AuditEvent::EntryPasswordCopied { timestamp, .. }
        | AuditEvent::EntryProtectedFieldRevealed { timestamp, .. }
        | AuditEvent::AuditCleared { timestamp } => *timestamp,
    }
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

    /// Acceptance-criteria integration test for issue #222: appending
    /// many events spanning *more than* the configured `retentionDays`
    /// and then forcing a compaction must leave only the in-window
    /// events on disk, in original append order, decryptable end-to-end.
    /// This is the end-to-end proof that the pure partition function,
    /// the storage rewrite, and the service-level glue actually compose.
    #[test]
    fn compact_drops_events_older_than_retention_and_keeps_the_rest_in_order() {
        use chrono::Duration;
        let (service, _dir, vault) = fresh_service();
        service.set_retention_days(30);
        // Suspend the lazy trigger so setup `record` calls don't
        // silently pre-compact behind the explicit compact assertion.
        service.set_lazy_trigger_enabled_for_test(false);

        let now = Utc::now();
        // 100 events: half outside the 30-day window, half inside.
        // Append timestamps in chronological order so the in-window
        // remainder must come back from `read` in the same order.
        for i in 0..50 {
            let evt = AuditEvent::VaultUnlockFailed {
                timestamp: now - Duration::days(60) + Duration::seconds(i),
                attempt_count: u32::try_from(i + 1).unwrap(),
            };
            service.record(&vault, &evt);
        }
        for i in 0..50 {
            let evt = AuditEvent::VaultUnlockFailed {
                timestamp: now - Duration::days(5) + Duration::seconds(i),
                attempt_count: u32::try_from(i + 1).unwrap(),
            };
            service.record(&vault, &evt);
        }

        let stats = service.compact(&vault).expect("compact");
        assert_eq!(stats.kept, 50);
        assert_eq!(stats.dropped, 50);

        let events = service.read(&vault, &AuditFilter::default()).expect("read");
        assert_eq!(events.len(), 50);
        // Every surviving event must be inside the 30-day window and in
        // append order — the partition pass preserves chronological
        // order so the audit panel reads as a continuous stream.
        let cutoff = now - Duration::days(30);
        for (i, evt) in events.iter().enumerate() {
            match evt {
                AuditEvent::VaultUnlockFailed {
                    timestamp,
                    attempt_count,
                } => {
                    assert!(*timestamp >= cutoff, "kept event {i} is past the cutoff");
                    assert_eq!(*attempt_count, u32::try_from(i + 1).unwrap());
                }
                other => panic!("unexpected variant in kept log: {other:?}"),
            }
        }
    }

    /// Idempotency: a second compact run touches nothing because the
    /// first pass already left only in-window events behind.
    #[test]
    fn compact_is_idempotent_against_an_already_compact_log() {
        use chrono::Duration;
        let (service, _dir, vault) = fresh_service();
        service.set_retention_days(30);
        service.set_lazy_trigger_enabled_for_test(false);

        let now = Utc::now();
        for i in 0..5 {
            let evt = AuditEvent::VaultUnlockFailed {
                timestamp: now - Duration::days(60) + Duration::seconds(i),
                attempt_count: u32::try_from(i + 1).unwrap(),
            };
            service.record(&vault, &evt);
        }
        for i in 0..5 {
            let evt = AuditEvent::VaultUnlockFailed {
                timestamp: now - Duration::hours(1) + Duration::seconds(i),
                attempt_count: u32::try_from(i + 1).unwrap(),
            };
            service.record(&vault, &evt);
        }

        let first = service.compact(&vault).expect("first compact");
        assert_eq!(first.dropped, 5);

        let second = service.compact(&vault).expect("second compact");
        assert_eq!(second.dropped, 0, "second compact must drop nothing");
        assert_eq!(second.kept, 5);
    }

    /// Lazy trigger #1: when the oldest event already on disk is older
    /// than `retentionDays`, the very next `record` call automatically
    /// runs a compaction pass — the user never has to manually trim and
    /// the audit panel never accumulates ancient events. Verified by
    /// configuring a 1-day retention, backdating one event, then
    /// recording a fresh one and reading back the post-trigger state.
    #[test]
    fn record_triggers_compaction_when_cached_oldest_is_past_retention() {
        use chrono::Duration;
        let (service, _dir, vault) = fresh_service();
        service.set_retention_days(1);

        let now = Utc::now();
        let stale = AuditEvent::VaultUnlockFailed {
            timestamp: now - Duration::days(3),
            attempt_count: 1,
        };
        let fresh = AuditEvent::VaultUnlockFailed {
            timestamp: now,
            attempt_count: 2,
        };

        service.record(&vault, &stale);
        // After the stale append the cached-oldest is 3 days ago. The
        // next record's lazy-trigger check should fire and rewrite the
        // log without the stale entry.
        service.record(&vault, &fresh);

        let events = service.read(&vault, &AuditFilter::default()).expect("read");
        assert_eq!(events.len(), 1, "compaction must drop the stale entry");
        match &events[0] {
            AuditEvent::VaultUnlockFailed { attempt_count, .. } => {
                assert_eq!(*attempt_count, 2);
            }
            other => panic!("expected VaultUnlockFailed, got {other:?}"),
        }
    }

    /// Lazy trigger #2: when the on-disk file crosses the size cap, the
    /// next `record` automatically compacts. Tested with a synthetic
    /// small cap so we don't have to actually write 10 MiB.
    #[test]
    fn record_triggers_compaction_when_file_exceeds_size_cap() {
        let (service, _dir, vault) = fresh_service();
        // Cap small enough that two appends will push it past — each
        // base64 frame is roughly 90+ bytes. The third record's trigger
        // check must rewrite the file.
        service.set_size_cap_for_test(100);

        let now = Utc::now();
        for i in 1..=3u32 {
            service.record(
                &vault,
                &AuditEvent::VaultUnlockFailed {
                    timestamp: now,
                    attempt_count: i,
                },
            );
        }

        // After the size-cap trigger, the kept set must satisfy the cap
        // OR be the single surviving event (per the floor rule).
        let log_path = service.log_path_for(&vault);
        let size = std::fs::metadata(&log_path).map(|m| m.len()).unwrap_or(0);
        let events = service.read(&vault, &AuditFilter::default()).expect("read");
        assert!(
            size <= 100 || events.len() == 1,
            "post-trigger size {size} must respect cap or be the single-event floor (events={})",
            events.len()
        );
        // And the LATEST event must be among the survivors — the size
        // cap drops oldest-first, never the just-appended event.
        match events.last().expect("at least one event") {
            AuditEvent::VaultUnlockFailed { attempt_count, .. } => {
                assert_eq!(*attempt_count, 3);
            }
            other => panic!("expected VaultUnlockFailed, got {other:?}"),
        }
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
