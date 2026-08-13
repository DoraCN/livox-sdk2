# Livox SDK2 for Rust

[English](README.md) | [简体中文](README.zh-CN.md)

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.77-red.svg)](https://blog.rust-lang.org/2024/03/21/Rust-1.77.0.html)
[![Rust edition 2021](https://img.shields.io/badge/edition-2021-orange.svg)](https://doc.rust-lang.org/edition-guide/rust-2021/)

Rust bindings for the official [Livox SDK2](https://github.com/Livox-SDK/Livox-SDK2),
supporting **HAP** and **Mid-360** (also Mid-360s / Avia2) LiDARs on
**x86_64** and **aarch64** (NVIDIA Jetson).

The official C++ SDK is **vendored and compiled from source at build time** —
no `cmake`, no `make install`, no system library install. A single
`cargo build` produces a self-contained binary.

> **Status note**: this project is under active development (v0.1.0). The API
> may change before 1.0. Feedback and issues are welcome.

---

## Table of contents

1. [Features](#1-features)
2. [Workspace layout](#2-workspace-layout)
3. [Prerequisites](#3-prerequisites)
4. [Getting started (no LiDAR needed)](#4-getting-started-no-lidar-needed)
5. [Connecting your LiDAR](#5-connecting-your-lidar)
6. [Running the examples](#6-running-the-examples)
7. [Using the library in your project](#7-using-the-library-in-your-project)
8. [Config file reference](#8-config-file-reference)
9. [Troubleshooting](#9-troubleshooting)
10. [License & acknowledgements](#10-license--acknowledgements)

---

## 1. Features

- **Zero system dependencies**: the official C++ SDK (MIT) is vendored under
  `livox-sdk2-sys/vendor` and compiled by `build.rs` using the `cc` crate.
- **Two layers**:
  - `livox-sdk2-sys` — raw, unmodified `bindgen` FFI bindings (62 C functions).
  - `livox-sdk2` — safe, high-level API (RAII lifecycle, thread-safe
    callbacks, unaligned-read-safe packet views).
- **Device discovery** — the SDK auto-detects LiDARs on the network and
  reports each one's real `SN` and `lidar_ip` (authoritative answer to
  "which IP is a LiDAR?").
- **Parsed data** — point clouds (Cartesian high/low, spherical, double-echo)
  returned as `Vec<Point>` in **meters**; 6-axis IMU returned as
  `Vec<ImuPoint>` in **rad/s** and **g**.
- **Tested on Jetson** — full-rate streams verified on JetPack (Mid-360:
  200k points/s, 200 Hz IMU).

## 2. Workspace layout

| Path | Description |
|------|-------------|
| `livox-sdk2-sys/` | Raw FFI crate. `build.rs` compiles the vendored SDK and runs `bindgen`. |
| `livox-sdk2-sys/vendor/` | Vendored official Livox-SDK2 source (headers + core + 3rdparty, MIT). |
| `livox-sdk2/` | Safe high-level crate: `Sdk`, `Packet`, `Point`, `ImuPoint`, `DeviceInfo`. |
| `livox-sdk2/examples/` | Runnable examples used for bring-up and testing. |
| `docs/` | Design notes (installation plan, etc.). |

## 3. Prerequisites

Supported OS: **Ubuntu 18.04+** (and derivatives), including JetPack 5
(Ubuntu 20.04) / JetPack 6 (Ubuntu 22.04) on Jetson. Architectures: x86_64,
aarch64.

### 3.1 Install Rust (≥ 1.77)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

> In mainland China you may want the rsproxy/ustc mirrors or a proxy — see
> [Troubleshooting](#9-troubleshooting).

### 3.2 Install a C++11 compiler and libclang (for bindgen)

```bash
sudo apt update
sudo apt install -y g++ clang libclang-dev
```

- `g++` — compiles the vendored C++ SDK.
- `clang` + `libclang-dev` — required by `bindgen` at build time only.

Jetson / JetPack: `g++` is preinstalled; install `clang libclang-dev` if missing.

### 3.3 Verify the toolchain

```bash
rustc --version   # expect >= 1.77
g++ --version     # any modern version
clang --version   # e.g. 14/15/17/18
```

## 4. Getting started (no LiDAR needed)

```bash
git clone git@github.com:DoraCN/livox-sdk2.git
cd livox-sdk2

# First build downloads crates and compiles the vendored SDK (aarch64: ~3-5 min)
cargo build --release

# Smoke test: prints the linked SDK version. No hardware, no config required.
cargo run --release --example version
```

Expected output:

```
Livox SDK2 version: 1.4.3
```

If you see this, the toolchain, the vendored SDK build and the FFI linkage all
work on your machine.

### Unit tests (also hardware-free)

```bash
cargo test
```

Expect `7 passed` (point-cloud and IMU parsers, including byte-layout and
spherical→Cartesian conversion tests).

## 5. Connecting your LiDAR

### 5.1 Hardware

- Power the LiDAR (Mid-360: 9–36 V DC, e.g. via the Livox power adapter).
- Connect the LiDAR's Ethernet port **directly to a NIC of your host** with a
  standard cable.

### 5.2 Network

The LiDAR ships with a static IP in `192.168.1.x` (Mid-360 default
`192.168.1.3`). The host NIC must be in the same subnet:

```bash
ip addr                                # list NICs and their IPs
sudo ip addr add 192.168.1.5/24 dev eth0   # temporary, non-persistent
```

To make it persistent on Ubuntu/JetPack (netplan):

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

> Use your actual NIC name from `ip addr` (e.g. `eth0`, `eno1`). You can also
> keep the DHCP address and set `host_ip` in the config to that address —
> see [§8](#8-config-file-reference).

### 5.3 Check reachability (optional, quick sanity check)

```bash
ping 192.168.1.3          # or: fping -g 192.168.1.1 192.168.1.254 -a
```

Reachable IPs respond — but the **authoritative** confirmation that an IP is a
LiDAR comes from the SDK itself (next step).

## 6. Running the examples

All examples take the config file as their first argument
(`livox-sdk2/examples/mid360_config.json` is a ready-to-use Mid-360 sample).

### 6.1 Discover devices

```bash
cargo run --release --example discover -- livox-sdk2/examples/mid360_config.json
```

Expected output (device data will differ):

```
[info] Init livox lidars succ.  [device_manager.cpp] [Init] [178]
SDK started, waiting 5 s for device discovery ...
device: handle=1895934144 type=9 (Mid-360) SN=47MDMBE0030413 IP=192.168.1.113
```

The printed `IP` is the real address of the LiDAR; its `SN` identifies the
unit. IPs that never appear are **not** LiDARs (router, other hosts, or
unreachable).

### 6.2 Point clouds

```bash
cargo run --release --example point_cloud -- livox-sdk2/examples/mid360_config.json 10
```

Expected output:

```
lidar 1895934144 type 9: 200064 points/s
lidar 1895934144 type 9: 200160 points/s
...
```

Mid-360 streams ~200k points/s at full rate. Lower numbers are fine (field of
view / environment dependent); zero means the LiDAR is not sending (see
[Troubleshooting](#9-troubleshooting)).

### 6.3 IMU data

```bash
cargo run --release --example imu -- livox-sdk2/examples/mid360_config.json 10
```

Expected output:

```
lidar 1895934144 type 9: 201 imu samples/s
  latest: gyro=(0.0115, -0.0102, -0.3691) rad/s acc=(-0.0383, 0.0115, 0.9966) g
```

At rest, `acc_z ≈ 1.0 g` (gravity) and gyro values hover around zero —
physically correct for a stationary LiDAR.

## 7. Using the library in your project

```toml
[dependencies]
# While the crates are not yet published to crates.io, use a path or git dep:
livox-sdk2 = { path = "/path/to/livox-sdk2/livox-sdk2" }
# or
livox-sdk2 = { git = "https://github.com/DoraCN/livox-sdk2.git" }
# Once published:
# livox-sdk2 = "0.1"
```

```rust
use livox_sdk2::{ImuPoint, Packet, Point, Sdk};

fn main() -> Result<(), String> {
    let mut sdk = Sdk::new("mid360_config.json")?;

    // Discovered devices (IP + SN):
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

## 8. Config file reference

The JSON config passed to `Sdk::new` (full parameter reference). The structure
is identical for the `MID360`, `HAP`, `Mid360s` and `Avia2` sections.

### Structure overview

```json
{
  "master_sdk": true,              // [optional] global
  "lidar_log_enable": false,       // [optional] global
  "lidar_log_cache_size_MB": 500,  // [optional] global
  "lidar_log_path": "./",          // [optional] global

  "MID360": {                       // device-type section
    "lidar_net_info": { ... },      // lidar-side ports (factory defaults)
    "host_net_info": [ ... ]        // host-side config (array or single object)
  }
}
```

### Global (top-level) fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `master_sdk` | bool | no | `true` | `true` = master SDK (send commands + receive data); `false` = slave SDK (multicast point cloud only). **Only one master per subnet.** |
| `lidar_log_enable` | bool | no | `false` | Enable LiDAR firmware logging to file. |
| `lidar_log_cache_size_MB` | uint | no | `500` | Log cache size in MB. Only read when `lidar_log_enable` is present. |
| `lidar_log_path` | string | no | `"./"` | Directory for firmware log files. |

> If `lidar_log_enable` is present, `lidar_log_cache_size_MB` and
> `lidar_log_path` become required — otherwise config parsing fails. To keep
> logging disabled, omit the whole group.

### `lidar_net_info` (lidar-side ports — factory defaults, required)

| Field | Type | Required | Mid-360 default | Description |
|-------|------|----------|-----------------|-------------|
| `cmd_data_port` | uint | yes | `56100` | Control-command port |
| `push_msg_port` | uint | yes | `56200` | Push-message port |
| `point_data_port` | uint | yes | `56300` | Point-cloud data port |
| `imu_data_port` | uint | yes | `56400` | IMU data port |
| `log_data_port` | uint | yes | `56500` | Firmware-log port |

> HAP uses different factory defaults (`56000/57000/58000/59000`, see official
> HAP docs); the field meanings are identical.

### `host_net_info` (host-side config)

Two accepted forms:

- **Array (new style, recommended)** — one entry per host; entries may carry a
  `lidar_ip` list.
- **Object (old style)** — a single host in auto-discovery mode (no `lidar_ip`).

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `host_ip` | string | yes | This host's NIC IP. The SDK binds its ports and receives data on it. **Must actually be assigned to the NIC**, otherwise the SDK fails with `bind failed`. |
| `lidar_ip` | string[] | no | LiDAR IPs to connect to. **Omit = broadcast auto-discovery** (SDK sends a detection packet on UDP 56000 and connects to responding LiDARs). Fill in = connect directly. |
| `multicast_ip` | string | no | Multicast address for point cloud / IMU (e.g. `224.1.1.5`) in multi-host setups. |
| `cmd_data_port` | uint | yes | Host control port (e.g. `56101`) |
| `push_msg_port` | uint | yes | Host push-message port (e.g. `56201`) |
| `point_data_port` | uint | yes | Host point-cloud port (e.g. `56301`) |
| `imu_data_port` | uint | yes | Host IMU port (e.g. `56401`) |
| `log_data_port` | uint | yes | Host log port (e.g. `56501`) |

> `lidar_ip` may be replaced by the alias `cmd_data_ip` (then `host_ip`
> becomes optional). When both are present, `host_ip` wins.

### Quick checklist (Jetson deployment)

1. `host_ip` must be an IP **already assigned** to the Jetson NIC (verify with
   `ip addr`), otherwise you get `bind failed` / `Create detection socket failed`.
2. LiDAR and Jetson must share a subnet (Mid-360 defaults to `192.168.1.x`).
3. Single LiDAR, single host: keep the sample `host_net_info` array and omit
   `lidar_ip` for auto-discovery.
4. Multiple LiDARs: list each IP in `lidar_ip`, or configure multicast per the
   official protocol.

## 9. Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| `error: no example target named 'version'` | Stale checkout or wrong directory | `git pull`; run cargo from the workspace root. |
| `bind failed` / `Create detection socket failed` | `host_ip` in config is not assigned to any NIC | Add the IP (`sudo ip addr add ...`) or set `host_ip` to the NIC's real IP. |
| `No LiDAR found.` | Wrong subnet, cable, power, or LiDAR off | Check `ip addr`, cable and power; `ping`/`fping` the subnet; verify the IPs with the discover example. |
| `libclang ... not found` / bindgen build errors | Missing clang toolchain | `sudo apt install clang libclang-dev`; optionally set `LIBCLANG_PATH`. |
| Cargo can't download crates (China) | Network restrictions | Use a proxy (`export https_proxy=http://192.168.0.1:7890 http_proxy=...`) or configure the rsproxy mirror in `.cargo/config.toml`. |
| `Create channel failed` / port in use | Another SDK/process bound the same ports | Kill the conflicting process or change the ports in the config. |
| Zero points but LiDAR discovered | LiDAR not sending (rare on Mid-360 — sending is on by default) | Check FOV/scan mode; contact Livox support if persistent. |

## 10. License & acknowledgements

- This project is licensed under **MIT** ([LICENSE](LICENSE)).
- The vendored official SDK keeps its own MIT license at
  `livox-sdk2-sys/vendor/LICENSE.txt`.
- Protocol details and hardware are provided by
  [Livox](https://www.livoxtech.com/) — see the
  [Livox-SDK2 repository](https://github.com/Livox-SDK/Livox-SDK2) and the
  [Mid-360 protocol wiki](https://livox-wiki-en.readthedocs.io/en/latest/tutorials/new_product/mid360/mid360.html).
