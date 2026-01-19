# Test Fixtures

This directory contains KDBX test files for integration testing.

> **WARNING:** These fixtures are for automated testing **only**. Never store real credentials, personal data, or any sensitive information in these files.

**Note:** These files are intentionally committed to the repository and contain only dummy test data. All files use the public test password: `test123` — this password must **never** be reused for any real database.

## Test Files

| File                           | Format   | Auth               | Content    |
|--------------------------------|----------|--------------------|------------|
| `test-kdbx4.kdbx`              | KDBX 4.0 | Password only      | Test entry |
| `test-kdbx3.kdbx`              | KDBX 3.1 | Password only      | Test entry |
| `test-keyfile-kdbx4.kdbx`      | KDBX 4.0 | Password + keyfile | Test entry |
| `test-keyfile-only-kdbx4.kdbx` | KDBX 4.0 | Keyfile only       | Test entry |
| `test-keyfile.keyx`            | -        | Keyfile for above  | -          |

## Recreating Test Files

If you need to recreate these files using [KeePassXC](https://keepassxc.org/):

### Password-only databases
1. Create new database with password `test123`
2. For KDBX3: Database Settings → Encryption → KDBX 3.1
3. Add test entry: Title="Test Entry", Username="testuser", Password="testpass123", URL="https://example.com"

### Password + keyfile database

1. Create new database with password `test123`
2. Add additional protection → select the existing `test-keyfile.keyx`
3. Add test entry: Title="Test Entry", Username="testuser", Password="testpass123", URL="https://example.com"

### Keyfile-only database

1. Create new database with **no password** (leave password field empty)
2. Add keyfile protection → select the existing `test-keyfile.keyx`
3. Add test entry: Title="Keyfile Only Entry", Username="testuser", Password="testpass123", URL="https://example.com"
