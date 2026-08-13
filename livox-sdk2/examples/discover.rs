//! Device discovery: prints every LiDAR the SDK detects, with its real IP/SN.
//!
//! Run: `cargo run --example discover -- <config.json>`
//!
//! This answers "which IP is a LiDAR?": any device printed here is a live
//! LiDAR the SDK is talking to. Wait up to 5 s for discovery, then exits.

use livox_sdk2::Sdk;
use std::time::Duration;

fn main() {
    let config = std::env::args()
        .nth(1)
        .unwrap_or_else(usage);
    let sdk = Sdk::new(&config).expect("SDK init failed");

    println!("SDK started, waiting 5 s for device discovery ...");
    std::thread::sleep(Duration::from_secs(5));

    let devices = sdk.devices();
    if devices.is_empty() {
        println!(
            "No LiDAR found.\n\
             - Is the Jetson NIC on the same subnet as the LiDAR (static IP)?\n\
             - Is `host_ip` in the config correct?\n\
             - Is the LiDAR powered and cabled?"
        );
    }
    for d in devices {
        println!(
            "device: handle={} type={} ({}) SN={} IP={}",
            d.handle,
            d.dev_type,
            d.type_name(),
            d.sn,
            d.lidar_ip
        );
    }
}

fn usage() -> String {
    eprintln!("usage: discover <config.json>");
    std::process::exit(2);
}
