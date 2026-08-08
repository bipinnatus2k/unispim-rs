//! 拼音串解析：拼音 -> 音节数组。
//!
//! 对应原 C 工程的 `syllable.c` 中的 `GetSyllable` / `ParsePinYinString` /
//! `ParsePinYinStringReverse`。

use crate::syllable_map::SYLLABLE_MAP;
use crate::syllable::{
    get_syllable_string, is_tone_char, MAX_INPUT_LENGTH, MAX_PINYIN_LENGTH, SYLLABLE_ANY_CHAR,
    SYLLABLE_SEPARATOR_CHAR, Syllable, TONE_1, TONE_2, TONE_3, TONE_4, TONE_CHAR_1, TONE_CHAR_2,
    TONE_CHAR_3, TONE_CHAR_4, VOW_ANY, CON_ANY,
};

/// 判断字符是否为音调字符。
fn tone_char_to_bit(ch: char) -> u16 {
    match ch {
        TONE_CHAR_1 => TONE_1,
        TONE_CHAR_2 => TONE_2,
        TONE_CHAR_3 => TONE_3,
        TONE_CHAR_4 => TONE_4,
        _ => 0,
    }
}

/// 通过拼音串查找音节（含模糊音判断）。
/// 返回 `Option<(Syllable, 拼音长度)>`。
pub fn get_syllable(pin_yin: &str, fuzzy_mode: u32) -> Option<(Syllable, usize)> {
    let mut py = pin_yin.to_string();
    if py.is_empty() || py.chars().count() > MAX_PINYIN_LENGTH {
        return None;
    }

    let mut has_separator = false;
    if py.ends_with(SYLLABLE_SEPARATOR_CHAR) {
        py.pop();
        has_separator = true;
        if py.is_empty() {
            return None;
        }
    }

    // 通配符
    if py.chars().count() == 1 && py.starts_with(SYLLABLE_ANY_CHAR) {
        return Some((
            Syllable::new(CON_ANY, VOW_ANY, 0),
            1 + usize::from(has_separator),
        ));
    }

    // 判断尾部是否有音调
    let mut has_tone = false;
    let last = py.chars().last().unwrap();
    if is_tone_char(last) {
        has_tone = true;
        py.pop();
        if py.is_empty() {
            return None;
        }
    }

    // 二分查找
    let mut low = 0usize;
    let mut hi = SYLLABLE_MAP.len() - 1;
    let mut found: Option<usize> = None;
    while low <= hi {
        let mid = (low + hi) / 2;
        let entry = &SYLLABLE_MAP[mid];
        let cmp = py.as_str().cmp(entry.py);
        match cmp {
            std::cmp::Ordering::Equal => {
                found = Some(mid);
                break;
            }
            std::cmp::Ordering::Less => {
                if mid == 0 {
                    break;
                }
                hi = mid - 1;
            }
            std::cmp::Ordering::Greater => low = mid + 1,
        }
    }

    let mid = found?;

    // 模糊音有效判断
    if SYLLABLE_MAP[mid].fuzzy != 0 && (SYLLABLE_MAP[mid].fuzzy & fuzzy_mode) == 0 {
        return None;
    }

    let mut syllable = Syllable::new(SYLLABLE_MAP[mid].con, SYLLABLE_MAP[mid].vow, 0);
    let py_len = py.chars().count();

    if has_tone {
        // 取音调字符
        let tone_ch = pin_yin.chars().nth(py_len).unwrap();
        syllable.set_tone(tone_char_to_bit(tone_ch));
    }

    let mut length = py_len;
    if has_tone {
        length += 1;
    }
    if has_separator {
        length += 1;
    }

    Some((syllable, length))
}

/// 判断拼音串是否为合法拼音（仅含字母、分隔符、音调、通配符）。
fn legal_pin_yin(pin_yin: &str) -> bool {
    pin_yin.chars().all(|c| {
        c.is_ascii_alphabetic()
            || c == SYLLABLE_SEPARATOR_CHAR
            || c == SYLLABLE_ANY_CHAR
            || is_tone_char(c)
    })
}

/// 正向递归解析。
fn process_parse(pin_yin: &str, fuzzy_mode: u32) -> Option<Vec<Syllable>> {
    let mut py = pin_yin;
    // 跳过音节切分符号
    if py.starts_with(SYLLABLE_SEPARATOR_CHAR) {
        py = &py[1..];
    }
    // 直接发现错误
    if py.starts_with('i') || py.starts_with('u') || py.starts_with('v') {
        return None;
    }
    if py.chars().next().map(is_tone_char).unwrap_or(false) {
        return None;
    }
    if py.len() > MAX_INPUT_LENGTH + 0x10 {
        return None;
    }

    let py_chars: Vec<char> = py.chars().collect();
    let max = MAX_PINYIN_LENGTH.min(py_chars.len());
    for i in (1..=max).rev() {
        let cur: String = py_chars[..i].iter().collect();
        let (syllable, syllable_string_length) = match get_syllable(&cur, fuzzy_mode) {
            Some(v) => v,
            None => continue,
        };

        if py_chars.len() == i {
            return Some(vec![syllable]);
        }

        // 解析剩余
        let rest: String = py_chars[syllable_string_length..].iter().collect();
        if let Some(mut remaining) = process_parse(&rest, fuzzy_mode) {
            let mut result = Vec::with_capacity(remaining.len() + 1);
            result.push(syllable);
            result.append(&mut remaining);
            return Some(result);
        }
    }
    None
}

/// 反向递归解析。
fn process_parse_reverse(pin_yin: &str, fuzzy_mode: u32) -> Option<Vec<Syllable>> {
    let py = pin_yin;
    if py.starts_with(SYLLABLE_SEPARATOR_CHAR) {
        return process_parse_reverse(&py[1..], fuzzy_mode);
    }
    if py.starts_with('i') || py.starts_with('u') || py.starts_with('v') {
        return None;
    }
    if py.chars().next().map(is_tone_char).unwrap_or(false) {
        return None;
    }
    if py.len() > MAX_INPUT_LENGTH + 0x10 {
        return None;
    }

    let py_chars: Vec<char> = py.chars().collect();
    let max = MAX_PINYIN_LENGTH.min(py_chars.len());
    for i in (1..=max).rev() {
        let cur: String = py_chars[..i].iter().collect();
        let (syllable, syllable_string_length) = match get_syllable(&cur, fuzzy_mode) {
            Some(v) => v,
            None => continue,
        };

        if py_chars.len() == i {
            return Some(vec![syllable]);
        }

        let rest: String = py_chars[syllable_string_length..].iter().collect();
        if let Some(mut remaining) = process_parse_reverse(&rest, fuzzy_mode) {
            let mut result = Vec::with_capacity(remaining.len() + 1);
            result.push(syllable);
            result.append(&mut remaining);
            return Some(result);
        }
    }
    None
}

/// 正向解析拼音串，返回音节数组。
pub fn parse_pin_yin_string(pin_yin: &str, fuzzy_mode: u32) -> Vec<Syllable> {
    process_parse(pin_yin, fuzzy_mode).unwrap_or_default()
}

/// 反向解析拼音串，返回音节数组。
pub fn parse_pin_yin_string_reverse(pin_yin: &str, fuzzy_mode: u32) -> Vec<Syllable> {
    if !legal_pin_yin(pin_yin) {
        return Vec::new();
    }
    let reverse = process_parse_reverse(pin_yin, fuzzy_mode);
    let forward = process_parse(pin_yin, fuzzy_mode);

    match (reverse, forward) {
        (Some(rev), Some(fwd)) => {
            // 音节数少的更优；相同使用正向
            if (fwd.len()) < (rev.len()) {
                fwd
            } else {
                rev
            }
        }
        (Some(rev), None) => rev,
        (None, Some(fwd)) => fwd,
        (None, None) => Vec::new(),
    }
}

/// 将音节数组转换为拼音串（用分隔符连接）。
pub fn syllables_to_string(syllables: &[Syllable]) -> String {
    syllables
        .iter()
        .map(|s| get_syllable_string(*s))
        .collect::<Vec<_>>()
        .join("'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single() {
        let s = parse_pin_yin_string_reverse("zhong", 0);
        assert_eq!(s.len(), 1);
        assert_eq!(get_syllable_string(s[0]), "zhong");
    }

    #[test]
    fn test_word() {
        let s = parse_pin_yin_string_reverse("zhongguo", 0);
        assert_eq!(s.len(), 2);
        assert_eq!(get_syllable_string(s[0]), "zhong");
        assert_eq!(get_syllable_string(s[1]), "guo");
    }

    #[test]
    fn test_ambiguous() {
        // xian 可以解析为 xi'an
        let s = parse_pin_yin_string_reverse("xian", 0);
        // 单个音节 xian 应优先
        assert_eq!(s.len(), 1);
        assert_eq!(get_syllable_string(s[0]), "xian");
    }

    #[test]
    fn test_tone() {
        let (syl, _len) = get_syllable("zhong$", 0).unwrap();
        assert_eq!(syl.tone(), TONE_4);
    }

    #[test]
    fn test_encoding_roundtrip() {
        // 音节二进制布局: 与 C 位域一致
        let s = Syllable::new(23, 22, 0); // zhong
        let raw = s.0;
        let decoded = Syllable(raw);
        assert_eq!(decoded.con(), 23);
        assert_eq!(decoded.vow(), 22);
    }
}
