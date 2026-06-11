# KDBX Library Decision

**Date**: January 2026 (decision) · updated June 2026 (dependency refresh)
**Status**: Accepted
**Current dependency**: `keepass = { version = "0.13", features = ["save_kdbx4"] }` (0.13.8 at time of writing)
**Issue**: [#6 - Add keepass-rs dependency and evaluate KDBX libraries](https://github.com/SchnitzelAndSpaetzle/mithril-vault/issues/6)

## Context

MithrilVault needs a Rust library to read and write KeePass database files (KDBX format). We evaluated three candidate libraries to find the best fit for our requirements.

## Decision

We chose **keepass-rs** (crate name: `keepass`) as our KDBX library.

## Candidates Evaluated

### 1. keepass-rs

- **Repository**: https://github.com/sseemayer/keepass-rs
- **Crate**: https://crates.io/crates/keepass
- **Version evaluated**: 0.8.16 (Jan 2026) — **currently pinned**: 0.13.x with the `save_kdbx4` feature (see [Current status](#current-status-june-2026))
- **License**: MIT

### 2. kdbx-rs

- **Repository**: https://github.com/tonyfinn/kdbx-rs
- **Crate**: https://crates.io/crates/kdbx-rs
- **Version**: 0.5.2
- **License**: GPL-3.0+

### 3. keepass-db

- **Repository**: https://github.com/penguin359/keepass-db
- **Crate**: https://crates.io/crates/keepass-db
- **Version**: 0.0.2
- **License**: MIT

## Evaluation Criteria

_This table is the January 2026 evaluation snapshot (keepass-rs 0.8.16). It is kept as the historical basis for the decision; see [Current status](#current-status-june-2026) for what changed on the 0.13 line._

| Criteria             | keepass-rs      | kdbx-rs  | keepass-db   |
| -------------------- | --------------- | -------- | ------------ |
| KDBX4 Read           | Full            | Full     | Full         |
| KDBX4 Write          | Experimental    | Full     | Experimental |
| KDBX3 Read           | Full            | Full     | Full         |
| KDBX3 Write          | No              | No       | Experimental |
| Key File Support     | Yes             | Yes      | Unknown      |
| License              | **MIT**         | GPL-3.0+ | MIT          |
| Downloads (all-time) | **136,779**     | 26,994   | 2,473        |
| Last Update          | **13 days ago** | Oct 2024 | ~2 years ago |
| Stars                | **139**         | 2        | 2            |
| Active Maintenance   | **Yes**         | Yes      | Limited      |
| Security Features    | zeroize, secstr | Standard | Standard     |

## Rationale

### 1. License Compatibility

- **keepass-rs** uses MIT license, which is maximally permissive and compatible with our MIT license
- **kdbx-rs** uses GPL-3.0+ which would require us to use a copyleft license
- Clear licensing reduces legal complexity

### 2. Community and Maintenance

- **136K+ downloads** indicates production usage and stability
- **139 stars** shows community trust
- **Active development** with commits from the past 2 weeks
- Large user base means bugs are found and fixed faster

### 3. Security Features

- Built-in support for `zeroize` crate for secure memory clearing
- Uses `secstr` for protected string handling
- Aligns with MithrilVault's security requirements

### 4. Feature Coverage

- Full KDBX4 and KDBX3 read support covers our MVP requirements
- Experimental write support is sufficient for initial development
- Key file support enables advanced authentication scenarios

### 5. API Ergonomics

- Clean, idiomatic Rust API
- Good documentation with examples
- CLI utilities for debugging (kp-dump-json, kp-show-db)

## Current status (June 2026)

The decision to use keepass-rs is unchanged. The dependency has since moved from the evaluated `0.8.16` to **`0.13.x`** (bumped in #278 / #279; pinned as `keepass = { version = "0.13", features = ["save_kdbx4"] }`, currently 0.13.8). What changed relative to the table above:

- **KDBX4 write is first-class, not experimental.** Database creation and saving (`create` / `save` / `save_as` in `KdbxService`) run in production behind the `save_kdbx4` cargo feature. The "Experimental" KDBX4-write rating in the snapshot table no longer applies.
- **The 0.13 API shape is what the codebase relies on**: borrow-checked entry handles (`EntryRef` / `EntryMut`), the Vault-level **attachment binary pool** (`add_attachment` / `attachments_named` / `attachment_by_name` / `remove_attachment_by_name`), custom icons, and `UPPERCASE` config enums (e.g. `OuterCipherConfig::AES256`).
- **KDF pairing**: `rust-argon2 = "3.0"` is pinned alongside to match keepass's transitive Argon2 dependency, so Argon2id KDF parameters line up. (See the "keepass-rs Crate Notes" in `CLAUDE.md`.)

## Known Limitations

1. **No KDBX3 write support**: New databases are created in KDBX4 format. KDBX3 databases can be read but saving will convert them to KDBX4.

2. **Keyfile-preserving save**: Keyfile-authenticated databases may not round-trip keyfile authentication on save in all cases (see TODOs in the keyfile handling code).

## Migration Path

If keepass-rs proves insufficient in the future:

1. **kdbx-rs** remains the main alternative. (Note the original write-support gap that motivated this fallback is now closed — keepass-rs 0.13 provides first-class KDBX4 write via `save_kdbx4` — so a migration would be driven by other factors.)
2. Our `KdbxService` abstraction layer isolates the library choice, making migration straightforward

## Implementation Notes

- `keepass = { version = "0.13", features = ["save_kdbx4"] }` in `src-tauri/Cargo.toml`, with `rust-argon2 = "3.0"` pinned to match its KDF dependency
- Implemented `KdbxService` in `src-tauri/src/services/kdbx/`, split across modules (`open.rs`, `save.rs`, `create.rs`, `entries.rs`, `groups.rs`, `custom_icons.rs`, `vault.rs`, …)
- Type conversions between keepass-rs types and our DTOs are in `src-tauri/src/services/kdbx/conversions.rs`
- Integration tests in `src-tauri/tests/kdbx_open.rs`, `src-tauri/tests/kdbx_entries_groups.rs`, `src-tauri/tests/kdbx_save.rs`, `src-tauri/tests/kdbx_create.rs`, `src-tauri/tests/kdbx_header.rs`

## References

- [keepass-rs GitHub](https://github.com/sseemayer/keepass-rs)
- [keepass-rs docs.rs](https://docs.rs/keepass)
- [KDBX4 File Format Documentation](https://palant.info/2023/03/29/documenting-keepass-kdbx4-file-format/)
