//! Raw FFI bindings to the [Livox SDK2](https://github.com/Livox-SDK/Livox-SDK2).
//!
//! This crate vendors the official C++ SDK source tree under `vendor/` and
//! compiles it from source at build time (`build.rs` via the `cc` crate).
//! The C API declarations from `livox_lidar_api.h` / `livox_lidar_def.h` are
//! turned into Rust bindings with `bindgen`.
//!
//! # Safety
//!
//! Every function here is the raw `extern "C"` API of the SDK. Callers must
//! follow the SDK lifecycle (`LivoxLidarSdkInit` → `LivoxLidarSdkStart` →
//! ... → `LivoxLidarSdkUninit`) and respect pointer validity for callbacks
//! and out-parameters. Prefer the safe wrapper crate [`livox-sdk2`](https://crates.io/crates/livox-sdk2).

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::all)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
