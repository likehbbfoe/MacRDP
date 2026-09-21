# MacRDP

[![Release](https://img.shields.io/github/v/release/likehbbfoe/MacRDP)](https://github.com/likehbbfoe/MacRDP/releases/latest)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
[![macOS](https://img.shields.io/badge/macOS-13%2B-black.svg)](https://www.apple.com/macos/)

中文 | [English](README_EN.md)

macOS 原生 RDP 服务端，可使用另一台 Mac 上的 Windows App 连接当前桌面。本仓库关注客户端兼容性、编码性能与稳定性。

## 版本与下载

最新发行版：**[v0.1.0](https://github.com/likehbbfoe/MacRDP/releases/tag/v0.1.0)**，首个公开源码版本。完整变更见 [CHANGELOG](CHANGELOG.md)，各版本的功能、修复与使用说明见 [Releases](https://github.com/likehbbfoe/MacRDP/releases)。

发行版提供源码 ZIP / tar.gz。下方说明对应持续开发的 `main`；固定标签 `v0.1.0` 保留发行时的 macOS 14+ 要求和配置行为。项目处于 `0.x` 开发阶段，共享当前登录桌面，不创建独立的 macOS 登录会话。

版本采用 `vMAJOR.MINOR.PATCH` 标签：补丁版本用于兼容性修复，次版本用于新功能；`0.x` 阶段的兼容性变化会在发行说明中注明。`main` 持续开发，需要固定版本时使用对应标签。

## 来源与许可证

本项目基于 **[x6nux/macrdp](https://github.com/x6nux/macrdp)** 进行后续开发，初始代码取自上游提交 `6be70bb`。屏幕采集、RDP 服务、编码和管理界面的原始实现来自该项目，感谢原作者及贡献者。

保留上游 **GPLv3** 许可证，详见 [LICENSE](LICENSE)。仓库中的两份 IronRDP 修改版保留各自的 MIT / Apache-2.0 许可证；这些文件并未重新授权为本项目许可证。

本仓库增加或修复：

- Windows App 新版本 GFX 能力协商：跳过未知能力版本，确认双方支持的版本，避免连接后完全黑屏。
- 依据实际编码器选择采集格式，修复硬件初始化失败后回退软件却继续采集 NV12 的问题。
- 软件编码的带补齐尺寸色彩转换加速、复用图像缓冲区；修复 AVC444 行距与双码流关键帧请求。
- 硬件编码身份校验和编码失败后的关键帧恢复。
- 凭据诊断输出脱敏及 TLS-only 空密码认证修复。

## 环境与构建

- macOS 13 或更高版本，已安装 Xcode Command Line Tools。
- 当前稳定版 Rust；前端开发另需 Node.js 20+。
- 运行桌面服务需要“屏幕录制”和“辅助功能”权限。

```sh
git clone https://github.com/likehbbfoe/MacRDP.git
cd MacRDP
cargo build --locked --release -p macrdp-server
cp config.example.toml config.toml
```

编辑 `config.toml`，**替换用户名及密码占位内容**，然后启动：

```sh
./target/release/macrdp-server --config config.toml
```

查看服务端版本：

```sh
./target/release/macrdp-server --version
```

构建 Tauri 管理界面：

```sh
cd macrdp-ui
npm ci
npm run tauri -- build --bundles app
```

在 Windows App 中添加电脑，地址填写服务端的局域网 IP 和端口，例如 `<Mac-IP>:13389`。使用配置中的 RDP 凭据，不是 macOS 登录密码。

## 编码配置

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

| 选项 | 行为 |
| --- | --- |
| `hardware` | 请求 VideoToolbox 硬件 H.264；无法初始化时服务端回退 OpenH264 并使用 BGRA 采集 |
| `software` | OpenH264 CPU 编码，色彩转换使用可用的 Accelerate/vImage 路径 |
| `auto` | 优先使用可用的 VideoToolbox 硬件编码，初始化失败后回退 OpenH264 |
| `avc420` | 单码流，建议优先使用 |
| `avc444` | 主/副双码流，需兼容的客户端，通常消耗更多编码资源和带宽 |

硬件编码使用系统媒体编码资源，不等同于通用 GPU 计算。帧率设置是目标值，实际性能取决于分辨率和工作负载。示例配置提供更多选项。

## 系统与客户端兼容

| 环境 | 主线处理方式 |
| --- | --- |
| macOS 13 Ventura | 构建目标最低版本；使用 ScreenCaptureKit，修改采集帧率需重启服务 |
| macOS 14 及更新版本 | 使用同一份主线代码，可用的系统能力按运行环境启用 |
| Apple Silicon | 尝试 VideoToolbox 低延迟硬件 H.264，优先使用 NV12 直接采集路径 |
| Intel Mac | 按 VideoToolbox 实际返回的硬件能力选择编码模式，不依赖芯片型号名单 |
| 不具备所需硬件编码能力或资源不足 | 自动回退 OpenH264 软件编码；也可主动选择 `software` |
| Windows App / Microsoft Remote Desktop | 按 RDP 能力协商选择 AVC444、AVC420 或传统图像传输，不依赖客户端版本号字符串 |
| 不支持 GFX / AVC 的客户端 | 回退传统位图传输，可用的压缩方式由 RDP 协商决定；仍要求客户端支持 Fast-Path 输出和 32 位色深 |
| macOS 12 及更早版本 | 当前原生采集依赖要求 macOS 13，尚不属于本构建的运行范围 |

服务端 macOS 和客户端 macOS 是两个独立要求：客户端应用能否安装，由 Microsoft 对该版本的要求决定。项目共享当前桌面，不能补充客户端应用自身缺失的系统支持。

硬件编码采用能力探测：低延迟模式不可用时尝试普通硬件会话，随后才回退软件。AVC444 还需客户端支持与足够的双会话编码资源。Apple 的 HEVC、AV1 或 ProRes 硬件能力不会自动转换为本服务的 RDP 输出格式；当前服务输出 H.264/AVC 或传统位图，解码由客户端完成。

## 配置选项

| 配置 | 可选值与含义 |
| --- | --- |
| `encoder` | `auto`（默认）、`hardware`、`software`；旧别名 `vt` / `videotoolbox` / `gpu` 和 `openh264` / `oh264` / `cpu` 继续可用 |
| `chroma_mode` | `avc420`（默认，兼容性优先）或 `avc444`（文字色彩细节优先，需要客户端支持） |
| `quality` | `low_latency`、`balanced`、`high_quality`；影响自动码率目标 |
| `frame_rate` | 1–120；旧机器或低带宽可从 15 / 30 开始 |
| `bitrate_mbps` | 1–1000，单位 Mbps；省略时自动计算，数值不是性能保证 |
| `resolution` | `auto`、明确尺寸（如 `1920x1080`）；兼容旧的整数缩放值 1–4 |
| `width` / `height` | 旧式尺寸配置，应同时为 0 或同时指定；优先使用 `resolution` 设置最终采集尺寸 |
| `show_cursor` | 是否在采集画面中包含光标 |

尺寸每边最大 8192 像素，总像素不超过 7680×4320。服务在启动前拒绝未知编码选项、无效帧率和超范围尺寸，避免配置拼写错误被静默忽略。

推荐配置：

- 日常局域网：`encoder = "auto"`、`chroma_mode = "avc420"`、30 fps、1920×1080。
- 偏重文字色彩：使用 `avc444`，客户端不支持时按协商结果降级。
- 旧机器或硬件资源紧张：`encoder = "software"`、`avc420`、15–30 fps，降低分辨率与码率。

更改采集或编码选项后，重新启动服务并重新连接客户端，使相关配置一起生效。兼容改进优先进入同一主线；只有无法共享实现的系统差异才考虑独立维护的正式版本。

## 已知限制

不同客户端及编码模式的兼容性仍有差异。

4K 动态 AVC444 仍可能出现 VideoToolbox 原生丢帧，暂不保证稳定的 4K/30fps。帧率和码率配置是目标值，硬件资源、画面变化和网络会影响实际表现。

目前仍需完善鼠标点击坐标与拖动、多显示器坐标、动态分辨率及长时间连接稳定性。CLI 与管理界面存在不同的显示处理路径。硬件资源不可用时性能会受到软件回退影响。

## 项目结构

- `crates/macrdp-server`：CLI 服务端。
- `crates/macrdp-core`、`macrdp-ui`：核心服务与 Tauri 管理界面。
- `crates/macrdp-capture`、`macrdp-input`：采集与输入。
- `crates/macrdp-encode`：VideoToolbox、OpenH264 与色彩转换。
- `crates/ironrdp-server-gfx`、`ironrdp-acceptor-patched`：修改后的协议依赖。

## 反馈与贡献

通过 [Issues](https://github.com/likehbbfoe/MacRDP/issues) 报告问题或提出功能建议，请注明 MacRDP 版本、macOS 版本、客户端版本及相关编码配置。请勿公开密码、证书私钥或其他个人信息。代码改进可提交至 `main` 的 Pull Request。

## 参考项目

- [x6nux/macrdp](https://github.com/x6nux/macrdp)：本项目直接基于的原始代码仓库。
- [IronRDP](https://github.com/Devolutions/IronRDP)：Rust RDP 协议实现。
- [FreeRDP](https://github.com/FreeRDP/FreeRDP)：协议与 AVC444 实现参考。
- [OpenH264](https://github.com/cisco/openh264)：软件 H.264 编解码。
- [RustDesk](https://github.com/rustdesk/rustdesk)：上游采集和输入架构的参考项目。
