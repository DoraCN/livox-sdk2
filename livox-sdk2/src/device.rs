use crate::ffi;
use std::collections::HashMap;
use std::os::raw::c_char;
use std::os::raw::c_void;
use std::sync::Mutex;
use std::sync::OnceLock;

/// A LiDAR device discovered and connected by the SDK.
///
/// The SDK reports this through the info-change callback for every device it
/// detects on the network — the authoritative way to know **which IP is a
/// LiDAR**: any `lidar_ip` appearing here is a live device the SDK is talking
/// to. IPs that never show up are not (reachable) LiDARs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    /// SDK-internal handle used to address this device in control calls.
    pub handle: u32,
    /// Device type, see the `kLivoxLidarType*` constants (e.g. Mid-360 = 9, HAP = 15).
    pub dev_type: u8,
    /// Serial number of the device.
    pub sn: String,
    /// IP address of the device on the network.
    pub lidar_ip: String,
}

impl DeviceInfo {
    /// Human-readable device type name.
    pub fn type_name(&self) -> &'static str {
        match self.dev_type {
            0 => "Hub",
            1 => "Mid-40",
            2 => "Tele",
            3 => "Horizon",
            6 => "Mid-70",
            7 => "Avia",
            9 => "Mid-360",
            10 => "Industrial-HAP",
            15 => "HAP",
            16 => "PA",
            35 => "Mid-360s",
            40 => "Avia2",
            _ => "Unknown",
        }
    }
}

static DEVICES: OnceLock<Mutex<HashMap<u32, DeviceInfo>>> = OnceLock::new();

type DeviceChangeHandler = Box<dyn FnMut(DeviceInfo) + Send>;

/// Optional user callback invoked on every device connect/info change.
pub(crate) static DEVICE_CB: OnceLock<Mutex<Option<DeviceChangeHandler>>> = OnceLock::new();

/// Registers the internal info-change handler and enables the device registry.
pub(crate) fn init() {
    DEVICES.get_or_init(|| Mutex::new(HashMap::new()));
    // SAFETY: `device_change_cb` is a static function and the registry
    // outlives the SDK; `client_data` is unused.
    unsafe {
        ffi::SetLivoxLidarInfoChangeCallback(Some(device_change_cb), std::ptr::null_mut());
    }
}

/// Snapshot of all devices currently known to the SDK.
pub(crate) fn snapshot() -> Vec<DeviceInfo> {
    let mut devices: Vec<DeviceInfo> = DEVICES
        .get()
        .map(|m| m.lock().unwrap().values().cloned().collect())
        .unwrap_or_default();
    devices.sort_by_key(|d| d.handle);
    devices
}

extern "C" fn device_change_cb(handle: u32, info: *const ffi::LivoxLidarInfo, _: *mut c_void) {
    if info.is_null() {
        return;
    }
    // SAFETY: `info` is valid for the duration of the callback. The struct is
    // byte-aligned (`#[repr(C)]`, alignment 1), so field reads are aligned.
    let raw = unsafe { &*info };
    let device = DeviceInfo {
        handle,
        dev_type: raw.dev_type,
        sn: cstr(&raw.sn),
        lidar_ip: cstr(&raw.lidar_ip),
    };
    if let Some(lock) = DEVICES.get() {
        lock.lock().unwrap().insert(handle, device.clone());
    }
    if let Some(lock) = DEVICE_CB.get() {
        let mut cb = lock.lock().unwrap();
        if let Some(f) = cb.as_mut() {
            f(device);
        }
    }
}

/// Reads a NUL-terminated C string array (up to `bytes.len()` bytes).
fn cstr(bytes: &[c_char]) -> String {
    let ptr = bytes.as_ptr() as *const u8;
    let mut len = 0;
    while len < bytes.len() && unsafe { *ptr.add(len) } != 0 {
        len += 1;
    }
    // SAFETY: `ptr` points to a valid buffer of `bytes.len()` bytes.
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf8_lossy(slice).into_owned()
}
