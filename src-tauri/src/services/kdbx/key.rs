use crate::dto::error::AppError;
use keepass::DatabaseKey;
use std::fs::File;

pub fn build_database_key(
    password: Option<&str>,
    keyfile_path: Option<&str>,
) -> Result<DatabaseKey, AppError> {
    let mut key = DatabaseKey::new();

    if let Some(pw) = password {
        key = key.with_password(pw);
    }

    if let Some(kf_path) = keyfile_path {
        let mut keyfile = File::open(kf_path)
            .map_err(|e| AppError::InvalidPath(format!("Keyfile not found: {e}")))?;
        key = key
            .with_keyfile(&mut keyfile)
            .map_err(|e| AppError::Kdbx(e.to_string()))?;
    }

    Ok(key)
}
