mod candidates;
mod http;
mod image;
mod lookup;

use crate::dto::error::AppError;
use keepass::db::Icon;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::KdbxService;
use candidates::build_favicon_candidates;
use http::{build_client, fetch_favicon_bytes};
use image::normalize_favicon_bytes;
use lookup::extract_hosts;

/// Entry-level outcome surfaced to the IPC layer after the pipeline result
/// has been reconciled with the entry's existing icon state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FaviconFetchOutcome {
    Updated,
    Unchanged,
    NotFound,
}

/// Pipeline-level result returned by `fetch_favicon_for_url`. Carries usable
/// bytes when a Candidate succeeded, or the list of attempted cooldown
/// domains when every Candidate failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FaviconFetchResult {
    Found {
        bytes: Vec<u8>,
        mime_type: String,
        cooldown_domain: String,
    },
    NotFound {
        attempted_domains: Vec<String>,
    },
}

/// Per-process record of which `cooldown_domain`s recently failed, so a bulk
/// auto-fetch doesn't hammer the same dead host across every Entry that uses
/// it. Bypassed by the manual "Refetch" path (`force=true`).
#[derive(Debug, Default)]
pub struct FaviconCooldown {
    failed_domains: Mutex<HashMap<String, Instant>>,
}

impl FaviconCooldown {
    pub const COOLDOWN: Duration = Duration::from_mins(15);

    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_on_cooldown(&self, domain: &str) -> Result<bool, AppError> {
        let mut failures = self.failed_domains.lock().map_err(|_| AppError::Lock)?;
        let Some(last_failed_at) = failures.get(domain) else {
            return Ok(false);
        };
        if last_failed_at.elapsed() >= Self::COOLDOWN {
            failures.remove(domain);
            return Ok(false);
        }
        Ok(true)
    }

    pub fn mark_failed(&self, domain: &str) -> Result<(), AppError> {
        let mut failures = self.failed_domains.lock().map_err(|_| AppError::Lock)?;
        failures.insert(domain.to_string(), Instant::now());
        Ok(())
    }

    pub fn clear(&self, domain: &str) -> Result<(), AppError> {
        let mut failures = self.failed_domains.lock().map_err(|_| AppError::Lock)?;
        failures.remove(domain);
        Ok(())
    }

    #[cfg(test)]
    #[allow(clippy::expect_used)]
    pub(crate) fn seed_for_test(&self, domain: &str, at: Instant) {
        self.failed_domains
            .lock()
            .expect("lock cooldown map")
            .insert(domain.to_string(), at);
    }

    #[cfg(test)]
    #[allow(clippy::expect_used)]
    pub(crate) fn stamp_for_test(&self, domain: &str) -> Option<Instant> {
        self.failed_domains
            .lock()
            .expect("lock cooldown map")
            .get(domain)
            .copied()
    }
}

/// Walks the Favicon Candidates derived from `raw_url`, consulting `cooldown`
/// to skip recently-failed domains, and returns the first Candidate whose
/// bytes pass image validation. Pure with respect to the Vault — the caller
/// decides how to reconcile the result with the Entry's icon state.
pub(crate) async fn fetch_favicon_for_url(
    raw_url: &str,
    allow_third_party_fallbacks: bool,
    force: bool,
    cooldown: &FaviconCooldown,
) -> Result<FaviconFetchResult, AppError> {
    let Some(hosts) = extract_hosts(raw_url) else {
        return Ok(FaviconFetchResult::NotFound {
            attempted_domains: Vec::new(),
        });
    };

    let client = build_client()?;
    let candidates = build_favicon_candidates(&hosts, allow_third_party_fallbacks);
    let mut attempted_domains: HashSet<String> = HashSet::new();

    for candidate in candidates {
        if !force && cooldown.is_on_cooldown(&candidate.cooldown_domain)? {
            continue;
        }
        attempted_domains.insert(candidate.cooldown_domain.clone());

        let Some((downloaded_bytes, content_type)) =
            fetch_favicon_bytes(&client, &candidate.fetch_url).await
        else {
            continue;
        };

        let (bytes, mime_type) = normalize_favicon_bytes(&downloaded_bytes, content_type);
        if bytes.is_empty() {
            continue;
        }

        return Ok(FaviconFetchResult::Found {
            bytes,
            mime_type,
            cooldown_domain: candidate.cooldown_domain,
        });
    }

    Ok(FaviconFetchResult::NotFound {
        attempted_domains: attempted_domains.into_iter().collect(),
    })
}

impl KdbxService {
    pub async fn fetch_entry_favicon(
        &self,
        db_id: &str,
        entry_id: &str,
        allow_third_party_fallbacks: bool,
        force: bool,
    ) -> Result<FaviconFetchOutcome, AppError> {
        let (entry_url, has_custom_icon) = self.with_vault(db_id, |vault| {
            let entry = vault.find_entry(entry_id)?;
            Ok((
                entry.get_url().map(str::to_string),
                matches!(entry.icon(), Some(Icon::Custom(_))),
            ))
        })?;

        if !force && has_custom_icon {
            return Ok(FaviconFetchOutcome::Unchanged);
        }

        let Some(raw_url) = entry_url else {
            return Ok(FaviconFetchOutcome::NotFound);
        };

        let result =
            fetch_favicon_for_url(&raw_url, allow_third_party_fallbacks, force, &self.favicons)
                .await?;

        match result {
            FaviconFetchResult::Found {
                bytes,
                mime_type,
                cooldown_domain,
            } => {
                let changed =
                    self.assign_entry_custom_icon(db_id, entry_id, &bytes, &mime_type, force)?;
                let _ = self.favicons.clear(&cooldown_domain);
                Ok(if changed {
                    FaviconFetchOutcome::Updated
                } else {
                    FaviconFetchOutcome::Unchanged
                })
            }
            FaviconFetchResult::NotFound { attempted_domains } => {
                for domain in attempted_domains {
                    let _ = self.favicons.mark_failed(&domain);
                }
                Ok(FaviconFetchOutcome::NotFound)
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::services::kdbx::test_support::create_test_database;

    #[test]
    fn fetch_entry_favicon_returns_not_found_without_url() {
        let (service, _dir, db_path, entry_id, _entry_b) = create_test_database();
        let outcome = tauri::async_runtime::block_on(
            service.fetch_entry_favicon(&db_path, &entry_id, false, true),
        )
        .expect("fetch favicon");

        assert_eq!(outcome, FaviconFetchOutcome::NotFound);
    }

    #[test]
    fn fetch_entry_favicon_returns_not_found_for_invalid_url() {
        let (service, _dir, db_path, entry_id, _entry_b) = create_test_database();
        service
            .update_entry(
                &db_path,
                &entry_id,
                crate::dto::entry::UpdateEntryData {
                    title: None,
                    username: None,
                    password: None,
                    url: Some("not-a-valid-url".to_string()),
                    notes: None,
                    icon_id: None,
                    tags: None,
                    custom_fields: None,
                    protected_custom_fields: None,
                    expires: None,
                    expiry_time: None,
                },
            )
            .expect("set invalid url");

        let outcome = tauri::async_runtime::block_on(
            service.fetch_entry_favicon(&db_path, &entry_id, false, true),
        )
        .expect("fetch favicon");

        assert_eq!(outcome, FaviconFetchOutcome::NotFound);
    }

    #[test]
    fn auto_fetch_does_not_refresh_cooldown_for_skipped_domain() {
        let (service, _dir, db_path, entry_id, _entry_b) = create_test_database();
        service
            .update_entry(
                &db_path,
                &entry_id,
                crate::dto::entry::UpdateEntryData {
                    title: None,
                    username: None,
                    password: None,
                    url: Some("https://example.com".to_string()),
                    notes: None,
                    icon_id: None,
                    tags: None,
                    custom_fields: None,
                    protected_custom_fields: None,
                    expires: None,
                    expiry_time: None,
                },
            )
            .expect("set url");

        let initial_stamp = Instant::now()
            .checked_sub(Duration::from_secs(30))
            .expect("build initial cooldown stamp");
        service.favicons.seed_for_test("example.com", initial_stamp);

        // Non-forced (auto-fetch) path must honor the cooldown and must not
        // bump the timestamp of a domain it never contacted.
        let outcome = tauri::async_runtime::block_on(
            service.fetch_entry_favicon(&db_path, &entry_id, false, false),
        )
        .expect("fetch favicon");

        assert_eq!(outcome, FaviconFetchOutcome::NotFound);

        let stamp = service
            .favicons
            .stamp_for_test("example.com")
            .expect("cooldown entry preserved");
        assert_eq!(
            stamp, initial_stamp,
            "auto-fetch must not refresh a cooldown stamp for a skipped domain"
        );
    }

    #[test]
    fn manual_force_fetch_bypasses_cooldown_and_attempts_the_domain() {
        let (service, _dir, db_path, entry_id, _entry_b) = create_test_database();
        service
            .update_entry(
                &db_path,
                &entry_id,
                crate::dto::entry::UpdateEntryData {
                    title: None,
                    username: None,
                    password: None,
                    url: Some("https://example.com".to_string()),
                    notes: None,
                    icon_id: None,
                    tags: None,
                    custom_fields: None,
                    protected_custom_fields: None,
                    expires: None,
                    expiry_time: None,
                },
            )
            .expect("set url");

        let initial_stamp = Instant::now()
            .checked_sub(Duration::from_secs(30))
            .expect("build initial cooldown stamp");
        service.favicons.seed_for_test("example.com", initial_stamp);

        // Force=true mirrors the manual "Refetch favicon" path. The cooldown
        // gate must be bypassed so the user actually retries the host. The
        // attempt then fails (no network in tests) and refreshes the stamp.
        let outcome = tauri::async_runtime::block_on(
            service.fetch_entry_favicon(&db_path, &entry_id, false, true),
        )
        .expect("fetch favicon");

        assert_eq!(outcome, FaviconFetchOutcome::NotFound);

        let stamp = service
            .favicons
            .stamp_for_test("example.com")
            .expect("cooldown entry preserved");
        assert!(
            stamp > initial_stamp,
            "force=true must contact the domain, which refreshes the failure stamp on miss"
        );
    }

    #[test]
    fn cooldown_tracks_recent_failures() {
        let cooldown = FaviconCooldown::new();

        cooldown
            .mark_failed("example.com")
            .expect("mark domain failed");

        assert!(
            cooldown
                .is_on_cooldown("example.com")
                .expect("read cooldown"),
            "recent failures should suppress repeated favicon fetches"
        );
        assert!(
            !cooldown
                .is_on_cooldown("other.example.com")
                .expect("read cooldown"),
            "cooldown is scoped to the failing domain"
        );
    }

    #[test]
    fn cooldown_expires_and_removes_stale_failures() {
        let cooldown = FaviconCooldown::new();
        let stale_failure_at = Instant::now()
            .checked_sub(FaviconCooldown::COOLDOWN)
            .and_then(|instant| instant.checked_sub(Duration::from_secs(1)))
            .expect("build stale favicon failure timestamp");
        cooldown.seed_for_test("example.com", stale_failure_at);

        assert!(
            !cooldown
                .is_on_cooldown("example.com")
                .expect("read cooldown"),
            "expired favicon failures should not block a retry"
        );
        assert!(
            cooldown.stamp_for_test("example.com").is_none(),
            "is_on_cooldown should evict stale entries"
        );
    }

    #[test]
    fn clear_cooldown_allows_immediate_retry() {
        let cooldown = FaviconCooldown::new();
        cooldown
            .mark_failed("example.com")
            .expect("mark domain failed");

        cooldown.clear("example.com").expect("clear domain failure");

        assert!(
            !cooldown
                .is_on_cooldown("example.com")
                .expect("read cooldown"),
            "clearing a failure should allow an explicit retry after a later success"
        );
    }
}
