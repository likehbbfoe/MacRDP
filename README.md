# MacRDP

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
[![macOS](https://img.shields.io/badge/macOS-14%2B-black.svg)](https://www.apple.com/macos/)

中文 | [English](README_EN.md)

macOS 原生 RDP 服务端，可使用另一台 Mac 上的 Windows App 连接当前桌面。本仓库关注客户端兼容性、编码性能与稳定性。

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

- macOS 14 或更高版本，已安装 Xcode Command Line Tools。
- 当前稳定版 Rust；前端开发另需 Node.js 20+。
- 运行桌面服务需要“屏幕录制”和“辅助功能”权限。

```sh
git clone https://github.com/likehbbfoe/MacRDP.git
cd MacRDP
cargo build --release -p macrdp-server
cp config.example.toml config.toml
```

编辑 `config.toml`，**替换用户名及密码占位内容**，然后启动：

```sh
./target/release/macrdp-server --config config.toml
```

在 Windows App 中添加电脑，地址填写服务端的局域网 IP 和端口，例如 `<Mac-IP>:13389`。使用配置中的 RDP 凭据，不是 macOS 登录密码。

## 编码配置

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

| 选项 | 行为 |
| --- | --- |
| `hardware` | 请求 VideoToolbox 硬件 H.264；无法初始化时服务端回退 OpenH264 并使用 BGRA 采集 |
| `software` | OpenH264 CPU 编码，色彩转换使用可用的 Accelerate/vImage 路径 |
| `auto` | 当前保持与 `software` 相同的行为 |
| `avc420` | 单码流，建议优先使用 |
| `avc444` | 主/副双码流，需兼容的客户端，通常消耗更多编码资源和带宽 |

硬件编码使用系统媒体编码资源，不等同于通用 GPU 计算。帧率设置是目标值，实际性能取决于分辨率和工作负载。示例配置提供更多选项。

## 已知限制

不同客户端及编码模式的兼容性仍有差异。

4K 动态 AVC444 仍可能出现 VideoToolbox 原生丢帧，暂不保证稳定的 4K/30fps。

目前仍需完善非 AVC 客户端回退、鼠标点击坐标与拖动、多显示器坐标、动态分辨率及长时间连接稳定性。CLI 与管理界面存在不同的显示处理路径。硬件资源不可用时性能会受到软件回退影响。

## 项目结构

- `crates/macrdp-server`：CLI 服务端。
- `crates/macrdp-core`、`macrdp-ui`：核心服务与 Tauri 管理界面。
- `crates/macrdp-capture`、`macrdp-input`：采集与输入。
- `crates/macrdp-encode`：VideoToolbox、OpenH264 与色彩转换。
- `crates/ironrdp-server-gfx`、`ironrdp-acceptor-patched`：修改后的协议依赖。

## 参考项目

- [x6nux/macrdp](https://github.com/x6nux/macrdp)：本项目直接基于的原始代码仓库。
- [IronRDP](https://github.com/Devolutions/IronRDP)：Rust RDP 协议实现。
- [FreeRDP](https://github.com/FreeRDP/FreeRDP)：协议与 AVC444 实现参考。
- [OpenH264](https://github.com/cisco/openh264)：软件 H.264 编解码。
- [RustDesk](https://github.com/rustdesk/rustdesk)：上游采集和输入架构的参考项目。
