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

## Building on Jetson / cross compiling

The SDK compiles natively on `aarch64-linux-gnu`; a `cargo build` (or `cargo
build --target aarch64-unknown-linux-gnu` from an x86_64 host with the target
installed) produces a self-contained static archive — no `make install`
required.

## License

MIT. The vendored SDK keeps its own MIT license under
`livox-sdk2-sys/vendor/LICENSE.txt`.
