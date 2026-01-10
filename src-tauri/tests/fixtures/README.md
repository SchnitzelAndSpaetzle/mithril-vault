# Test Fixtures

This directory contains KDBX test files for integration testing.

## Required Test Files

Create these files using [KeePassXC](https://keepassxc.org/) or similar KeePass client:

### test-kdbx4.kdbx
- **Format**: KDBX 4.0 (default in KeePassXC)
- **Password**: `test123`
- **Content**: Add at least one entry with title, username, password, and URL

### test-kdbx3.kdbx
- **Format**: KDBX 3.1 (select in KeePassXC: Database Settings → Encryption → KDBX 3.1)
- **Password**: `test123`
- **Content**: Add at least one entry

### test-keyfile.kdbx + test-keyfile.key
- **Format**: KDBX 4.0
- **Password**: `test123`
- **Key File**: Generate a key file named `test-keyfile.key`
- **Content**: Add at least one entry

## Creating Test Files with KeePassXC

1. Open KeePassXC
2. Click "New Database"
3. Set database name and continue
4. Set password to `test123`
5. For KDBX3: Go to Database Settings → Encryption → Select "KDBX 3.1 (legacy)"
6. For keyfile: Check "Add additional protection" and generate a key file
7. Add a test entry:
   - Title: "Test Entry"
   - Username: "testuser"
   - Password: "testpass123"
   - URL: "https://example.com"
8. Save the database to this directory

## Note

These files are `.gitignore`d by default for security. Each developer needs to create their own test fixtures locally.
