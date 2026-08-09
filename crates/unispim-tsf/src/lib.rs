//! 华宇拼音输入法 Windows TSF 文本服务 DLL。
//!
//! 对应 TSFexample 的 COM 服务器（Server.cpp / DllMain.cpp）：
//! - 导出 `DllGetClassObject` / `DllRegisterServer` / `DllUnregisterServer` / `DllCanUnloadNow`
//! - `IClassFactory` 工厂创建 `TextService` COM 对象
//! - 注册与注销（register.rs）
//!
//! 编译为 `cdylib`（Windows 上的 `unispim_tsf.dll`）。

mod register;
mod server;
mod text_service;
mod tsf;

pub use register::{register, register_with_path, unregister, CLSID_TEXT_SERVICE, GUID_PROFILE};
pub use server::{DllCanUnloadNow, DllGetClassObject, DllRegisterServer, DllUnregisterServer};
pub use text_service::TextService;
pub use tsf::{vk_to_key, TsfAdapter, TsfOp};

use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

/// 模块（DLL）句柄。
static G_HINST: AtomicUsize = AtomicUsize::new(0);

/// DLL 模块引用计数（-1 表示无引用）。
static G_REF_COUNT: AtomicI32 = AtomicI32::new(-1);

/// 记录 DLL 模块句柄。
pub fn set_module_handle(handle: Option<windows::Win32::Foundation::HINSTANCE>) {
    let raw = handle.map(|h| h.0 as usize).unwrap_or(0);
    G_HINST.store(raw, Ordering::SeqCst);
}

/// DLL 模块句柄。
pub fn module_handle() -> Option<windows::Win32::Foundation::HINSTANCE> {
    let raw = G_HINST.load(Ordering::SeqCst);
    if raw == 0 {
        None
    } else {
        Some(windows::Win32::Foundation::HINSTANCE(raw as *mut core::ffi::c_void))
    }
}

/// 增加 DLL 引用计数。
pub fn dll_add_ref() {
    G_REF_COUNT.fetch_add(1, Ordering::SeqCst);
}

/// 减少 DLL 引用计数。
pub fn dll_release() {
    G_REF_COUNT.fetch_sub(1, Ordering::SeqCst);
}

/// 当前 DLL 引用计数。
pub fn ref_count() -> i32 {
    G_REF_COUNT.load(Ordering::SeqCst)
}
