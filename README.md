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
    sdk.set_point_cloud_callback(|handle, dev_type, packet| {
        println!(
            "lidar {handle} (type {dev_type}): {} points, {} bytes",
            packet.dot_num(),
            packet.data().len()
        );
    });
    sdk.run();
}
```

## Building on Jetson / cross compiling

The SDK compiles natively on `aarch64-linux-gnu`; a `cargo build` (or `cargo
build --target aarch64-unknown-linux-gnu` from an x86_64 host with the target
installed) produces a self-contained static archive — no `make install`
required.

## License

MIT. The vendored SDK keeps its own MIT license under
`livox-sdk2-sys/vendor/LICENSE.txt`.
