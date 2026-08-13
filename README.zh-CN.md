# Livox SDK2 for Rust（中文版）

[English](README.md) | [简体中文](README.zh-CN.md)

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.77-red.svg)](https://blog.rust-lang.org/2024/03/21/Rust-1.77.0.html)
[![Rust edition 2021](https://img.shields.io/badge/edition-2021-orange.svg)](https://doc.rust-lang.org/edition-guide/rust-2021/)

官方 [Livox SDK2](https://github.com/Livox-SDK/Livox-SDK2) 的 Rust 绑定，
支持 **HAP** 与 **Mid-360**（含 Mid-360s / Avia2）激光雷达，运行于
**x86_64** 与 **aarch64**（NVIDIA Jetson）。

官方 C++ SDK 被**内置（vendored）并在构建时从源码编译**——无需 `cmake`、
无需 `make install`、无需安装任何系统库，一条 `cargo build` 产出自包含二进制。

> **状态说明**：项目处于积极开发阶段（v0.1.0），1.0 之前 API 可能有变动。
> 欢迎反馈与提 issue。

---

## 目录

1. [特性](#1-特性)
2. [工作区结构](#2-工作区结构)
3. [环境准备](#3-环境准备)
4. [快速上手（无需雷达）](#4-快速上手无需雷达)
5. [连接雷达](#5-连接雷达)
6. [运行示例](#6-运行示例)
7. [在项目中使用本库](#7-在项目中使用本库)
8. [配置文件说明](#8-配置文件说明)
9. [常见问题排查](#9-常见问题排查)
10. [许可证与致谢](#10-许可证与致谢)

---

## 1. 特性

- **零系统依赖**：官方 C++ SDK（MIT）内置在 `livox-sdk2-sys/vendor`，由
  `build.rs` 通过 `cc` crate 直接编译。
- **两层架构**：
  - `livox-sdk2-sys` — 未经修改的 `bindgen` 原始 FFI 绑定（62 个 C 函数）。
  - `livox-sdk2` — 安全高层 API（RAII 生命周期、线程安全回调、对齐安全的
    数据包视图）。
- **设备发现** — SDK 自动发现网络中的雷达，上报每台的真实 `SN` 与
  `lidar_ip`（「哪个 IP 是雷达」的权威答案）。
- **解析后的数据** — 点云（Cartesian 高/低精度、球坐标、双回波）返回
  `Vec<Point>`（单位**米**）；六轴 IMU 返回 `Vec<ImuPoint>`（单位
  **rad/s** 与 **g**）。
- **Jetson 实测** — 已在 JetPack 上验证满速数据流（Mid-360：
  200k 点/秒、200 Hz IMU）。

## 2. 工作区结构

| 路径 | 说明 |
|------|------|
| `livox-sdk2-sys/` | 原始 FFI crate。`build.rs` 编译内置 SDK 并运行 `bindgen`。 |
| `livox-sdk2-sys/vendor/` | 内置的官方 Livox-SDK2 源码（头文件 + 核心 + 3rdparty，MIT）。 |
| `livox-sdk2/` | 安全高层 crate：`Sdk`、`Packet`、`Point`、`ImuPoint`、`DeviceInfo`。 |
| `livox-sdk2/examples/` | 可直接运行的示例，用于上电测试与验证。 |
| `docs/` | 设计笔记（安装方案等）。 |

## 3. 环境准备

支持系统：**Ubuntu 18.04+**（及衍生版），含 Jetson 上的 JetPack 5
（Ubuntu 20.04）/ JetPack 6（Ubuntu 22.04）。架构：x86_64、aarch64。

### 3.1 安装 Rust（≥ 1.77）

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

> 国内网络建议配置 rsproxy/中科大镜像或代理——见[常见问题排查](#9-常见问题排查)。

### 3.2 安装 C++11 编译器与 libclang（bindgen 需要）

```bash
sudo apt update
sudo apt install -y g++ clang libclang-dev
```

- `g++` — 编译内置的 C++ SDK。
- `clang` + `libclang-dev` — 仅在构建期供 `bindgen` 使用。

Jetson / JetPack：`g++` 已预装；`clang libclang-dev` 缺失时再装。

### 3.3 验证工具链

```bash
rustc --version   # 期望 >= 1.77
g++ --version     # 任意现代版本均可
clang --version   # 如 14/15/17/18
```

## 4. 快速上手（无需雷达）

```bash
git clone git@github.com:DoraCN/livox-sdk2.git
cd livox-sdk2

# 首次构建会下载依赖并编译内置 SDK（aarch64 约 3-5 分钟）
cargo build --release

# 冒烟测试：打印所链接的 SDK 版本。无需硬件、无需配置文件。
cargo run --release --example version
```

预期输出：

```
Livox SDK2 version: 1.4.3
```

能看到这行，说明工具链、内置 SDK 编译、FFI 链接在你的机器上全部正常。

### 单元测试（同样无需硬件）

```bash
cargo test
```

期望 `7 passed`（点云与 IMU 解析器，含字节布局与球坐标→直角坐标换算测试）。

## 5. 连接雷达

### 5.1 硬件

- 给雷达供电（Mid-360：9–36 V DC，如使用 Livox 电源适配器）。
- 用网线将雷达的以太网口**直连到主机的一个网口**。

### 5.2 网络

雷达出厂为 `192.168.1.x` 网段的静态 IP（Mid-360 默认 `192.168.1.3`）。
主机网卡必须与雷达同网段：

```bash
ip addr                                # 列出网卡及其 IP
sudo ip addr add 192.168.1.5/24 dev eth0   # 临时配置（重启失效）
```

Ubuntu/JetPack 上持久化（netplan）：

```bash
sudo tee /etc/netplan/99-lidar.yaml <<'EOF'
network:
  version: 2
  ethernets:
    eth0:
      addresses:
        - 192.168.1.5/24
EOF
sudo netplan apply
```

> 网卡名以 `ip addr` 实际输出为准（如 `eth0`、`eno1`）。也可以保留 DHCP
> 地址，并把 config 里的 `host_ip` 改成该地址——见
> [§8](#8-配置文件说明)。

### 5.3 连通性检查（可选，快速验证）

```bash
ping 192.168.1.3          # 或: fping -g 192.168.1.1 192.168.1.254 -a
```

能 ping 通的 IP 说明在线——但「哪个 IP 是雷达」的**权威**确认来自 SDK 本身
（下一步）。

## 6. 运行示例

所有示例的第一个参数都是配置文件（`livox-sdk2/examples/mid360_config.json`
是现成的 Mid-360 样例）。

### 6.1 设备发现

```bash
cargo run --release --example discover -- livox-sdk2/examples/mid360_config.json
```

预期输出（设备信息因机而异）：

```
[info] Init livox lidars succ.  [device_manager.cpp] [Init] [178]
SDK started, waiting 5 s for device discovery ...
device: handle=1895934144 type=9 (Mid-360) SN=47MDMBE0030413 IP=192.168.1.113
```

打印出的 `IP` 就是雷达的真实地址，`SN` 是机身序列号。没出现在列表里的 IP
**不是**雷达（路由器/其他主机/不可达）。

### 6.2 点云

```bash
cargo run --release --example point_cloud -- livox-sdk2/examples/mid360_config.json 10
```

预期输出：

```
lidar 1895934144 type 9: 200064 points/s
lidar 1895934144 type 9: 200160 points/s
...
```

Mid-360 满速约 20 万点/秒。数值略低正常（视场/环境相关）；为 0 说明雷达
没发数据（见[常见问题排查](#9-常见问题排查)）。

### 6.3 IMU 数据

```bash
cargo run --release --example imu -- livox-sdk2/examples/mid360_config.json 10
```

预期输出：

```
lidar 1895934144 type 9: 201 imu samples/s
  latest: gyro=(0.0115, -0.0102, -0.3691) rad/s acc=(-0.0383, 0.0115, 0.9966) g
```

静止时 `acc_z ≈ 1.0 g`（重力）、gyro 在 0 附近波动——静止雷达的物理正确值。

## 7. 在项目中使用本库

```toml
[dependencies]
# crate 尚未发布到 crates.io 之前，用 path 或 git 依赖：
livox-sdk2 = { path = "/path/to/livox-sdk2/livox-sdk2" }
# 或
livox-sdk2 = { git = "https://github.com/DoraCN/livox-sdk2.git" }
# 发布后：
# livox-sdk2 = "0.1"
```

```rust
use livox_sdk2::{ImuPoint, Packet, Point, Sdk};

fn main() -> Result<(), String> {
    let mut sdk = Sdk::new("mid360_config.json")?;

    // 已发现的设备（IP + SN）：
    for dev in sdk.devices() {
        println!("{} @ {} (SN {})", dev.type_name(), dev.lidar_ip, dev.sn);
    }

    // 解析后的点云，xyz 单位米：
    sdk.set_point_cloud_callback(|handle, dev_type, packet| {
        let cloud: Vec<Point> = packet.points();
        println!("lidar {handle} (type {dev_type}): {} points", cloud.len());
    });

    // 解析后的六轴 IMU，gyro 单位 rad/s，accel 单位 g：
    sdk.set_imu_callback(|_handle, _dev_type, packet| {
        let imu: Vec<ImuPoint> = packet.imu_points();
        if let Some(s) = imu.first() {
            println!("acc_z = {} g", s.acc_z);
        }
    });

    // 设备接入/信息变化通知：
    sdk.set_device_change_callback(|dev| println!("device change: {dev:?}"));

    sdk.run() // 阻塞运行；drop 时自动反初始化 SDK
}
```

### API 一览

| API | 说明 |
|-----|------|
| `Sdk::new(config_path)` | 初始化并启动 SDK（RAII：`Drop` 自动反初始化）。 |
| `Sdk::devices()` | 已连接雷达快照（`DeviceInfo { handle, dev_type, sn, lidar_ip }`）。 |
| `Sdk::set_device_change_callback(...)` | 设备接入/信息变化通知。 |
| `Sdk::set_point_cloud_callback(...)` | 逐包点云回调（参数 `Packet`）。 |
| `Sdk::set_imu_callback(...)` | 逐包 IMU 回调（参数 `Packet`，数据类型为 IMU）。 |
| `Sdk::set_info_callback(...)` | SDK 文本状态消息。 |
| `Packet::points()` | 解析载荷为 `Vec<Point>`（按 `data_type` 自动选格式）。 |
| `Packet::imu_points()` | 解析载荷为 `Vec<ImuPoint>`。 |
| `Packet::data()` | 原始载荷字节，供自定义解析。 |
| `Packet::timestamp() / dot_num() / ...` | 头部字段的无对齐安全读取。 |
| `sdk_version()` | 所链接 SDK 版本 `(major, minor, patch)`，无需初始化。 |

## 8. 配置文件说明

传给 `Sdk::new` 的 JSON 配置（完整参数说明）。`MID360`、`HAP`、
`Mid360s`、`Avia2` 各段结构完全相同。

### 结构总览

```json
{
  "master_sdk": true,              // [可选] 全局
  "lidar_log_enable": false,       // [可选] 全局
  "lidar_log_cache_size_MB": 500,  // [可选] 全局
  "lidar_log_path": "./",          // [可选] 全局

  "MID360": {                       // 设备类型段
    "lidar_net_info": { ... },      // 雷达侧端口（出厂默认）
    "host_net_info": [ ... ]        // 主机侧配置（数组或单个对象）
  }
}
```

### 全局字段（顶层）

| 字段 | 类型 | 必填 | 默认 | 说明 |
|------|------|------|------|------|
| `master_sdk` | bool | 否 | `true` | `true`=主 SDK（可发控制命令并收数据）；`false`=从 SDK（仅收组播点云）。**同一网段只允许一个主 SDK**。 |
| `lidar_log_enable` | bool | 否 | `false` | 是否开启雷达固件日志（写入文件）。 |
| `lidar_log_cache_size_MB` | uint | 否 | `500` | 日志缓存大小（MB）。仅在 `lidar_log_enable` 存在时被读取。 |
| `lidar_log_path` | string | 否 | `"./"` | 固件日志保存目录。 |

> `lidar_log_enable` 存在时，`lidar_log_cache_size_MB` 与 `lidar_log_path`
> 变为必填，否则解析报错。不需要日志就整组省略（默认禁用）。

### `lidar_net_info`（雷达侧端口——出厂默认，必填）

| 字段 | 类型 | 必填 | Mid-360 默认 | 说明 |
|------|------|------|------|------|
| `cmd_data_port` | uint | 是 | `56100` | 收发控制命令的端口 |
| `push_msg_port` | uint | 是 | `56200` | 接收推送消息端口 |
| `point_data_port` | uint | 是 | `56300` | 接收点云数据端口 |
| `imu_data_port` | uint | 是 | `56400` | 接收 IMU 数据端口 |
| `log_data_port` | uint | 是 | `56500` | 接收固件日志端口 |

> HAP 出厂默认端口不同（`56000/57000/58000/59000`，见官方 HAP 文档），字段含义一致。

### `host_net_info`（主机侧配置）

两种写法，SDK 都能解析：

- **数组（新写法，推荐）**：每个元素是一个主机；元素可带 `lidar_ip` 列表。
- **对象（旧写法）**：单个主机，自动发现模式（无 `lidar_ip`）。

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `host_ip` | string | 是 | 本机（Jetson）网卡 IP，SDK 绑定端口和接收数据都依赖它。**必须已配置在该网卡上**，否则报 `bind failed`。 |
| `lidar_ip` | string[] | 否 | 要连接的雷达 IP 列表。**省略 = 广播自动发现**（SDK 在 UDP 56000 发探测包，雷达自动应答接入）；填写 = 直连指定 IP。 |
| `multicast_ip` | string | 否 | 点云/IMU 组播地址（如 `224.1.1.5`），多主机共享数据场景使用。 |
| `cmd_data_port` | uint | 是 | 主机侧控制端口（如 `56101`） |
| `push_msg_port` | uint | 是 | 主机侧推送消息端口（如 `56201`） |
| `point_data_port` | uint | 是 | 主机侧点云端口（如 `56301`） |
| `imu_data_port` | uint | 是 | 主机侧 IMU 端口（如 `56401`） |
| `log_data_port` | uint | 是 | 主机侧日志端口（如 `56501`） |

> `lidar_ip` 可用别名 `cmd_data_ip` 替代（此时 `host_ip` 可省略）。两者同时
> 存在时以 `host_ip` 为准。

### 快速核对清单（Jetson 部署）

1. `host_ip` 必须是 Jetson 网卡**实际已配置**的 IP（`ip addr` 确认），否则报
   `bind failed` / `Create detection socket failed`。
2. 雷达与 Jetson 同网段（Mid-360 默认 `192.168.1.x`）。
3. 单雷达单主机：保持样例 `host_net_info` 数组写法、省略 `lidar_ip` 即可自动发现。
4. 多雷达：在 `lidar_ip` 里列出各雷达 IP，或按官方协议配置组播。

## 9. 常见问题排查

| 现象 | 原因 | 解决 |
|------|------|------|
| `error: no example target named 'version'` | 代码过旧或目录不对 | `git pull`；在工作区根目录运行 cargo。 |
| `bind failed` / `Create detection socket failed` | config 的 `host_ip` 未配置在任何网卡上 | 加 IP（`sudo ip addr add ...`）或把 `host_ip` 改成网卡实际 IP。 |
| `No LiDAR found.` | 网段不对 / 网线 / 供电 / 雷达关机 | 检查 `ip addr`、网线与供电；`ping`/`fping` 扫描网段；用 discover 示例确认。 |
| `libclang ... not found` / bindgen 构建报错 | 缺 clang 工具链 | `sudo apt install clang libclang-dev`；必要时设 `LIBCLANG_PATH`。 |
| cargo 无法下载依赖（国内） | 网络受限 | 配置代理（`export https_proxy=http://192.168.0.1:7890 http_proxy=...`）或在 `.cargo/config.toml` 配置 rsproxy 镜像。 |
| `Create channel failed` / 端口被占用 | 其他 SDK/进程占用相同端口 | 结束冲突进程或修改 config 端口。 |
| 能发现雷达但点数为 0 | 雷达未发数据（Mid-360 默认开启发送，此情况少见） | 检查 FOV/扫描模式；持续存在联系 Livox 支持。 |

## 10. 许可证与致谢

- 本项目以 **MIT** 协议开源（[LICENSE](LICENSE)）。
- 内置的官方 SDK 保留其自有 MIT 许可证，见
  `livox-sdk2-sys/vendor/LICENSE.txt`。
- 协议细节与硬件由 [Livox](https://www.livoxtech.com/) 提供——参见
  [Livox-SDK2 仓库](https://github.com/Livox-SDK/Livox-SDK2)与
  [Mid-360 协议 wiki](https://livox-wiki-en.readthedocs.io/en/latest/tutorials/new_product/mid360/mid360.html)。
