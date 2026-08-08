# DiskScry

[English](./README.md) | 简体中文

[![CI](https://github.com/ZHOUSJ6/DiskScry/actions/workflows/ci.yml/badge.svg)](https://github.com/ZHOUSJ6/DiskScry/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/ZHOUSJ6/DiskScry)](https://github.com/ZHOUSJ6/DiskScry/releases/latest)

DiskScry 是一个只读的 Rust CLI/TUI 工具，用于查看物理硬盘和 SMART 健康信息。它以接近 CrystalDiskInfo 的可读形式呈现 ATA 与 NVMe 数据，同时保留设备返回的精确计数器和原始值。

`v0.1.0` 开发者预览版支持 Apple Silicon 和 Intel Mac。Linux 支持尚在计划中，Windows 暂不考虑。

## 功能

- 通过 macOS 原生 API 发现内置和外置物理硬盘
- 不依赖 smartmontools，直接读取 ATA 和 NVMe SMART 数据
- 无法读取 SMART 时仍然显示外置硬盘
- 明确区分 `Available`、`Unavailable` 和 `Failed` 三种 SMART 状态
- 同时显示可读指标、十六进制指标代码、人类可读单位和精确原始数据
- CLI、TUI、帮助和诊断信息支持英文与简体中文
- 仅在当前会话内记录温度样本，不写入硬盘
- 通过 Disk Arbitration 响应硬盘插入和移除事件

## 安装

### 使用 Cargo

需要先安装 Rust 和 macOS Command Line Tools：

```bash
cargo install --git https://github.com/ZHOUSJ6/DiskScry --tag v0.1.0 --locked
```

### 使用预编译的 macOS 二进制文件

从 [v0.1.0 Release](https://github.com/ZHOUSJ6/DiskScry/releases/tag/v0.1.0) 下载与 Mac 对应的压缩包：

| Mac | 压缩包 |
| --- | --- |
| Apple Silicon | `diskscry-v0.1.0-aarch64-apple-darwin.tar.gz` |
| Intel | `diskscry-v0.1.0-x86_64-apple-darwin.tar.gz` |

解压后，将 `diskscry` 放入 `PATH` 中的目录：

```bash
tar -xzf diskscry-v0.1.0-aarch64-apple-darwin.tar.gz
mkdir -p "$HOME/.local/bin"
install -m 755 diskscry "$HOME/.local/bin/diskscry"
```

开发者预览版二进制文件尚未签名，也未经过 Apple 公证。安装前请使用 Release 中的 `SHA256SUMS` 文件校验下载的压缩包。

## 使用方法

不带子命令运行 `diskscry` 会打开 TUI：

```bash
diskscry
```

| 命令 | 说明 |
| --- | --- |
| `diskscry list [--json]` | 列出物理硬盘 |
| `diskscry show <device> [--json]` | 按设备 ID 或设备节点显示一块硬盘 |
| `diskscry watch [<device>]` | 持续刷新硬盘信息 |

全局选项可以放在子命令之前或之后：

| 选项 | 说明 |
| --- | --- |
| `--lang <en\|zh-CN>` | 指定人机界面语言 |
| `--interval <seconds>` | 设置 SMART 刷新间隔；`0` 表示关闭定时刷新 |

示例：

```bash
diskscry --lang zh-CN
diskscry list --json
diskscry show /dev/disk0
diskscry watch /dev/disk0 --interval 30
```

## SMART 状态

硬盘发现与 SMART 读取相互独立。无论传输接口是否提供 SMART，所有已发现的物理硬盘都会保留在列表中。

| 状态 | 含义 |
| --- | --- |
| `Available` | SMART 数据读取并解析成功 |
| `Unavailable` | 设备或公开传输接口没有提供可读取的 SMART 数据 |
| `Failed` | DiskScry 尝试通过受支持的接口读取，但遇到权限、原生 API 或解析错误 |

无法读取或读取失败的设备在中文界面显示 `SMART 不可用`，英文界面显示 `SMART unavailable`。缺失的数据不会被判定为健康。

可读视图为 NVMe 指标显示 `01` 至 `0F` 的代码，为 ATA 属性显示设备返回的 ID，例如 `05` 和 `C5`。在 TUI 中按 `v` 可以查看未经修改的原始 JSON 表示。

## 语言选择

DiskScry 按以下顺序选择界面语言：

1. `--lang en` 或 `--lang zh-CN`
2. `LC_ALL`
3. `LANG`
4. 默认使用英文

命令名、选项名、设备标识符、JSON 键和 JSON 枚举值始终使用与语言无关的英文。

## TUI 按键

| 按键 | 操作 |
| --- | --- |
| `j` / `k`、`↑` / `↓` | 选择硬盘 |
| `Tab` | 切换概览、SMART 和本次会话页面 |
| `v` | 切换可读 SMART 与原始 JSON 视图 |
| `PageUp` / `PageDown` | 按页滚动 SMART 数据 |
| `Home` / `End` | 跳到 SMART 数据开头或末尾 |
| `r` | 立即刷新 |
| `q` | 退出 |

## 安全性与限制

- DiskScry 只使用只读原生接口，不会向设备发送写入命令。
- DiskScry 不会调用 `sudo`、修改权限或在失败后静默切换读取后端。
- 读取 SMART 可能唤醒休眠的机械硬盘。TUI 和 `watch` 默认每 60 秒刷新一次。
- 部分 USB 转接桥不会通过 macOS 公开接口提供 SMART；这些硬盘仍会显示，但会明确标记为 SMART 不可用。
- 开发者预览版不包含历史数据持久化、通知、自检、特权辅助程序、签名和公证。

DiskScry 使用 CrystalDiskInfo 9.9.1 作为固定版本的通用 ATA 属性名称参考，但不会链接或打包 CrystalDiskInfo 代码。第三方归属信息见 [THIRD_PARTY_LICENSES.md](./THIRD_PARTY_LICENSES.md)。

## 开发

通过 Cargo 运行时，应用参数必须放在 `--` 分隔符之后：

```bash
cargo run -- --lang zh-CN
```

在仓库根目录执行完整质量检查：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo build --release --target aarch64-apple-darwin --locked
cargo build --release --target x86_64-apple-darwin --locked
```
