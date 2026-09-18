//! 输入法引擎（纯状态机）。
//!
//! 对应原 C 工程的 editor/kernel 的核心逻辑，但作为平台无关的库实现：
//! 输入按键 -> 更新状态 -> 产生动作（候选 / 上屏 / 预编辑）。
//! 实际的平台集成（Wayland / X11 / Windows TSF）由 imekit 适配器完成。

use crate::ci::{process_ci_candidates, CiCandidate};
use crate::hzdata::HzData;
use crate::parse::{parse_pin_yin_string_reverse, syllables_to_string};
use crate::syllable::Syllable;
use crate::wordlib::WordLib;

/// 一页显示的候选数（与原 MAX_CANDIDATES_PER_LINE 一致）。
pub const CANDIDATES_PER_PAGE: usize = 9;
/// 最大候选页数。
pub const MAX_PAGE_COUNT: usize = 32;

/// 输入法模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImeMode {
    /// 中文输入。
    Chinese,
    /// 英文输入。
    English,
}

/// 一个候选。
#[derive(Debug, Clone)]
pub enum Candidate {
    /// 词候选。
    Ci(CiCandidate),
    /// 单字候选。
    Zi(crate::hzdata::HzItem),
}impl Candidate {
    /// 候选文本。
    pub fn text(&self) -> String {
        match self {
            Candidate::Ci(c) => String::from_utf16_lossy(&c.item.hz),
            Candidate::Zi(z) => {
                if let Some(c) = char::from_u32(z.hz) {
                    c.to_string()
                } else {
                    String::new()
                }
            }
        }
    }
}

/// 引擎产生的一个动作。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImeAction {
    /// 无动作。
    None,
    /// 上屏文本。
    Commit(String),
    /// 更新预编辑（候选/拼音）。
    UpdatePreedit(String),
    /// 清除预编辑。
    ClearPreedit,
    /// 更新候选列表（候选文本、页码）。
    UpdateCandidates {
        candidates: Vec<String>,
        page: usize,
        page_count: usize,
    },
}

/// 引擎输出。
#[derive(Debug, Clone)]
pub struct EngineOutput {
    pub actions: Vec<ImeAction>,
}

impl Default for EngineOutput {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineOutput {
    pub fn new() -> Self {
        EngineOutput { actions: Vec::new() }
    }
}

/// 按键输入（已转换为抽象按键）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyInput {
    /// 字母（a-z）。
    Letter(char),
    /// 数字键（候选选择）。
    Digit(u8),
    /// 空格（确认候选）。
    Space,
    /// 退格。
    Backspace,
    /// 回车（直接上屏拼音）。
    Enter,
    /// 上翻页。
    PageUp,
    /// 下翻页。
    PageDown,
    /// 其它字符（标点等，直接上屏）。
    Char(char),
    /// 无操作按键。
    Ignored,
}

/// 输入法引擎。
#[derive(Clone)]
pub struct ImeEngine {
    /// 模式。
    pub mode: ImeMode,
    /// 拼音缓冲区。
    pub composition: String,
    /// 音节。
    pub syllables: Vec<Syllable>,
    /// 当前候选（全部，跨页）。
    pub candidates: Vec<Candidate>,
    /// 当前页。
    pub page: usize,
    /// 词库。
    wordlibs: Vec<WordLib>,
    /// 汉字数据。
    hzdata: Option<HzData>,
    /// 模糊音模式。
    pub fuzzy_mode: u32,
    /// 上一个字符是否为数字。
    pub last_digital: bool,
}

impl ImeEngine {
    /// 创建引擎。
    pub fn new(wordlibs: Vec<WordLib>, hzdata: Option<HzData>) -> Self {
        ImeEngine {
            mode: ImeMode::Chinese,
            composition: String::new(),
            syllables: Vec::new(),
            candidates: Vec::new(),
            page: 0,
            wordlibs,
            hzdata,
            fuzzy_mode: 0,
            last_digital: false,
        }
    }

    /// 是否为空（无词库且无汉字数据）。
    pub fn is_empty(&self) -> bool {
        self.wordlibs.is_empty() && self.hzdata.is_none()
    }

    /// 是否有可用资源（词库或汉字数据）。
    pub fn has_resources(&self) -> bool {
        !self.wordlibs.is_empty() || self.hzdata.is_some()
    }

    /// 处理按键，产生动作。
    pub fn handle_key(&mut self, key: &KeyInput) -> EngineOutput {
        let mut out = EngineOutput::new();
        match key {
            KeyInput::Letter(c) => {
                self.last_digital = false;
                if self.mode == ImeMode::Chinese {
                    self.composition.push(*c);
                    self.update_composition(&mut out);
                } else {
                    out.actions.push(ImeAction::Commit(c.to_string()));
                }
            }
            KeyInput::Space => {
                self.last_digital = false;
                if self.mode == ImeMode::Chinese {
                    self.handle_space(&mut out);
                } else {
                    out.actions.push(ImeAction::Commit(" ".to_string()));
                }
            }
            KeyInput::Digit(d) => {
                self.last_digital = true;
                if self.mode == ImeMode::Chinese && !self.composition.is_empty() {
                    self.select_candidate(*d, &mut out);
                } else {
                    out.actions.push(ImeAction::Commit(d.to_string()));
                }
            }
            KeyInput::Backspace => {
                self.last_digital = false;
                if self.mode == ImeMode::Chinese && !self.composition.is_empty() {
                    self.composition.pop();
                    self.update_composition(&mut out);
                } else {
                    // 交给应用处理
                    out.actions.push(ImeAction::None);
                }
            }
            KeyInput::Enter => {
                self.last_digital = false;
                if self.mode == ImeMode::Chinese && !self.composition.is_empty() {
                    // 直接上屏拼音
                    let text = self.composition.clone();
                    self.clear_composition(&mut out);
                    out.actions.push(ImeAction::Commit(text));
                }
            }
            KeyInput::PageUp => {
                if self.mode == ImeMode::Chinese && !self.candidates.is_empty() {
                    self.page = self.page.saturating_sub(1);
                    self.push_candidates(&mut out);
                }
            }
            KeyInput::PageDown => {
                if self.mode == ImeMode::Chinese && !self.candidates.is_empty() {
                    let max_page = self.page_count().saturating_sub(1);
                    if self.page < max_page {
                        self.page += 1;
                    }
                    self.push_candidates(&mut out);
                }
            }
            KeyInput::Char(c) => {
                self.last_digital = c.is_ascii_digit();
                if self.mode == ImeMode::Chinese && !self.composition.is_empty() {
                    // 有未完成拼音，先上屏拼音，再上屏符号
                    let text = self.composition.clone();
                    self.clear_composition(&mut out);
                    out.actions.push(ImeAction::Commit(text));
                }
                out.actions.push(ImeAction::Commit(c.to_string()));
            }
            KeyInput::Ignored => {}
        }
        out
    }

    /// 总页数。
    pub fn page_count(&self) -> usize {
        if self.candidates.is_empty() {
            0
        } else {
            self.candidates.len().div_ceil(CANDIDATES_PER_PAGE)
        }
    }

    /// 当前页候选。
    pub fn current_page_candidates(&self) -> Vec<&Candidate> {
        let start = self.page * CANDIDATES_PER_PAGE;
        self.candidates
            .iter()
            .skip(start)
            .take(CANDIDATES_PER_PAGE)
            .collect()
    }

    /// 拼音串（当前解析结果）。
    pub fn preedit_pinyin(&self) -> String {
        if self.composition.is_empty() {
            String::new()
        } else if self.syllables.is_empty() {
            self.composition.clone()
        } else {
            syllables_to_string(&self.syllables)
        }
    }

    /// 清除全部候选。
    pub fn clear_candidates(&mut self) {
        self.candidates.clear();
        self.page = 0;
    }

    /// 重置引擎。
    pub fn reset(&mut self) {
        self.composition.clear();
        self.syllables.clear();
        self.clear_candidates();
    }

    /// 更新组合状态（重新解析拼音并生成候选）。
    fn update_composition(&mut self, out: &mut EngineOutput) {
        // 解析拼音
        self.syllables = parse_pin_yin_string_reverse(&self.composition, self.fuzzy_mode);
        if self.syllables.is_empty() {
            self.clear_candidates();
            out.actions.push(ImeAction::UpdatePreedit(self.composition.clone()));
            return;
        }
        // 生成候选
        self.candidates.clear();
        self.page = 0;

        // 词候选
        for wl in &self.wordlibs {
            let mut cands = process_ci_candidates(wl, &self.syllables, self.fuzzy_mode);
            // 音节长度与词长不同时也允许（模糊词长）
            for cand in cands.drain(..) {
                self.candidates.push(Candidate::Ci(cand));
            }
        }

        // 单音节：加入单字候选（按词频降序）
        if self.syllables.len() == 1
            && let Some(hzdata) = &self.hzdata {
                let mut zi_cands = hzdata.get_zi_candidates_with_fuzzy(&self.syllables[0], self.fuzzy_mode);
                zi_cands.sort_by_key(|b| std::cmp::Reverse(b.freq));
                for z in zi_cands {
                    self.candidates.push(Candidate::Zi(z));
                }
            }

        // 跨词库去重：相同文本保留词频最高者
        self.dedup_candidates();

        // 更新预编辑与候选
        out.actions.push(ImeAction::UpdatePreedit(self.preedit_pinyin()));
        self.push_candidates(out);
    }

    /// 跨词库去重：相同文本（同音节数）保留词频最高者，并保持稳定顺序。
    fn dedup_candidates(&mut self) {
        use std::collections::HashMap;
        let mut best: HashMap<(String, usize), (usize, u32)> = HashMap::new(); // (text, syl_len) -> (idx, freq)
        let mut kept = Vec::with_capacity(self.candidates.len());

        for (idx, cand) in self.candidates.iter().enumerate() {
            let text = cand.text();
            let syl_len = match cand {
                Candidate::Ci(c) => c.item.syllable_length,
                Candidate::Zi(_) => 1,
            };
            let freq = match cand {
                Candidate::Ci(c) => c.item.freq,
                Candidate::Zi(z) => z.freq as u32,
            };
            match best.get(&(text.clone(), syl_len)) {
                Some(&(existing_idx, existing_freq)) => {
                    if freq > existing_freq {
                        // 新候选词频更高：替换
                        if let Some(slot) = kept.iter().position(|&i| i == existing_idx) {
                            kept[slot] = idx;
                        }
                        best.insert((text, syl_len), (idx, freq));
                    }
                    // 否则保留原候选
                }
                None => {
                    best.insert((text, syl_len), (idx, freq));
                    kept.push(idx);
                }
            }
        }

        // 按原顺序重建
        let mut result = Vec::with_capacity(kept.len());
        for idx in kept {
            result.push(self.candidates[idx].clone());
        }
        self.candidates = result;
    }

    /// 空格处理：无候选时直接上屏拼音，有候选时选择第一个。
    fn handle_space(&mut self, out: &mut EngineOutput) {
        if self.composition.is_empty() {
            out.actions.push(ImeAction::Commit(" ".to_string()));
            return;
        }
        if self.candidates.is_empty() {
            let text = self.composition.clone();
            self.clear_composition(out);
            out.actions.push(ImeAction::Commit(text));
            return;
        }
        // 选择第一个候选
        self.select_candidate_at(0, out);
    }

    /// 选择第 n 个候选（1-9）。
    fn select_candidate(&mut self, digit: u8, out: &mut EngineOutput) {
        if digit == 0 {
            return;
        }
        let idx = (digit as usize) - 1;
        self.select_candidate_at(idx, out);
    }

    /// 选择全局候选序号。
    fn select_candidate_at(&mut self, idx: usize, out: &mut EngineOutput) {
        let global = self.page * CANDIDATES_PER_PAGE + idx;
        if global >= self.candidates.len() {
            return;
        }
        let text = self.candidates[global].text();
        self.clear_composition(out);
        out.actions.push(ImeAction::Commit(text));
    }

    /// 清除组合状态。
    fn clear_composition(&mut self, out: &mut EngineOutput) {
        self.composition.clear();
        self.syllables.clear();
        self.clear_candidates();
        out.actions.push(ImeAction::ClearPreedit);
    }

    /// 推送当前页候选。
    fn push_candidates(&mut self, out: &mut EngineOutput) {
        let page_count = self.page_count();
        if self.candidates.is_empty() {
            out.actions.push(ImeAction::UpdateCandidates {
                candidates: Vec::new(),
                page: 0,
                page_count: 0,
            });
            return;
        }
        let cands: Vec<String> = self
            .current_page_candidates()
            .iter()
            .map(|c| c.text())
            .collect();
        out.actions.push(ImeAction::UpdateCandidates {
            candidates: cands,
            page: self.page,
            page_count,
        });
    }

    /// 获取用于调试的完整候选文本列表。
    pub fn candidate_texts(&self) -> Vec<String> {
        self.candidates.iter().map(|c| c.text()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wordlib::WordLib;

    fn test_engine() -> (ImeEngine, Vec<String>) {
        // 构造一个小词库
        let mut wl = WordLib::create_empty("test", "t", 1);
        let add = |wl: &mut WordLib, hz: &str, py: &str, freq: u32| {
            let hz: Vec<u16> = hz.encode_utf16().collect();
            let syl = parse_pin_yin_string_reverse(py, 0);
            assert_eq!(hz.len(), syl.len(), "{}", py);
            wl.add_ci(&hz, &syl, freq, true).unwrap();
        };
        add(&mut wl, "中国", "zhongguo", 100);
        add(&mut wl, "人民", "renmin", 90);
        add(&mut wl, "电脑", "diannao", 80);
        add(&mut wl, "计算机", "jisuanji", 70);
        let engine = ImeEngine::new(vec![wl], None);
        (engine, vec!["中国".to_string(), "人民".to_string(), "电脑".to_string(), "计算机".to_string()])
    }

    #[test]
    fn test_composition_flow() {
        let (mut engine, _) = test_engine();
        engine.handle_key(&KeyInput::Letter('z'));
        assert_eq!(engine.composition, "z");
        assert_eq!(engine.syllables.len(), 1);
        engine.handle_key(&KeyInput::Letter('h'));
        assert_eq!(engine.composition, "zh");
        engine.handle_key(&KeyInput::Letter('o'));
        engine.handle_key(&KeyInput::Letter('n'));
        engine.handle_key(&KeyInput::Letter('g'));
        assert_eq!(engine.composition, "zhong");
        // 单音节 zhong 需要完整的 zhongxx 词才能匹配；这里验证解析正确
        assert_eq!(engine.syllables.len(), 1);
        assert_eq!(engine.preedit_pinyin(), "zhong");
    }

    #[test]
    fn test_space_select_first() {
        let (mut engine, _) = test_engine();
        for c in "zhongguo".chars() {
            engine.handle_key(&KeyInput::Letter(c));
        }
        assert!(!engine.candidates.is_empty());
        let first = engine.candidates[0].text();
        let out = engine.handle_key(&KeyInput::Space);
        let commit = out.actions.iter().find_map(|a| match a {
            ImeAction::Commit(s) => Some(s.clone()),
            _ => None,
        });
        assert_eq!(commit, Some(first.clone()));
        assert!(engine.composition.is_empty());
        let _ = out;
    }

    #[test]
    fn test_digit_select() {
        let (mut engine, _) = test_engine();
        // 输入 renmin：词库中 人民/任命/人敏... 仅有1个同音节词，用 diannao 测试
        for c in "diannao".chars() {
            engine.handle_key(&KeyInput::Letter(c));
        }
        let count = engine.candidates.len();
        assert_eq!(count, 1, "候选应为 1 个: {}", count);
        let first = engine.candidates[0].text();
        let out = engine.handle_key(&KeyInput::Digit(1));
        let commit = out.actions.iter().find_map(|a| match a {
            ImeAction::Commit(s) => Some(s.clone()),
            _ => None,
        });
        assert_eq!(commit, Some(first.clone()));
        assert!(engine.composition.is_empty());
        let _ = out;
    }

    #[test]
    fn test_backspace() {
        let (mut engine, _) = test_engine();
        for c in "zhongguo".chars() {
            engine.handle_key(&KeyInput::Letter(c));
        }
        assert_eq!(engine.composition, "zhongguo");
        engine.handle_key(&KeyInput::Backspace);
        assert_eq!(engine.composition, "zhonggu");
        // 拼音 zhonggu 应重新解析
        assert!(!engine.syllables.is_empty());
    }

    #[test]
    fn test_enter_commits_pinyin() {
        let (mut engine, _) = test_engine();
        for c in "wo".chars() {
            engine.handle_key(&KeyInput::Letter(c));
        }
        let out = engine.handle_key(&KeyInput::Enter);
        let commit = out.actions.iter().find_map(|a| match a {
            ImeAction::Commit(s) => Some(s.clone()),
            _ => None,
        });
        assert_eq!(commit, Some("wo".to_string()));
        let _ = out;
    }

    #[test]
    fn test_page_navigation() {
        let (mut engine, _) = test_engine();
        // 输入 renmin，候选超过一页时翻页
        for c in "renmin".chars() {
            engine.handle_key(&KeyInput::Letter(c));
        }
        // 记录页数
        let page_count = engine.page_count();
        if page_count > 1 {
            let out = engine.handle_key(&KeyInput::PageDown);
            assert_eq!(engine.page, 1);
            let _ = out;
            let out = engine.handle_key(&KeyInput::PageUp);
            assert_eq!(engine.page, 0);
            let _ = out;
        }
    }

    #[test]
    fn test_symbol_commit() {
        let (mut engine, _) = test_engine();
        for c in "zhong".chars() {
            engine.handle_key(&KeyInput::Letter(c));
        }
        let out = engine.handle_key(&KeyInput::Char(','));
        let commits: Vec<String> = out
            .actions
            .iter()
            .filter_map(|a| match a {
                ImeAction::Commit(s) => Some(s.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(commits, vec!["zhong".to_string(), ",".to_string()]);
        assert!(engine.composition.is_empty());
    }

    #[test]
    fn test_english_mode() {
        let (mut engine, _) = test_engine();
        engine.mode = ImeMode::English;
        let out = engine.handle_key(&KeyInput::Letter('h'));
        let commits: Vec<String> = out
            .actions
            .iter()
            .filter_map(|a| match a {
                ImeAction::Commit(s) => Some(s.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(commits, vec!["h".to_string()]);
        assert!(engine.composition.is_empty());
    }

    #[test]
    fn test_preedit_pinyin() {
        let (mut engine, _) = test_engine();
        for c in "zhongguo".chars() {
            engine.handle_key(&KeyInput::Letter(c));
        }
        assert_eq!(engine.preedit_pinyin(), "zhong'guo");
    }

    #[test]
    fn test_ci_syllable_length_fuzzy() {
        // FUZZY_CI_SYLLABLE_LENGTH 模式下词长可与音节长不同
        let (mut engine, _) = test_engine();
        engine.fuzzy_mode = crate::syllable::FUZZY_CI_SYLLABLE_LENGTH;
        for c in "jisuanji".chars() {
            engine.handle_key(&KeyInput::Letter(c));
        }
        let texts = engine.candidate_texts();
        assert!(
            texts.iter().any(|t| t == "计算机"),
            "jisuanji 应含计算机: {:?}",
            texts
        );
    }

    #[test]
    fn test_dedup_across_wordlibs() {
        // 两个词库都有"中国"，去重后应只保留一个
        let mut wl1 = WordLib::create_empty("t1", "t", 1);
        let mut wl2 = WordLib::create_empty("t2", "t", 1);
        let add = |wl: &mut WordLib, hz: &str, py: &str, freq: u32| {
            let hz: Vec<u16> = hz.encode_utf16().collect();
            let syl = parse_pin_yin_string_reverse(py, 0);
            assert_eq!(hz.len(), syl.len(), "{}", py);
            wl.add_ci(&hz, &syl, freq, true).unwrap();
        };
        add(&mut wl1, "中国", "zhongguo", 100);
        add(&mut wl2, "中国", "zhongguo", 90);
        add(&mut wl2, "中过", "zhongguo", 50);
        let mut engine = ImeEngine::new(vec![wl1, wl2], None);

        for c in "zhongguo".chars() {
            engine.handle_key(&KeyInput::Letter(c));
        }
        let texts = engine.candidate_texts();
        // 中国只出现一次
        let count = texts.iter().filter(|t| *t == "中国").count();
        assert_eq!(count, 1, "中国应去重为 1 个: {:?}", texts);
        // 保留词频更高的（wl1 的 100）
        let china = engine
            .candidates
            .iter()
            .find(|c| c.text() == "中国")
            .unwrap();
        if let Candidate::Ci(ci) = china {
            assert_eq!(ci.item.freq, 100);
        } else {
            panic!("应为词候选");
        }
        // 中过保留
        assert!(texts.iter().any(|t| t == "中过"), "{:?}", texts);
    }
}
