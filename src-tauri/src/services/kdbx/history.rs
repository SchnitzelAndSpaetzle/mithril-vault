// SPDX-License-Identifier: MIT
//! Entry-History retention: resolving the per-Vault `History Limit`
//! (`Meta.history_max_items`) into a retention policy and applying it at the
//! snapshot chokepoint (ADR-0008).

use crate::dto::database::VaultHistorySettings;
use crate::dto::error::AppError;

use super::KdbxService;

/// The per-Vault Entry-History retention, resolved from the raw KDBX
/// `Meta.history_max_items` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HistoryRetention {
    /// No new snapshots; existing history is pruned to zero lazily on the
    /// Entry's next edit (raw value `0`).
    Disabled,
    /// History grows without bound (raw value negative).
    Unlimited,
    /// Keep the newest `n` versions, pruning the oldest on append (raw value
    /// positive `n`, or the default when the field is absent).
    Limited(usize),
}

/// The retention applied when `Meta.history_max_items` is absent. Newly created
/// Vaults start from `Meta::default()` and never set the field, so absence must
/// resolve to a bounded default — *not* unbounded history (ADR-0008).
pub(crate) const DEFAULT_HISTORY_MAX_ITEMS: usize = 10;

/// Resolves the raw `Meta.history_max_items` into a [`HistoryRetention`]:
/// absent → default 10, negative → unlimited, `0` → disabled, positive `n` →
/// keep newest `n`.
pub(crate) fn resolve_history_retention(max_items: Option<isize>) -> HistoryRetention {
    match max_items {
        None => HistoryRetention::Limited(DEFAULT_HISTORY_MAX_ITEMS),
        Some(n) if n < 0 => HistoryRetention::Unlimited,
        Some(0) => HistoryRetention::Disabled,
        // `n > 0`: clamp the (practically tiny) value into `usize`.
        Some(n) => HistoryRetention::Limited(usize::try_from(n).unwrap_or(usize::MAX)),
    }
}

/// Narrows the stored `isize` field into the `i32` carried over IPC. The KDBX
/// format writes this field as a 32-bit int, so real values always fit; an
/// out-of-range value (never produced by this app) saturates rather than wraps.
fn isize_to_i32(value: isize) -> i32 {
    i32::try_from(value).unwrap_or(if value < 0 { i32::MIN } else { i32::MAX })
}

impl KdbxService {
    /// Reads the per-Vault Entry-History retention from KDBX `Meta` — the
    /// writable vault-meta surface, distinct from the read-only Database Config
    /// crypto snapshot (ADR-0008). `max_items` is the raw stored value (`None`
    /// when the field is absent), faithfully round-tripped for the UI to map.
    pub fn get_vault_history_settings(
        &self,
        db_id: &str,
    ) -> Result<VaultHistorySettings, AppError> {
        self.with_vault(db_id, |vault| {
            Ok(VaultHistorySettings {
                max_items: vault.db().meta.history_max_items.map(isize_to_i32),
            })
        })
    }

    /// Writes the per-Vault `History Limit` into KDBX `Meta.history_max_items`
    /// and marks the Vault modified so the change persists on next save. Passing
    /// `None` clears the field back to absent (effective default 10).
    /// `Meta.history_max_size` is left untouched (preserved, not enforced).
    pub fn update_vault_history_settings(
        &self,
        db_id: &str,
        max_items: Option<i32>,
    ) -> Result<(), AppError> {
        self.with_vault_mut(db_id, |vault| {
            vault.db_mut().meta.history_max_items = max_items.map(|v| v as isize);
            vault.mark_modified();
            Ok(())
        })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::services::kdbx::test_support::create_test_database;

    #[test]
    fn history_settings_default_to_absent_on_a_new_vault() {
        let (service, _dir, db_path, _a, _b) = create_test_database();
        let settings = service
            .get_vault_history_settings(&db_path)
            .expect("read history settings");
        assert_eq!(
            settings.max_items, None,
            "a brand-new Vault never sets the field"
        );
    }

    #[test]
    fn history_settings_round_trip_through_save_and_reopen() {
        let (service, _dir, db_path, _a, _b) = create_test_database();

        service
            .update_vault_history_settings(&db_path, Some(25))
            .expect("write history settings");

        // Visible immediately on the open Vault.
        assert_eq!(
            service
                .get_vault_history_settings(&db_path)
                .expect("read back")
                .max_items,
            Some(25)
        );

        // And it travels with the file: persist, close, reopen fresh.
        service.save(&db_path).expect("save vault");
        service.close(&db_path).expect("close vault");

        let reopened = KdbxService::new();
        reopened.open(&db_path, "testpass").expect("reopen vault");
        assert_eq!(
            reopened
                .get_vault_history_settings(&db_path)
                .expect("read after reopen")
                .max_items,
            Some(25),
            "history_max_items must persist to disk and read back"
        );
    }

    #[test]
    fn updating_history_settings_leaves_the_byte_cap_untouched() {
        // `Meta.history_max_size` is preserved on save but not enforced (v1):
        // writing the items limit must not disturb it (ADR-0008).
        let (service, _dir, db_path, _a, _b) = create_test_database();
        service
            .with_vault_mut(&db_path, |vault| {
                vault.db_mut().meta.history_max_size = Some(6 * 1024 * 1024);
                vault.mark_modified();
                Ok(())
            })
            .expect("seed byte cap");

        service
            .update_vault_history_settings(&db_path, Some(5))
            .expect("write items limit");
        service.save(&db_path).expect("save vault");
        service.close(&db_path).expect("close vault");

        let reopened = KdbxService::new();
        reopened.open(&db_path, "testpass").expect("reopen vault");
        let preserved = reopened
            .with_vault(&db_path, |vault| Ok(vault.db().meta.history_max_size))
            .expect("read byte cap");
        assert_eq!(
            preserved,
            Some(6 * 1024 * 1024),
            "history_max_size is preserved untouched"
        );
    }

    #[test]
    fn absent_resolves_to_the_bounded_default() {
        // A brand-new Vault never sets the field; absence must cap history at
        // the default of 10, not leave it unbounded (ADR-0008).
        assert_eq!(
            resolve_history_retention(None),
            HistoryRetention::Limited(DEFAULT_HISTORY_MAX_ITEMS)
        );
    }

    #[test]
    fn negative_resolves_to_unlimited() {
        assert_eq!(
            resolve_history_retention(Some(-1)),
            HistoryRetention::Unlimited
        );
    }

    #[test]
    fn zero_resolves_to_disabled() {
        assert_eq!(
            resolve_history_retention(Some(0)),
            HistoryRetention::Disabled
        );
    }

    #[test]
    fn positive_resolves_to_keep_newest_n() {
        assert_eq!(
            resolve_history_retention(Some(25)),
            HistoryRetention::Limited(25)
        );
    }
}
