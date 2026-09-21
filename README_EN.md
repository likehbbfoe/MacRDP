# MacRDP

[![Release](https://img.shields.io/github/v/release/likehbbfoe/MacRDP)](https://github.com/likehbbfoe/MacRDP/releases/latest)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)

[中文](README.md) | English

A native macOS RDP server for connecting to the current desktop, including from Windows App on another Mac. This repository focuses on compatibility, encoding performance and stability.

## Releases

Latest release: **[v0.1.0](https://github.com/likehbbfoe/MacRDP/releases/tag/v0.1.0)**, the first public source release. See [CHANGELOG](CHANGELOG.md) for version history and [Releases](https://github.com/likehbbfoe/MacRDP/releases) for features, fixes and usage notes.

Releases provide source ZIP / tar.gz archives. The instructions below describe ongoing `main` development; the fixed `v0.1.0` tag retains its original macOS 14+ requirement and configuration behavior. The project is in `0.x` development and shares the currently logged-in desktop rather than creating a separate macOS login session.

Tags follow `vMAJOR.MINOR.PATCH`: patch releases contain compatible fixes and minor releases add features. Compatibility changes during `0.x` development are called out in release notes. Use a version tag for a fixed snapshot; `main` tracks ongoing development.

## Upstream and licensing

This project is based on **[x6nux/macrdp](https://github.com/x6nux/macrdp)**, initially imported from commit `6be70bb`. The original capture, RDP server, encoding and management UI implementations come from that project. Credit belongs to its author and contributors.

The upstream **GPLv3** license is retained; see [LICENSE](LICENSE). The modified IronRDP crates retain their own MIT / Apache-2.0 license files.

Changes include forward-compatible GFX capability negotiation, correct capture format after hardware-to-software fallback, accelerated conversion for padded software frames, reusable buffers, AVC444 row layout and keyframe fixes, hardware identity checks, recovery keyframes and credential redaction.

## Build and run

Requires macOS 13+, Xcode Command Line Tools and current stable Rust. UI development also requires Node.js 20+. Desktop serving requires Screen Recording and Accessibility permissions.

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
resolution = "1920x1080"
frame_rate = 30
encoder = "hardware"
chroma_mode = "avc420"
quality = "balanced"
bitrate_mbps = 20
```

- `hardware`: VideoToolbox hardware H.264; the server falls back to OpenH264 with BGRA capture if initialization fails.
- `software`: OpenH264 with Accelerate/vImage conversion where available.
- `auto`: prefers available VideoToolbox hardware and falls back to OpenH264 on initialization failure.
- `avc420`: single stream; recommended as the default.
- `avc444`: dual stream; requires a compatible client and additional encoding resources.

VideoToolbox uses system media encoding resources, not general-purpose GPU compute. Configured frame rate is a target, not a performance guarantee.

## System and client compatibility

| Environment | Mainline behavior |
| --- | --- |
| macOS 13 Ventura | Minimum deployment target; ScreenCaptureKit capture, with a service restart for capture-rate changes |
| macOS 14 and newer | The same mainline code enables available system capabilities |
| Apple Silicon | Tries low-latency VideoToolbox H.264 and the direct NV12 capture path |
| Intel Mac | Selects hardware encoding based on VideoToolbox capabilities rather than a processor-model list |
| Unavailable hardware support or resources | Falls back to OpenH264; `software` can also be selected explicitly |
| Windows App / Microsoft Remote Desktop | Negotiates AVC444, AVC420 or traditional image transport using RDP capabilities rather than client-version strings |
| Clients without GFX / AVC | Uses traditional bitmap transport with negotiated compression; Fast-Path output and 32-bit color are still required |
| macOS 12 and older | Outside the current build's runtime range because the capture dependency requires macOS 13 |

Server OS requirements are separate from the client application's own installation requirements, which are determined by Microsoft. This project shares the active desktop and cannot add operating-system support to the client application.

If low-latency hardware mode is unavailable, the encoder retries an ordinary hardware session before software fallback. AVC444 also needs client support and resources for two encoder sessions. Apple HEVC, AV1 or ProRes hardware capabilities do not imply a compatible RDP output mode: this server currently sends H.264/AVC or traditional bitmaps; decoding happens in the client.

## Configuration reference

| Setting | Values and behavior |
| --- | --- |
| `encoder` | `auto` (default), `hardware`, `software`; legacy aliases `vt` / `videotoolbox` / `gpu` and `openh264` / `oh264` / `cpu` remain supported |
| `chroma_mode` | `avc420` (default, compatibility first) or `avc444` (better chroma detail with a compatible client) |
| `quality` | `low_latency`, `balanced`, `high_quality`; influences the automatic bitrate target |
| `frame_rate` | 1–120; 15 / 30 are useful starting points on older hardware or slower networks |
| `bitrate_mbps` | 1–1000 Mbps; omit for an automatic target; this is not a performance guarantee |
| `resolution` | `auto`, explicit dimensions such as `1920x1080`, or legacy scale values 1–4 |
| `width` / `height` | Legacy dimensions; both must be zero or both positive; prefer `resolution` for final capture sizing |
| `show_cursor` | Include the cursor in captured frames |

Dimensions are bounded to 8192 pixels per side and a total of 7680×4320 pixels. Unknown encoding options, invalid frame rates and out-of-range dimensions are rejected before startup.

Start with `auto`, `avc420`, 30 fps and 1920×1080 for ordinary LAN use. Choose `avc444` for chroma detail when the client supports it. For constrained hardware, use `software`, `avc420`, 15–30 fps and a lower resolution/bitrate.

After changing capture or encoding options, restart the service and reconnect the client so the related settings take effect together. Compatibility improvements stay on one mainline where possible; separate maintained versions are reserved for system differences that cannot share an implementation.

## Known limitations

Compatibility varies across clients and codec modes.

4K changing-content AVC444 can still produce native VideoToolbox frame drops; stable 4K/30fps is not guaranteed.

Known remaining work includes mouse button coordinates and dragging, multiple displays, dynamic resolution and long-running sessions. CLI and UI use different display paths.

## Feedback and contributions

Report problems and feature requests through [Issues](https://github.com/likehbbfoe/MacRDP/issues). Include the MacRDP, macOS and client versions plus relevant encoding settings. Keep passwords, certificate private keys and personal information private. Code contributions can target `main` through a pull request.

## References

- [x6nux/macrdp](https://github.com/x6nux/macrdp): original codebase for this project.
- [IronRDP](https://github.com/Devolutions/IronRDP): Rust RDP implementation.
- [FreeRDP](https://github.com/FreeRDP/FreeRDP): protocol and AVC444 reference.
- [OpenH264](https://github.com/cisco/openh264): software H.264 encoding and decoding.
- [RustDesk](https://github.com/rustdesk/rustdesk): upstream inspiration for capture and input architecture.
