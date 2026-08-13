//! Point cloud streaming: parses and counts points from each packet.
//!
//! Run: `cargo run --example point_cloud -- <config.json> [seconds]`
//!
//! Prints a per-second point rate, then exits after `seconds` (default 10).

use livox_sdk2::Sdk;
use std::time::{Duration, Instant};

fn main() {
    let config = std::env::args().nth(1).unwrap_or_else(usage);
    let seconds = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    let mut sdk = Sdk::new(&config).expect("SDK init failed");

    let mut count = 0usize;
    let mut last_report = Instant::now();
    sdk.set_point_cloud_callback(move |handle, dev_type, packet| {
        count += packet.points().len();
        if last_report.elapsed() >= Duration::from_secs(1) {
            println!("lidar {handle} type {dev_type}: {count} points/s",);
            count = 0;
            last_report = Instant::now();
        }
    });

    println!("collecting point clouds for {seconds} s ...");
    std::thread::sleep(Duration::from_secs(seconds));
    println!("done");
}

fn usage() -> String {
    eprintln!("usage: point_cloud <config.json> [seconds]");
    std::process::exit(2);
}
