# Test Fixtures

This directory contains KDBX test files for integration testing.

**Note:** These files are committed to the repository and contain only dummy test data.
All files use password: `test123`

## Test Files

| File | Format | Auth | Content |
|------|--------|------|---------|
| `test-kdbx4.kdbx` | KDBX 4.0 | Password only | Test entry |
| `test-kdbx3.kdbx` | KDBX 3.1 | Password only | Test entry |
| `test-keyfile-kdbx4.kdbx` | KDBX 4.0 | Password + keyfile | Test entry |
| `test-keyfile.keyx` | - | Keyfile for above | - |

## Recreating Test Files

If you need to recreate these files using [KeePassXC](https://keepassxc.org/):

1. Create new database with password `test123`
2. For KDBX3: Database Settings → Encryption → KDBX 3.1
3. For keyfile: Add additional protection → generate keyfile
4. Add test entry: Title="Test Entry", Username="testuser", Password="testpass123", URL="https://example.com"
