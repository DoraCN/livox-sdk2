# Livox SDK2 for Rust

Rust bindings for the [Livox SDK2](https://github.com/Livox-SDK/Livox-SDK2)
(HAP / Mid-360 LiDARs), designed for ARM (e.g. NVIDIA Jetson) and x86_64.

The official C++ SDK is **vendored and compiled from source at build time** —
no system library install needed.

## Workspace layout

| Crate | Role |
|-------|------|
| `livox-sdk2-sys` | Raw `bindgen` FFI bindings to the C API; builds the vendored SDK with `cc`. |
| `livox-sdk2` | Safe, high-level wrapper (SDK lifecycle + callbacks + `Packet` view). |

## Prerequisites

- CMake-free: no system deps beyond a C++11 compiler and pthread
- `bindgen` requires `libclang` at build time:
  - Ubuntu/Debian: `sudo apt install libclang-dev`
  - JetPack (Jetson): usually preinstalled; else `sudo apt install libclang-dev`

## Usage

Add to `Cargo.toml`:

```toml
[dependencies]
livox-sdk2 = "0.1"
```

```rust
use livox_sdk2::Sdk;

fn main() {
    let mut sdk = Sdk::new("mid360_config.json").expect("failed to init SDK");

    // Discover devices / verify which IPs are LiDARs:
    for dev in sdk.devices() {
        println!("{} @ {} (SN {})", dev.type_name(), dev.lidar_ip, dev.sn);
    }

    // Get parsed point clouds directly (x/y/z in meters + reflectivity):
    sdk.set_point_cloud_callback(|handle, dev_type, packet| {
        let cloud: Vec<_> = packet.points();
        println!("lidar {handle} (type {dev_type}): {} points", cloud.len());
    });

    sdk.run();
}
```

## High-level API

- **`Sdk::devices()`** — snapshot of every LiDAR the SDK has connected, each
  with its real `sn` and `lidar_ip`.
- **`Sdk::set_device_change_callback(...)`** — notified whenever a device
  connects or its info changes.
- **`Packet::points()`** — parses the raw payload into `Vec<Point>` (meters),
  auto-selecting Cartesian high/low, spherical, or double-echo format from the
  packet's `data_type`.
- **`Packet::data()`** — raw payload bytes for custom parsing (e.g. IMU).

## Which IP is a LiDAR?

The SDK broadcasts a detection packet on UDP port `56000`. Every LiDAR on the
same subnet replies and the SDK connects to it, then reports it through the
device-info callback. The **authoritative** way to know which IPs are LiDARs is
therefore `Sdk::devices()` (or `set_device_change_callback`) after
`LivoxLidarSdkStart`:

- If an IP listed in your config's `lidar_ip` never shows up in `devices()`,
  it is not a reachable LiDAR (wrong subnet, no cable, or powered off).
- Prerequisites: the host NIC must have a static IP on the same subnet as the
  LiDAR (e.g. `192.168.1.5/24` for Mid-360 default `192.168.1.3`).
- You can also `ping <candidate-ip>` as a cheap reachability check, but only
  the SDK's device report confirms it is actually a LiDAR.

## Config file: `mid360_config.json` parameter reference

The config is a JSON file passed to `Sdk::new` / `LivoxLidarSdkInit`. This
reference covers the Mid-360 sample (`livox-sdk2/examples/mid360_config.json`);
the structure is identical for `HAP`, `Mid360s` and `Avia2` sections.

### Structure overview

```json
{
  "master_sdk": true,              // [可选] 全局
  "lidar_log_enable": false,       // [可选] 全局
  "lidar_log_cache_size_MB": 500,  // [可选] 全局
  "lidar_log_path": "./",          // [可选] 全局

  "MID360": {                       // 设备类型段：MID360 / HAP / Mid360s / Avia2
    "lidar_net_info": { ... },      // 雷达侧端口（出厂默认，一般不改）
    "host_net_info": [ ... ]        // 主机侧配置（数组或单个对象）
  }
}
```

### Global (top-level) fields

| 字段 | 类型 | 必填 | 默认 | 说明 |
|------|------|------|------|------|
| `master_sdk` | bool | 否 | `true` | `true`=主 SDK（可发控制命令并收数据）；`false`=从 SDK（仅收组播点云）。**同一网段只允许一个主 SDK**。 |
| `lidar_log_enable` | bool | 否 | `false` | 是否开启雷达固件日志（写入文件）。 |
| `lidar_log_cache_size_MB` | uint | 否 | `500` | 日志缓存大小（MB）。仅在 `lidar_log_enable` 存在时被读取。 |
| `lidar_log_path` | string | 否 | `"./"` | 固件日志保存目录。 |

> 注意：SDK 中 `lidar_log_enable` 存在但缺少 `lidar_log_cache_size_MB`/`lidar_log_path`
> 时，配置解析会报错；若不需要日志，直接省略整组字段即可（默认禁用）。

### Device-type section（`MID360` / `HAP` / `Mid360s` / `Avia2`）

#### `lidar_net_info`（雷达侧端口，出厂固定，必填）

| 字段 | 类型 | 必填 | Mid-360 默认 | 说明 |
|------|------|------|------|------|
| `cmd_data_port` | uint | 是 | `56100` | 收发控制命令的端口 |
| `push_msg_port` | uint | 是 | `56200` | 接收推送消息端口 |
| `point_data_port` | uint | 是 | `56300` | 接收点云数据端口 |
| `imu_data_port` | uint | 是 | `56400` | 接收 IMU 数据端口 |
| `log_data_port` | uint | 是 | `56500` | 接收固件日志端口 |

> HAP 的默认端口不同（`56000/57000/58000/59000`，见官方 HAP 文档），其他段参数含义一致。

#### `host_net_info`（主机侧配置）

两种写法，SDK 都能解析：

- **数组（新写法，推荐）**：每一项是一个主机，可多主机并存；可带 `lidar_ip` 列表。
- **对象（旧写法）**：单个主机，自动发现模式（无 `lidar_ip`）。

数组项字段：

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `host_ip` | string | 是 | 本机（Jetson）网卡 IP，SDK 绑定端口和接收数据都依赖它。**必须已配置在该网卡上**，否则 bind 失败。 |
| `lidar_ip` | string[] | 否 | 要连接的雷达 IP 列表。**省略 = 广播自动发现**（SDK 在 UDP 56000 发探测包，雷达自动应答接入）；填写 = 直连指定 IP。 |
| `multicast_ip` | string | 否 | 点云/IMU 组播地址（如 `224.1.1.5`）。用于多主机共享数据的场景。 |
| `cmd_data_port` | uint | 是 | 主机侧控制端口（如 `56101`） |
| `push_msg_port` | uint | 是 | 主机侧推送消息端口（如 `56201`） |
| `point_data_port` | uint | 是 | 主机侧点云端口（如 `56301`） |
| `imu_data_port` | uint | 是 | 主机侧 IMU 端口（如 `56401`） |
| `log_data_port` | uint | 是 | 主机侧日志端口（如 `56501`） |

> `lidar_ip` 也可用别名 `cmd_data_ip`（此时可省略 `host_ip`）。`host_ip` 与
> `cmd_data_ip` 同时存在时以 `host_ip` 为准。

### 快速核对清单（Jetson 部署）

1. `host_ip` = Jetson 网卡**实际已配置**的 IP（`ip addr` 确认），否则报 `bind failed`。
2. 雷达与 Jetson 同网段（Mid-360 默认 `192.168.1.x`）。
3. 单雷达单主机：保持样例中的 `host_net_info` 数组写法、省略 `lidar_ip` 即可自动发现。
4. 多雷达：在 `lidar_ip` 里列出各雷达 IP，或按官方协议配置组播。

## Building on Jetson / cross compiling

The SDK compiles natively on `aarch64-linux-gnu`; a `cargo build` (or `cargo
build --target aarch64-unknown-linux-gnu` from an x86_64 host with the target
installed) produces a self-contained static archive — no `make install`
required.

## License

MIT. The vendored SDK keeps its own MIT license under
`livox-sdk2-sys/vendor/LICENSE.txt`.
