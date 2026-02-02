// SPDX-License-Identifier: MIT

use crate::dto::error::AppError;
use rand::rngs::OsRng;
use rand::RngCore;
use std::fmt::Write as FmtWrite;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Formats 32 bytes as hex with spaces (8 chars per group, 4 groups per line).
/// This matches the `KeePass` 2.x keyfile format.
fn format_hex_key(bytes: &[u8; 32]) -> String {
    let mut result = String::with_capacity(80);

    for (i, chunk) in bytes.chunks(4).enumerate() {
        // Add hex for this 4-byte chunk (8 hex chars)
        for byte in chunk {
            let _ = write!(result, "{byte:02X}");
        }

        // Add space or newline after each group
        if i == 3 {
            // After 4th group (end of first line), add newline and indent
            result.push_str("\n            ");
        } else if i < 7 {
            // Add space between groups (except at the very end)
            result.push(' ');
        }
    }

    result
}

/// Generates a `KeePass` 2.x compatible keyfile in XML format (.keyx).
///
/// The keyfile contains:
/// - 32 bytes of cryptographically random data (256 bits)
/// - A hash attribute for integrity verification (first 4 bytes of key as hex)
/// - XML structure compatible with `KeePass` 2.x
///
/// # Parameters
/// - `output_path`: Path where the keyfile will be written
///
/// # Returns
/// - `Ok(())` on success
/// - `Err(AppError)` if file creation fails or the path is invalid
pub fn generate_keyfile(output_path: &str) -> Result<(), AppError> {
    let path = Path::new(output_path);

    // Validate parent directory exists
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            return Err(AppError::InvalidPath(format!(
                "Parent directory does not exist: {}",
                parent.display()
            )));
        }
    }

    // Generate 32 random bytes (256 bits)
    let mut key_bytes = [0u8; 32];
    OsRng.fill_bytes(&mut key_bytes);

    // Format as hex with spaces (8 chars per group, 4 groups per line)
    let hex_formatted = format_hex_key(&key_bytes);

    // Calculate hash (first 4 bytes of key as hex, uppercase)
    let hash = format!(
        "{:02X}{:02X}{:02X}{:02X}",
        key_bytes[0], key_bytes[1], key_bytes[2], key_bytes[3]
    );

    // Build XML structure matching KeePass 2.x format
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<KeyFile>
    <Meta>
        <Version>2.0</Version>
    </Meta>
    <Key>
        <Data Hash="{hash}">
            {hex_formatted}
        </Data>
    </Key>
</KeyFile>
"#
    );

    // Write to file
    let mut file = File::create(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            AppError::InvalidPath(format!("Permission denied: {output_path}"))
        } else {
            AppError::Io(e.to_string())
        }
    })?;

    file.write_all(xml.as_bytes())?;

    // Ensure data is flushed to disk
    file.sync_all()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_format_hex_key() {
        let bytes: [u8; 32] = [
            0x0F, 0xCA, 0x93, 0x3C, 0x02, 0xED, 0xF2, 0x12, 0xA3, 0x15, 0x60, 0x44, 0xB9, 0x46,
            0x3C, 0x3E, 0xDB, 0x0A, 0x87, 0x54, 0x24, 0x7F, 0x8E, 0x6E, 0x09, 0xA1, 0xA7, 0x3D,
            0x58, 0x2D, 0xFE, 0xBC,
        ];

        let formatted = format_hex_key(&bytes);

        // Should have 8 groups of 8 hex chars each
        assert!(formatted.contains("0FCA933C"));
        assert!(formatted.contains("02EDF212"));
        assert!(formatted.contains("A3156044"));
        assert!(formatted.contains("B9463C3E"));
        assert!(formatted.contains("DB0A8754"));
        assert!(formatted.contains("247F8E6E"));
        assert!(formatted.contains("09A1A73D"));
        assert!(formatted.contains("582DFEBC"));
    }

    #[test]
    fn test_generate_keyfile_creates_valid_xml() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let keyfile_path = temp_dir.path().join("test.keyx");
        let path_str = keyfile_path
            .to_str()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid path"))?;

        generate_keyfile(path_str)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err.to_string()))?;

        // Verify file exists
        assert!(keyfile_path.exists());

        // Read and verify content
        let content = fs::read_to_string(&keyfile_path)?;

        // Check XML structure
        assert!(content.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(content.contains("<KeyFile>"));
        assert!(content.contains("<Version>2.0</Version>"));
        assert!(content.contains("<Key>"));
        assert!(content.contains("<Data Hash="));
        assert!(content.contains("</KeyFile>"));

        Ok(())
    }

    #[test]
    fn test_generate_keyfile_hash_matches_first_bytes() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let keyfile_path = temp_dir.path().join("test.keyx");
        let path_str = keyfile_path
            .to_str()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid path"))?;

        generate_keyfile(path_str)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err.to_string()))?;

        let content = fs::read_to_string(&keyfile_path)?;

        // Extract hash from Hash attribute
        let hash_start = content.find("Hash=\"").ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "Hash not found")
        })? + 6;
        let hash_end = content[hash_start..].find('"').ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "Hash end not found")
        })? + hash_start;
        let hash = &content[hash_start..hash_end];

        // Extract first hex group from data (after the closing > of <Data Hash="...">)
        let data_tag_end = content.find("Hash=\"").ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "Hash not found")
        })?;
        let data_content_start = content[data_tag_end..]
            .find('>')
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "> not found"))?
            + data_tag_end
            + 1;
        let data_content = &content[data_content_start..];

        // Find the first hex character
        let hex_start = data_content
            .find(|c: char| c.is_ascii_hexdigit())
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "Hex not found"))?;
        let first_hex = &data_content[hex_start..hex_start + 8];

        // Hash should match first 4 bytes (8 hex chars)
        assert_eq!(hash, first_hex);

        Ok(())
    }

    #[test]
    fn test_generate_keyfile_different_each_time() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;

        let path1 = temp_dir.path().join("key1.keyx");
        let path2 = temp_dir.path().join("key2.keyx");

        let path1_str = path1
            .to_str()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid path"))?;
        let path2_str = path2
            .to_str()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid path"))?;

        generate_keyfile(path1_str)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err.to_string()))?;
        generate_keyfile(path2_str)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err.to_string()))?;

        let content1 = fs::read_to_string(&path1)?;
        let content2 = fs::read_to_string(&path2)?;

        // Contents should be different (different random keys)
        assert_ne!(content1, content2);

        Ok(())
    }

    #[test]
    fn test_generate_keyfile_invalid_parent_directory() {
        let result = generate_keyfile("/nonexistent/directory/test.keyx");
        assert!(result.is_err());
        assert!(matches!(result, Err(AppError::InvalidPath(_))));
    }
}
