use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let vendor = manifest.join("vendor");
    let sdk = vendor.join("sdk_core");
    let inc = vendor.join("include");
    let third = vendor.join("3rdparty");

    let sources = [
        third.join("FastCRC/FastCRCsw.cpp"),
        sdk.join("device_manager.cpp"),
        sdk.join("livox_lidar_sdk.cpp"),
        sdk.join("params_check.cpp"),
        sdk.join("parse_cfg_file.cpp"),
        sdk.join("upgrade_manager.cpp"),
        sdk.join("base/io_loop.cpp"),
        sdk.join("base/thread_base.cpp"),
        sdk.join("base/io_thread.cpp"),
        sdk.join("base/logging.cpp"),
        sdk.join("base/network/unix/network_util.cpp"),
        sdk.join("base/multiple_io/multiple_io_base.cpp"),
        sdk.join("base/multiple_io/multiple_io_epoll.cpp"),
        sdk.join("base/multiple_io/multiple_io_poll.cpp"),
        sdk.join("base/multiple_io/multiple_io_select.cpp"),
        sdk.join("base/multiple_io/multiple_io_kqueue.cpp"),
        sdk.join("base/wake_up/unix/wake_up_pipe.cpp"),
        sdk.join("comm/comm_port.cpp"),
        sdk.join("comm/sdk_protocol.cpp"),
        sdk.join("comm/generate_seq.cpp"),
        sdk.join("upgrade/firmware.cpp"),
        sdk.join("upgrade/livox_lidar_upgrader.cpp"),
        sdk.join("logger_handler/logger_manager.cpp"),
        sdk.join("logger_handler/logger_handler.cpp"),
        sdk.join("logger_handler/file_manager.cpp"),
        sdk.join("data_handler/data_handler.cpp"),
        sdk.join("command_handler/command_impl.cpp"),
        sdk.join("command_handler/general_command_handler.cpp"),
        sdk.join("command_handler/hap_command_handler.cpp"),
        sdk.join("command_handler/mid360_command_handler.cpp"),
        sdk.join("command_handler/build_request.cpp"),
        sdk.join("command_handler/parse_lidar_state_info.cpp"),
        sdk.join("command_handler/mid360s_command_handler.cpp"),
        sdk.join("command_handler/avia2_command_handler.cpp"),
        sdk.join("debug_point_cloud_handler/debug_point_cloud_manager.cpp"),
        sdk.join("debug_point_cloud_handler/debug_point_cloud_handler.cpp"),
    ];

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++11")
        .warnings(false)
        .include(&inc)
        .include(&third)
        .include(third.join("spdlog"))
        .include(&sdk);
    for src in &sources {
        build.file(src);
    }
    #[cfg(target_os = "linux")]
    build.flag("-pthread");
    build.compile("livox_lidar_sdk");

    println!("cargo:rustc-link-lib=stdc++");

    let bindings = bindgen::Builder::default()
        .header(inc.join("livox_lidar_api.h").to_str().unwrap())
        .clang_arg("-x")
        .clang_arg("c++")
        .clang_arg(format!("-I{}", inc.display()))
        .allowlist_file(r".*livox_lidar_api\.h")
        .allowlist_file(r".*livox_lidar_def\.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("failed to generate Livox SDK2 bindings");

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("failed to write Livox SDK2 bindings");

    println!("cargo:rerun-if-changed=vendor/include/livox_lidar_api.h");
    println!("cargo:rerun-if-changed=vendor/include/livox_lidar_def.h");
}
