//! TSF 文本服务的注册与注销。
//!
//! 对应 TSFexample 的 `Register.cpp`：
//! - 注册表注册（HKCR\CLSID\{...} + InProcServer32）
//! - TSF 文本服务 profile 注册（ITfInputProcessorProfiles）
//! - TSF 分类注册（ITfCategoryMgr）

use windows::core::{GUID, PCWSTR};
use windows::Win32::Foundation::WIN32_ERROR;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_ALL, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::LibraryLoader::{GetModuleFileNameW, GetModuleHandleW};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegSetValueExW, HKEY, HKEY_CLASSES_ROOT,
    KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ,
};
use windows::Win32::UI::TextServices::{
    CLSID_TF_CategoryMgr, CLSID_TF_InputProcessorProfiles, GUID_TFCAT_TIP_KEYBOARD,
    ITfCategoryMgr, ITfInputProcessorProfiles,
};

/// 文本服务的 CLSID。
pub const CLSID_TEXT_SERVICE: GUID = GUID::from_u128(0xe7ea138e_69f8_11d7_a6ea_00065b84435c);
/// 输入法 profile GUID。
pub const GUID_PROFILE: GUID = GUID::from_u128(0xe7ea138f_69f8_11d7_a6ea_00065b84435c);

/// 文本服务名称。
pub const TEXT_SERVICE_DESC: &str = "华宇拼音输入法";
/// 线程模型。
pub const THREADING_MODEL: &str = "Apartment";
/// 语言 ID（简体中文 0x0804）。
pub const TEXTSERVICE_LANGID: u16 = 0x0804;

/// 注册表前缀。
const REG_PREFIX: &str = "CLSID\\";

fn ok(e: WIN32_ERROR) -> bool {
    e.0 == 0
}

/// 将 GUID 转为注册表路径格式字符串 `{xxxxxxxx-xxxx-...}`。
fn guid_to_string(guid: &GUID) -> String {
    format!(
        "{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        guid.data1,
        guid.data2,
        guid.data3,
        guid.data4[0],
        guid.data4[1],
        guid.data4[2],
        guid.data4[3],
        guid.data4[4],
        guid.data4[5],
        guid.data4[6],
        guid.data4[7]
    )
}

/// 获取当前模块（DLL）的完整路径。
///
/// 优先使用 `DllRegisterServer` 中通过 `set_module_handle` 保存的 DLL 模块句柄，
/// 否则回退到当前进程主模块路径（便于独立注册工具使用）。
fn module_path() -> String {
    use windows::Win32::Foundation::HMODULE;
    unsafe {
        // 若已通过 DllRegisterServer 保存了 DLL 模块句柄，则使用它
        if let Some(hmod) = crate::module_handle() {
            let hmod = HMODULE(hmod.0);
            let mut buf = vec![0u16; 1024];
            let len = GetModuleFileNameW(Some(hmod), &mut buf);
            if len > 0 {
                return String::from_utf16_lossy(&buf[..len as usize]);
            }
        }
        let hmod = GetModuleHandleW(PCWSTR::null()).unwrap_or_default();
        let mut buf = vec![0u16; 1024];
        let len = GetModuleFileNameW(Some(hmod), &mut buf);
        String::from_utf16_lossy(&buf[..len as usize])
    }
}

/// 将宽字符串转为注册表需要的字节（含结尾 NUL）。
fn wide_bytes(wide: &[u16]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(wide.len() * 2 + 2);
    for &u in wide {
        bytes.extend_from_slice(&u.to_le_bytes());
    }
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes
}

/// 注册 COM 服务器（注册表）。
///
/// `dll_path` 为 `None` 时自动探测当前模块路径。
fn register_server(dll_path: Option<&str>) -> bool {
    let key_path = format!("{}{}", REG_PREFIX, guid_to_string(&CLSID_TEXT_SERVICE));
    let key_path: Vec<u16> = key_path.encode_utf16().collect();
    let key_path = PCWSTR(key_path.as_ptr());

    let mut hkey: HKEY = HKEY(std::ptr::null_mut());
    let err = unsafe {
        RegCreateKeyExW(
            HKEY_CLASSES_ROOT,
            key_path,
            None,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut hkey,
            None,
        )
    };
    if !ok(err) {
        return false;
    }

    let desc: Vec<u16> = TEXT_SERVICE_DESC.encode_utf16().collect();
    let err = unsafe {
        RegSetValueExW(hkey, None, None, REG_SZ, Some(&wide_bytes(&desc)))
    };
    if !ok(err) {
        unsafe { let _ = RegCloseKey(hkey); }
        return false;
    }

    // InProcServer32
    let inproc: Vec<u16> = "InProcServer32".encode_utf16().collect();
    let inproc = PCWSTR(inproc.as_ptr());
    let mut hsub: HKEY = HKEY(std::ptr::null_mut());
    let err = unsafe {
        RegCreateKeyExW(
            hkey,
            inproc,
            None,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut hsub,
            None,
        )
    };
    if !ok(err) {
        unsafe { let _ = RegCloseKey(hkey); }
        return false;
    }

    // 默认值 = DLL 路径（优先使用显式指定路径）
    let dll = match dll_path {
        Some(p) => p.to_string(),
        None => module_path(),
    };
    let module_wide: Vec<u16> = dll.encode_utf16().collect();
    let err = unsafe {
        RegSetValueExW(hsub, None, None, REG_SZ, Some(&wide_bytes(&module_wide)))
    };
    if ok(err) {
        // ThreadingModel = Apartment
        let model: Vec<u16> = THREADING_MODEL.encode_utf16().collect();
        let name: Vec<u16> = "ThreadingModel".encode_utf16().collect();
        unsafe {
            let _ = RegSetValueExW(
                hsub,
                PCWSTR(name.as_ptr()),
                None,
                REG_SZ,
                Some(&wide_bytes(&model)),
            );
        }
    }

    unsafe {
        let _ = RegCloseKey(hsub);
        let _ = RegCloseKey(hkey);
    }
    true
}

/// 注销 COM 服务器（注册表）。
fn unregister_server() {
    let key_path = format!("{}{}", REG_PREFIX, guid_to_string(&CLSID_TEXT_SERVICE));
    let key_path: Vec<u16> = key_path.encode_utf16().collect();
    unsafe {
        let _ = RegDeleteTreeW(HKEY_CLASSES_ROOT, PCWSTR(key_path.as_ptr()));
    }
}

/// 注册 TSF 输入法 profile。
fn register_profiles() -> bool {
    unsafe {
        let profiles: ITfInputProcessorProfiles = match CoCreateInstance(
            &CLSID_TF_InputProcessorProfiles,
            None,
            CLSCTX_INPROC_SERVER,
        ) {
            Ok(p) => p,
            Err(_) => return false,
        };
        if profiles.Register(&CLSID_TEXT_SERVICE).is_err() {
            return false;
        }
        let desc: Vec<u16> = TEXT_SERVICE_DESC.encode_utf16().collect();
        let module_path = module_path();
        let icon: Vec<u16> = module_path.encode_utf16().collect();
        profiles
            .AddLanguageProfile(
                &CLSID_TEXT_SERVICE,
                TEXTSERVICE_LANGID,
                &GUID_PROFILE,
                &desc,
                &icon,
                0,
            )
            .is_ok()
    }
}

/// 注销 TSF 输入法 profile。
fn unregister_profiles() {
    unsafe {
        if let Ok(profiles) = CoCreateInstance::<_, ITfInputProcessorProfiles>(
            &CLSID_TF_InputProcessorProfiles,
            None,
            CLSCTX_INPROC_SERVER,
        ) {
            let _ = profiles.Unregister(&CLSID_TEXT_SERVICE);
        }
    }
}

/// 注册 TSF 分类（键盘输入法）。
fn register_categories() -> bool {
    unsafe {
        let category_mgr: ITfCategoryMgr =
            match CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_ALL) {
                Ok(c) => c,
                Err(_) => return false,
            };
        category_mgr
            .RegisterCategory(&CLSID_TEXT_SERVICE, &GUID_TFCAT_TIP_KEYBOARD, &CLSID_TEXT_SERVICE)
            .is_ok()
    }
}

/// 注销 TSF 分类。
fn unregister_categories() {
    unsafe {
        if let Ok(category_mgr) =
            CoCreateInstance::<_, ITfCategoryMgr>(&CLSID_TF_CategoryMgr, None, CLSCTX_ALL)
        {
            let _ = category_mgr.UnregisterCategory(
                &CLSID_TEXT_SERVICE,
                &GUID_TFCAT_TIP_KEYBOARD,
                &CLSID_TEXT_SERVICE,
            );
        }
    }
}

/// 注册全部（DllRegisterServer 入口）。
///
/// `dll_path` 为 `None` 时自动探测当前模块路径（适用于 DLL 自身调用
/// `DllRegisterServer`，此时模块句柄已在 server.rs 中设置）。
pub fn register_with_path(dll_path: Option<&str>) -> bool {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }
    if !register_server(dll_path) || !register_profiles() || !register_categories() {
        unregister();
        return false;
    }
    true
}

/// 注册全部（自动探测模块路径）。
pub fn register() -> bool {
    register_with_path(None)
}

/// 注销全部（DllUnregisterServer 入口）。
pub fn unregister() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }
    unregister_profiles();
    unregister_categories();
    unregister_server();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guid_string() {
        let s = guid_to_string(&CLSID_TEXT_SERVICE);
        assert!(s.starts_with("{"));
        assert!(s.ends_with("}"));
        assert_eq!(s.len(), 38);
        assert_eq!(s, "{E7EA138E-69F8-11D7-A6EA-00065B84435C}");
    }

    #[test]
    fn test_wide_bytes() {
        let wide: Vec<u16> = "AB".encode_utf16().collect();
        let b = wide_bytes(&wide);
        assert_eq!(b, vec![0x41, 0x00, 0x42, 0x00, 0x00, 0x00]);
    }
}
