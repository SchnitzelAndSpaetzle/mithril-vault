// SPDX-License-Identifier: MIT

//! Stable per-Vault identifier derived from the canonicalized filesystem path.
//!
//! Used as the on-disk filename for the per-Vault audit log so the directory
//! layout doesn't leak which Vaults exist (the file name is a 64-char hex
//! digest, not the readable path). Two paths that resolve to the same canonical
//! file — symlink and target, `./foo.kdbx` and `/abs/foo.kdbx` — produce the
//! same hash.

use sha2::{Digest, Sha256};
use std::fmt::Write;
use std::path::Path;

/// Hex-encoded SHA-256 of the canonicalized form of `path`, used as the
/// per-Vault audit log filename stem.
pub fn hash_vault_path(path: &Path) -> String {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let mut hasher = Sha256::new();
    hasher.update(canonical.to_string_lossy().as_bytes());
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        // write! into a String cannot fail.
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn hash_is_stable_for_same_canonical_path() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("vault.kdbx");
        fs::write(&target, b"x").expect("write");

        let a = hash_vault_path(&target);
        let b = hash_vault_path(&target);
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
    }

    #[test]
    #[cfg(unix)]
    fn symlink_hashes_to_same_value_as_target() {
        use std::os::unix::fs::symlink;

        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("vault.kdbx");
        fs::write(&target, b"x").expect("write");

        let link = dir.path().join("link.kdbx");
        symlink(&target, &link).expect("symlink");

        let target_hash = hash_vault_path(&target);
        let link_hash = hash_vault_path(&link);
        assert_eq!(target_hash, link_hash);
    }
}
