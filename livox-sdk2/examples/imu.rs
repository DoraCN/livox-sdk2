//! IMU data streaming: parses and counts 6-axis IMU samples per packet.
//!
//! Run: `cargo run --example imu -- <config.json> [seconds]`
//!
//! Prints the IMU sample rate every second plus the latest gyro/accel sample,
//! then exits after `seconds` (default 10).

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
    let mut last_sample = None;
    let mut last_report = Instant::now();
    sdk.set_imu_callback(move |handle, dev_type, packet| {
        let samples = packet.imu_points();
        count += samples.len();
        if let Some(s) = samples.first() {
            last_sample = Some(*s);
        }
        if last_report.elapsed() >= Duration::from_secs(1) {
            println!("lidar {handle} type {dev_type}: {count} imu samples/s",);
            if let Some(s) = last_sample {
                println!(
                    "  latest: gyro=({:.4}, {:.4}, {:.4}) rad/s acc=({:.4}, {:.4}, {:.4}) g",
                    s.gyro_x, s.gyro_y, s.gyro_z, s.acc_x, s.acc_y, s.acc_z
                );
            }
            count = 0;
            last_report = Instant::now();
        }
    });

    println!("collecting IMU data for {seconds} s ...");
    std::thread::sleep(Duration::from_secs(seconds));
    println!("done");
}

fn usage() -> String {
    eprintln!("usage: imu <config.json> [seconds]");
    std::process::exit(2);
}
