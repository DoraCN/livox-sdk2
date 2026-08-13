//! Safe, high-level Rust bindings for the [Livox SDK2](https://github.com/Livox-SDK/Livox-SDK2).
//!
//! The underlying C API is provided by the [`livox-sdk2-sys`] crate, which
//! compiles the official vendored C++ SDK from source at build time — no
//! system library install required.
//!
//! # Minimal example
//!
//! ```no_run
//! use livox_sdk2::Sdk;
//!
//! let mut sdk = Sdk::new("mid360_config.json").expect("failed to init SDK");
//!
//! // Which IPs are LiDARs? The SDK reports every device it finds:
//! for dev in sdk.devices() {
//!     println!("{} @ {} (SN {})", dev.type_name(), dev.lidar_ip, dev.sn);
//! }
//!
//! // Get parsed point clouds directly:
//! sdk.set_point_cloud_callback(|handle, dev_type, packet| {
//!     let cloud: Vec<_> = packet.points();
//!     println!("lidar {handle} (type {dev_type}): {} points", cloud.len());
//! });
//! sdk.run(); // blocks until Ctrl-C, then uninitializes on drop
//! ```

mod device;
mod points;

pub use device::DeviceInfo;
pub use points::Point;

pub(crate) use livox_sdk2_sys as ffi;

use std::os::raw::{c_char, c_void};
use std::sync::{Mutex, OnceLock};

/// Size of the `LivoxLidarEthernetPacket` header (bytes before `data`).
const PACKET_HEADER_SIZE: usize = std::mem::offset_of!(ffi::LivoxLidarEthernetPacket, data);

type PointCloudHandler = Box<dyn FnMut(u32, u8, Packet<'_>) + Send>;
type ImuHandler = Box<dyn FnMut(u32, u8, Packet<'_>) + Send>;
type InfoHandler = Box<dyn FnMut(u32, u8, &str) + Send>;

static POINT_CLOUD_CB: OnceLock<Mutex<Option<PointCloudHandler>>> = OnceLock::new();
static IMU_CB: OnceLock<Mutex<Option<ImuHandler>>> = OnceLock::new();
static INFO_CB: OnceLock<Mutex<Option<InfoHandler>>> = OnceLock::new();

/// Safe view over one ethernet packet delivered by the SDK.
///
/// The underlying C struct is `#[repr(C, packed)]`; accessing its fields by
/// reference would be unaligned and thus undefined behaviour in Rust. This
/// type copies every scalar field out with unaligned reads and exposes the
/// point/IMU payload as a byte slice. It borrows the packet, which is only
/// valid for the duration of the callback it was delivered to.
///
/// Use [`Packet::points`] to obtain the parsed point cloud, or [`Packet::data`]
/// for the raw payload bytes.
#[derive(Copy, Clone)]
pub struct Packet<'a> {
    raw: &'a ffi::LivoxLidarEthernetPacket,
    data_len: usize,
}

impl<'a> Packet<'a> {
    fn new(raw: &'a ffi::LivoxLidarEthernetPacket) -> Self {
        let length = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!(raw.length)) };
        let data_len = length.saturating_sub(PACKET_HEADER_SIZE as u16) as usize;
        Self { raw, data_len }
    }

    pub fn version(&self) -> u8 {
        unsafe { std::ptr::read_unaligned(std::ptr::addr_of!(self.raw.version)) }
    }

    /// Total packet length in bytes (header + payload).
    pub fn length(&self) -> u16 {
        unsafe { std::ptr::read_unaligned(std::ptr::addr_of!(self.raw.length)) }
    }

    /// Unit: 0.1 µs.
    pub fn time_interval(&self) -> u16 {
        unsafe { std::ptr::read_unaligned(std::ptr::addr_of!(self.raw.time_interval)) }
    }

    /// Number of points in this packet.
    pub fn dot_num(&self) -> u16 {
        unsafe { std::ptr::read_unaligned(std::ptr::addr_of!(self.raw.dot_num)) }
    }

    pub fn udp_cnt(&self) -> u16 {
        unsafe { std::ptr::read_unaligned(std::ptr::addr_of!(self.raw.udp_cnt)) }
    }

    pub fn frame_cnt(&self) -> u8 {
        unsafe { std::ptr::read_unaligned(std::ptr::addr_of!(self.raw.frame_cnt)) }
    }

    /// Data type, e.g. point cloud vs IMU.
    pub fn data_type(&self) -> u8 {
        unsafe { std::ptr::read_unaligned(std::ptr::addr_of!(self.raw.data_type)) }
    }

    pub fn time_type(&self) -> u8 {
        unsafe { std::ptr::read_unaligned(std::ptr::addr_of!(self.raw.time_type)) }
    }

    pub fn crc32(&self) -> u32 {
        unsafe { std::ptr::read_unaligned(std::ptr::addr_of!(self.raw.crc32)) }
    }

    /// Raw 8-byte timestamp.
    pub fn timestamp(&self) -> [u8; 8] {
        unsafe { std::ptr::read_unaligned(std::ptr::addr_of!(self.raw.timestamp)) }
    }

    /// Raw payload bytes (point cloud or IMU) following the packet header.
    pub fn data(&self) -> &'a [u8] {
        let ptr = std::ptr::addr_of!(self.raw.data) as *const u8;
        // SAFETY: the SDK guarantees the underlying UDP datagram spans
        // `length` bytes; `length - header` bytes are readable here.
        unsafe { std::slice::from_raw_parts(ptr, self.data_len) }
    }
}

extern "C" fn point_cloud_cb(
    handle: u32,
    dev_type: u8,
    data: *mut ffi::LivoxLidarEthernetPacket,
    _client_data: *mut c_void,
) {
    let Some(lock) = POINT_CLOUD_CB.get() else {
        return;
    };
    let mut cb = lock.lock().unwrap();
    if let Some(f) = cb.as_mut() {
        // SAFETY: the SDK guarantees `data` is valid for the callback.
        let packet = Packet::new(unsafe { &*data });
        f(handle, dev_type, packet);
    }
}

extern "C" fn imu_cb(
    handle: u32,
    dev_type: u8,
    data: *mut ffi::LivoxLidarEthernetPacket,
    _client_data: *mut c_void,
) {
    let Some(lock) = IMU_CB.get() else {
        return;
    };
    let mut cb = lock.lock().unwrap();
    if let Some(f) = cb.as_mut() {
        let packet = Packet::new(unsafe { &*data });
        f(handle, dev_type, packet);
    }
}

extern "C" fn info_cb(
    handle: u32,
    dev_type: u8,
    info: *const c_char,
    _client_data: *mut c_void,
) {
    let Some(lock) = INFO_CB.get() else {
        return;
    };
    let mut cb = lock.lock().unwrap();
    if let Some(f) = cb.as_mut() {
        // SAFETY: `info` is a NUL-terminated string valid for the callback.
        let s = unsafe { std::ffi::CStr::from_ptr(info) }.to_string_lossy();
        f(handle, dev_type, &s);
    }
}

/// High-level handle to the Livox SDK.
///
/// Created with [`Sdk::new`] and uninitialized automatically when dropped.
pub struct Sdk {
    _private: (),
}

impl Sdk {
    /// Initializes and starts the SDK from a JSON config file.
    ///
    /// `config_path` points to a Livox config file such as `mid360_config.json`.
    /// A device-registry callback is installed so that [`Sdk::devices`] reflects
    /// every LiDAR the SDK detects.
    pub fn new(config_path: &str) -> Result<Self, String> {
        let path = std::ffi::CString::new(config_path).map_err(|_| "config path contains NUL")?;
        let host_ip = std::ffi::CString::new("").unwrap();
        // SAFETY: both pointers are valid for the call; the SDK copies what it needs.
        let ok = unsafe { ffi::LivoxLidarSdkInit(path.as_ptr(), host_ip.as_ptr(), std::ptr::null()) };
        if !ok {
            return Err("LivoxLidarSdkInit failed".to_string());
        }
        device::init();
        let ok = unsafe { ffi::LivoxLidarSdkStart() };
        if !ok {
            unsafe { ffi::LivoxLidarSdkUninit() };
            return Err("LivoxLidarSdkStart failed".to_string());
        }
        Ok(Self { _private: () })
    }

    /// Snapshot of all LiDAR devices currently known to the SDK.
    ///
    /// Each entry contains the device's real `lidar_ip` — this is the
    /// authoritative answer to "which IP is a LiDAR". IPs listed in the config
    /// that never appear here were not reachable (wrong subnet, cable, or the
    /// device is off).
    pub fn devices(&self) -> Vec<DeviceInfo> {
        device::snapshot()
    }

    /// Registers a callback invoked whenever a device connects or its info changes.
    pub fn set_device_change_callback<F>(&mut self, f: F)
    where
        F: FnMut(DeviceInfo) + Send + 'static,
    {
        device::DEVICE_CB
            .get_or_init(|| Mutex::new(None))
            .lock()
            .unwrap()
            .replace(Box::new(f));
    }

    /// Registers the point-cloud callback.
    pub fn set_point_cloud_callback<F>(&mut self, f: F)
    where
        F: FnMut(u32, u8, Packet<'_>) + Send + 'static,
    {
        POINT_CLOUD_CB
            .get_or_init(|| Mutex::new(None))
            .lock()
            .unwrap()
            .replace(Box::new(f));
        // SAFETY: `point_cloud_cb` and the callback registry outlive the call.
        unsafe { ffi::SetLivoxLidarPointCloudCallBack(Some(point_cloud_cb), std::ptr::null_mut()) };
    }

    /// Registers the IMU-data callback.
    pub fn set_imu_callback<F>(&mut self, f: F)
    where
        F: FnMut(u32, u8, Packet<'_>) + Send + 'static,
    {
        IMU_CB
            .get_or_init(|| Mutex::new(None))
            .lock()
            .unwrap()
            .replace(Box::new(f));
        // SAFETY: `imu_cb` and the callback registry outlive the call.
        unsafe { ffi::SetLivoxLidarImuDataCallback(Some(imu_cb), std::ptr::null_mut()) };
    }

    /// Registers the info/status callback.
    pub fn set_info_callback<F>(&mut self, f: F)
    where
        F: FnMut(u32, u8, &str) + Send + 'static,
    {
        INFO_CB
            .get_or_init(|| Mutex::new(None))
            .lock()
            .unwrap()
            .replace(Box::new(f));
        // SAFETY: `info_cb` and the callback registry outlive the call.
        unsafe { ffi::SetLivoxLidarInfoCallback(Some(info_cb), std::ptr::null_mut()) };
    }

    /// Blocks forever (or until the process is interrupted).
    pub fn run(&self) -> ! {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(3600));
        }
    }
}

impl Drop for Sdk {
    fn drop(&mut self) {
        // SAFETY: must only be called once, after the SDK was initialized.
        unsafe { ffi::LivoxLidarSdkUninit() };
    }
}
