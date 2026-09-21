# MacRDP

[![Release](https://img.shields.io/github/v/release/likehbbfoe/MacRDP)](https://github.com/likehbbfoe/MacRDP/releases/latest)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)

[中文](README.md) | English

A native macOS RDP server for connecting to the current desktop, including from Windows App on another Mac. This repository focuses on compatibility, encoding performance and stability.

## Releases

Current version: **[v0.1.0](https://github.com/likehbbfoe/MacRDP/releases/tag/v0.1.0)**, the first public source release. See [CHANGELOG](CHANGELOG.md) for version history and [Releases](https://github.com/likehbbfoe/MacRDP/releases) for features, fixes and usage notes.

This release provides source ZIP / tar.gz archives; build the CLI or management UI using the instructions below. The project is in `0.x` development and shares the currently logged-in desktop rather than creating a separate macOS login session.

Tags follow `vMAJOR.MINOR.PATCH`: patch releases contain compatible fixes and minor releases add features. Compatibility changes during `0.x` development are called out in release notes. Use a version tag for a fixed snapshot; `main` tracks ongoing development.

## Upstream and licensing

This project is based on **[x6nux/macrdp](https://github.com/x6nux/macrdp)**, initially imported from commit `6be70bb`. The original capture, RDP server, encoding and management UI implementations come from that project. Credit belongs to its author and contributors.

The upstream **GPLv3** license is retained; see [LICENSE](LICENSE). The modified IronRDP crates retain their own MIT / Apache-2.0 license files.

Changes include forward-compatible GFX capability negotiation, correct capture format after hardware-to-software fallback, accelerated conversion for padded software frames, reusable buffers, AVC444 row layout and keyframe fixes, hardware identity checks, recovery keyframes and credential redaction.

## Build and run

Requires macOS 14+, Xcode Command Line Tools and current stable Rust. UI development also requires Node.js 20+. Desktop serving requires Screen Recording and Accessibility permissions.

```sh
git clone https://github.com/likehbbfoe/MacRDP.git
cd MacRDP
cargo build --locked --release -p macrdp-server
cp config.example.toml config.toml
```

Replace the username and password placeholders before starting:

```sh
./target/release/macrdp-server --config config.toml
```

Display the server version:

```sh
./target/release/macrdp-server --version
```

Build the Tauri management UI:

```sh
cd macrdp-ui
npm ci
npm run tauri -- build --bundles app
```

Connect Windows App to `<Mac-IP>:13389` using the configured RDP credentials, not the macOS login password.

## Encoding

```toml
port = 13389
username = "rdp-user"
password = "CHANGE_ME"
width = 1920
height = 1080
frame_rate = 30
encoder = "hardware"
chroma_mode = "avc420"
quality = "balanced"
bitrate_mbps = 20
```

- `hardware`: VideoToolbox hardware H.264; the server falls back to OpenH264 with BGRA capture if initialization fails.
- `software`: OpenH264 with Accelerate/vImage conversion where available.
- `auto`: currently retains the software behavior.
- `avc420`: single stream; recommended as the default.
- `avc444`: dual stream; requires a compatible client and additional encoding resources.

VideoToolbox uses system media encoding resources, not general-purpose GPU compute. Configured frame rate is a target, not a performance guarantee.

## Known limitations

Compatibility varies across clients and codec modes.

4K changing-content AVC444 can still produce native VideoToolbox frame drops; stable 4K/30fps is not guaranteed.

Known remaining work includes non-AVC fallback, mouse button coordinates and dragging, multiple displays, dynamic resolution and long-running sessions. CLI and UI use different display paths.

## Feedback and contributions

Report problems and feature requests through [Issues](https://github.com/likehbbfoe/MacRDP/issues). Include the MacRDP, macOS and client versions plus relevant encoding settings. Keep passwords, certificate private keys and personal information private. Code contributions can target `main` through a pull request.

## References

- [x6nux/macrdp](https://github.com/x6nux/macrdp): original codebase for this project.
- [IronRDP](https://github.com/Devolutions/IronRDP): Rust RDP implementation.
- [FreeRDP](https://github.com/FreeRDP/FreeRDP): protocol and AVC444 reference.
- [OpenH264](https://github.com/cisco/openh264): software H.264 encoding and decoding.
- [RustDesk](https://github.com/rustdesk/rustdesk): upstream inspiration for capture and input architecture.
