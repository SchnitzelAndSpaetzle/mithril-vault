// SPDX-License-Identifier: MIT

//! AEAD frame format for audit log entries.
//!
//! Each audit record is independently encrypted with XChaCha20-Poly1305 using
//! a fresh random 24-byte nonce. The on-disk frame is `nonce || ciphertext`,
//! base64-encoded so the log stays line-oriented JSONL.
//!
//! The 24-byte nonce makes random-per-record safe — at 192 bits, the birthday
//! collision probability is negligible for any practical record count.

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use rand::rand_core::TryRng;
use rand::rngs::SysRng;
use thiserror::Error;

pub const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 24;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("invalid audit key length")]
    InvalidKey,

    #[error("frame too short")]
    FrameTooShort,

    #[error("nonce RNG failed")]
    Rng,

    #[error("frame base64-decode failed")]
    Base64,

    #[error("authentication failed (wrong key or tampered frame)")]
    AuthFailed,
}

/// Encrypts a plaintext byte slice into a frame: `nonce || ciphertext+tag`.
pub fn encrypt(key: &[u8; KEY_LEN], plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let cipher = XChaCha20Poly1305::new_from_slice(key).map_err(|_| CryptoError::InvalidKey)?;
    let mut nonce_bytes = [0u8; NONCE_LEN];
    SysRng
        .try_fill_bytes(&mut nonce_bytes)
        .map_err(|_| CryptoError::Rng)?;
    let nonce = XNonce::from(nonce_bytes);
    let mut ct = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|_| CryptoError::AuthFailed)?;
    let mut frame = Vec::with_capacity(NONCE_LEN + ct.len());
    frame.extend_from_slice(nonce.as_slice());
    frame.append(&mut ct);
    Ok(frame)
}

/// Decrypts a frame produced by [`encrypt`] back to plaintext.
pub fn decrypt(key: &[u8; KEY_LEN], frame: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if frame.len() < NONCE_LEN {
        return Err(CryptoError::FrameTooShort);
    }
    let cipher = XChaCha20Poly1305::new_from_slice(key).map_err(|_| CryptoError::InvalidKey)?;
    let (nonce_bytes, ciphertext) = frame.split_at(NONCE_LEN);
    let nonce = XNonce::try_from(nonce_bytes).map_err(|_| CryptoError::FrameTooShort)?;
    cipher
        .decrypt(&nonce, ciphertext)
        .map_err(|_| CryptoError::AuthFailed)
}

/// Encodes a binary frame as a base64 string suitable for JSONL.
pub fn encode_frame(frame: &[u8]) -> String {
    BASE64.encode(frame)
}

/// Decodes a base64-encoded frame string back to bytes.
pub fn decode_frame(s: &str) -> Result<Vec<u8>, CryptoError> {
    BASE64.decode(s.trim()).map_err(|_| CryptoError::Base64)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    fn test_key() -> [u8; KEY_LEN] {
        let mut k = [0u8; KEY_LEN];
        for (i, b) in k.iter_mut().enumerate() {
            *b = u8::try_from(i & 0xff).unwrap();
        }
        k
    }

    #[test]
    fn frame_round_trips() {
        let key = test_key();
        let pt = b"hello audit log";
        let frame = encrypt(&key, pt).expect("encrypt");
        let recovered = decrypt(&key, &frame).expect("decrypt");
        assert_eq!(recovered, pt);
    }

    #[test]
    fn tampered_ciphertext_fails() {
        let key = test_key();
        let mut frame = encrypt(&key, b"some plaintext").expect("encrypt");
        // Flip a bit in the ciphertext (past the 24-byte nonce prefix).
        let i = NONCE_LEN + 1;
        frame[i] ^= 0x01;
        assert!(matches!(
            decrypt(&key, &frame),
            Err(CryptoError::AuthFailed)
        ));
    }

    #[test]
    fn tampered_nonce_fails() {
        let key = test_key();
        let mut frame = encrypt(&key, b"some plaintext").expect("encrypt");
        frame[0] ^= 0x01;
        assert!(matches!(
            decrypt(&key, &frame),
            Err(CryptoError::AuthFailed)
        ));
    }

    #[test]
    fn wrong_key_fails() {
        let key = test_key();
        let mut wrong = test_key();
        wrong[0] ^= 0xff;
        let frame = encrypt(&key, b"secret").expect("encrypt");
        assert!(matches!(
            decrypt(&wrong, &frame),
            Err(CryptoError::AuthFailed)
        ));
    }

    #[test]
    fn nonces_differ_across_encryptions() {
        let key = test_key();
        let a = encrypt(&key, b"same plaintext").expect("encrypt");
        let b = encrypt(&key, b"same plaintext").expect("encrypt");
        assert_ne!(&a[..NONCE_LEN], &b[..NONCE_LEN]);
        assert_ne!(a, b);
    }

    #[test]
    fn base64_round_trip() {
        let key = test_key();
        let frame = encrypt(&key, b"x").expect("encrypt");
        let encoded = encode_frame(&frame);
        let decoded = decode_frame(&encoded).expect("decode");
        assert_eq!(decoded, frame);
    }
}
