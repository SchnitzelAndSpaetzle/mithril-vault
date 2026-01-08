# Signed Commits Setup

This project prefers signed commits so GitHub can mark contributions as "Verified".
The instructions below use GPG, which is supported across Windows, macOS, and Linux.

## Windows (GPG)

1. Install Gpg4win: https://www.gpg4win.org/
2. Generate a key:
   - `gpg --full-generate-key`
3. List keys:
   - `gpg --list-secret-keys --keyid-format=long`
4. Configure Git:
   - `git config --global user.signingkey <KEYID>`
   - `git config --global commit.gpgsign true`
5. Export your public key:
   - `gpg --armor --export <KEYID>`
6. Add the key to GitHub:
   - https://github.com/settings/gpg/new

## macOS (GPG)

1. Install GPG:
   - `brew install gnupg`
2. Generate a key:
   - `gpg --full-generate-key`
3. List keys:
   - `gpg --list-secret-keys --keyid-format=long`
4. Configure Git:
   - `git config --global user.signingkey <KEYID>`
   - `git config --global commit.gpgsign true`
5. Export your public key:
   - `gpg --armor --export <KEYID>`
6. Add the key to GitHub:
   - https://github.com/settings/gpg/new

## Linux (GPG)

1. Install GPG:
   - `sudo apt install gnupg` (or your distro equivalent)
2. Generate a key:
   - `gpg --full-generate-key`
3. List keys:
   - `gpg --list-secret-keys --keyid-format=long`
4. Configure Git:
   - `git config --global user.signingkey <KEYID>`
   - `git config --global commit.gpgsign true`
5. Export your public key:
   - `gpg --armor --export <KEYID>`
6. Add the key to GitHub:
   - https://github.com/settings/gpg/new

## Verify

- Create a commit and ensure GitHub shows a "Verified" badge on the commit.
- If you need to sign a single commit manually, use `git commit -S`.

## References

- GitHub docs: https://docs.github.com/en/authentication/managing-commit-signature-verification
