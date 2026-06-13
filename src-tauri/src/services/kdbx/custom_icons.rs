use crate::dto::database::CustomIconData;
use crate::dto::error::AppError;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use keepass::db::{Icon, Times};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use uuid::Uuid;

use super::entries::snapshot_entry_history;
use super::KdbxService;

impl KdbxService {
    /// Returns custom icons for the database, keyed by UUID.
    pub fn get_custom_icons(
        &self,
        db_id: &str,
    ) -> Result<HashMap<String, CustomIconData>, AppError> {
        self.with_vault(db_id, |vault| {
            let mut icons = HashMap::new();
            for icon in vault.db().iter_all_custom_icons() {
                icons.insert(
                    icon.id().uuid().to_string(),
                    CustomIconData {
                        mime_type: detect_icon_mime(&icon.data),
                        data: STANDARD.encode(&icon.data),
                    },
                );
            }
            Ok(icons)
        })
    }

    pub fn set_entry_custom_icon(
        &self,
        db_id: &str,
        entry_id: &str,
        icon_uuid: &str,
    ) -> Result<bool, AppError> {
        let parsed_uuid = Uuid::parse_str(icon_uuid)
            .map_err(|error| AppError::InvalidInput(error.to_string()))?;

        self.with_vault_mut(db_id, |vault| {
            let icon_cid = vault
                .db()
                .iter_all_custom_icons()
                .find(|icon| icon.id().uuid() == parsed_uuid)
                .map(|icon| icon.id());
            let Some(icon_cid) = icon_cid else {
                return Err(AppError::InvalidInput(format!(
                    "custom icon {icon_uuid} not found in database"
                )));
            };

            let changed = {
                let mut entry = vault.entry_mut(entry_id)?;
                if matches!(entry.icon(), Some(Icon::Custom(cid)) if *cid == icon_cid) {
                    false
                } else {
                    // Snapshot the prior icon state before swapping (#323).
                    let before = (*entry.as_ref()).clone();
                    entry
                        .set_icon_custom(icon_cid)
                        .map_err(|e| AppError::Kdbx(e.to_string()))?;
                    entry.times.last_modification = Some(Times::now());
                    snapshot_entry_history(&mut entry, before);
                    true
                }
            };

            if changed {
                vault.mark_modified();
            }
            Ok(changed)
        })
    }

    pub fn clear_entry_custom_icon(&self, db_id: &str, entry_id: &str) -> Result<bool, AppError> {
        self.with_vault_mut(db_id, |vault| {
            let changed = {
                let mut entry = vault.entry_mut(entry_id)?;
                if matches!(entry.icon(), Some(Icon::Custom(_))) {
                    // Snapshot the prior (icon-bearing) state before removing it (#323).
                    let before = (*entry.as_ref()).clone();
                    entry.set_icon_none();
                    entry.times.last_modification = Some(Times::now());
                    snapshot_entry_history(&mut entry, before);
                    true
                } else {
                    false
                }
            };

            if changed {
                vault.mark_modified();
            }
            Ok(changed)
        })
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
        self.with_vault_mut(db_id, |vault| {
            let eid = vault.find_entry_id(entry_id)?;

            let already_has = matches!(
                vault.db().entry(eid).and_then(|e| e.icon().cloned()),
                Some(Icon::Custom(_))
            );

            if !force && already_has {
                return Ok(false);
            }

            let target_hash = hash_bytes(icon_bytes);
            let existing_cid = vault
                .db()
                .iter_all_custom_icons()
                .find(|icon| hash_bytes(&icon.data) == target_hash)
                .map(|icon| icon.id());

            let changed = {
                let mut entry = vault
                    .db_mut()
                    .entry_mut(eid)
                    .ok_or_else(|| AppError::EntryNotFound(entry_id.to_string()))?;

                // Snapshot the prior icon state before any swap (#323). Cloned
                // up front, kept only when the icon actually changes below.
                let before = (*entry.as_ref()).clone();

                match existing_cid {
                    Some(cid)
                        if matches!(entry.icon(), Some(Icon::Custom(current)) if *current == cid) =>
                    {
                        false
                    }
                    Some(cid) => {
                        entry
                            .set_icon_custom(cid)
                            .map_err(|e| AppError::Kdbx(e.to_string()))?;
                        entry.times.last_modification = Some(Times::now());
                        snapshot_entry_history(&mut entry, before);
                        true
                    }
                    None => {
                        entry.set_icon_custom_new(icon_bytes.to_vec());
                        entry.times.last_modification = Some(Times::now());
                        snapshot_entry_history(&mut entry, before);
                        true
                    }
                }
            };

            if changed {
                vault.mark_modified();
            }
            Ok(changed)
        })
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
    use crate::services::kdbx::test_support::create_test_database;

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

        let icon_uuid = service
            .with_vault(&db_path, |vault| {
                Ok(vault
                    .db()
                    .iter_all_custom_icons()
                    .next()
                    .expect("custom icon exists")
                    .id()
                    .uuid()
                    .to_string())
            })
            .expect("vault scope");

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
                    expires: None,
                    expiry_time: None,
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
                    expires: None,
                    expiry_time: None,
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

        service
            .with_vault(&db_path, |vault| {
                assert_eq!(vault.db().iter_all_custom_icons().count(), 1);

                let icon_for = |entry_id: &str| -> uuid::Uuid {
                    let entry = vault.find_entry(entry_id).expect("entry ref");
                    match entry.icon() {
                        Some(Icon::Custom(cid)) => cid.uuid(),
                        other => unreachable!("entry {entry_id} has no custom icon: {other:?}"),
                    }
                };
                assert_eq!(icon_for(&entry_a), icon_for(&entry_b));
                Ok(())
            })
            .expect("vault scope");
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

        service
            .with_vault(&db_path, |vault| {
                let entry = vault.find_entry(&entry_a).expect("entry ref");
                assert!(!matches!(entry.icon(), Some(Icon::Custom(_))));
                Ok(())
            })
            .expect("vault scope");
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

        service
            .with_vault(&db_path, |vault| {
                let mut icons: Vec<Vec<u8>> = vault
                    .db()
                    .iter_all_custom_icons()
                    .map(|icon| icon.data.clone())
                    .collect();
                assert_eq!(icons.len(), 1);
                assert_eq!(icons.pop().expect("icon data"), first_icon);
                Ok(())
            })
            .expect("vault scope");
    }

    #[test]
    fn assigning_a_custom_icon_snapshots_the_prior_state() {
        // Setting an Entry's Custom Icon / Favicon is a content change, so the
        // chokepoint captures the prior (iconless) state first (#323).
        let (service, _dir, db_path, entry_a, _entry_b) = create_test_database();
        let icon_bytes = [0x89, b'P', b'N', b'G', 7];

        assert!(service
            .assign_entry_custom_icon(&db_path, &entry_a, &icon_bytes, "image/png", true)
            .expect("assign icon"));

        let history = service
            .list_entry_history(&db_path, &entry_a)
            .expect("list history after icon assign");
        assert_eq!(history.len(), 1, "assigning an icon captures one version");

        service
            .with_vault(&db_path, |vault| {
                let entry = vault.find_entry(&entry_a).expect("entry ref");
                assert!(
                    !matches!(
                        entry.historical(0).expect("version").icon(),
                        Some(Icon::Custom(_))
                    ),
                    "the snapshot predates the icon, so it carries no custom icon"
                );
                Ok(())
            })
            .expect("vault scope");
    }

    #[test]
    fn setting_an_existing_custom_icon_snapshots_the_prior_state() {
        // Pointing an Entry at an already-stored Custom Icon also snapshots.
        let (service, _dir, db_path, entry_a, entry_b) = create_test_database();
        let icon_bytes = [0x89, b'P', b'N', b'G', 8];
        service
            .assign_entry_custom_icon(&db_path, &entry_a, &icon_bytes, "image/png", true)
            .expect("seed icon in pool");
        let icon_uuid = service
            .with_vault(&db_path, |vault| {
                Ok(vault
                    .db()
                    .iter_all_custom_icons()
                    .next()
                    .expect("custom icon exists")
                    .id()
                    .uuid()
                    .to_string())
            })
            .expect("vault scope");

        assert!(service
            .set_entry_custom_icon(&db_path, &entry_b, &icon_uuid)
            .expect("set icon on b"));

        let history = service
            .list_entry_history(&db_path, &entry_b)
            .expect("list history after set");
        assert_eq!(history.len(), 1, "setting an icon captures one version");
    }

    #[test]
    fn clearing_a_custom_icon_snapshots_the_prior_state() {
        // Removing a Custom Icon is a content change too: the prior state (with
        // the icon) must be recoverable.
        let (service, _dir, db_path, entry_a, _entry_b) = create_test_database();
        let icon_bytes = [0x89, b'P', b'N', b'G', 9];
        service
            .assign_entry_custom_icon(&db_path, &entry_a, &icon_bytes, "image/png", true)
            .expect("assign icon");
        let before = service
            .list_entry_history(&db_path, &entry_a)
            .expect("history after assign")
            .len();

        assert!(service
            .clear_entry_custom_icon(&db_path, &entry_a)
            .expect("clear icon"));

        let after = service
            .list_entry_history(&db_path, &entry_a)
            .expect("history after clear")
            .len();
        assert_eq!(after, before + 1, "clearing the icon adds one version");
    }

    #[test]
    fn a_no_op_icon_set_snapshots_nothing() {
        // Re-pointing an Entry at the icon it already has changes nothing, so it
        // must not accrue a history version.
        let (service, _dir, db_path, entry_a, _entry_b) = create_test_database();
        let icon_bytes = [0x89, b'P', b'N', b'G', 10];
        service
            .assign_entry_custom_icon(&db_path, &entry_a, &icon_bytes, "image/png", true)
            .expect("assign icon");
        let icon_uuid = service
            .with_vault(&db_path, |vault| {
                Ok(vault
                    .db()
                    .iter_all_custom_icons()
                    .next()
                    .expect("custom icon exists")
                    .id()
                    .uuid()
                    .to_string())
            })
            .expect("vault scope");
        let before = service
            .list_entry_history(&db_path, &entry_a)
            .expect("history after assign")
            .len();

        assert!(
            !service
                .set_entry_custom_icon(&db_path, &entry_a, &icon_uuid)
                .expect("re-set same icon"),
            "re-setting the same icon reports no change"
        );

        let after = service
            .list_entry_history(&db_path, &entry_a)
            .expect("history after no-op set")
            .len();
        assert_eq!(after, before, "a no-op icon set accrues no version");
    }
}
