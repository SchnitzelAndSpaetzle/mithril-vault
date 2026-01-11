# KDBX Library Decision

**Date**: January 2026
**Status**: Accepted
**Issue**: [#6 - Add keepass-rs dependency and evaluate KDBX libraries](https://github.com/SchnitzelAndSpaetzle/mithril-vault/issues/6)

## Context

MithrilVault needs a Rust library to read and write KeePass database files (KDBX format). We evaluated three candidate libraries to find the best fit for our requirements.

## Decision

We chose **keepass-rs** (crate name: `keepass`) as our KDBX library.

## Candidates Evaluated

### 1. keepass-rs
- **Repository**: https://github.com/sseemayer/keepass-rs
- **Crate**: https://crates.io/crates/keepass
- **Version**: 0.8.16
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

| Criteria | keepass-rs | kdbx-rs | keepass-db |
|----------|-----------|---------|-----------|
| KDBX4 Read | Full | Full | Full |
| KDBX4 Write | Experimental | Full | Experimental |
| KDBX3 Read | Full | Full | Full |
| KDBX3 Write | No | No | Experimental |
| Key File Support | Yes | Yes | Unknown |
| License | **MIT** | GPL-3.0+ | MIT |
| Downloads (all-time) | **136,779** | 26,994 | 2,473 |
| Last Update | **13 days ago** | Oct 2024 | ~2 years ago |
| Stars | **139** | 2 | 2 |
| Active Maintenance | **Yes** | Yes | Limited |
| Security Features | zeroize, secstr | Standard | Standard |

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

## Known Limitations

1. **Write support is functional but evolving**: Creating and saving databases (`create`, `save`, `save_as` in `KdbxService`) is implemented and covered by tests. The API may evolve as we add more features. Keyfile-authenticated databases currently cannot be saved with keyfile authentication preserved (see TODO in code).

2. **No KDBX3 write support**: New databases are created in KDBX4 format. KDBX3 databases can be read but saving will convert them to KDBX4.

## Migration Path

If keepass-rs proves insufficient in the future:
1. **kdbx-rs** offers full KDBX4 write support (consider if/when write becomes critical)
2. Our `KdbxService` abstraction layer isolates the library choice, making migration straightforward

## Implementation Notes

- Added `keepass = "0.8"` to `src-tauri/Cargo.toml`
- Implemented `KdbxService` in `src-tauri/src/services/kdbx.rs`
- Type conversions between keepass-rs types and our models are in the same file
- Integration tests in `src-tauri/tests/kdbx_integration.rs`

## References

- [keepass-rs GitHub](https://github.com/sseemayer/keepass-rs)
- [keepass-rs docs.rs](https://docs.rs/keepass)
- [KDBX4 File Format Documentation](https://palant.info/2023/03/29/documenting-keepass-kdbx4-file-format/)
