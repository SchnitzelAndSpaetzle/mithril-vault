// SPDX-License-Identifier: MIT

use crate::dto::error::AppError;
use rand::rngs::OsRng;
use rand::RngCore;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use tauri::AppHandle;
use tauri::Manager;
use tauri::Runtime;
use tauri_plugin_stronghold::stronghold::Stronghold;
use zeroize::Zeroizing;

const SESSION_CLIENT: &[u8] = b"mithrilvault-session";
const SESSION_RECORD_KEY: &[u8] = b"session-key";
const SESSION_SNAPSHOT_FILE: &str = "session.hold";
const DEFAULT_SESSION_TTL_SECS: u64 = 300;

pub struct SecureStorageService {
    stronghold: Mutex<Stronghold>,
    snapshot_path: PathBuf,
}

impl SecureStorageService {
    /// Creates secure storage rooted in app data.
    pub fn new<R: Runtime>(app: &AppHandle<R>) -> Result<Self, AppError> {
        let data_dir = app
            .path()
            .app_local_data_dir()
            .map_err(|err| AppError::SecureStorage(err.to_string()))?;
        std::fs::create_dir_all(&data_dir)?;

        let snapshot_path = data_dir.join(SESSION_SNAPSHOT_FILE);
        Self::new_with_path(snapshot_path)
    }

    fn new_with_path(snapshot_path: PathBuf) -> Result<Self, AppError> {
        if snapshot_path.exists() {
            std::fs::remove_file(&snapshot_path)?;
        }

        // Use Zeroizing<Vec<u8>> to ensure our copy of the key is cleared from memory.
        // Stronghold takes ownership of a clone; our Zeroizing wrapper ensures our copy is zeroized.
        let mut key = Zeroizing::new(vec![0u8; 32]);
        OsRng.fill_bytes(&mut key);

        let stronghold = Stronghold::new(snapshot_path.clone(), (*key).clone())
            .map_err(|err| AppError::SecureStorage(err.to_string()))?;

        Ok(Self {
            stronghold: Mutex::new(stronghold),
            snapshot_path,
        })
    }

    /// Stores a session key with a TTL.
    pub fn store_session_key(&self, key: &[u8], ttl: Duration) -> Result<(), AppError> {
        let stronghold = self.stronghold.lock().map_err(|_| AppError::Lock)?;
        let client = Self::get_or_create_client(&stronghold)?;

        client
            .store()
            .insert(SESSION_RECORD_KEY.to_vec(), key.to_vec(), Some(ttl))
            .map_err(|err| AppError::SecureStorage(err.to_string()))?;

        stronghold
            .save()
            .map_err(|err| AppError::SecureStorage(err.to_string()))?;

        Ok(())
    }

    /// Checks if a session key is stored.
    pub fn session_key_present(&self) -> Result<bool, AppError> {
        let stronghold = self.stronghold.lock().map_err(|_| AppError::Lock)?;
        let client = Self::get_or_create_client(&stronghold)?;

        let record = client
            .store()
            .get(SESSION_RECORD_KEY)
            .map_err(|err| AppError::SecureStorage(err.to_string()))?;

        Ok(record.is_some())
    }

    /// Loads the session key if present.
    pub fn load_session_key(&self) -> Result<Option<Vec<u8>>, AppError> {
        let stronghold = self.stronghold.lock().map_err(|_| AppError::Lock)?;
        let client = Self::get_or_create_client(&stronghold)?;

        client
            .store()
            .get(SESSION_RECORD_KEY)
            .map_err(|err| AppError::SecureStorage(err.to_string()))
    }

    /// Clears the stored session key.
    pub fn clear_session_key(&self) -> Result<(), AppError> {
        let stronghold = self.stronghold.lock().map_err(|_| AppError::Lock)?;
        let client = Self::get_or_create_client(&stronghold)?;

        let _ = client
            .store()
            .delete(SESSION_RECORD_KEY)
            .map_err(|err| AppError::SecureStorage(err.to_string()))?;

        stronghold
            .save()
            .map_err(|err| AppError::SecureStorage(err.to_string()))?;

        if self.snapshot_path.exists() {
            std::fs::remove_file(&self.snapshot_path)?;
        }

        Ok(())
    }

    /// Returns the default session TTL.
    pub fn default_session_ttl() -> Duration {
        Duration::from_secs(DEFAULT_SESSION_TTL_SECS)
    }

    fn get_or_create_client(stronghold: &Stronghold) -> Result<iota_stronghold::Client, AppError> {
        // First try to get from in-memory state
        if let Ok(client) = stronghold.get_client(SESSION_CLIENT) {
            return Ok(client);
        }

        // Not in memory, try to load from snapshot
        if stronghold.load_client(SESSION_CLIENT).is_ok() {
            if let Ok(client) = stronghold.get_client(SESSION_CLIENT) {
                return Ok(client);
            }
        }

        // Still not found, create new client
        stronghold
            .create_client(SESSION_CLIENT)
            .map_err(|err| AppError::SecureStorage(err.to_string()))?;

        stronghold
            .get_client(SESSION_CLIENT)
            .map_err(|err| AppError::SecureStorage(err.to_string()))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_service() -> (SecureStorageService, tempfile::TempDir) {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let snapshot_path = temp_dir.path().join("test_session.hold");
        let service =
            SecureStorageService::new_with_path(snapshot_path).expect("Failed to create service");
        (service, temp_dir)
    }

    #[test]
    fn test_store_and_load_session_key() {
        let (service, _temp_dir) = create_test_service();
        let test_key = b"test_session_key_12345";

        // Initially, no session key should be present
        assert!(!service.session_key_present().expect("presence check"));
        assert!(service.load_session_key().expect("load").is_none());

        // Store a session key
        service
            .store_session_key(test_key, Duration::from_secs(3600))
            .expect("store");

        // Verify it's present and can be loaded
        assert!(service.session_key_present().expect("presence check"));
        let loaded = service.load_session_key().expect("load");
        assert_eq!(loaded, Some(test_key.to_vec()));
    }

    #[test]
    fn test_clear_session_key() {
        let (service, _temp_dir) = create_test_service();
        let test_key = b"test_session_key";

        // Store and then clear (use long TTL to avoid timing issues in CI)
        service
            .store_session_key(test_key, Duration::from_secs(3600))
            .expect("store");
        assert!(service.session_key_present().expect("presence check"));

        service.clear_session_key().expect("clear");

        // Verify it's cleared
        assert!(!service.session_key_present().expect("presence check"));
        assert!(service.load_session_key().expect("load").is_none());
    }

    #[test]
    fn test_overwrite_session_key() {
        let (service, _temp_dir) = create_test_service();
        let key1 = b"first_key";
        let key2 = b"second_key";

        service
            .store_session_key(key1, Duration::from_secs(3600))
            .expect("store first");
        service
            .store_session_key(key2, Duration::from_secs(3600))
            .expect("store second");

        let loaded = service.load_session_key().expect("load");
        assert_eq!(loaded, Some(key2.to_vec()));
    }

    #[test]
    fn test_default_session_ttl() {
        let ttl = SecureStorageService::default_session_ttl();
        assert_eq!(ttl, Duration::from_secs(300));
    }

    // Note: TTL expiration testing is omitted because Stronghold's TTL handling
    // behaves unreliably with short TTLs in CI environments (release mode, Ubuntu).
    // The TTL functionality is provided by Stronghold; our code simply passes
    // the TTL value through to the library.
}
