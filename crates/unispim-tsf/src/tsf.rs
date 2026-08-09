//! TSF 与输入法引擎的适配层。
//!
//! 将 `unispim_core::ime::ImeEngine` 的状态机接入 Windows TSF：
//! - 按键 -> 引擎
//! - 引擎动作 -> ITfRange 写文本 / ITfComposition 预编辑 / ITfCandidateList 候选

use unispim_core::hzdata::HzData;
use unispim_core::ime::{ImeAction, ImeEngine, KeyInput};
use unispim_core::wordlib::WordLib;

use crate::register::{DATA_DIR_REG_KEY, DATA_DIR_REG_VALUE};

/// 一条待写入的文本变化。
#[derive(Debug, Clone)]
pub enum TsfOp {
    /// 上屏文本。
    Commit(String),
    /// 更新预编辑（拼音/候选）。
    Preedit {
        /// 预编辑文本。
        text: String,
        /// 光标位置（字符数）。
        cursor: i32,
    },
    /// 清空预编辑。
    ClearPreedit,
    /// 更新候选列表。
    Candidates {
        candidates: Vec<String>,
        page: usize,
        page_count: usize,
    },
}

/// 从注册表读取数据目录。
///
/// 注册表路径：`HKLM\SOFTWARE\uniSpim` 值 `DataDir`。
pub fn data_dir_from_registry() -> Option<String> {
    use windows::Win32::System::Registry::{
        RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ,
    };
    use windows::core::PCWSTR;
    unsafe {
        let key: Vec<u16> = DATA_DIR_REG_KEY.encode_utf16().collect();
        let value: Vec<u16> = DATA_DIR_REG_VALUE.encode_utf16().collect();
        let mut buf = [0u16; 1024];
        let mut len = (buf.len() * 2) as u32;
        let err = RegGetValueW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(key.as_ptr()),
            PCWSTR(value.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr() as *mut core::ffi::c_void),
            Some(&mut len),
        );
        if err.0 == 0 {
            let chars = (len as usize) / 2;
            let end = buf[..chars.min(buf.len())]
                .iter()
                .position(|&u| u == 0)
                .unwrap_or(chars.min(buf.len()));
            Some(String::from_utf16_lossy(&buf[..end]))
        } else {
            None
        }
    }
}

/// 从数据目录加载词库与汉字数据。
///
/// 目录结构约定（与仓库 `data/unispim6` 一致）：
/// ```text
/// <dir>/wordlib/*.uwl
/// <dir>/zi/hzpy.dat
/// ```
pub fn load_engine_from_dir(dir: &str) -> ImeEngine {
    let wordlib_dir = std::path::Path::new(dir).join("wordlib");
    let mut wordlibs = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&wordlib_dir) {
        let mut paths: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        paths.sort_by_key(|e| e.file_name().to_string_lossy().to_string());
        for entry in paths {
            let path = entry.path();
            if path.extension().map(|e| e == "uwl").unwrap_or(false)
                && let Ok(wl) = WordLib::from_file(&path.to_string_lossy()) {
                    log_debug(&format!(
                        "加载词库: {} ({} 条)",
                        path.display(),
                        wl.header.word_count
                    ));
                    wordlibs.push(wl);
                }
        }
    }
    let hzpy = std::path::Path::new(dir).join("zi/hzpy.dat");
    let hzdata = HzData::from_file(&hzpy.to_string_lossy());
    ImeEngine::new(wordlibs, hzdata)
}

fn log_debug(_msg: &str) {
    // DLL 中无控制台，暂时留空；可改为 OutputDebugString
}

/// TSF 适配器：持有引擎与符号状态，将按键转换为 `TsfOp` 序列。
#[derive(Clone)]
pub struct TsfAdapter {
    /// 引擎。
    engine: ImeEngine,
    /// 上一字符是否为数字。
    last_digital: bool,
    /// 符号状态。
    symbol: unispim_core::symbol::SymbolState,
}

impl TsfAdapter {
    /// 创建适配器。
    pub fn new(engine: ImeEngine) -> Self {
        TsfAdapter {
            engine,
            last_digital: false,
            symbol: unispim_core::symbol::SymbolState::default(),
        }
    }

    /// 创建带真实数据的适配器（供 DLL 创建使用）。
    ///
    /// 依次尝试：
    /// 1. 注册表 `HKLM\SOFTWARE\uniSpim\DataDir` 指定的数据目录
    /// 2. 当前工作目录下的 `data/unispim6`
    /// 3. DLL 所在目录下的 `data/unispim6`
    pub fn default_engine() -> Self {
        let mut engine = None;

        // 1. 注册表
        if let Some(dir) = data_dir_from_registry()
            && std::path::Path::new(&dir).exists() {
                let e = load_engine_from_dir(&dir);
                if !e.is_empty() {
                    engine = Some(e);
                }
            }

        // 2. 当前工作目录
        if engine.is_none()
            && let Ok(cwd) = std::env::current_dir() {
                let dir = cwd.join("data/unispim6");
                if dir.exists() {
                    let e = load_engine_from_dir(&dir.to_string_lossy());
                    if !e.is_empty() {
                        engine = Some(e);
                    }
                }
            }

        // 3. DLL 所在目录
        if engine.is_none()
            && let Some(hmod) = crate::module_handle() {
                use windows::Win32::Foundation::HMODULE;
                use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
                let hmod = HMODULE(hmod.0);
                let mut buf = vec![0u16; 1024];
                let len = unsafe { GetModuleFileNameW(Some(hmod), &mut buf) };
                if len > 0 {
                    let dll = String::from_utf16_lossy(&buf[..len as usize]);
                    if let Some(dir) = std::path::Path::new(&dll).parent() {
                        let data_dir = dir.join("data/unispim6");
                        if data_dir.exists() {
                            let e = load_engine_from_dir(&data_dir.to_string_lossy());
                            if !e.is_empty() {
                                engine = Some(e);
                            }
                        }
                    }
                }
            }

        TsfAdapter::new(engine.unwrap_or_else(|| ImeEngine::new(Vec::new(), None)))
    }

    /// 引擎引用。
    pub fn engine(&self) -> &ImeEngine {
        &self.engine
    }

    /// 引擎可变引用。
    pub fn engine_mut(&mut self) -> &mut ImeEngine {
        &mut self.engine
    }

    /// 重置（清空组合）。
    pub fn reset(&mut self) {
        self.engine.reset();
        self.last_digital = false;
    }

    /// 处理一个虚拟键码，产生 TSF 操作序列。
    ///
    /// `vk` 为 Windows 虚拟键码（VK_*）。
    pub fn handle_vk(&mut self, vk: u16) -> Vec<TsfOp> {
        let key = vk_to_key(vk);
        let output = self.engine.handle_key(&key);
        let mut ops = Vec::new();
        for action in output.actions {
            match action {
                ImeAction::None => {}
                ImeAction::Commit(text) => {
                    // 中文模式标点转换（仅标点/空白，字母数字等原样输出）
                    let mut final_text = String::new();
                    for ch in text.chars() {
                        if self.engine.mode == unispim_core::ime::ImeMode::Chinese
                            && (unispim_core::symbol::is_symbol_char(ch) || ch == ' ')
                        {
                            match unispim_core::symbol::get_symbol(
                                ch,
                                &mut self.symbol,
                                true,
                                self.last_digital,
                            ) {
                                Some(sym) => final_text.push_str(&sym),
                                None => final_text.push(ch),
                            }
                        } else {
                            final_text.push(ch);
                        }
                    }
                    self.last_digital = text.chars().last().map(|c| c.is_ascii_digit()).unwrap_or(false);
                    ops.push(TsfOp::Commit(final_text));
                }
                ImeAction::UpdatePreedit(text) => {
                    let cursor = text.chars().count() as i32;
                    ops.push(TsfOp::Preedit { text, cursor });
                }
                ImeAction::ClearPreedit => {
                    ops.push(TsfOp::ClearPreedit);
                }
                ImeAction::UpdateCandidates {
                    candidates,
                    page,
                    page_count,
                } => {
                    ops.push(TsfOp::Candidates {
                        candidates,
                        page,
                        page_count,
                    });
                }
            }
        }
        ops
    }

    /// 当前候选（当前页）。
    pub fn current_candidates(&self) -> Vec<String> {
        self.engine
            .current_page_candidates()
            .iter()
            .map(|c| c.text())
            .collect()
    }
}

/// 虚拟键码 -> 抽象按键。
pub fn vk_to_key(vk: u16) -> KeyInput {
    // VK_0-VK_9 = 0x30-0x39
    if (0x30..=0x39).contains(&vk) {
        return KeyInput::Digit((vk - 0x30) as u8);
    }
    // A-Z = 0x41-0x5A（注意可能带 Shift，需结合大小写）
    if (0x41..=0x5A).contains(&vk) {
        let c = (vk - 0x41) as u8 + b'a';
        return KeyInput::Letter(c as char);
    }
    match vk {
        0x20 => KeyInput::Space,                 // VK_SPACE
        0x08 => KeyInput::Backspace,             // VK_BACK
        0x0D => KeyInput::Enter,                 // VK_RETURN
        0x21 => KeyInput::PageUp,                // VK_PRIOR
        0x22 => KeyInput::PageDown,              // VK_NEXT
        // 小键盘 0-9
        0x60..=0x69 => KeyInput::Digit((vk - 0x60) as u8),
        // 标点符号 VK（OEM 键）
        0xBA => KeyInput::Char(';'),
        0xBB => KeyInput::Char('='),
        0xBC => KeyInput::Char(','),
        0xBD => KeyInput::Char('-'),
        0xBE => KeyInput::Char('.'),
        0xBF => KeyInput::Char('/'),
        0xC0 => KeyInput::Char('`'),
        0xDB => KeyInput::Char('['),
        0xDC => KeyInput::Char('\\'),
        0xDD => KeyInput::Char(']'),
        0xDE => KeyInput::Char('\''),
        _ => KeyInput::Ignored,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use unispim_core::hzdata::HzData;
    use unispim_core::wordlib::WordLib;

    fn test_adapter() -> TsfAdapter {
        let mut wl = WordLib::create_empty("t", "t", 1);
        let add = |wl: &mut WordLib, hz: &str, py: &str, freq: u32| {
            let hz: Vec<u16> = hz.encode_utf16().collect();
            let syl = unispim_core::parse::parse_pin_yin_string_reverse(py, 0);
            assert_eq!(hz.len(), syl.len(), "{}", py);
            wl.add_ci(&hz, &syl, freq, true).unwrap();
        };
        add(&mut wl, "中国", "zhongguo", 100);
        add(&mut wl, "人民", "renmin", 90);
        let engine = ImeEngine::new(vec![wl], None::<HzData>);
        TsfAdapter::new(engine)
    }

    #[test]
    fn test_vk_letters() {
        assert_eq!(vk_to_key(0x41), KeyInput::Letter('a'));
        assert_eq!(vk_to_key(0x5A), KeyInput::Letter('z'));
        assert_eq!(vk_to_key(0x20), KeyInput::Space);
        assert_eq!(vk_to_key(0x08), KeyInput::Backspace);
        assert_eq!(vk_to_key(0x0D), KeyInput::Enter);
        assert_eq!(vk_to_key(0x31), KeyInput::Digit(1));
        assert_eq!(vk_to_key(0x62), KeyInput::Digit(2));
    }

    #[test]
    fn test_commit_flow() {
        let mut ad = test_adapter();
        // 输入 zhongguo -> 空格选中"中国"
        for c in "zhongguo".chars() {
            let vk = (c as u16) - 'a' as u16 + 0x41;
            ad.handle_vk(vk);
        }
        assert_eq!(ad.engine().composition, "zhongguo");
        let ops = ad.handle_vk(0x20); // 空格
        let commits: Vec<String> = ops
            .iter()
            .filter_map(|op| match op {
                TsfOp::Commit(s) => Some(s.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(commits, vec!["中国".to_string()]);
        assert!(ad.engine().composition.is_empty());
    }

    #[test]
    fn test_symbol_commit() {
        let mut ad = test_adapter();
        // 输入 zhong 后输入逗号，先上屏拼音再上屏中文逗号
        for c in "zhong".chars() {
            let vk = (c as u16) - 'a' as u16 + 0x41;
            ad.handle_vk(vk);
        }
        let ops = ad.handle_vk(0xBC); // VK_OEM_COMMA
        let commits: Vec<String> = ops
            .iter()
            .filter_map(|op| match op {
                TsfOp::Commit(s) => Some(s.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(commits, vec!["zhong".to_string(), "，".to_string()]);
    }

    #[test]
    fn test_load_real_data_dir() {
        // 从 workspace 根向上查找 data/unispim6，验证真实数据加载
        let mut dir = std::env::current_dir().ok();
        let mut found = None;
        while let Some(d) = dir {
            let candidate = d.join("data/unispim6");
            if candidate.join("wordlib/sys.uwl").exists() && candidate.join("zi/hzpy.dat").exists() {
                found = Some(candidate);
                break;
            }
            dir = d.parent().map(|p| p.to_path_buf());
        }
        let Some(data_dir) = found else {
            eprintln!("跳过：未找到 data/unispim6");
            return;
        };

        let engine = load_engine_from_dir(&data_dir.to_string_lossy());
        assert!(!engine.is_empty(), "应加载到词库或汉字数据");

        // 验证能产生候选
        let mut ad = TsfAdapter::new(engine);
        for c in "zhongguo".chars() {
            let vk = (c as u16) - 'a' as u16 + 0x41;
            ad.handle_vk(vk);
        }
        let candidates = ad.current_candidates();
        assert!(
            candidates.iter().any(|s| s == "中国"),
            "zhongguo 应产生候选 中国: {:?}",
            &candidates[..candidates.len().min(10)]
        );
    }
}
