# Livox SDK2 for Rust（中文版）

[English](README.md) | [简体中文](README.zh-CN.md)

[Livox SDK2](https://github.com/Livox-SDK/Livox-SDK2)（HAP / Mid-360 激光雷达）的 Rust 绑定，
面向 ARM（如 NVIDIA Jetson）与 x86_64。

官方 C++ SDK 被**内置（vendored）并在构建时从源码编译**——无需安装任何系统库。

## 工作区结构

| Crate | 作用 |
|-------|------|
| `livox-sdk2-sys` | 通过 `cc` 编译内置 SDK，`bindgen` 生成 C API 的原始 FFI 绑定。 |
| `livox-sdk2` | 安全高层封装（SDK 生命周期 + 回调 + `Packet` 视图）。 |

## 环境要求

- 无需 CMake：除 C++11 编译器和 pthread 外无系统依赖
- `bindgen` 构建时需要 `libclang`：
  - Ubuntu/Debian：`sudo apt install libclang-dev`
  - JetPack（Jetson）：一般自带，缺了再装 `sudo apt install libclang-dev`

## 基本用法

在 `Cargo.toml` 中添加：

```toml
[dependencies]
livox-sdk2 = "0.1"
```

```rust
use livox_sdk2::Sdk;

fn main() {
    let mut sdk = Sdk::new("mid360_config.json").expect("failed to init SDK");

    // 发现设备 / 验证哪个 IP 是雷达：
    for dev in sdk.devices() {
        println!("{} @ {} (SN {})", dev.type_name(), dev.lidar_ip, dev.sn);
    }

    // 直接获取解析后的点云（xyz 单位米 + 反射率）：
    sdk.set_point_cloud_callback(|handle, dev_type, packet| {
        let cloud: Vec<_> = packet.points();
        println!("lidar {handle} (type {dev_type}): {} points", cloud.len());
    });

    sdk.run();
}
```

## 高层接口

- **`Sdk::devices()`** — 当前所有已连接雷达的快照，含真实 `sn` 与 `lidar_ip`。
- **`Sdk::set_device_change_callback(...)`** — 设备接入或信息变化时通知。
- **`Packet::points()`** — 将原始载荷解析为 `Vec<Point>`（单位米），按包内
  `data_type` 自动选择 Cartesian 高/低精度、球坐标、双回波格式。
- **`Packet::data()`** — 原始载荷字节，用于自定义解析（如 IMU）。

## 示例（也用于新板卡测试）

| 示例 | 用途 |
|------|------|
| `cargo run --release --example version` | 冒烟测试——打印链接的 SDK 版本，无需雷达。 |
| `cargo run --release --example discover -- <config.json>` | 列出发现的雷达及真实 IP/SN。 |
| `cargo run --release --example point_cloud -- <config.json> [秒数]` | 解析并统计每秒点数。 |

## 如何验证哪个 IP 是激光雷达？

SDK 会在 UDP 56000 端口广播探测包。同网段内所有雷达应答并被自动连接，
随后通过设备信息回调上报。**权威判定方法是 `Sdk::devices()`**（或
`set_device_change_callback`），在 `LivoxLidarSdkStart` 之后调用：

- config 里 `lidar_ip` 中列出的 IP 若始终不出现在 `devices()` 里，说明不可达
  （网段不对 / 没接线 / 未上电）。
- 前提：主机网卡必须与雷达同网段并配置静态 IP（如 Mid-360 默认
  `192.168.1.3`，主机配 `192.168.1.5/24`）。
- 可以先用 `ping <候选ip>` 做快速连通性检查，但只有 SDK 的设备报告才能确认
  它是真正的雷达。

## 配置文件说明（`mid360_config.json`）

配置文件是传给 `Sdk::new` / `LivoxLidarSdkInit` 的 JSON。下面以 Mid-360 样例
（`livox-sdk2/examples/mid360_config.json`）为准；`HAP`、`Mid360s`、`Avia2`
各段结构完全相同。

### 结构总览

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

### 全局字段（顶层）

| 字段 | 类型 | 必填 | 默认 | 说明 |
|------|------|------|------|------|
| `master_sdk` | bool | 否 | `true` | `true`=主 SDK（可发控制命令并收数据）；`false`=从 SDK（仅收组播点云）。**同一网段只允许一个主 SDK**。 |
| `lidar_log_enable` | bool | 否 | `false` | 是否开启雷达固件日志（写入文件）。 |
| `lidar_log_cache_size_MB` | uint | 否 | `500` | 日志缓存大小（MB）。仅在 `lidar_log_enable` 存在时被读取。 |
| `lidar_log_path` | string | 否 | `"./"` | 固件日志保存目录。 |

> 注意：`lidar_log_enable` 存在但缺少 `lidar_log_cache_size_MB`/`lidar_log_path`
> 时配置解析会报错；不需要日志时省略整组字段即可（默认禁用）。

### 设备类型段（`MID360` / `HAP` / `Mid360s` / `Avia2`）

#### `lidar_net_info`（雷达侧端口，出厂固定，必填）

| 字段 | 类型 | 必填 | Mid-360 默认 | 说明 |
|------|------|------|------|------|
| `cmd_data_port` | uint | 是 | `56100` | 收发控制命令的端口 |
| `push_msg_port` | uint | 是 | `56200` | 接收推送消息端口 |
| `point_data_port` | uint | 是 | `56300` | 接收点云数据端口 |
| `imu_data_port` | uint | 是 | `56400` | 接收 IMU 数据端口 |
| `log_data_port` | uint | 是 | `56500` | 接收固件日志端口 |

> HAP 的默认端口不同（`56000/57000/58000/59000`，见官方 HAP 文档），字段含义一致。

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

## Jetson / 交叉编译

SDK 可在 `aarch64-linux-gnu` 原生编译；`cargo build`（或在 x86_64 主机装好目标
架构后 `cargo build --target aarch64-unknown-linux-gnu`）即可产出自包含的静态
库，无需 `make install`。

## 许可证

MIT。内置 SDK 保留其自有 MIT 许可证，见 `livox-sdk2-sys/vendor/LICENSE.txt`。
