# Security Policy

MithrilVault is a password manager, and we take security extremely seriously. This document outlines our security practices and how to report vulnerabilities.

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

**Please do NOT report security vulnerabilities through public GitHub issues.**

If you discover a security vulnerability in MithrilVault, please report it responsibly.

## Security Contact

- Primary: GitHub Security Advisories (preferred)
- Email: aleksandar.z.boric@gmail.com

### Preferred Method: GitHub Security Advisories

1. Go to the [Security Advisories](https://github.com/SchnitzelAndSpaetzle/mithril-vault/security/advisories) page
2. Click "New draft security advisory"
3. Fill out the form with details about the vulnerability

### Alternative: Email

If you cannot use GitHub Security Advisories, you may email the maintainers directly. Please include:

- A description of the vulnerability
- Steps to reproduce
- Potential impact
- Any suggested fixes (if applicable)

### What to Include

When reporting a vulnerability, please provide:

1. **Type of vulnerability** (e.g., buffer overflow, SQL injection, XSS, etc.)
2. **Location** of the affected source code (file, line number if possible)
3. **Steps to reproduce** the issue
4. **Proof-of-concept** or exploit code (if possible)
5. **Impact** of the vulnerability
6. **Suggested fix** (if you have one)

### What to Expect

- **Acknowledgment**: We will acknowledge receipt within 48 hours
- **Initial Assessment**: We will provide an initial assessment within 7 days
- **Updates**: We will keep you informed of our progress
- **Resolution**: We aim to release a fix within 90 days for most vulnerabilities
- **Credit**: We will credit you in the release notes (unless you prefer anonymity)

### Safe Harbor

If you act in good faith, avoid privacy violations, and do not exploit the vulnerability beyond what is necessary to demonstrate it, we will not pursue legal action against you.

## Security Best Practices

### For Users

1. **Use a strong master password** - Your master password should be long, unique, and memorable
2. **Keep MithrilVault updated** - Always use the latest version
3. **Enable auto-lock** - Configure automatic database locking after inactivity
4. **Verify downloads** - Only download from official sources and verify checksums when available
5. **Regular backups** - Keep encrypted backups of your database in secure locations

### For Contributors

1. **Never log sensitive data** - No passwords, keys, or personal information in logs
2. **Use zeroize** - Always clear sensitive data from memory when done
3. **Validate all input** - Especially data coming from the frontend
4. **Follow secure coding guidelines** - See [AGENTS.md](AGENTS.md) for detailed requirements
5. **Request security review** - Tag security-sensitive PRs for additional review

## Security Architecture

MithrilVault is designed with security in mind:

- **Cryptographic operations in Rust** - All encryption/decryption happens in the Rust backend
- **Memory zeroization** - Sensitive data is cleared from memory when no longer needed
- **Auto-lock** - Database automatically locks after configurable inactivity period
- **Clipboard clearing** - Copied passwords are automatically cleared from clipboard
- **No cloud by default** - Your data stays local unless you explicitly enable sync

## Scope

The following are in scope for security reports:

- Vulnerabilities in MithrilVault application code
- Issues in the KDBX handling logic
- Cryptographic weaknesses
- Authentication/authorization bypasses
- Data exposure vulnerabilities
- Memory safety issues

The following are out of scope:

- Social engineering attacks
- Physical attacks requiring device access
- Denial of service attacks
- Issues in third-party dependencies (report these to the relevant project)
- Issues requiring user to install malware first

## Bug Bounty

We do not currently have a formal bug bounty program. However, we deeply appreciate security researchers who report vulnerabilities responsibly and will acknowledge their contributions publicly (with permission).

## Acknowledgments

We would like to thank the following security researchers for their responsible disclosures:

*No disclosures yet - be the first!*

---

Thank you for helping keep MithrilVault and its users safe!
