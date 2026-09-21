# Changelog

Notable changes to MacRDP are recorded here. Release tags follow `vMAJOR.MINOR.PATCH`.
During `0.x` development, compatibility changes are documented in each release.

## [0.1.0] - 2026-09-21

First public source release of this repository, based on [x6nux/macrdp](https://github.com/x6nux/macrdp) at `6be70bb`.

### Added

- Native macOS current-desktop RDP serving through a CLI and a Tauri management UI.
- Configurable VideoToolbox hardware H.264 and OpenH264 software encoding, with AVC420 and optional AVC444 dual streams.
- Service controls, configuration, permission status and connection information in the management UI.
- A CLI `--version` option and an in-app link to published release notes.
- Versioned source archives and release information in both Chinese and English.

### Improved

- Accelerate/vImage color conversion for software frames with padded dimensions, plus reusable image buffers and borrowed YUV planes.
- Capture format selection based on the initialized encoder: supported hardware AVC420 uses NV12, while software fallback uses BGRA.
- Independent VideoToolbox sessions for AVC444 main and auxiliary streams, including bitrate updates and recovery keyframes.
- Credential diagnostic redaction and frontend dependency updates.
- Shared version and license metadata for the project Rust crates.

### Fixed

- Windows App black screens caused by rejecting unknown GFX capability versions during negotiation.
- Hardware initialization fallback that previously left software encoding paired with NV12 capture.
- AVC444 padded row stride and keyframe handling across both streams.
- Native hardware encoding failures being treated as successful empty frames.
- TLS-only credential matching that previously accepted empty passwords.
- A hardcoded UI version that disagreed with the package version.
- A placeholder update check that always reported no update; the UI now links to actual releases.
- Duplicate Swift runtime loading caused by preferring an older Xcode runtime over the system runtime.

### Requirements and known limitations

- macOS 14 or later; Screen Recording and Accessibility permissions are required.
- Source distribution: building requires current stable Rust and Xcode Command Line Tools; the management UI additionally requires Node.js 20+.
- The server shares the active desktop and does not provide separate user login sessions.
- AVC420 is recommended. Hardware initialization can fall back to software; `auto` currently selects software.
- 4K changing-content AVC444 can encounter native VideoToolbox frame drops; stable 4K/30fps is not guaranteed.
- Non-AVC fallback, mouse coordinates and dragging, multiple displays, dynamic resolution and long-session reliability remain areas for improvement.

### Attribution

- The original capture, protocol integration, input and management UI implementations come from [x6nux/macrdp](https://github.com/x6nux/macrdp).
- GPLv3 is retained. The modified IronRDP crates retain their MIT / Apache-2.0 licenses.

[0.1.0]: https://github.com/likehbbfoe/MacRDP/releases/tag/v0.1.0
