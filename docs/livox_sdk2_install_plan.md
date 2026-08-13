# Livox SDK2 驱动安装与 Rust FFI 方案（Jetson）

## 1. 仓库分析结论

官方仓库: <https://github.com/Livox-SDK/Livox-SDK2>（当前 master，最新版本 1.4.3）

对 Rust FFI 绑定非常友好，原因如下：

| 项目 | 分析结果 |
|------|----------|
| API 形态 | **纯 C 接口**（`extern "C"` 包裹），头文件只依赖 `<stdint.h>` / `<stdbool.h>`，不含任何 C++ 特性 |
| 依赖 | 完全自带：`3rdparty/` 内置 spdlog、rapidjson、FastCRC，系统层只需 pthread |
| 构建工具 | CMake 3.0+、g++（支持 C++11） |
| 产物 | 编译产生 `liblivox_lidar_sdk_static.a` 与 `liblivox_lidar_sdk_shared.so` |
| 安装位置 | `make install` 后：头文件 `/usr/local/include`，库 `/usr/local/lib` |
| 架构 | x86 / ARM(aarch64) 均支持，源码层面无交叉编译障碍 |

### 核心 API（约 60 个函数）

- 生命周期: `LivoxLidarSdkInit()` / `LivoxLidarSdkStart()` / `LivoxLidarSdkUninit()`
- 数据回调: `SetLivoxLidarPointCloudCallBack()`（点云）、`SetLivoxLidarImuDataCallback()`（IMU）、`SetLivoxLidarInfoCallback()`（状态）
- 观察者: `LivoxLidarAddPointCloudObserver()` / `LivoxLidarRemovePointCloudObserver()`
- 控制: `SetLivoxLidarScanPattern()`、`SetLivoxLidarEchoMode()`、`SetLivoxLidarFovCfg0/1()`、`LivoxLidarRequestReboot()` 等
- 升级: `SetLivoxLidarUpgradeFirmwarePath()`

### 绑定需要特别注意的两个坑

1. **`LivoxLidarSdkInit()` 带 C++ 默认参数**：
   ```c
   bool LivoxLidarSdkInit(const char* path,
                          const char* host_ip = "",          // C++ 默认参数，ABI 仍是 3 个形参
                          const LivoxLidarLoggerCfgInfo* log_cfg_info = nullptr);
   ```
   Rust 里必须声明为 3 个参数并显式传 `""` 和 `null`，不能只传 1 个。

2. **点云/IMU 数据都是 UDP 以太网包**，回调里拿到的是 `LivoxLidarEthernetPacket*` 指针。SDK 内部自己起线程收发，无需额外网络库。

---

## 2. 两种驱动接入方案

### 方案 A（推荐）：编译期用 `cc` 自带 SDK 源码 + `bindgen` 生成绑定

- 把 Livox-SDK2 源码**内置进 crate**（git submodule 或 vendored）。
- `build.rs` 用 `cc` crate 直接编译全部 `.cpp`（含 3rdparty），`bindgen` 生成绑定。
- **优点**：Jetson 上 `cargo build` 一键完成，无需预先安装系统库，二进制自包含、易交叉/离线部署，版本与源码锁定。
- **缺点**：编译时间较长（SDK 约 30+ 个 cpp）。

### 方案 B（传统）：预先 `make install` 系统库 + 链接动态库

- 在 Jetson 上按官方流程安装到 `/usr/local/lib`。
- Rust 侧 `build.rs` 只跑 `bindgen`（或手写 `extern`），链接 `-llivox_lidar_sdk_shared`。
- **优点**：编译快，符合官方支持流程，可用官方 samples 先自测。
- **缺点**：部署时目标机必须也装了该库，需维护 `LD_LIBRARY_PATH`。

> 建议：先走**方案 B** 在 Jetson 上跑通官方 sample 验证硬件/网络，再做**方案 A** 进 Rust 工程。

---

## 3. Jetson 上的安装步骤（方案 B）

### 3.1 环境检查

```bash
uname -m                 # 应为 aarch64
sudo apt update
sudo apt install -y cmake g++ git   # cmake 3.0+，gcc 支持 C++11
```

### 3.2 编译并安装 SDK

```bash
cd ~
git clone https://github.com/Livox-SDK/Livox-SDK2.git
cd Livox-SDK2
mkdir build && cd build
cmake .. && make -j$(nproc)
sudo make install
```

验证：

```bash
ls -l /usr/local/lib/liblivox_lidar_sdk_*
ls -l /usr/local/include/livox_lidar_*
```

### 3.3 用官方 sample 自测（先于 Rust 验证）

```bash
cd samples/livox_lidar_quick_start
./livox_lidar_quick_start ../../../samples/livox_lidar_quick_start/mid360_config.json
```

预期输出（日志含设备 handle 与点云数据回调）：

```
[info] Data Handle Init Succ.
[info] Create detection channel detection socket:0
```

**卸载**（如需要）：`sudo rm -rf /usr/local/lib/liblivox_lidar_sdk_* /usr/local/include/livox_lidar_*`

### 3.4 网络配置（关键，Mid-360 / HAP 均为 UDP 以太网）

- 给 Jetson 网口配置静态 IP（如 `192.168.1.5`，子网 `255.255.255.0`），与雷达 `lidar_ip` 同网段。
- 修改 config JSON 中 `host_ip` 与 `lidar_ip` 与实际一致：
  ```json
  {
    "HAP": {
      "host_net_info": [{
        "lidar_ip": ["192.168.1.10"],
        "host_ip": "192.168.1.5",
        ...
      }]
    }
  }
  ```
- 多雷达场景：注意 `master_sdk` 全局只能有一个，其余设为 `false`。

---

## 4. Rust FFI 绑定要点（供后续实现）

### 4.1 bindgen 用法示意（方案 A 的 build.rs 片段）

```rust
// build.rs
fn main() {
    // 用 cc crate 编译 vendored 源码（含 3rdparty，-pthread）
    // 再用 bindgen 基于 include/ 生成绑定：
    let bindings = bindgen::Builder::default()
        .header("Livox-SDK2/include/livox_lidar_api.h")
        .allowlist_function("LivoxLidarSdk.*|SetLivoxLidar.*|...")
        .allowlist_type("LivoxLidar.*|LivoxLidarEthernetPacket|livox_status")
        .clang_arg("-ILivox-SDK2/include")
        .generate()?;
    bindings.write_to_file("src/bindings.rs")?;
}
```

### 4.2 手动 extern 声明的关键片段（最小可用集）

```rust
#[repr(C)]
pub struct LivoxLidarEthernetPacket {
    pub header: u64, pub version: u8, pub slot: u8, pub lidar_type: u8,
    pub port_id: u8, pub pack_cnt: u8, pub reserved: u32,
    pub timestamp_type: u32, pub time_stamp: u64, pub packet: *const u8, ...
}
pub type LivoxLidarPointCloudCallBack =
    extern "C" fn(u32 /*handle*/, u8 /*dev_type*/,
                  *mut LivoxLidarEthernetPacket, *mut c_void);

extern "C" {
    pub fn LivoxLidarSdkInit(path: *const c_char, host_ip: *const c_char,
                             log_cfg_info: *const LivoxLidarLoggerCfgInfo) -> bool; // 3 参数！
    pub fn LivoxLidarSdkStart() -> bool;
    pub fn LivoxLidarSdkUninit();
    pub fn SetLivoxLidarPointCloudCallBack(cb: LivoxLidarPointCloudCallBack,
                                           client_data: *mut c_void);
}
```

### 4.3 其他注意事项

- 回调参数类型/长度务必对照 `include/livox_lidar_def.h`（`dev_type`、`livox_status` 等枚举值）。
- 回调会从 SDK 内部线程触发，Rust 侧需用 `Arc`/channel 转发，勿在回调里阻塞。
- 内存布局敏感的结构体建议用 bindgen 生成而不是手写，避免字段顺序/对齐出错。

---

## 5. 推荐落地路线

1. Jetson 上按 §3 完成 SDK 安装并用官方 sample 自测（排除硬件/网络问题）。
2. 在现有 Rust 工程中按方案 B 先实现最小绑定（Init → Start → 点云回调），跑通数据流。
3. 验证性能与稳定性后，切换为方案 A（vendored + build.rs），实现离线、可复现构建。
4. 可选：集成 `serde`/`rkyv` 序列化点云供上层消费。
