//! 音节处理模块。
//!
//! 对应原 C 工程中的 `syllable.h` / `syllable.c`，负责：
//! - 音节（声母/韵母/音调）的编码与常量定义
//! - 拼音串 -> 音节数组的解析（`ParsePinYinString` / `ParsePinYinStringReverse`）
//! - 音节 -> 拼音串（`GetSyllableString`）
//! - 模糊音（fuzzy）匹配

/// 声母（consonant）编号。
pub const CON_ERROR: u8 = 255;
pub const CON_NULL: u8 = 0;
pub const CON_B: u8 = 1;
pub const CON_C: u8 = 2;
pub const CON_CH: u8 = 3;
pub const CON_D: u8 = 4;
pub const CON_F: u8 = 5;
pub const CON_G: u8 = 6;
pub const CON_H: u8 = 7;
pub const CON_J: u8 = 8;
pub const CON_K: u8 = 9;
pub const CON_L: u8 = 10;
pub const CON_M: u8 = 11;
pub const CON_N: u8 = 12;
pub const CON_P: u8 = 13;
pub const CON_Q: u8 = 14;
pub const CON_R: u8 = 15;
pub const CON_S: u8 = 16;
pub const CON_SH: u8 = 17;
pub const CON_T: u8 = 18;
pub const CON_W: u8 = 19;
pub const CON_X: u8 = 20;
pub const CON_Y: u8 = 21;
pub const CON_Z: u8 = 22;
pub const CON_ZH: u8 = 23;
pub const CON_END: u8 = 24;
pub const CON_ANY: u8 = 25;

/// 韵母（vowel）编号。
pub const VOW_ERROR: u8 = 255;
pub const VOW_NULL: u8 = 0;
pub const VOW_A: u8 = 1;
pub const VOW_AI: u8 = 2;
pub const VOW_AN: u8 = 3;
pub const VOW_ANG: u8 = 4;
pub const VOW_AO: u8 = 5;
pub const VOW_E: u8 = 6;
pub const VOW_EI: u8 = 7;
pub const VOW_EN: u8 = 8;
pub const VOW_ENG: u8 = 9;
pub const VOW_ER: u8 = 10;
pub const VOW_I: u8 = 11;
pub const VOW_IA: u8 = 12;
pub const VOW_IAN: u8 = 13;
pub const VOW_IANG: u8 = 14;
pub const VOW_IAO: u8 = 15;
pub const VOW_IE: u8 = 16;
pub const VOW_IN: u8 = 17;
pub const VOW_ING: u8 = 18;
pub const VOW_IONG: u8 = 19;
pub const VOW_IU: u8 = 20;
pub const VOW_O: u8 = 21;
pub const VOW_ONG: u8 = 22;
pub const VOW_OU: u8 = 23;
pub const VOW_U: u8 = 24;
pub const VOW_UA: u8 = 25;
pub const VOW_UAI: u8 = 26;
pub const VOW_UAN: u8 = 27;
pub const VOW_UANG: u8 = 28;
pub const VOW_UE: u8 = 29;
pub const VOW_UI: u8 = 30;
pub const VOW_UN: u8 = 31;
pub const VOW_UO: u8 = 32;
pub const VOW_V: u8 = 33;
pub const VOW_END: u8 = 34;
pub const VOW_ANY: u8 = 35;

/// 音调编码（位标志）。
pub const TONE_0: u16 = 0;
pub const TONE_1: u16 = 1 << 0;
pub const TONE_2: u16 = 1 << 1;
pub const TONE_3: u16 = 1 << 2;
pub const TONE_4: u16 = 1 << 3;

pub const TONE_CHAR_1: char = '!';
pub const TONE_CHAR_2: char = '@';
pub const TONE_CHAR_3: char = '#';
pub const TONE_CHAR_4: char = '$';

/// 音节分隔符（用户输入）与系统内部分隔符。
pub const SYLLABLE_SEPARATOR_CHAR: char = '\'';
pub const SYLLABLE_ANY_CHAR: char = '*';

/// 模糊音选项（位标志）。
pub const FUZZY_Z_ZH: u32 = 1 << 0;
pub const FUZZY_C_CH: u32 = 1 << 1;
pub const FUZZY_S_SH: u32 = 1 << 2;
pub const FUZZY_G_K: u32 = 1 << 3;
pub const FUZZY_L_N: u32 = 1 << 4;
pub const FUZZY_L_R: u32 = 1 << 5;
pub const FUZZY_F_H: u32 = 1 << 6;
pub const FUZZY_F_HU: u32 = 1 << 7;
pub const FUZZY_HUANG_WANG: u32 = 1 << 8;
pub const FUZZY_AN_ANG: u32 = 1 << 9;
pub const FUZZY_EN_ENG: u32 = 1 << 10;
pub const FUZZY_IN_ING: u32 = 1 << 11;
pub const FUZZY_REV_Z_ZH: u32 = 1 << 12;
pub const FUZZY_REV_C_CH: u32 = 1 << 13;
pub const FUZZY_REV_S_SH: u32 = 1 << 14;
pub const FUZZY_REV_G_K: u32 = 1 << 15;
pub const FUZZY_REV_L_N: u32 = 1 << 16;
pub const FUZZY_REV_L_R: u32 = 1 << 17;
pub const FUZZY_REV_F_H: u32 = 1 << 18;
pub const FUZZY_REV_F_HU: u32 = 1 << 19;
pub const FUZZY_REV_HUANG_WANG: u32 = 1 << 20;
pub const FUZZY_REV_AN_ANG: u32 = 1 << 21;
pub const FUZZY_REV_EN_ENG: u32 = 1 << 22;
pub const FUZZY_REV_IN_ING: u32 = 1 << 23;
pub const FUZZY_ZCS_IN_CI: u32 = 1 << 24;
pub const FUZZY_SUPER: u32 = 1 << 25;
pub const FUZZY_CI_SYLLABLE_LENGTH: u32 = 1 << 26;

/// 拼音模式。
pub const PINYIN_QUANPIN: i32 = 0;
pub const PINYIN_SHUANGPIN: i32 = 1;

/// 最大音节省略常量。
pub const MAX_PINYIN_LENGTH: usize = 8;
pub const MAX_INPUT_LENGTH: usize = 64;
pub const MAX_SYLLABLE_PER_INPUT: usize = 32;

/// 一个音节：声母(5bit) + 韵母(6bit) + 音调(5bit)，打包为一个 `u16`。
///
/// 与原 C 中 `SYLLABLE` 的位域布局完全一致，因此可以直接读写词库二进制。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Syllable(pub u16);

impl Syllable {
    /// 与原 C 位域一致：con 为低 5 位，vow 为中间 6 位，tone 为高 5 位。
    pub fn new(con: u8, vow: u8, tone: u16) -> Self {
        let raw = ((tone & 0x1F) << 11) | ((vow as u16 & 0x3F) << 5) | (con as u16 & 0x1F);
        Syllable(raw)
    }

    #[inline]
    pub fn con(&self) -> u8 {
        (self.0 & 0x1F) as u8
    }

    #[inline]
    pub fn vow(&self) -> u8 {
        ((self.0 >> 5) & 0x3F) as u8
    }

    #[inline]
    pub fn tone(&self) -> u16 {
        (self.0 >> 11) & 0x1F
    }

    pub fn set_con(&mut self, con: u8) {
        self.0 = (self.0 & !0x1F) | (con as u16 & 0x1F);
    }

    pub fn set_vow(&mut self, vow: u8) {
        self.0 = (self.0 & !(0x3F << 5)) | ((vow as u16 & 0x3F) << 5);
    }

    pub fn set_tone(&mut self, tone: u16) {
        self.0 = (self.0 & !(0x1F << 11)) | ((tone & 0x1F) << 11);
    }

    pub fn is_null(&self) -> bool {
        self.con() == CON_NULL && self.vow() == VOW_NULL && self.tone() == TONE_0
    }
}

/// 判断字符是否为音调字符。
pub fn is_tone_char(ch: char) -> bool {
    matches!(ch, '!' | '@' | '#' | '$')
}

/// 判断字符是否为音节通配符。
pub fn is_separator_char(ch: char) -> bool {
    ch == SYLLABLE_SEPARATOR_CHAR
}

/// 音调位 -> 数字字符串（与原 `tone_to_string` 一致）。
/// 输入为音调位数值（1, 2, 4, 8），分别对应 1~4 声。
pub fn tone_to_string(tone: u16) -> &'static str {
    match tone {
        TONE_1 => "1",
        TONE_2 => "2",
        TONE_3 => "3",
        TONE_4 => "4",
        _ => "",
    }
}

/// 声母编号 -> 字符串。
pub fn con_to_string(con: u8) -> &'static str {
    const TABLE: [&str; 26] = [
        "", "b", "c", "ch", "d", "f", "g", "h", "j", "k", "l", "m", "n", "p", "q", "r", "s", "sh",
        "t", "w", "x", "y", "z", "zh", "", "*",
    ];
    if (con as usize) < TABLE.len() {
        TABLE[con as usize]
    } else {
        "?"
    }
}

/// 韵母编号 -> 字符串。
pub fn vow_to_string(vow: u8) -> &'static str {
    const TABLE: [&str; 36] = [
        "", "a", "ai", "an", "ang", "ao", "e", "ei", "en", "eng", "er", "i", "ia", "ian", "iang",
        "iao", "ie", "in", "ing", "iong", "iu", "o", "ong", "ou", "u", "ua", "uai", "uan", "uang",
        "ue", "ui", "un", "uo", "v", "", "",
    ];
    if (vow as usize) < TABLE.len() {
        TABLE[vow as usize]
    } else {
        "?"
    }
}

/// 将音节转换为拼音串（不携带音调）。
pub fn get_syllable_string(syllable: Syllable) -> String {
    let con = syllable.con();
    let vow = syllable.vow();
    if con > CON_ANY || vow > VOW_ANY {
        return "?".into();
    }
    let mut s = String::new();
    s.push_str(con_to_string(con));
    s.push_str(vow_to_string(vow));
    s
}

/// 将音节转换为带声调数字的拼音串（如 `ba3`）。
pub fn get_syllable_string_with_tone(syllable: Syllable) -> String {
    let base = get_syllable_string(syllable);
    let tone = syllable.tone();
    if tone == TONE_0 {
        base
    } else {
        // 最多一个音调位
        for i in 0..4u16 {
            let bit = 1u16 << i;
            if tone & bit != 0 {
                return format!("{}{}", base, i + 1);
            }
        }
        base
    }
}

/// 判断一个音节的声母是否以指定字母开头（用于单键取词）。
pub fn syllable_start_with_letter(ch: char, syllable: Syllable) -> bool {
    if ch == SYLLABLE_ANY_CHAR {
        return true;
    }
    if syllable.con() != CON_NULL {
        con_to_string(syllable.con()).starts_with(ch)
    } else if syllable.vow() != VOW_NULL {
        vow_to_string(syllable.vow()).starts_with(ch)
    } else {
        false
    }
}

/// 判断两个音节是否完全相同。
pub fn same_syllable(s1: Syllable, s2: Syllable) -> bool {
    s1 == s2
}

/// 判断第一个音节声母集合是否包含第二个音节声母（含模糊音）。
pub fn contain_con(syllable: Syllable, checked: Syllable, fuzzy_mode: u32) -> bool {
    if syllable.con() == checked.con() {
        return true;
    }
    if syllable.con() == CON_ANY {
        return true;
    }
    // 模糊音判断
    let s = syllable.con();
    let c = checked.con();
    match (s, c) {
        (CON_Z, CON_ZH) if fuzzy_mode & FUZZY_Z_ZH != 0 => return true,
        (CON_ZH, CON_Z) if fuzzy_mode & FUZZY_REV_Z_ZH != 0 => return true,
        (CON_C, CON_CH) if fuzzy_mode & FUZZY_C_CH != 0 => return true,
        (CON_CH, CON_C) if fuzzy_mode & FUZZY_REV_C_CH != 0 => return true,
        (CON_S, CON_SH) if fuzzy_mode & FUZZY_S_SH != 0 => return true,
        (CON_SH, CON_S) if fuzzy_mode & FUZZY_REV_S_SH != 0 => return true,
        (CON_G, CON_K) if fuzzy_mode & FUZZY_G_K != 0 => return true,
        (CON_K, CON_G) if fuzzy_mode & FUZZY_REV_G_K != 0 => return true,
        (CON_L, CON_N) if fuzzy_mode & FUZZY_L_N != 0 => return true,
        (CON_N, CON_L) if fuzzy_mode & FUZZY_REV_L_N != 0 => return true,
        (CON_L, CON_R) if fuzzy_mode & FUZZY_L_R != 0 => return true,
        (CON_R, CON_L) if fuzzy_mode & FUZZY_REV_L_R != 0 => return true,
        (CON_F, CON_H) if fuzzy_mode & FUZZY_F_H != 0 => return true,
        (CON_H, CON_F) if fuzzy_mode & FUZZY_REV_F_H != 0 => return true,
        _ => {}
    }
    false
}

/// 判断第一个音节韵母集合是否包含第二个音节韵母（含模糊音）。
pub fn contain_vow(syllable: Syllable, checked: Syllable, fuzzy_mode: u32) -> bool {
    if syllable.vow() == checked.vow() {
        return true;
    }
    if syllable.vow() == VOW_ANY {
        return true;
    }
    let s = syllable.vow();
    let c = checked.vow();
    match (s, c) {
        (VOW_AN, VOW_ANG) if fuzzy_mode & FUZZY_AN_ANG != 0 => return true,
        (VOW_ANG, VOW_AN) if fuzzy_mode & FUZZY_REV_AN_ANG != 0 => return true,
        (VOW_EN, VOW_ENG) if fuzzy_mode & FUZZY_EN_ENG != 0 => return true,
        (VOW_ENG, VOW_EN) if fuzzy_mode & FUZZY_REV_EN_ENG != 0 => return true,
        (VOW_IN, VOW_ING) if fuzzy_mode & FUZZY_IN_ING != 0 => return true,
        (VOW_ING, VOW_IN) if fuzzy_mode & FUZZY_REV_IN_ING != 0 => return true,
        _ => {}
    }
    false
}

/// 判断音节是否与另一个音节匹配（声母+韵母+音调，含模糊）。
pub fn contain_syllable(syllable: Syllable, checked: Syllable, fuzzy_mode: u32) -> bool {
    if !contain_con(syllable, checked, fuzzy_mode) {
        return false;
    }
    if !contain_vow(syllable, checked, fuzzy_mode) {
        return false;
    }
    true
}

/// 判断音节是否与另一个音节匹配（含音调，用于带音调过滤）。
pub fn contain_syllable_with_tone(syllable: Syllable, checked: Syllable, fuzzy_mode: u32) -> bool {
    if !contain_syllable(syllable, checked, fuzzy_mode) {
        return false;
    }
    if syllable.tone() == TONE_0 {
        return true;
    }
    syllable.tone() & checked.tone() != 0
}

/// 判断一个音节是否能以指定字母开头。
#[inline]
pub fn contains_letter_start(ch: char, syllable: Syllable) -> bool {
    syllable_start_with_letter(ch, syllable)
}
