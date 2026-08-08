//! TSF 与输入法引擎的适配层。
//!
//! 将 `unispim_rs::ime::ImeEngine` 的状态机接入 Windows TSF：
//! - 按键 -> 引擎
//! - 引擎动作 -> ITfRange 写文本 / ITfComposition 预编辑 / ITfCandidateList 候选

use crate::ime::{ImeAction, ImeEngine, KeyInput};

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

/// TSF 适配器：持有引擎与符号状态，将按键转换为 `TsfOp` 序列。
#[derive(Clone)]
pub struct TsfAdapter {
    /// 引擎。
    engine: ImeEngine,
    /// 上一字符是否为数字。
    last_digital: bool,
    /// 符号状态。
    symbol: crate::symbol::SymbolState,
}

impl TsfAdapter {
    /// 创建适配器。
    pub fn new(engine: ImeEngine) -> Self {
        TsfAdapter {
            engine,
            last_digital: false,
            symbol: crate::symbol::SymbolState::default(),
        }
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
                        if self.engine.mode == crate::ime::ImeMode::Chinese
                            && (crate::symbol::is_symbol_char(ch) || ch == ' ')
                        {
                            match crate::symbol::get_symbol(
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
    use crate::hzdata::HzData;
    use crate::wordlib::WordLib;

    fn test_adapter() -> TsfAdapter {
        let mut wl = WordLib::create_empty("t", "t", 1);
        let add = |wl: &mut WordLib, hz: &str, py: &str, freq: u32| {
            let hz: Vec<u16> = hz.encode_utf16().collect();
            let syl = crate::parse::parse_pin_yin_string_reverse(py, 0);
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
}
