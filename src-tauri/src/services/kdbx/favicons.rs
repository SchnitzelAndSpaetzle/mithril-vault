use crate::dto::error::AppError;
use image::imageops::FilterType;
use image::ImageFormat;
use keepass::db::{Entry as KeepassEntry, Icon, Node, Times};
use reqwest::header::CONTENT_TYPE;
use reqwest::redirect::Policy;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::io::Cursor;
use url::Url;
use uuid::Uuid;

use super::KdbxService;

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
    ) -> Result<bool, AppError> {
        let (entry_url, has_custom_icon) = {
            let normalized_path = Self::normalize_path(db_id);
            let databases = self.lock_databases()?;
            let open_db = databases
                .get(&normalized_path)
                .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;
            let entry = find_entry_by_id_ref(&open_db.db.root, entry_id)
                .ok_or_else(|| AppError::EntryNotFound(entry_id.to_string()))?;
            (
                entry.get_url().map(str::to_string),
                entry.custom_icon_uuid.is_some(),
            )
        };

        if !force && has_custom_icon {
            return Ok(false);
        }

        let Some(raw_url) = entry_url else {
            return Ok(false);
        };

        let Some((exact_host, root_host)) = extract_hosts(&raw_url) else {
            return Ok(false);
        };

        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(10))
            .redirect(Policy::limited(5))
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
            attempted_domains.insert(candidate.cooldown_domain.clone());

            if self.is_favicon_domain_on_cooldown(&candidate.cooldown_domain)? {
                continue;
            }

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
            return Ok(changed);
        }

        for domain in attempted_domains {
            let _ = self.mark_favicon_domain_failed(&domain);
        }

        Ok(false)
    }

    pub fn clear_entry_custom_icon(&self, db_id: &str, entry_id: &str) -> Result<bool, AppError> {
        let normalized_path = Self::normalize_path(db_id);
        let mut databases = self.lock_databases()?;
        let open_db = databases
            .get_mut(&normalized_path)
            .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;
        let Some((entry, _group_id)) = find_entry_by_id_mut(&mut open_db.db.root, entry_id) else {
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

        let Some((entry, _group_id)) = find_entry_by_id_mut(&mut open_db.db.root, entry_id) else {
            return Err(AppError::EntryNotFound(entry_id.to_string()));
        };

        if !force && entry.custom_icon_uuid.is_some() {
            return Ok(false);
        }

        let target_hash = hash_bytes(icon_bytes);
        let existing_uuid = open_db
            .db
            .meta
            .custom_icons
            .icons
            .iter()
            .find_map(|icon| (hash_bytes(&icon.data) == target_hash).then_some(icon.uuid));

        let icon_uuid = if let Some(uuid) = existing_uuid {
            uuid
        } else {
            let uuid = Uuid::new_v4();
            open_db.db.meta.custom_icons.icons.push(Icon {
                uuid,
                data: icon_bytes.to_vec(),
            });
            uuid
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
    let host = parsed.host_str()?.to_ascii_lowercase();
    let root = get_root_host(&host);
    Some((host, root))
}

fn get_root_host(host: &str) -> Option<String> {
    if host.parse::<std::net::Ipv4Addr>().is_ok() || host.parse::<std::net::Ipv6Addr>().is_ok() {
        return None;
    }

    let parts: Vec<&str> = host.split('.').filter(|label| !label.is_empty()).collect();
    if parts.len() < 3 {
        return None;
    }

    Some(parts[parts.len() - 2..].join("."))
}

async fn fetch_favicon_bytes(
    client: &reqwest::Client,
    fetch_url: &str,
) -> Option<(Vec<u8>, Option<String>)> {
    let response = client.get(fetch_url).send().await.ok()?;
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

    let bytes = response.bytes().await.ok()?;
    if bytes.is_empty() || bytes.len() > FAVICON_MAX_BYTES {
        return None;
    }

    if !has_known_image_signature(&bytes) && !looks_like_svg(&bytes) {
        return None;
    }

    Some((bytes.to_vec(), content_type))
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
mod tests {
    use super::*;
    use crate::domain::secure::SecureString;
    use crate::dto::database::DatabaseCreationOptions;
    use crate::dto::entry::CreateEntryData;
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
    fn fetch_entry_favicon_returns_false_without_url() {
        let (service, _dir, db_path, entry_id, _entry_b) = create_test_database();
        let changed = tauri::async_runtime::block_on(
            service.fetch_entry_favicon(&db_path, &entry_id, false, true),
        )
        .expect("fetch favicon");

        assert!(!changed);
    }

    #[test]
    fn fetch_entry_favicon_returns_false_for_invalid_url() {
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

        let changed = tauri::async_runtime::block_on(
            service.fetch_entry_favicon(&db_path, &entry_id, false, true),
        )
        .expect("fetch favicon");

        assert!(!changed);
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
        assert_eq!(open_db.db.meta.custom_icons.icons.len(), 1);

        let icon_a = find_entry_by_id_ref(&open_db.db.root, &entry_a)
            .and_then(|entry| entry.custom_icon_uuid)
            .expect("entry A icon");
        let icon_b = find_entry_by_id_ref(&open_db.db.root, &entry_b)
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
        let entry = find_entry_by_id_ref(&open_db.db.root, &entry_a).expect("entry");
        assert!(entry.custom_icon_uuid.is_none());
    }
}
