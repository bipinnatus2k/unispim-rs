//! COM 服务器：类工厂与 DLL 导出。
//!
//! 对应 TSFexample 的 `Server.cpp`。

use windows::core::{implement, Interface, Ref, Result as WinResult, GUID, IUnknown};
use windows::Win32::Foundation::{BOOL, HINSTANCE, S_FALSE, S_OK};
use windows::Win32::System::Com::{IClassFactory, IClassFactory_Impl};

use crate::register::{unregister, CLSID_TEXT_SERVICE};
use crate::text_service::TextService;
use crate::tsf::TsfAdapter;
use crate::{dll_add_ref, dll_release};

/// 类工厂：为文本服务 CLSID 创建 `TextService` 对象。
#[implement(IClassFactory)]
pub struct ClassFactory {
    /// 要创建的 COM 类的 CLSID。
    pub clsid: GUID,
}

impl ClassFactory {
    /// 创建类工厂。
    pub fn new(clsid: GUID) -> Self {
        ClassFactory { clsid }
    }
}

impl IClassFactory_Impl for ClassFactory_Impl {
    fn CreateInstance(
        &self,
        punkouter: Ref<'_, windows::core::IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut core::ffi::c_void,
    ) -> WinResult<()> {
        unsafe {
            if !punkouter.is_null() {
                return Err(windows::core::Error::from(
                    windows::Win32::Foundation::CLASS_E_NOAGGREGATION,
                ));
            }
            let adapter = TsfAdapter::default_engine();
            let service = TextService::new(adapter);
            let unknown: IUnknown = service.into();
            query_interface(&unknown, riid, ppvobject)
        }
    }

    fn LockServer(&self, flock: BOOL) -> WinResult<()> {
        if flock.0 != 0 {
            dll_add_ref();
        } else {
            dll_release();
        }
        Ok(())
    }
}

/// 对任意 COM 接口执行 QueryInterface。
///
/// 通过该接口的 IUnknown vtable 调用 QueryInterface。
/// 不额外增加/释放引用计数（由调用方持有的 `cf` 保证对象存活）。
unsafe fn query_interface(
    obj: &impl Interface,
    riid: *const GUID,
    ppvobject: *mut *mut core::ffi::c_void,
) -> WinResult<()> {
    if ppvobject.is_null() {
        return Err(windows::core::Error::from(windows::Win32::Foundation::E_POINTER));
    }
    unsafe {
        *ppvobject = std::ptr::null_mut();
    }
    let vtable = unsafe { obj.assume_vtable::<IUnknown>() };
    let hr = unsafe { (vtable.QueryInterface)(obj.as_raw(), riid, ppvobject) };
    if hr.is_ok() {
        Ok(())
    } else {
        Err(windows::core::Error::from(hr))
    }
}

/// `DllGetClassObject`：供 COM 运行时获取类工厂。
///
/// # Safety
/// 标准 COM DLL 导出函数，由系统调用。
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut core::ffi::c_void,
) -> windows::core::HRESULT {
    if ppv.is_null() {
        return windows::Win32::Foundation::E_POINTER;
    }
    unsafe {
        *ppv = std::ptr::null_mut();
    }
    if rclsid.is_null() || riid.is_null() {
        return windows::Win32::Foundation::E_INVALIDARG;
    }

    if unsafe { *rclsid != CLSID_TEXT_SERVICE } {
        return windows::Win32::Foundation::CLASS_E_CLASSNOTAVAILABLE;
    }

    let factory = ClassFactory::new(CLSID_TEXT_SERVICE);
    let cf: IClassFactory = factory.into();
    match unsafe { query_interface(&cf, riid, ppv) } {
        Ok(()) => S_OK,
        Err(e) => e.into(),
    }
}

/// `DllRegisterServer`：注册文本服务。
///
/// # Safety
/// 标准 COM DLL 导出函数。
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllRegisterServer() -> windows::core::HRESULT {
    crate::set_module_handle(module_handle_of_self());
    if crate::register::register() {
        S_OK
    } else {
        windows::Win32::Foundation::E_FAIL
    }
}

/// `DllUnregisterServer`：注销文本服务。
///
/// # Safety
/// 标准 COM DLL 导出函数。
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllUnregisterServer() -> windows::core::HRESULT {
    unregister();
    S_OK
}

/// `DllCanUnloadNow`：询问是否可卸载。
///
/// # Safety
/// 标准 COM DLL 导出函数。
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllCanUnloadNow() -> windows::core::HRESULT {
    let count = crate::ref_count();
    if count >= 0 {
        S_FALSE
    } else {
        S_OK
    }
}

/// 获取当前模块（DLL）句柄。
fn module_handle_of_self() -> Option<HINSTANCE> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::HMODULE;
    use windows::Win32::System::LibraryLoader::{
        GetModuleHandleExW, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
    };
    unsafe {
        let mut handle: HMODULE = HMODULE(std::ptr::null_mut());
        let _ = GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
            PCWSTR(module_handle_of_self as *const u8 as *const u16),
            &mut handle,
        );
        if !handle.0.is_null() {
            Some(HINSTANCE(handle.0))
        } else {
            None
        }
    }
}
