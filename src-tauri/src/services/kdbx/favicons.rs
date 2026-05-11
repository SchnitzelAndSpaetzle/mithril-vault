use crate::dto::error::AppError;
use image::imageops::FilterType;
use image::ImageFormat;
use keepass::db::{Entry as KeepassEntry, Icon, Node, Times};
use reqwest::header::CONTENT_TYPE;
use reqwest::redirect::Policy;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::io::Cursor;
use url::Url;
use uuid::Uuid;

use super::KdbxService;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FaviconFetchOutcome {
    Updated,
    Unchanged,
    NotFound,
}

const FAVICON_MAX_BYTES: usize = 512 * 1024;
const GOOGLE_FAVICON_URL: &str = "https://www.google.com/s2/favicons";
const ICON_HORSE_URL: &str = "https://icon.horse/icon";

impl KdbxService {
    pub async fn fetch_entry_favicon(
        &self,
        db_id: &str,
        entry_id: &str,
        allow_third_party_fallbacks: bool,
        force: bool,
    ) -> Result<FaviconFetchOutcome, AppError> {
        let (entry_url, has_custom_icon) = {
            let normalized_path = Self::normalize_path(db_id);
            let databases = self.lock_databases()?;
            let open_db = databases
                .get(&normalized_path)
                .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;
            let db = open_db.db_or_locked()?;
            let entry = find_entry_by_id_ref(&db.root, entry_id)
                .ok_or_else(|| AppError::EntryNotFound(entry_id.to_string()))?;
            (
                entry.get_url().map(str::to_string),
                entry.custom_icon_uuid.is_some(),
            )
        };

        if !force && has_custom_icon {
            return Ok(FaviconFetchOutcome::Unchanged);
        }

        let Some(raw_url) = entry_url else {
            return Ok(FaviconFetchOutcome::NotFound);
        };

        let Some((exact_host, root_host)) = extract_hosts(&raw_url) else {
            return Ok(FaviconFetchOutcome::NotFound);
        };

        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(10))
            .redirect(Policy::custom(|attempt| {
                if attempt.url().scheme() != "https" {
                    attempt.stop()
                } else if attempt.previous().len() >= 5 {
                    attempt.error("too many redirects")
                } else {
                    attempt.follow()
                }
            }))
            .user_agent("MithrilVault/0.1")
            .build()
            .map_err(|error| AppError::Io(error.to_string()))?;

        let candidates = build_favicon_candidates(
            &exact_host,
            root_host.as_deref(),
            allow_third_party_fallbacks,
        );
        let mut attempted_domains = HashSet::new();

        for candidate in candidates {
            if !force && self.is_favicon_domain_on_cooldown(&candidate.cooldown_domain)? {
                continue;
            }
            attempted_domains.insert(candidate.cooldown_domain.clone());

            let Some((downloaded_bytes, content_type)) =
                fetch_favicon_bytes(&client, &candidate.fetch_url).await
            else {
                continue;
            };

            let (icon_bytes, mime_type) = normalize_favicon_bytes(&downloaded_bytes, content_type);
            if icon_bytes.is_empty() {
                continue;
            }

            let changed =
                self.assign_entry_custom_icon(db_id, entry_id, &icon_bytes, &mime_type, force)?;
            let _ = self.clear_favicon_domain_failure(&candidate.cooldown_domain);
            return Ok(if changed {
                FaviconFetchOutcome::Updated
            } else {
                FaviconFetchOutcome::Unchanged
            });
        }

        for domain in attempted_domains {
            let _ = self.mark_favicon_domain_failed(&domain);
        }

        Ok(FaviconFetchOutcome::NotFound)
    }

    pub fn set_entry_custom_icon(
        &self,
        db_id: &str,
        entry_id: &str,
        icon_uuid: &str,
    ) -> Result<bool, AppError> {
        let parsed_uuid = Uuid::parse_str(icon_uuid)
            .map_err(|error| AppError::InvalidInput(error.to_string()))?;

        let normalized_path = Self::normalize_path(db_id);
        let mut databases = self.lock_databases()?;
        let open_db = databases
            .get_mut(&normalized_path)
            .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;
        let db = open_db.db_mut_or_locked()?;

        let icon_exists = db
            .meta
            .custom_icons
            .icons
            .iter()
            .any(|icon| icon.uuid == parsed_uuid);
        if !icon_exists {
            return Err(AppError::InvalidInput(format!(
                "custom icon {icon_uuid} not found in database"
            )));
        }

        let Some((entry, _group_id)) = find_entry_by_id_mut(&mut db.root, entry_id) else {
            return Err(AppError::EntryNotFound(entry_id.to_string()));
        };

        if entry.custom_icon_uuid == Some(parsed_uuid) {
            return Ok(false);
        }

        entry.custom_icon_uuid = Some(parsed_uuid);
        entry.times.set_last_modification(Times::now());
        open_db.is_modified = true;
        Ok(true)
    }

    pub fn clear_entry_custom_icon(&self, db_id: &str, entry_id: &str) -> Result<bool, AppError> {
        let normalized_path = Self::normalize_path(db_id);
        let mut databases = self.lock_databases()?;
        let open_db = databases
            .get_mut(&normalized_path)
            .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;
        let db = open_db.db_mut_or_locked()?;
        let Some((entry, _group_id)) = find_entry_by_id_mut(&mut db.root, entry_id) else {
            return Err(AppError::EntryNotFound(entry_id.to_string()));
        };

        if entry.custom_icon_uuid.is_none() {
            return Ok(false);
        }

        entry.custom_icon_uuid = None;
        entry.times.set_last_modification(Times::now());
        open_db.is_modified = true;
        Ok(true)
    }

    fn assign_entry_custom_icon(
        &self,
        db_id: &str,
        entry_id: &str,
        icon_bytes: &[u8],
        _mime_type: &str,
        force: bool,
    ) -> Result<bool, AppError> {
        let normalized_path = Self::normalize_path(db_id);
        let mut databases = self.lock_databases()?;
        let open_db = databases
            .get_mut(&normalized_path)
            .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;

        let db = open_db.db_mut_or_locked()?;

        let already_has = {
            let entry_ref = find_entry_by_id_ref(&db.root, entry_id)
                .ok_or_else(|| AppError::EntryNotFound(entry_id.to_string()))?;
            entry_ref.custom_icon_uuid.is_some()
        };

        if !force && already_has {
            return Ok(false);
        }

        let target_hash = hash_bytes(icon_bytes);
        let existing_uuid = db
            .meta
            .custom_icons
            .icons
            .iter()
            .find_map(|icon| (hash_bytes(&icon.data) == target_hash).then_some(icon.uuid));

        let icon_uuid = if let Some(uuid) = existing_uuid {
            uuid
        } else {
            let uuid = Uuid::new_v4();
            db.meta.custom_icons.icons.push(Icon {
                uuid,
                data: icon_bytes.to_vec(),
            });
            uuid
        };

        let Some((entry, _group_id)) = find_entry_by_id_mut(&mut db.root, entry_id) else {
            return Err(AppError::EntryNotFound(entry_id.to_string()));
        };

        if entry.custom_icon_uuid == Some(icon_uuid) {
            return Ok(false);
        }

        entry.custom_icon_uuid = Some(icon_uuid);
        entry.times.set_last_modification(Times::now());
        open_db.is_modified = true;
        Ok(true)
    }
}

#[derive(Debug, Clone)]
struct FaviconCandidate {
    fetch_url: String,
    cooldown_domain: String,
}

fn build_favicon_candidates(
    exact_host: &str,
    root_host: Option<&str>,
    allow_third_party_fallbacks: bool,
) -> Vec<FaviconCandidate> {
    let mut hosts = vec![exact_host.to_string()];
    if let Some(root) = root_host {
        if root != exact_host {
            hosts.push(root.to_string());
        }
    }

    let mut candidates = Vec::new();
    for host in &hosts {
        candidates.push(FaviconCandidate {
            fetch_url: format!("https://{host}/favicon.ico"),
            cooldown_domain: host.clone(),
        });
    }

    if allow_third_party_fallbacks {
        for host in &hosts {
            candidates.push(FaviconCandidate {
                fetch_url: format!("{GOOGLE_FAVICON_URL}?domain={host}&sz=64"),
                cooldown_domain: host.clone(),
            });
            candidates.push(FaviconCandidate {
                fetch_url: format!("{ICON_HORSE_URL}/{host}"),
                cooldown_domain: host.clone(),
            });
        }
    }

    candidates
}

fn extract_hosts(entry_url: &str) -> Option<(String, Option<String>)> {
    let parsed = Url::parse(entry_url).ok()?;
    match parsed.scheme() {
        "http" | "https" => {}
        _ => return None,
    }
    let host = parsed.host_str()?.to_ascii_lowercase();
    let root = get_root_host(&host);
    Some((host, root))
}

fn get_root_host(host: &str) -> Option<String> {
    if host.parse::<std::net::Ipv4Addr>().is_ok() || host.parse::<std::net::Ipv6Addr>().is_ok() {
        return None;
    }

    // Use the Public Suffix List to find the registrable domain (eTLD+1).
    // If the host already IS the registrable apex (e.g. example.co.uk) or
    // sits directly on a public suffix that we shouldn't generalize to
    // (e.g. user.github.io, app.netlify.app), domain_str returns the host
    // itself and we skip the root-host fallback.
    let registrable = psl::domain_str(host)?;
    if registrable.eq_ignore_ascii_case(host) {
        return None;
    }
    Some(registrable.to_ascii_lowercase())
}

async fn fetch_favicon_bytes(
    client: &reqwest::Client,
    fetch_url: &str,
) -> Option<(Vec<u8>, Option<String>)> {
    let mut response = client.get(fetch_url).send().await.ok()?;
    let requested_https = Url::parse(fetch_url).is_ok_and(|url| url.scheme() == "https");
    if requested_https && response.url().scheme() != "https" {
        return None;
    }

    if !response.status().is_success() {
        return None;
    }

    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(normalize_content_type);

    if let Some(ref value) = content_type {
        if !is_potential_image_content_type(value) {
            return None;
        }
    }

    if let Some(len) = response.content_length() {
        if len > FAVICON_MAX_BYTES as u64 {
            return None;
        }
    }

    let mut bytes: Vec<u8> = Vec::new();
    loop {
        match response.chunk().await {
            Ok(Some(chunk)) => {
                if bytes.len().saturating_add(chunk.len()) > FAVICON_MAX_BYTES {
                    return None;
                }
                bytes.extend_from_slice(&chunk);
            }
            Ok(None) => break,
            Err(_) => return None,
        }
    }

    if bytes.is_empty() {
        return None;
    }

    if !has_known_image_signature(&bytes) && !looks_like_svg(&bytes) {
        return None;
    }

    Some((bytes, content_type))
}

fn normalize_content_type(value: &str) -> String {
    value
        .split(';')
        .next()
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .unwrap_or_default()
}

fn is_potential_image_content_type(content_type: &str) -> bool {
    content_type.starts_with("image/")
        || content_type == "application/xml"
        || content_type == "text/xml"
        || content_type == "application/octet-stream"
}

fn normalize_favicon_bytes(bytes: &[u8], content_type: Option<String>) -> (Vec<u8>, String) {
    if let Ok(image) = image::load_from_memory(bytes) {
        let resized = image.resize(64, 64, FilterType::Lanczos3);
        let mut output = Cursor::new(Vec::new());
        if resized.write_to(&mut output, ImageFormat::Png).is_ok() {
            return (output.into_inner(), "image/png".to_string());
        }
    }

    let mime = content_type.unwrap_or_else(|| detect_icon_mime(bytes));
    (bytes.to_vec(), mime)
}

pub(crate) fn detect_icon_mime(bytes: &[u8]) -> String {
    if looks_like_svg(bytes) {
        return "image/svg+xml".to_string();
    }

    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        return "image/png".to_string();
    }
    if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        return "image/jpeg".to_string();
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return "image/gif".to_string();
    }
    if bytes.starts_with(&[0x00, 0x00, 0x01, 0x00]) {
        return "image/x-icon".to_string();
    }
    if bytes.starts_with(b"BM") {
        return "image/bmp".to_string();
    }
    if bytes.starts_with(b"RIFF") && bytes.len() > 11 && &bytes[8..12] == b"WEBP" {
        return "image/webp".to_string();
    }
    if bytes.starts_with(&[0x49, 0x49, 0x2A, 0x00]) || bytes.starts_with(&[0x4D, 0x4D, 0x00, 0x2A])
    {
        return "image/tiff".to_string();
    }

    "application/octet-stream".to_string()
}

fn has_known_image_signature(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x89, b'P', b'N', b'G'])
        || (bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF)
        || bytes.starts_with(b"GIF87a")
        || bytes.starts_with(b"GIF89a")
        || bytes.starts_with(&[0x00, 0x00, 0x01, 0x00])
        || bytes.starts_with(b"BM")
        || (bytes.starts_with(b"RIFF") && bytes.len() > 11 && &bytes[8..12] == b"WEBP")
        || bytes.starts_with(&[0x49, 0x49, 0x2A, 0x00])
        || bytes.starts_with(&[0x4D, 0x4D, 0x00, 0x2A])
}

fn looks_like_svg(bytes: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    text.trim_start().starts_with("<svg") || text.contains("<svg")
}

fn hash_bytes(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

fn find_entry_by_id_ref<'a>(group: &'a keepass::db::Group, id: &str) -> Option<&'a KeepassEntry> {
    for node in &group.children {
        match node {
            Node::Entry(entry) => {
                if entry.uuid.to_string() == id {
                    return Some(entry);
                }
            }
            Node::Group(child) => {
                if let Some(found) = find_entry_by_id_ref(child, id) {
                    return Some(found);
                }
            }
        }
    }
    None
}

fn find_entry_by_id_mut<'a>(
    group: &'a mut keepass::db::Group,
    id: &str,
) -> Option<(&'a mut KeepassEntry, String)> {
    let group_id = group.uuid.to_string();

    for node in &mut group.children {
        match node {
            Node::Entry(entry) => {
                if entry.uuid.to_string() == id {
                    return Some((entry, group_id));
                }
            }
            Node::Group(child) => {
                if let Some(found) = find_entry_by_id_mut(child, id) {
                    return Some(found);
                }
            }
        }
    }

    None
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::domain::secure::SecureString;
    use crate::dto::database::DatabaseCreationOptions;
    use crate::dto::entry::CreateEntryData;
    use image::{ImageBuffer, Rgba};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use tempfile::TempDir;

    fn create_test_database() -> (KdbxService, TempDir, String, String, String) {
        let dir = tempfile::tempdir().expect("create temp dir");
        let db_path = dir.path().join("favicon-tests.kdbx");
        let db_path_str = db_path.to_string_lossy().to_string();

        let options = DatabaseCreationOptions {
            create_default_groups: true,
            kdf_memory: Some(1024 * 1024),
            kdf_iterations: Some(1),
            kdf_parallelism: Some(1),
            description: None,
        };

        let service = KdbxService::new();
        service
            .create_database(
                &db_path_str,
                Some("testpass"),
                None,
                "Favicon Tests",
                &options,
            )
            .expect("create db");
        let info = service.get_info(&db_path_str).expect("database info");

        let entry_a = service
            .create_entry(
                &db_path_str,
                &info.root_group_id,
                CreateEntryData {
                    title: "Entry A".to_string(),
                    username: "alice".to_string(),
                    password: SecureString::from("secret"),
                    url: None,
                    notes: None,
                    icon_id: Some(0),
                    tags: None,
                    custom_fields: None,
                    protected_custom_fields: None,
                },
            )
            .expect("create entry A");
        let entry_b = service
            .create_entry(
                &db_path_str,
                &info.root_group_id,
                CreateEntryData {
                    title: "Entry B".to_string(),
                    username: "bob".to_string(),
                    password: SecureString::from("secret"),
                    url: None,
                    notes: None,
                    icon_id: Some(0),
                    tags: None,
                    custom_fields: None,
                    protected_custom_fields: None,
                },
            )
            .expect("create entry B");

        (service, dir, db_path_str, entry_a.id, entry_b.id)
    }

    fn tiny_png() -> Vec<u8> {
        let image = ImageBuffer::from_pixel(2, 2, Rgba([0_u8, 128, 255, 255]));
        let mut output = Cursor::new(Vec::new());
        image
            .write_to(&mut output, ImageFormat::Png)
            .expect("write png");
        output.into_inner()
    }

    fn serve_once(response: Vec<u8>) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let address = listener.local_addr().expect("read local addr");
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request);
            // Client may close the connection mid-write (e.g. when the
            // streaming cap rejects an oversized body), so a broken pipe
            // here is expected.
            let _ = stream.write_all(&response);
        });

        (format!("http://{address}/favicon.ico"), handle)
    }

    #[test]
    fn build_candidates_tries_exact_host_before_root_host() {
        let candidates = build_favicon_candidates("app.example.com", Some("example.com"), false);
        let urls: Vec<&str> = candidates
            .iter()
            .map(|candidate| candidate.fetch_url.as_str())
            .collect();

        assert_eq!(urls.len(), 2);
        assert_eq!(urls[0], "https://app.example.com/favicon.ico");
        assert_eq!(urls[1], "https://example.com/favicon.ico");
    }

    #[test]
    fn build_candidates_adds_opt_in_third_party_sources() {
        let candidates = build_favicon_candidates("app.example.com", Some("example.com"), true);

        assert!(candidates.iter().any(|candidate| {
            candidate.fetch_url == "https://www.google.com/s2/favicons?domain=app.example.com&sz=64"
        }));
        assert!(candidates
            .iter()
            .any(|candidate| candidate.fetch_url == "https://icon.horse/icon/app.example.com"));
    }

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
                },
            )
            .expect("set url");

        let initial_stamp = std::time::Instant::now()
            .checked_sub(std::time::Duration::from_secs(30))
            .expect("build initial cooldown stamp");
        {
            let mut failures = service
                .favicon_failed_domains
                .lock()
                .expect("lock cooldown map");
            failures.insert("example.com".to_string(), initial_stamp);
        }

        // Non-forced (auto-fetch) path must honor the cooldown and must not
        // bump the timestamp of a domain it never contacted.
        let outcome = tauri::async_runtime::block_on(
            service.fetch_entry_favicon(&db_path, &entry_id, false, false),
        )
        .expect("fetch favicon");

        assert_eq!(outcome, FaviconFetchOutcome::NotFound);

        let failures = service
            .favicon_failed_domains
            .lock()
            .expect("lock cooldown map");
        let stamp = failures
            .get("example.com")
            .copied()
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
                },
            )
            .expect("set url");

        let initial_stamp = std::time::Instant::now()
            .checked_sub(std::time::Duration::from_secs(30))
            .expect("build initial cooldown stamp");
        {
            let mut failures = service
                .favicon_failed_domains
                .lock()
                .expect("lock cooldown map");
            failures.insert("example.com".to_string(), initial_stamp);
        }

        // Force=true mirrors the manual "Refetch favicon" path. The cooldown
        // gate must be bypassed so the user actually retries the host. The
        // attempt then fails (no network in tests) and refreshes the stamp.
        let outcome = tauri::async_runtime::block_on(
            service.fetch_entry_favicon(&db_path, &entry_id, false, true),
        )
        .expect("fetch favicon");

        assert_eq!(outcome, FaviconFetchOutcome::NotFound);

        let failures = service
            .favicon_failed_domains
            .lock()
            .expect("lock cooldown map");
        let stamp = failures
            .get("example.com")
            .copied()
            .expect("cooldown entry preserved");
        assert!(
            stamp > initial_stamp,
            "force=true must contact the domain, which refreshes the failure stamp on miss"
        );
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
    fn set_entry_custom_icon_assigns_existing_icon() {
        let (service, _dir, db_path, entry_a, entry_b) = create_test_database();
        let icon_bytes = [0x89, b'P', b'N', b'G', 99];
        service
            .assign_entry_custom_icon(&db_path, &entry_a, &icon_bytes, "image/png", true)
            .expect("seed icon");

        let icon_uuid = {
            let normalized = KdbxService::normalize_path(&db_path);
            let databases = service.lock_databases().expect("lock databases");
            let open_db = databases.get(&normalized).expect("open db");
            let db = open_db.db_or_locked().expect("unlocked db");
            db.meta.custom_icons.icons[0].uuid.to_string()
        };

        let changed = service
            .set_entry_custom_icon(&db_path, &entry_b, &icon_uuid)
            .expect("set icon on b");
        assert!(changed);

        let unchanged = service
            .set_entry_custom_icon(&db_path, &entry_b, &icon_uuid)
            .expect("set icon on b again");
        assert!(!unchanged);
    }

    #[test]
    fn set_entry_custom_icon_rejects_unknown_uuid() {
        let (service, _dir, db_path, entry_a, _entry_b) = create_test_database();
        let result = service.set_entry_custom_icon(
            &db_path,
            &entry_a,
            "00000000-0000-0000-0000-000000000000",
        );

        assert!(matches!(result, Err(AppError::InvalidInput(_))));
    }

    #[test]
    fn extract_hosts_normalizes_subdomain_and_root_host() {
        let hosts = extract_hosts("https://APP.Example.COM/login").expect("extract hosts");

        assert_eq!(hosts.0, "app.example.com");
        assert_eq!(hosts.1.as_deref(), Some("example.com"));
        assert_eq!(
            extract_hosts("https://login.example.co.uk")
                .expect("extract multi-label public suffix root")
                .1
                .as_deref(),
            Some("example.co.uk")
        );

        let bare = extract_hosts("https://example.co.uk").expect("registrable apex");
        assert_eq!(bare.0, "example.co.uk");
        assert!(
            bare.1.is_none(),
            "the registrable apex has no further root to fall back to"
        );

        let ip = extract_hosts("http://127.0.0.1:3000").expect("ip literal");
        assert_eq!(ip.0, "127.0.0.1");
        assert!(ip.1.is_none(), "ip literals never produce a root host");

        assert_eq!(extract_hosts("not a url"), None);
    }

    #[test]
    fn extract_hosts_rejects_non_http_schemes() {
        // Non-web entries (ssh, db, ldap, etc.) must not trigger favicon
        // network activity. extract_hosts returns None so fetch_entry_favicon
        // short-circuits with NotFound before any DNS or HTTP traffic.
        for url in [
            "ssh://bastion.corp.example/",
            "postgres://db.internal:5432/prod",
            "ldap://corp-ad.internal/",
            "file:///etc/hosts",
            "ftp://files.example.com/",
            "obsidian://open?vault=Notes",
        ] {
            assert_eq!(extract_hosts(url), None, "expected no fetch for {url}");
        }
    }

    #[test]
    fn extract_hosts_skips_root_fallback_for_psl_hosting_platforms() {
        // PSL entries like github.io / netlify.app / vercel.app are public
        // suffixes themselves, so user.github.io is already the registrable
        // apex. The root-host fallback must be skipped so we never fetch
        // the provider's marketing-page favicon by accident.
        for host in [
            "user.github.io",
            "team.netlify.app",
            "demo.vercel.app",
            "tenant.azurewebsites.net",
        ] {
            let url = format!("https://{host}/");
            let parsed = extract_hosts(&url).expect("parse psl host");
            assert_eq!(parsed.0, host);
            assert!(
                parsed.1.is_none(),
                "{host} should not produce a root-host fallback"
            );
        }
    }

    #[test]
    fn detect_icon_mime_recognizes_supported_signatures() {
        assert_eq!(
            detect_icon_mime(b"  \n<svg viewBox=\"0 0 1 1\" />"),
            "image/svg+xml"
        );
        assert_eq!(detect_icon_mime(&[0x89, b'P', b'N', b'G']), "image/png");
        assert_eq!(detect_icon_mime(&[0xFF, 0xD8, 0xFF]), "image/jpeg");
        assert_eq!(detect_icon_mime(b"GIF89a"), "image/gif");
        assert_eq!(detect_icon_mime(&[0x00, 0x00, 0x01, 0x00]), "image/x-icon");
        assert_eq!(detect_icon_mime(b"BM..."), "image/bmp");
        assert_eq!(detect_icon_mime(b"RIFF....WEBP"), "image/webp");
        assert_eq!(detect_icon_mime(&[0x49, 0x49, 0x2A, 0x00]), "image/tiff");
        assert_eq!(detect_icon_mime(b"plain text"), "application/octet-stream");
    }

    #[test]
    fn normalize_favicon_bytes_resizes_decodable_images_to_png() {
        let original = tiny_png();
        let (normalized, mime_type) =
            normalize_favicon_bytes(&original, Some("image/png".to_string()));

        assert_eq!(mime_type, "image/png");
        assert!(normalized.starts_with(&[0x89, b'P', b'N', b'G']));
        assert!(
            image::load_from_memory(&normalized).is_ok(),
            "normalized favicon should remain a decodable image"
        );
    }

    #[test]
    fn normalize_favicon_bytes_preserves_svg_with_content_type() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg"></svg>"#;
        let (normalized, mime_type) =
            normalize_favicon_bytes(svg, Some("image/svg+xml".to_string()));

        assert_eq!(normalized, svg);
        assert_eq!(mime_type, "image/svg+xml");
    }

    #[test]
    fn fetch_favicon_bytes_accepts_valid_image_responses() {
        let png = tiny_png();
        let response = [
            b"HTTP/1.1 200 OK\r\nContent-Type: image/png; charset=utf-8\r\nContent-Length: "
                .as_slice(),
            png.len().to_string().as_bytes(),
            b"\r\n\r\n",
            png.as_slice(),
        ]
        .concat();
        let (url, handle) = serve_once(response);
        let client = reqwest::Client::new();

        let fetched =
            tauri::async_runtime::block_on(fetch_favicon_bytes(&client, &url)).expect("fetch png");
        handle.join().expect("server finishes");

        assert_eq!(fetched.0, png);
        assert_eq!(fetched.1.as_deref(), Some("image/png"));
    }

    #[test]
    fn fetch_favicon_bytes_rejects_non_image_content_type() {
        let body = b"<html>not an icon</html>";
        let response = [
            b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: ".as_slice(),
            body.len().to_string().as_bytes(),
            b"\r\n\r\n",
            body.as_slice(),
        ]
        .concat();
        let (url, handle) = serve_once(response);
        let client = reqwest::Client::new();

        let fetched = tauri::async_runtime::block_on(fetch_favicon_bytes(&client, &url));
        handle.join().expect("server finishes");

        assert!(fetched.is_none());
    }

    #[test]
    fn fetch_favicon_bytes_caps_streamed_body_without_content_length() {
        // No Content-Length header — the streaming reader must enforce the
        // cap on its own and short-circuit before buffering the whole body.
        let oversized = vec![0_u8; FAVICON_MAX_BYTES + 1];
        let response = [
            b"HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nTransfer-Encoding: chunked\r\n\r\n"
                .as_slice(),
            format!("{:x}\r\n", oversized.len()).as_bytes(),
            oversized.as_slice(),
            b"\r\n0\r\n\r\n",
        ]
        .concat();
        let (url, handle) = serve_once(response);
        let client = reqwest::Client::new();

        let fetched = tauri::async_runtime::block_on(fetch_favicon_bytes(&client, &url));
        let _ = handle.join();

        assert!(fetched.is_none());
    }

    #[test]
    fn fetch_favicon_bytes_rejects_non_https_redirects() {
        // Server replies with a redirect to an http:// target. The redirect
        // policy should stop following, so the final response stays 302 and
        // is_success() filters it out.
        let response = b"HTTP/1.1 302 Found\r\nLocation: http://example.invalid/favicon.ico\r\nContent-Length: 0\r\n\r\n".to_vec();
        let (url, handle) = serve_once(response);
        let client = reqwest::Client::builder()
            .redirect(Policy::custom(|attempt| {
                if attempt.url().scheme() != "https" {
                    attempt.stop()
                } else if attempt.previous().len() >= 5 {
                    attempt.error("too many redirects")
                } else {
                    attempt.follow()
                }
            }))
            .build()
            .expect("client");

        let fetched = tauri::async_runtime::block_on(fetch_favicon_bytes(&client, &url));
        handle.join().expect("server finishes");

        assert!(fetched.is_none());
    }

    #[test]
    fn fetch_favicon_bytes_rejects_oversized_icons() {
        let oversized = vec![0_u8; FAVICON_MAX_BYTES + 1];
        let response = [
            b"HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: ".as_slice(),
            oversized.len().to_string().as_bytes(),
            b"\r\n\r\n",
            oversized.as_slice(),
        ]
        .concat();
        let (url, handle) = serve_once(response);
        let client = reqwest::Client::new();

        let fetched = tauri::async_runtime::block_on(fetch_favicon_bytes(&client, &url));
        handle.join().expect("server finishes");

        assert!(fetched.is_none());
    }

    #[test]
    fn assign_entry_custom_icon_deduplicates_icon_bytes() {
        let (service, _dir, db_path, entry_a, entry_b) = create_test_database();
        let icon_bytes = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0x00];

        let changed_a = service
            .assign_entry_custom_icon(&db_path, &entry_a, &icon_bytes, "image/png", true)
            .expect("assign icon A");
        let changed_b = service
            .assign_entry_custom_icon(&db_path, &entry_b, &icon_bytes, "image/png", true)
            .expect("assign icon B");

        assert!(changed_a);
        assert!(changed_b);

        let normalized = KdbxService::normalize_path(&db_path);
        let databases = service.lock_databases().expect("lock databases");
        let open_db = databases.get(&normalized).expect("open db");
        let db = open_db.db_or_locked().expect("unlocked db");
        assert_eq!(db.meta.custom_icons.icons.len(), 1);

        let icon_a = find_entry_by_id_ref(&db.root, &entry_a)
            .and_then(|entry| entry.custom_icon_uuid)
            .expect("entry A icon");
        let icon_b = find_entry_by_id_ref(&db.root, &entry_b)
            .and_then(|entry| entry.custom_icon_uuid)
            .expect("entry B icon");
        assert_eq!(icon_a, icon_b);
    }

    #[test]
    fn clear_entry_custom_icon_detaches_icon_uuid() {
        let (service, _dir, db_path, entry_a, _entry_b) = create_test_database();
        let icon_bytes = [1_u8, 2, 3, 4, 5];
        service
            .assign_entry_custom_icon(
                &db_path,
                &entry_a,
                &icon_bytes,
                "application/octet-stream",
                true,
            )
            .expect("assign icon");

        let cleared = service
            .clear_entry_custom_icon(&db_path, &entry_a)
            .expect("clear icon");
        let cleared_again = service
            .clear_entry_custom_icon(&db_path, &entry_a)
            .expect("clear icon again");

        assert!(cleared);
        assert!(!cleared_again);

        let normalized = KdbxService::normalize_path(&db_path);
        let databases = service.lock_databases().expect("lock databases");
        let open_db = databases.get(&normalized).expect("open db");
        let db = open_db.db_or_locked().expect("unlocked db");
        let entry = find_entry_by_id_ref(&db.root, &entry_a).expect("entry");
        assert!(entry.custom_icon_uuid.is_none());
    }

    #[test]
    fn assign_entry_custom_icon_respects_existing_icon_without_force() {
        let (service, _dir, db_path, entry_a, _entry_b) = create_test_database();
        let first_icon = [0x89, b'P', b'N', b'G', 1];
        let replacement_icon = [0x89, b'P', b'N', b'G', 2];

        assert!(service
            .assign_entry_custom_icon(&db_path, &entry_a, &first_icon, "image/png", true)
            .expect("assign first icon"));
        assert!(
            !service
                .assign_entry_custom_icon(&db_path, &entry_a, &replacement_icon, "image/png", false)
                .expect("skip replacement"),
            "non-forced favicon fetches should preserve a user-selected icon"
        );

        let normalized = KdbxService::normalize_path(&db_path);
        let databases = service.lock_databases().expect("lock databases");
        let open_db = databases.get(&normalized).expect("open db");
        let db = open_db.db_or_locked().expect("unlocked db");
        assert_eq!(db.meta.custom_icons.icons.len(), 1);
        assert_eq!(db.meta.custom_icons.icons[0].data, first_icon);
    }
}
