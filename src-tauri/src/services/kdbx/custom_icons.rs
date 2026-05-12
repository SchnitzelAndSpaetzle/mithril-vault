use crate::dto::database::CustomIconData;
use crate::dto::error::AppError;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use keepass::db::{Icon, Times};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use uuid::Uuid;

use super::mapping::find_entry_id;
use super::KdbxService;

impl KdbxService {
    /// Returns custom icons for the database, keyed by UUID.
    pub fn get_custom_icons(
        &self,
        db_id: &str,
    ) -> Result<HashMap<String, CustomIconData>, AppError> {
        let normalized_path = Self::normalize_path(db_id);
        let databases = self.lock_databases()?;
        let open_db = databases
            .get(&normalized_path)
            .ok_or_else(|| AppError::DatabaseNotFound(db_id.to_string()))?;

        let db = open_db.db_or_locked()?;
        let mut icons = HashMap::new();
        for icon in db.iter_all_custom_icons() {
            icons.insert(
                icon.id().uuid().to_string(),
                CustomIconData {
                    mime_type: detect_icon_mime(&icon.data),
                    data: STANDARD.encode(&icon.data),
                },
            );
        }

        Ok(icons)
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

        let icon_cid = db
            .iter_all_custom_icons()
            .find(|icon| icon.id().uuid() == parsed_uuid)
            .map(|icon| icon.id());
        let Some(icon_cid) = icon_cid else {
            return Err(AppError::InvalidInput(format!(
                "custom icon {icon_uuid} not found in database"
            )));
        };

        let eid = find_entry_id(db, entry_id)
            .ok_or_else(|| AppError::EntryNotFound(entry_id.to_string()))?;
        let mut entry = db
            .entry_mut(eid)
            .ok_or_else(|| AppError::EntryNotFound(entry_id.to_string()))?;

        if matches!(entry.icon(), Some(Icon::Custom(cid)) if *cid == icon_cid) {
            return Ok(false);
        }

        entry
            .set_icon_custom(icon_cid)
            .map_err(|e| AppError::Kdbx(e.to_string()))?;
        entry.times.last_modification = Some(Times::now());
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

        let eid = find_entry_id(db, entry_id)
            .ok_or_else(|| AppError::EntryNotFound(entry_id.to_string()))?;
        let mut entry = db
            .entry_mut(eid)
            .ok_or_else(|| AppError::EntryNotFound(entry_id.to_string()))?;

        if !matches!(entry.icon(), Some(Icon::Custom(_))) {
            return Ok(false);
        }

        entry.set_icon_none();
        entry.times.last_modification = Some(Times::now());
        open_db.is_modified = true;
        Ok(true)
    }

    /// Writes `icon_bytes` to the Vault as a Custom Icon and links it to the
    /// Entry. Dedupes by content hash so multiple Entries pointing at the same
    /// image share one stored icon. When `force` is false and the Entry
    /// already carries a Custom Icon, the existing icon is preserved.
    pub(crate) fn assign_entry_custom_icon(
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

        let eid = find_entry_id(db, entry_id)
            .ok_or_else(|| AppError::EntryNotFound(entry_id.to_string()))?;

        let already_has = matches!(
            db.entry(eid).and_then(|e| e.icon().cloned()),
            Some(Icon::Custom(_))
        );

        if !force && already_has {
            return Ok(false);
        }

        let target_hash = hash_bytes(icon_bytes);
        let existing_cid = db
            .iter_all_custom_icons()
            .find(|icon| hash_bytes(&icon.data) == target_hash)
            .map(|icon| icon.id());

        let mut entry = db
            .entry_mut(eid)
            .ok_or_else(|| AppError::EntryNotFound(entry_id.to_string()))?;

        match existing_cid {
            Some(cid) => {
                if matches!(entry.icon(), Some(Icon::Custom(current)) if *current == cid) {
                    return Ok(false);
                }
                entry
                    .set_icon_custom(cid)
                    .map_err(|e| AppError::Kdbx(e.to_string()))?;
            }
            None => {
                entry.set_icon_custom_new(icon_bytes.to_vec());
            }
        }

        entry.times.last_modification = Some(Times::now());
        open_db.is_modified = true;
        Ok(true)
    }
}

/// Best-effort MIME inference for Custom Icon bytes. Used both when
/// serializing stored icons over IPC and when a Favicon fetch lacks a
/// trustworthy Content-Type header.
pub fn detect_icon_mime(bytes: &[u8]) -> String {
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

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::domain::secure::SecureString;
    use crate::dto::database::DatabaseCreationOptions;
    use crate::dto::entry::CreateEntryData;
    use tempfile::TempDir;

    fn create_test_database() -> (KdbxService, TempDir, String, String, String) {
        let dir = tempfile::tempdir().expect("create temp dir");
        let db_path = dir.path().join("custom-icon-tests.kdbx");
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
                "Custom Icon Tests",
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
            let uuid = db
                .iter_all_custom_icons()
                .next()
                .expect("custom icon exists")
                .id()
                .uuid()
                .to_string();
            uuid
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
    fn entry_with_custom_icon_reports_no_builtin_icon_id() {
        // keepass 0.12 made builtin and custom icons mutually exclusive on
        // Entry. convert_entry must reflect that honestly: when a custom
        // icon is set, iconId is null and customIconUuid carries the UUID.
        // The previous "always emit iconId=0" workaround caused the
        // frontend to echo iconId=0 back on the next save and silently
        // destroy the custom icon — see
        // update_entry_without_icon_id_preserves_custom_icon.
        let (service, _dir, db_path, entry_a, _entry_b) = create_test_database();
        let icon_bytes = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 7];

        service
            .assign_entry_custom_icon(&db_path, &entry_a, &icon_bytes, "image/png", true)
            .expect("assign favicon");

        let entries = service.list_entries(&db_path, None).expect("list entries");
        let entry = entries
            .iter()
            .find(|e| e.id == entry_a)
            .expect("entry returned by list_entries");
        assert!(
            entry.icon_id.is_none(),
            "iconId must be None when the entry has only a custom icon"
        );
        assert!(
            entry.custom_icon_uuid.is_some(),
            "customIconUuid must round-trip"
        );
    }

    #[test]
    fn update_entry_without_icon_id_preserves_custom_icon() {
        // Regression: editing a non-icon field on a favicon-bearing entry
        // used to destroy the favicon because the frontend echoed back
        // iconId=0 and set_icon_builtin(0) cleared Icon::Custom. Sending
        // icon_id: None must leave the entry's icon untouched.
        let (service, _dir, db_path, entry_a, _entry_b) = create_test_database();
        let icon_bytes = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 9];
        service
            .assign_entry_custom_icon(&db_path, &entry_a, &icon_bytes, "image/png", true)
            .expect("assign favicon");

        service
            .update_entry(
                &db_path,
                &entry_a,
                crate::dto::entry::UpdateEntryData {
                    title: Some("Renamed".to_string()),
                    username: None,
                    password: None,
                    url: None,
                    notes: None,
                    icon_id: None,
                    tags: None,
                    custom_fields: None,
                    protected_custom_fields: None,
                },
            )
            .expect("update title");

        let entry = service.get_entry(&db_path, &entry_a).expect("get entry");
        assert!(
            entry.custom_icon_uuid.is_some(),
            "favicon must survive an unrelated field update"
        );
        assert!(
            entry.icon_id.is_none(),
            "the entry has no builtin icon and the update didn't touch one"
        );
    }

    #[test]
    fn update_entry_with_builtin_zero_switches_off_custom_icon() {
        // Picker-to-Key (icon 0) path on a custom-icon entry: the frontend
        // sends iconId=Some(0) (dirty) and the customIconUuid clear is
        // applied separately. The backend must end in Icon::BuiltIn(0).
        let (service, _dir, db_path, entry_a, _entry_b) = create_test_database();
        let icon_bytes = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 10];
        service
            .assign_entry_custom_icon(&db_path, &entry_a, &icon_bytes, "image/png", true)
            .expect("assign favicon");

        service
            .update_entry(
                &db_path,
                &entry_a,
                crate::dto::entry::UpdateEntryData {
                    title: None,
                    username: None,
                    password: None,
                    url: None,
                    notes: None,
                    icon_id: Some(0),
                    tags: None,
                    custom_fields: None,
                    protected_custom_fields: None,
                },
            )
            .expect("update icon to builtin 0");

        let entry = service.get_entry(&db_path, &entry_a).expect("get entry");
        assert_eq!(entry.icon_id, Some(0));
        assert!(
            entry.custom_icon_uuid.is_none(),
            "switching to builtin 0 must drop the custom icon"
        );
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
        assert_eq!(db.iter_all_custom_icons().count(), 1);

        let icon_for = |entry_id: &str| -> uuid::Uuid {
            let eid = find_entry_id(db, entry_id).expect("entry id");
            let entry = db.entry(eid).expect("entry ref");
            match entry.icon() {
                Some(Icon::Custom(cid)) => cid.uuid(),
                other => unreachable!("entry {entry_id} has no custom icon: {other:?}"),
            }
        };
        assert_eq!(icon_for(&entry_a), icon_for(&entry_b));
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
        let eid = find_entry_id(db, &entry_a).expect("entry id");
        let entry = db.entry(eid).expect("entry ref");
        assert!(!matches!(entry.icon(), Some(Icon::Custom(_))));
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
        let mut icons: Vec<Vec<u8>> = db
            .iter_all_custom_icons()
            .map(|icon| icon.data.clone())
            .collect();
        assert_eq!(icons.len(), 1);
        assert_eq!(icons.pop().expect("icon data"), first_icon);
    }
}
