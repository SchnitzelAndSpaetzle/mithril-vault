# MithrilVault

[![CI](https://github.com/SchnitzelAndSpaetzle/mithril-vault/actions/workflows/ci.yml/badge.svg)](https://github.com/SchnitzelAndSpaetzle/mithril-vault/actions/workflows/ci.yml)
[![Security](https://github.com/SchnitzelAndSpaetzle/mithril-vault/actions/workflows/security.yml/badge.svg)](https://github.com/SchnitzelAndSpaetzle/mithril-vault/actions/workflows/security.yml)
[![codecov](https://codecov.io/gh/SchnitzelAndSpaetzle/mithril-vault/branch/main/graph/badge.svg)](https://codecov.io/gh/SchnitzelAndSpaetzle/mithril-vault)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)

A modern, cross-platform password manager with full KeePass compatibility. Built with Tauri, React, and Rust for security and performance.

<!-- Screenshot placeholder -->
<!-- ![MithrilVault Screenshot](docs/images/screenshot.png) -->

## Features

- **KeePass Compatible** - Full support for KDBX4 and KDBX3 database formats
- **Cross-Platform** - Native apps for Linux, Windows, and macOS
- **Modern UI** - Clean, intuitive interface built with React
- **Secure by Design** - All cryptographic operations in Rust, memory zeroization, auto-lock
- **Open Source** - Fully auditable codebase under GPL-3.0

### Planned Features

- Browser extension for autofill
- TOTP (two-factor authentication) support
- Password health reports
- Cloud sync integration
- Mobile support (iOS, Android)

## Installation

### Pre-built Binaries

Download the latest release for your platform from the [Releases](https://github.com/SchnitzelAndSpaetzle/mithril-vault/releases) page.

### Build from Source

See [Building from Source](#building-from-source) below.

## Building from Source

### Prerequisites

- [Node.js](https://nodejs.org/) (v18 or later)
- [Bun](https://bun.sh/) (recommended) or npm
- [Rust](https://www.rust-lang.org/tools/install) (1.70 or later)
- Platform-specific dependencies:

#### Linux (Debian/Ubuntu)

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

#### macOS

```bash
xcode-select --install
```

#### Windows

- Install [Microsoft Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
- Install [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)

### Build Steps

1. Clone the repository:

```bash
git clone https://github.com/SchnitzelAndSpaetzle/mithril-vault.git
cd mithril-vault
```

2. Install dependencies:

```bash
bun install
```

3. Run in development mode:

```bash
bun run dev-desktop
```

4. Build for production:

```bash
bun run tauri build
```

The built application will be in `src-tauri/target/release/bundle/`.

## Development

### Available Scripts

| Command                 | Description                              |
| ----------------------- | ---------------------------------------- |
| `bun run dev-desktop`   | Start development server with hot reload |
| `bun run build`         | Build frontend for production            |
| `bun run tauri build`   | Build complete application               |
| `bun run test`          | Run tests                                |
| `bun run test:coverage` | Run tests with coverage                  |
| `bun run lint`          | Run ESLint                               |
| `bun run format`        | Format code with Prettier                |
| `bun run check`         | Run lint and format checks               |

### Project Structure

```
mithril-vault/
├── src/                 # React frontend
│   ├── components/      # Reusable UI components
│   ├── views/           # Page-level components
│   ├── hooks/           # Custom React hooks
│   ├── stores/          # Zustand state stores
│   └── lib/             # Utilities and types
├── src-tauri/           # Rust backend
│   └── src/
│       ├── commands/    # Tauri IPC commands
│       ├── services/    # Business logic
│       └── models/      # Data structures
└── docs/                # Documentation
```

See [AGENTS.md](AGENTS.md) for detailed architecture documentation.

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Security

MithrilVault takes security seriously. If you discover a security vulnerability, please see our [Security Policy](SECURITY.md) for responsible disclosure guidelines.

## License

MithrilVault is licensed under the [GNU General Public License v3.0](LICENSE).

## Acknowledgments

- [KeePass](https://keepass.info/) - The original password manager and KDBX format
- [KeePassXC](https://keepassxc.org/) - Inspiration and reference implementation
- [Tauri](https://tauri.app/) - Secure application framework
- [keepass-rs](https://crates.io/crates/keepass) - Rust KDBX library
