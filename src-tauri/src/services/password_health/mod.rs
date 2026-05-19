// SPDX-License-Identifier: MIT

//! Password Health report.
//!
//! Per-Vault, strictly-local assessment of the cleartext passwords stored
//! inside an unlocked Vault. The analyzer is a pure function so the policy
//! can be exercised exhaustively without touching the filesystem, the IPC
//! layer, or `keepass-rs`. See `docs/adr/0002-password-health-report.md`
//! for the architectural decisions this module implements.

pub mod analyzer;
pub mod cache;
pub mod service;
