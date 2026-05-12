// SPDX-License-Identifier: MIT

use crate::services::kdbx::KdbxService;
use crate::services::settings::SettingsService;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Emitter, Manager, Runtime};

const CHECK_INTERVAL_SECS: u64 = 15;

pub struct AutoLockService {
    last_activity: Arc<AtomicU64>,
}

impl AutoLockService {
    pub fn new() -> Self {
        Self {
            last_activity: Arc::new(AtomicU64::new(current_epoch_secs())),
        }
    }

    pub fn report_activity(&self) {
        self.last_activity
            .store(current_epoch_secs(), Ordering::SeqCst);
    }

    pub fn seconds_since_activity(&self) -> u64 {
        current_epoch_secs().saturating_sub(self.last_activity.load(Ordering::SeqCst))
    }
}

impl Default for AutoLockService {
    fn default() -> Self {
        Self::new()
    }
}

fn current_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

pub fn start_auto_lock_task<R: Runtime>(app_handle: &tauri::AppHandle<R>) {
    let handle = app_handle.clone();

    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(CHECK_INTERVAL_SECS)).await;

            let Some(auto_lock) = handle.try_state::<Arc<AutoLockService>>() else {
                continue;
            };
            let Some(settings) = handle.try_state::<Arc<SettingsService>>() else {
                continue;
            };
            let Some(kdbx) = handle.try_state::<Arc<KdbxService>>() else {
                continue;
            };

            let timeout = settings
                .get_settings()
                .map_or(300, |s| s.preferences.security.auto_lock_timeout);

            // 0 means disabled
            if timeout == 0 {
                continue;
            }

            let elapsed = auto_lock.seconds_since_activity();
            if elapsed >= u64::from(timeout) {
                if let Ok(locked_paths) = kdbx.lock_all() {
                    if !locked_paths.is_empty() {
                        let _ = handle.emit("database-locked", &locked_paths);
                        // Reset activity to prevent repeated firing
                        auto_lock.report_activity();
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_sets_initial_activity() {
        let service = AutoLockService::new();
        let now = current_epoch_secs();
        let last = service.last_activity.load(Ordering::SeqCst);
        assert!(now.abs_diff(last) <= 1);
    }

    #[test]
    fn test_report_activity_updates_timestamp() {
        let service = AutoLockService::new();
        // Manually set to old value
        service.last_activity.store(1000, Ordering::SeqCst);
        service.report_activity();
        let last = service.last_activity.load(Ordering::SeqCst);
        let now = current_epoch_secs();
        assert!(now.abs_diff(last) <= 1);
    }

    #[test]
    fn test_seconds_since_activity_increases() {
        let service = AutoLockService::new();
        // Set activity to 10 seconds ago
        let past = current_epoch_secs() - 10;
        service.last_activity.store(past, Ordering::SeqCst);
        let elapsed = service.seconds_since_activity();
        assert!(elapsed >= 10);
        assert!(elapsed <= 11);
    }

    #[test]
    fn test_default_creates_service() {
        let service = AutoLockService::default();
        let now = current_epoch_secs();
        let last = service.last_activity.load(Ordering::SeqCst);
        assert!(now.abs_diff(last) <= 1);
    }
}
