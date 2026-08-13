# livox-sdk2

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.77-red.svg)](https://blog.rust-lang.org/2024/03/21/Rust-1.77.0.html)
[![Rust edition 2021](https://img.shields.io/badge/edition-2021-orange.svg)](https://doc.rust-lang.org/edition-guide/rust-2021/)

Safe, high-level Rust bindings for the official
[Livox SDK2](https://github.com/Livox-SDK/Livox-SDK2), supporting **HAP** and
**Mid-360** (also Mid-360s / Avia2) LiDARs on **x86_64** and **aarch64**
(NVIDIA Jetson).

The official C++ SDK is **vendored and compiled from source at build time** by
the underlying [`livox-sdk2-sys`](https://crates.io/crates/livox-sdk2-sys)
crate — no `cmake`, no `make install`, no system library install. A single
`cargo build` produces a self-contained binary.

> **Status note**: active development (v0.1.0). The API may change before 1.0.

## Features

- **RAII SDK lifecycle** — `Sdk::new()` initializes and starts the SDK;
  dropping it uninitializes cleanly.
- **Device discovery** — `Sdk::devices()` reports every LiDAR the SDK has
  connected, with its real `SN` and `lidar_ip` (the authoritative answer to
  "which IP is a LiDAR?").
- **Parsed point clouds** — `Packet::points()` returns `Vec<Point>`
  (x/y/z in **meters**, reflectivity, tag), auto-selecting Cartesian
  high/low, spherical, or double-echo format from the packet's `data_type`.
- **Parsed IMU** — `Packet::imu_points()` returns `Vec<ImuPoint>`
  (gyro in **rad/s**, accel in **g**, per the official protocol).
- **Unaligned-safe packet views** — the raw C structs are `#[repr(packed)]`;
  `Packet` reads every field with unaligned-safe copies.
- **Tested on Jetson** — full-rate streams verified on JetPack (Mid-360:
  200k points/s, 200 Hz IMU).

## Requirements

- Rust ≥ 1.77 (MSRV)
- A C++11 compiler (`g++` or `clang++`) and `libclang` at **build time**
  (used by `bindgen`):
  ```bash
  sudo apt install -y g++ clang libclang-dev
  ```
- OS: Ubuntu 18.04+ (JetPack 5/6 included). Architectures: x86_64, aarch64.

## Installation

```toml
[dependencies]
livox-sdk2 = "0.1"
```

## Usage

```rust
use livox_sdk2::{ImuPoint, Packet, Point, Sdk};

fn main() -> Result<(), String> {
    let mut sdk = Sdk::new("mid360_config.json")?;

    // Discovered devices (real IP + SN):
    for dev in sdk.devices() {
        println!("{} @ {} (SN {})", dev.type_name(), dev.lidar_ip, dev.sn);
    }

    // Parsed point cloud, x/y/z in meters:
    sdk.set_point_cloud_callback(|handle, dev_type, packet| {
        let cloud: Vec<Point> = packet.points();
        println!("lidar {handle} (type {dev_type}): {} points", cloud.len());
    });

    // Parsed 6-axis IMU, gyro in rad/s, accel in g:
    sdk.set_imu_callback(|_handle, _dev_type, packet| {
        let imu: Vec<ImuPoint> = packet.imu_points();
        if let Some(s) = imu.first() {
            println!("acc_z = {} g", s.acc_z);
        }
    });

    // Notified whenever a device connects / its info changes:
    sdk.set_device_change_callback(|dev| println!("device change: {dev:?}"));

    sdk.run() // blocks; uninitializes the SDK on drop
}
```

### API overview

| API | Description |
|-----|-------------|
| `Sdk::new(config_path)` | Initialize + start the SDK (RAII: `Drop` uninitializes). |
| `Sdk::devices()` | Snapshot of connected LiDARs (`DeviceInfo { handle, dev_type, sn, lidar_ip }`). |
| `Sdk::set_device_change_callback(...)` | Device connect/info-change notifications. |
| `Sdk::set_point_cloud_callback(...)` | Per-packet callback with `Packet`. |
| `Sdk::set_imu_callback(...)` | Per-packet callback with `Packet` (IMU data type). |
| `Sdk::set_info_callback(...)` | Text status messages from the SDK. |
| `Packet::points()` | Parse payload into `Vec<Point>` (format auto-selected by `data_type`). |
| `Packet::imu_points()` | Parse payload into `Vec<ImuPoint>`. |
| `Packet::data()` | Raw payload bytes for custom parsing. |
| `Packet::timestamp() / dot_num() / ...` | Raw header fields via unaligned-safe reads. |
| `sdk_version()` | Linked SDK version `(major, minor, patch)` — no init required. |

## More documentation

The project repository contains a complete guide, including:

- Step-by-step hardware & network setup (static IP, netplan)
- Runnable bring-up examples (`discover`, `point_cloud`, `imu`) with
  expected outputs
- Full `mid360_config.json` parameter reference
- Troubleshooting table

See the [repository README](https://github.com/DoraCN/livox-sdk2#readme)
(also available in [中文](https://github.com/DoraCN/livox-sdk2/blob/main/README.zh-CN.md)).

## Safety

The raw C API lives in `livox-sdk2-sys`; this crate wraps it so that:

- the SDK lifecycle is enforced by RAII (no double-init/use-after-uninit),
- packed C structs are only ever read via unaligned-safe accesses,
- callbacks are dispatched through mutex-protected registries (safe to
  register from any thread before use).

The one deliberate low-level escape hatch is `Packet::data()`, which returns
the raw payload bytes — treat them as untrusted sensor input.

## License

MIT. The vendored SDK keeps its own MIT license (see `livox-sdk2-sys`).
