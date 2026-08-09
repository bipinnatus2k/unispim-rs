//! 词汇候选检索（CI）。
//!
//! 对应原 C 工程的 `ci.c`：根据音节数组从词库中检索候选，并进行排重与排序。

use crate::syllable::{contain_syllable, Syllable};
use crate::wordlib::WordLib;

/// 一个候选词。
#[derive(Debug, Clone)]
pub struct CiCandidate {
    pub item: crate::wordlib::WordItem,
    pub type_: i32,
}

/// 从单个词库检索与给定音节序列匹配的候选（不含通配符）。
pub fn get_ci_in_wordlib(
    wordlib: &WordLib,
    syllables: &[Syllable],
    fuzzy_mode: u32,
) -> Vec<CiCandidate> {
    let syllable_count = syllables.len();
    let mut candidates = Vec::new();
    if syllable_count < 2 {
        return candidates;
    }
    for item in wordlib.iter_ci() {
        if item.syllable_length != syllable_count {
            continue;
        }
        if !compare_syllables(syllables, &item.syllables, fuzzy_mode) {
            continue;
        }
        candidates.push(CiCandidate {
            item: item.clone(),
            type_: 1, // CI_TYPE_NORMAL
        });
    }
    candidates
}

/// 比较一组音节，判断是否匹配（含模糊音）。
pub fn compare_syllables(
    syllables: &[Syllable],
    checked: &[Syllable],
    fuzzy_mode: u32,
) -> bool {
    if syllables.len() != checked.len() {
        return false;
    }
    for (a, b) in syllables.iter().zip(checked.iter()) {
        if !contain_syllable(*a, *b, fuzzy_mode) {
            return false;
        }
    }
    true
}

/// 对所有候选排重（按汉字去重，保留词频最高者）。
pub fn unify_candidates(candidates: &mut Vec<CiCandidate>) {
    // 按 (hz, ci_length) 排序
    candidates.sort_by(|a, b| {
        a.item
            .hz
            .cmp(&b.item.hz)
            .then(a.item.ci_length.cmp(&b.item.ci_length))
    });
    let mut new_count = 0usize;
    let mut i = 0usize;
    while i < candidates.len() {
        if new_count > 0
            && candidates[i].item.hz == candidates[new_count - 1].item.hz
            && candidates[i].item.ci_length == candidates[new_count - 1].item.ci_length
        {
            i += 1;
            continue;
        }
        candidates.swap(i, new_count);
        new_count += 1;
        i += 1;
    }
    candidates.truncate(new_count);
}

/// 对候选排序（按词频降序）。
pub fn sort_candidates(candidates: &mut [CiCandidate]) {
    candidates.sort_by(|a, b| b.item.freq.cmp(&a.item.freq).then(a.item.hz.cmp(&b.item.hz)));
}

/// 综合处理：检索 + 排重 + 排序。
pub fn process_ci_candidates(
    wordlib: &WordLib,
    syllables: &[Syllable],
    fuzzy_mode: u32,
) -> Vec<CiCandidate> {
    let mut candidates = get_ci_in_wordlib(wordlib, syllables, fuzzy_mode);
    unify_candidates(&mut candidates);
    sort_candidates(&mut candidates);
    candidates
}
