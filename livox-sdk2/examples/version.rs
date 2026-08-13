//! Smoke test: prints the linked Livox SDK2 version.
//!
//! Does **not** require a LiDAR or config file — verifies that the vendored
//! C++ SDK was compiled and linked correctly on this platform.
//!
//! Run: `cargo run --example version`

fn main() {
    let (major, minor, patch) = livox_sdk2::sdk_version();
    println!("Livox SDK2 version: {major}.{minor}.{patch}");
}
