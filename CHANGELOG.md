# Changelog

Notable changes to MacRDP are recorded here. Release tags follow `vMAJOR.MINOR.PATCH`.
During `0.x` development, compatibility changes are documented in each release.

## [Unreleased]

### Added

- A macOS 13 deployment target shared by the CLI and management UI.
- Capability-based bitmap fallback for clients without GFX/AVC, including a bounded graphics negotiation timeout. Fast-Path output and 32-bit color remain required.
- A configuration reference covering encoder selection, AVC420/AVC444, Apple media hardware, legacy aliases and resolution limits.
- Validation for encoder options, frame rates, bitrates and dimensions, with visible configuration errors in the management UI.

### Changed

- `auto` now prefers available VideoToolbox hardware before falling back to OpenH264. Select `software` to retain explicit CPU encoding.
- VideoToolbox retries compatible H.264 profiles and ordinary hardware sessions when low-latency options are unavailable.
- Capture and encoder changes in the management UI request a service restart so their settings take effect together.

### Fixed

- Indefinite display waits when a client does not establish an AVC graphics channel; late negotiation cannot override a selected bitmap session.
- NV12 capture conversion for bitmap fallback, with color-space-aware BGRA output.
- Bitmap widths that are not multiples of four, padded row handling and incomplete initial updates.
- Frame draining that could discard the newest available image.
- CoreGraphics fallback image dimensions and capture updates that could reset unrelated settings.
- Failed VideoToolbox session and CoreFoundation temporary-object cleanup.
- Numeric configuration overflow and invalid updates replacing the active UI configuration.

### Compatibility

- macOS 12 and earlier require a separate capture implementation and remain outside this build's runtime range.
- Older client behavior is selected by negotiated protocol capabilities; slow-path-only or lower-color-depth clients are outside the supported path.
- The existing `v0.1.0` release remains unchanged and retains its original macOS 14+ requirement.

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

[Unreleased]: https://github.com/likehbbfoe/MacRDP/compare/v0.1.0...main
[0.1.0]: https://github.com/likehbbfoe/MacRDP/releases/tag/v0.1.0
