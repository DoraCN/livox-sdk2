# Livox SDK2 for Rust

[English](README.md) | [简体中文](README.zh-CN.md)

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

## Examples (also used for testing on a fresh board)

| Example | Purpose |
|---------|---------|
| `cargo run --release --example version` | Smoke test — prints linked SDK version; no LiDAR required. |
| `cargo run --release --example discover -- <config.json>` | Lists every LiDAR found with real IP/SN. |
| `cargo run --release --example point_cloud -- <config.json> [secs]` | Parses and counts points per second. |
| `cargo run --release --example imu -- <config.json> [secs]` | Parses and counts 6-axis IMU samples per second. |

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

## Config file reference (`mid360_config.json`)

The config is a JSON file passed to `Sdk::new` / `LivoxLidarSdkInit`. This
reference covers the Mid-360 sample (`livox-sdk2/examples/mid360_config.json`);
the structure is identical for the `HAP`, `Mid360s` and `Avia2` sections.

### Structure overview

```json
{
  "master_sdk": true,              // [optional] global
  "lidar_log_enable": false,       // [optional] global
  "lidar_log_cache_size_MB": 500,  // [optional] global
  "lidar_log_path": "./",          // [optional] global

  "MID360": {                       // device-type section: MID360 / HAP / Mid360s / Avia2
    "lidar_net_info": { ... },      // lidar-side ports (factory defaults, usually unchanged)
    "host_net_info": [ ... ]        // host-side config (array or single object)
  }
}
```

### Global (top-level) fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `master_sdk` | bool | no | `true` | `true` = master SDK (sends commands + receives data); `false` = slave SDK (multicast point cloud only). **Only one master allowed per subnet.** |
| `lidar_log_enable` | bool | no | `false` | Enable lidar firmware logging to file. |
| `lidar_log_cache_size_MB` | uint | no | `500` | Log cache size in MB. Only read when `lidar_log_enable` is present. |
| `lidar_log_path` | string | no | `"./"` | Directory for firmware log files. |

> If `lidar_log_enable` is present but `lidar_log_cache_size_MB` / `lidar_log_path`
> are missing, config parsing fails. Omit the whole log group to keep logs disabled.

### Device-type section (`MID360` / `HAP` / `Mid360s` / `Avia2`)

#### `lidar_net_info` (lidar-side ports, factory defaults, required)

| Field | Type | Required | Mid-360 default | Description |
|-------|------|----------|-----------------|-------------|
| `cmd_data_port` | uint | yes | `56100` | Control command port |
| `push_msg_port` | uint | yes | `56200` | Push message port |
| `point_data_port` | uint | yes | `56300` | Point cloud data port |
| `imu_data_port` | uint | yes | `56400` | IMU data port |
| `log_data_port` | uint | yes | `56500` | Firmware log port |

> HAP uses different defaults (56000/57000/58000/59000, see the official HAP
> docs); the field meanings are identical.

#### `host_net_info` (host-side config)

Two forms are accepted:

- **Array (new style, recommended)** — one entry per host; may carry a `lidar_ip` list.
- **Object (old style)** — single host in auto-discovery mode (no `lidar_ip`).

Array-entry fields:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `host_ip` | string | yes | This host's NIC IP. The SDK binds ports and receives data on it. **Must be actually assigned to the NIC**, otherwise the bind fails. |
| `lidar_ip` | string[] | no | LiDAR IPs to connect to. **Omit = broadcast auto-discovery** (SDK sends a detection packet on UDP 56000 and connects to responding LiDARs); fill in = direct connect. |
| `multicast_ip` | string | no | Multicast address for point cloud / IMU (e.g. `224.1.1.5`), for multi-host sharing. |
| `cmd_data_port` | uint | yes | Host control port (e.g. `56101`) |
| `push_msg_port` | uint | yes | Host push message port (e.g. `56201`) |
| `point_data_port` | uint | yes | Host point cloud port (e.g. `56301`) |
| `imu_data_port` | uint | yes | Host IMU port (e.g. `56401`) |
| `log_data_port` | uint | yes | Host log port (e.g. `56501`) |

> `lidar_ip` may be replaced by the alias `cmd_data_ip` (in which case
> `host_ip` becomes optional). When both `host_ip` and `cmd_data_ip` are
> present, `host_ip` wins.

### Quick checklist (Jetson deployment)

1. `host_ip` must be an IP **already assigned** to the Jetson NIC (confirm with
   `ip addr`), otherwise the SDK fails with `bind failed`.
2. The LiDAR and Jetson must be on the same subnet (Mid-360 defaults to
   `192.168.1.x`).
3. Single LiDAR, single host: keep the sample `host_net_info` array and omit
   `lidar_ip` for auto-discovery.
4. Multiple LiDARs: list each IP in `lidar_ip`, or configure multicast per the
   official protocol.

## Building on Jetson / cross compiling

The SDK compiles natively on `aarch64-linux-gnu`; a `cargo build` (or `cargo
build --target aarch64-unknown-linux-gnu` from an x86_64 host with the target
installed) produces a self-contained static archive — no `make install`
required.

## License

MIT. The vendored SDK keeps its own MIT license under
`livox-sdk2-sys/vendor/LICENSE.txt`.
