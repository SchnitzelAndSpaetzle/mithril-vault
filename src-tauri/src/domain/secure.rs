// SPDX-License-Identifier: MIT

//! Secure memory types that automatically zeroize on drop.
//!
//! These types wrap sensitive data (passwords, keys) and ensure:
//! - Memory is zeroized when dropped (via `zeroize` crate)
//! - Debug/Display implementations never expose the actual content
//! - Serde support for Tauri IPC (deserialization works, serialization warns)

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::ops::Deref;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// A secure string wrapper that automatically zeroizes on drop.
///
/// Use this type for passwords and other sensitive string data.
/// The inner value is never exposed in Debug or Display output.
#[derive(Clone, Default, Zeroize, ZeroizeOnDrop)]
pub struct SecureString(String);

impl SecureString {
    /// Creates a new `SecureString` from a string.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns true if the inner string is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the length of the inner string.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns the inner string as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Deref for SecureString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<String> for SecureString {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for SecureString {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl fmt::Debug for SecureString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SecureString").field(&"[REDACTED]").finish()
    }
}

impl fmt::Display for SecureString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[REDACTED]")
    }
}

impl PartialEq for SecureString {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for SecureString {}

impl<'de> Deserialize<'de> for SecureString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(Self(value))
    }
}

impl Serialize for SecureString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Warning: Serializing SecureString should be avoided.
        // If you see this in logs, consider whether the data should be serialized.
        self.0.serialize(serializer)
    }
}

/// A secure byte vector wrapper that automatically zeroizes on drop.
///
/// Use this type for keys, keyfile contents, and other sensitive binary data.
/// The inner value is never exposed in Debug or Display output.
#[derive(Clone, Default, Zeroize, ZeroizeOnDrop)]
pub struct SecureBytes(Vec<u8>);

impl SecureBytes {
    /// Creates a new `SecureBytes` from a byte vector.
    #[must_use]
    pub fn new(value: impl Into<Vec<u8>>) -> Self {
        Self(value.into())
    }

    /// Returns true if the inner byte vector is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the length of the inner byte vector.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns the inner bytes as a slice.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl Deref for SecureBytes {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Vec<u8>> for SecureBytes {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}

impl From<&[u8]> for SecureBytes {
    fn from(value: &[u8]) -> Self {
        Self(value.to_vec())
    }
}

impl fmt::Debug for SecureBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SecureBytes").field(&"[REDACTED]").finish()
    }
}

impl fmt::Display for SecureBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[REDACTED]")
    }
}

impl PartialEq for SecureBytes {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for SecureBytes {}

impl<'de> Deserialize<'de> for SecureBytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Vec::<u8>::deserialize(deserializer)?;
        Ok(Self(value))
    }
}

impl Serialize for SecureBytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Warning: Serializing SecureBytes should be avoided.
        // If you see this in logs, consider whether the data should be serialized.
        self.0.serialize(serializer)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    // SecureString tests

    #[test]
    fn test_secure_string_creation() {
        let secure = SecureString::new("password123");
        assert_eq!(secure.as_str(), "password123");
    }

    #[test]
    fn test_secure_string_from_string() {
        let secure = SecureString::from(String::from("secret"));
        assert_eq!(secure.as_str(), "secret");
    }

    #[test]
    fn test_secure_string_from_str() {
        let secure = SecureString::from("secret");
        assert_eq!(secure.as_str(), "secret");
    }

    #[test]
    fn test_secure_string_deref() {
        let secure = SecureString::new("test");
        let str_ref: &str = &secure;
        assert_eq!(str_ref, "test");
    }

    #[test]
    fn test_secure_string_debug_redacted() {
        let secure = SecureString::new("secret_password");
        let debug_output = format!("{secure:?}");
        assert!(!debug_output.contains("secret_password"));
        assert!(debug_output.contains("[REDACTED]"));
    }

    #[test]
    fn test_secure_string_display_redacted() {
        let secure = SecureString::new("secret_password");
        let display_output = format!("{secure}");
        assert!(!display_output.contains("secret_password"));
        assert_eq!(display_output, "[REDACTED]");
    }

    #[test]
    fn test_secure_string_clone() {
        let original = SecureString::new("cloneable");
        let cloned = original.clone();
        assert_eq!(original.as_str(), cloned.as_str());
    }

    #[test]
    fn test_secure_string_is_empty() {
        let empty = SecureString::new("");
        let non_empty = SecureString::new("x");
        assert!(empty.is_empty());
        assert!(!non_empty.is_empty());
    }

    #[test]
    fn test_secure_string_len() {
        let secure = SecureString::new("hello");
        assert_eq!(secure.len(), 5);
    }

    #[test]
    fn test_secure_string_default() {
        let secure = SecureString::default();
        assert!(secure.is_empty());
    }

    #[test]
    fn test_secure_string_equality() {
        let a = SecureString::new("same");
        let b = SecureString::new("same");
        let c = SecureString::new("different");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_secure_string_serde_roundtrip() {
        let original = SecureString::new("serializable");
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: SecureString = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original.as_str(), deserialized.as_str());
    }

    // SecureBytes tests

    #[test]
    fn test_secure_bytes_creation() {
        let secure = SecureBytes::new(vec![1, 2, 3, 4]);
        assert_eq!(secure.as_bytes(), &[1, 2, 3, 4]);
    }

    #[test]
    fn test_secure_bytes_from_vec() {
        let secure = SecureBytes::from(vec![5, 6, 7]);
        assert_eq!(secure.as_bytes(), &[5, 6, 7]);
    }

    #[test]
    fn test_secure_bytes_from_slice() {
        let data: &[u8] = &[8, 9, 10];
        let secure = SecureBytes::from(data);
        assert_eq!(secure.as_bytes(), &[8, 9, 10]);
    }

    #[test]
    fn test_secure_bytes_deref() {
        let secure = SecureBytes::new(vec![1, 2, 3]);
        let slice: &[u8] = &secure;
        assert_eq!(slice, &[1, 2, 3]);
    }

    #[test]
    fn test_secure_bytes_debug_redacted() {
        let secure = SecureBytes::new(vec![1, 2, 3, 4, 5]);
        let debug_output = format!("{secure:?}");
        assert!(!debug_output.contains("[1, 2, 3, 4, 5]"));
        assert!(debug_output.contains("[REDACTED]"));
    }

    #[test]
    fn test_secure_bytes_display_redacted() {
        let secure = SecureBytes::new(vec![1, 2, 3]);
        let display_output = format!("{secure}");
        assert_eq!(display_output, "[REDACTED]");
    }

    #[test]
    fn test_secure_bytes_clone() {
        let original = SecureBytes::new(vec![1, 2, 3]);
        let cloned = original.clone();
        assert_eq!(original.as_bytes(), cloned.as_bytes());
    }

    #[test]
    fn test_secure_bytes_is_empty() {
        let empty = SecureBytes::new(vec![]);
        let non_empty = SecureBytes::new(vec![1]);
        assert!(empty.is_empty());
        assert!(!non_empty.is_empty());
    }

    #[test]
    fn test_secure_bytes_len() {
        let secure = SecureBytes::new(vec![1, 2, 3, 4, 5]);
        assert_eq!(secure.len(), 5);
    }

    #[test]
    fn test_secure_bytes_default() {
        let secure = SecureBytes::default();
        assert!(secure.is_empty());
    }

    #[test]
    fn test_secure_bytes_equality() {
        let a = SecureBytes::new(vec![1, 2, 3]);
        let b = SecureBytes::new(vec![1, 2, 3]);
        let c = SecureBytes::new(vec![4, 5, 6]);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_secure_bytes_serde_roundtrip() {
        let original = SecureBytes::new(vec![1, 2, 3, 4, 5]);
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: SecureBytes = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original.as_bytes(), deserialized.as_bytes());
    }
}
