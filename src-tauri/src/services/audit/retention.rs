// SPDX-License-Identifier: MIT

//! Retention policy for the audit log.
//!
//! Two limits applied in order:
//! 1. **Age** — events with timestamp older than `now - max_age` are dropped.
//! 2. **Size cap** — if the surviving events would still occupy more than
//!    `max_bytes` on disk, the oldest survivors are dropped until under the
//!    cap. A *single* event larger than the cap is retained: the cap is a
//!    defense-in-depth ceiling, not a guarantee — silently dropping the
//!    only thing in the log would lose the user-facing record entirely.
//!
//! [`partition_by_retention`] is a pure function so the policy can be
//! exercised exhaustively without touching the filesystem or the system
//! clock; the audit service wires it into the lazy compaction trigger that
//! fires from [`super::service::AuditService::record`].

use chrono::{DateTime, Duration, Utc};

use crate::services::audit::format::AuditEvent;

/// A decrypted audit event paired with the byte length it occupies on
/// disk (base64-encoded frame + newline). Pairing the size next to the
/// event lets [`partition_by_retention`] apply the size cap without
/// re-encoding to measure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SizedEvent {
    pub event: AuditEvent,
    pub encoded_len: usize,
}

/// Partition `events` into `(keep, drop)` by the two retention policies.
///
/// `now` is passed in so the function is pure: tests pin the clock and
/// the service supplies `Utc::now()`. Within each output vector, events
/// stay in input order — callers append in chronological order, so the
/// kept slice round-trips back to disk preserving append order.
///
/// Age cutoff is inclusive: an event with timestamp exactly equal to
/// `now - max_age` is kept. The boundary is on the *cutoff* side so a
/// pinned 90-day retention doesn't lose the 90-day-old event a user
/// would expect to still be there.
pub fn partition_by_retention(
    events: Vec<SizedEvent>,
    now: DateTime<Utc>,
    max_age: Duration,
    max_bytes: usize,
) -> (Vec<SizedEvent>, Vec<SizedEvent>) {
    let cutoff = now - max_age;

    let mut keep: Vec<SizedEvent> = Vec::with_capacity(events.len());
    let mut drop: Vec<SizedEvent> = Vec::new();

    for item in events {
        if event_timestamp(&item.event) >= cutoff {
            keep.push(item);
        } else {
            drop.push(item);
        }
    }

    // Apply the size cap: while the kept set occupies more than `max_bytes`
    // AND there's more than one event to drop from, push the oldest kept
    // event into drop. The "more than one" guard is what implements the
    // single-event-bigger-than-cap rule — we never produce an empty kept
    // set just because one frame is oversized.
    let mut total: usize = keep.iter().map(|f| f.encoded_len).sum();
    while total > max_bytes && keep.len() > 1 {
        let oldest = keep.remove(0);
        total = total.saturating_sub(oldest.encoded_len);
        drop.push(oldest);
    }

    (keep, drop)
}

fn event_timestamp(event: &AuditEvent) -> DateTime<Utc> {
    match event {
        AuditEvent::VaultUnlockFailed { timestamp, .. }
        | AuditEvent::VaultOpened { timestamp }
        | AuditEvent::VaultLocked { timestamp, .. }
        | AuditEvent::EntryPasswordRevealed { timestamp, .. }
        | AuditEvent::EntryPasswordCopied { timestamp, .. }
        | AuditEvent::EntryProtectedFieldRevealed { timestamp, .. }
        | AuditEvent::PreferencesSecurityChanged { timestamp, .. }
        | AuditEvent::AuditCleared { timestamp } => *timestamp,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(days_ago: i64, now: DateTime<Utc>) -> DateTime<Utc> {
        now - Duration::days(days_ago)
    }

    fn opened(ts: DateTime<Utc>, encoded_len: usize) -> SizedEvent {
        SizedEvent {
            event: AuditEvent::VaultOpened { timestamp: ts },
            encoded_len,
        }
    }

    fn now_fixed() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 17, 12, 0, 0).unwrap()
    }

    #[test]
    fn empty_input_partitions_to_empty_keep_and_empty_drop() {
        let (keep, drop) =
            partition_by_retention(Vec::new(), now_fixed(), Duration::days(90), 10_000);
        assert!(keep.is_empty());
        assert!(drop.is_empty());
    }

    /// All events younger than `max_age` survive untouched and `drop`
    /// stays empty — the most common steady-state shape (no retention
    /// pressure yet) must not churn frames.
    #[test]
    fn all_young_events_are_kept() {
        let now = now_fixed();
        let events = vec![
            opened(at(1, now), 100),
            opened(at(2, now), 100),
            opened(at(3, now), 100),
        ];
        let (keep, drop) = partition_by_retention(events.clone(), now, Duration::days(90), 10_000);
        assert_eq!(keep, events);
        assert!(drop.is_empty());
    }

    /// Every event older than `max_age` ends up in `drop`. The size cap
    /// is irrelevant here — age is the primary policy and must clear the
    /// log on its own when nothing is fresh.
    #[test]
    fn all_old_events_are_dropped() {
        let now = now_fixed();
        let events = vec![
            opened(at(200, now), 100),
            opened(at(150, now), 100),
            opened(at(100, now), 100),
        ];
        let (keep, drop) = partition_by_retention(events.clone(), now, Duration::days(90), 10_000);
        assert!(keep.is_empty());
        assert_eq!(drop, events);
    }

    /// The age cutoff is inclusive: an event whose timestamp is exactly
    /// `now - max_age` is kept. Pinning this means a 90-day retention
    /// doesn't lose the 90-day-old record a user would expect to find.
    #[test]
    fn event_exactly_at_age_boundary_is_kept() {
        let now = now_fixed();
        let max_age = Duration::days(90);
        let on_boundary = opened(now - max_age, 100);
        let one_second_older = opened(now - max_age - Duration::seconds(1), 100);

        let (keep, drop) = partition_by_retention(
            vec![one_second_older.clone(), on_boundary.clone()],
            now,
            max_age,
            10_000,
        );
        assert_eq!(keep, vec![on_boundary]);
        assert_eq!(drop, vec![one_second_older]);
    }

    /// When the size cap would still be exceeded after the age pass, the
    /// oldest *surviving* events are dropped until the kept set is under
    /// the cap. The drop set therefore contains BOTH the age-evicted and
    /// the size-evicted events; size-evicted ones are appended after the
    /// age-evicted ones to make the audit trail of "what compaction did"
    /// readable in order.
    #[test]
    fn over_size_cap_drops_oldest_survivors_after_age_pass() {
        let now = now_fixed();
        // Each event is 100 bytes; cap is 250 bytes; 4 young events
        // would total 400 — must drop the two oldest survivors to land
        // at 200, which is under the cap.
        let young_oldest = opened(at(10, now), 100);
        let young_old = opened(at(8, now), 100);
        let young_new = opened(at(6, now), 100);
        let young_newest = opened(at(4, now), 100);
        let too_old = opened(at(200, now), 100);

        let (keep, drop) = partition_by_retention(
            vec![
                too_old.clone(),
                young_oldest.clone(),
                young_old.clone(),
                young_new.clone(),
                young_newest.clone(),
            ],
            now,
            Duration::days(90),
            250,
        );

        assert_eq!(keep, vec![young_new, young_newest]);
        assert_eq!(drop, vec![too_old, young_oldest, young_old]);
    }

    /// A single event whose encoded length exceeds the cap is RETAINED.
    /// The cap is a defense-in-depth ceiling, not a guarantee — silently
    /// dropping the only thing in the log would lose the user-facing
    /// record entirely. Better to overflow the cap than to vanish.
    #[test]
    fn single_event_larger_than_cap_is_retained() {
        let now = now_fixed();
        let oversized = opened(at(1, now), 50_000);
        let (keep, drop) =
            partition_by_retention(vec![oversized.clone()], now, Duration::days(90), 10_000);
        assert_eq!(keep, vec![oversized]);
        assert!(drop.is_empty());
    }
}
