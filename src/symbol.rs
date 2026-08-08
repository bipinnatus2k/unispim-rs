//! 中英文符号模块。
//!
//! 对应原 C 工程的 `symbol.c`。默认符号表来自
//! `share_segment.c` 中 `default_share_segment.symbol_table`。

/// 全角英文字符。
const FULL_SHAPE: &str = "ａｂｃｄｅｆｇｈｉｊｋｌｍｎｏｐｑｒｓｔｕｖｗｘｙｚ\
ＡＢＣＤＥＦＧＨＩＪＫＬＭＮＯＰＱＲＳＴＵＶＷＸＹＺ\
０１２３４５６７８９";

/// 单引号（成对，左/右）。
const QUOTATION1: [char; 2] = ['‘', '’'];
/// 双引号（成对，左/右）。
const QUOTATION2: [char; 2] = ['“', '”'];

/// 符号表条目。
#[derive(Debug, Clone, Copy)]
pub struct SymbolItem {
    /// 英文符号。
    pub english: char,
    /// 中文符号。
    pub chinese: &'static str,
}

/// 默认符号表（与 share_segment.c 一致）。
pub const DEFAULT_SYMBOL_TABLE: [SymbolItem; 32] = [
    SymbolItem { english: '\'', chinese: "‘" },
    SymbolItem { english: '"', chinese: "“" },
    SymbolItem { english: ',', chinese: "，" },
    SymbolItem { english: '.', chinese: "。" },
    SymbolItem { english: '<', chinese: "《" },
    SymbolItem { english: '>', chinese: "》" },
    SymbolItem { english: '/', chinese: "、" },
    SymbolItem { english: '?', chinese: "？" },
    SymbolItem { english: ';', chinese: "；" },
    SymbolItem { english: ':', chinese: "：" },
    SymbolItem { english: '[', chinese: "【" },
    SymbolItem { english: ']', chinese: "】" },
    SymbolItem { english: '{', chinese: "｛" },
    SymbolItem { english: '}', chinese: "｝" },
    SymbolItem { english: '-', chinese: "-" },
    SymbolItem { english: '_', chinese: "——" },
    SymbolItem { english: '=', chinese: "＝" },
    SymbolItem { english: '+', chinese: "＋" },
    SymbolItem { english: '|', chinese: "｜" },
    SymbolItem { english: '\\', chinese: "、" },
    SymbolItem { english: '~', chinese: "～" },
    SymbolItem { english: '`', chinese: "·" },
    SymbolItem { english: '!', chinese: "！" },
    SymbolItem { english: '@', chinese: "@" },
    SymbolItem { english: '#', chinese: "＃" },
    SymbolItem { english: '$', chinese: "￥" },
    SymbolItem { english: '%', chinese: "％" },
    SymbolItem { english: '^', chinese: "……" },
    SymbolItem { english: '&', chinese: "＆" },
    SymbolItem { english: '*', chinese: "＊" },
    SymbolItem { english: '(', chinese: "（" },
    SymbolItem { english: ')', chinese: "）" },
];

/// 符号状态：引号开闭、上一字符是否数字。
#[derive(Debug, Clone, Copy, Default)]
pub struct SymbolState {
    /// 单引号当前为左（未闭合）。
    pub q1_open: bool,
    /// 双引号当前为左（未闭合）。
    pub q2_open: bool,
}

/// 判断是否为符号字符。
pub fn is_symbol_char(ch: char) -> bool {
    DEFAULT_SYMBOL_TABLE.iter().any(|s| s.english == ch)
}

/// 获得中文符号。
///
/// - `chinese_mode`: 中文符号模式（否则输出英文符号）
/// - `last_digital`: 上一字符是否为数字
/// - 返回 `Some(输出)` 或 `None`（使用原始字符）。
pub fn get_symbol(ch: char, state: &mut SymbolState, chinese_mode: bool, last_digital: bool) -> Option<String> {
    let Some(idx) = DEFAULT_SYMBOL_TABLE.iter().position(|s| s.english == ch) else {
        // 全角字符
        if chinese_mode {
            return full_shape(ch);
        }
        return None;
    };

    if !chinese_mode {
        return Some(ch.to_string());
    }

    // 数字后使用英文符号（数字后面的标点保留英文，如 3.14）
    if last_digital && !matches!(ch, '(' | ')' | '[' | ']') {
        return Some(ch.to_string());
    }

    match idx {
        0 => {
            // 单引号成对
            let c = if state.q1_open { QUOTATION1[1] } else { QUOTATION1[0] };
            state.q1_open = !state.q1_open;
            Some(c.to_string())
        }
        1 => {
            // 双引号成对
            let c = if state.q2_open { QUOTATION2[1] } else { QUOTATION2[0] };
            state.q2_open = !state.q2_open;
            Some(c.to_string())
        }
        15 => Some("——".to_string()), // 下划线 -> 破折号
        26 => Some("￥".to_string()),  // $ -> 人民币符号
        27 => Some("……".to_string()), // ^ -> 省略号
        i => Some(DEFAULT_SYMBOL_TABLE[i].chinese.to_string()),
    }
}

/// 全角字符转换。
pub fn full_shape(ch: char) -> Option<String> {
    match ch {
        ' ' => Some("　".to_string()),
        'a'..='z' => Some(FULL_SHAPE.chars().nth(ch as usize - 'a' as usize)?.to_string()),
        'A'..='Z' => Some(FULL_SHAPE.chars().nth(26 + ch as usize - 'A' as usize)?.to_string()),
        '0'..='9' => Some(FULL_SHAPE.chars().nth(52 + ch as usize - '0' as usize)?.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol() {
        let mut state = SymbolState::default();
        assert_eq!(get_symbol(',', &mut state, true, false), Some("，".to_string()));
        assert_eq!(get_symbol('.', &mut state, true, false), Some("。".to_string()));
        assert_eq!(get_symbol('?', &mut state, true, false), Some("？".to_string()));
        assert_eq!(get_symbol('!', &mut state, true, false), Some("！".to_string()));
        // 数字后英文标点
        assert_eq!(get_symbol('.', &mut state, true, true), Some(".".to_string()));
    }

    #[test]
    fn test_quote_pairing() {
        let mut state = SymbolState::default();
        assert_eq!(get_symbol('"', &mut state, true, false), Some("“".to_string()));
        assert_eq!(get_symbol('"', &mut state, true, false), Some("”".to_string()));
    }

    #[test]
    fn test_full_shape() {
        assert_eq!(full_shape('a'), Some("ａ".to_string()));
        assert_eq!(full_shape(' '), Some("　".to_string()));
        assert_eq!(full_shape('9'), Some("９".to_string()));
    }
}
