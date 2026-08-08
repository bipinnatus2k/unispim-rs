//! 词库（.uwl）文件格式：读取、写入、创建与查询。
//!
//! 对应原 C 工程的 `wordlib.h` / `wordlib.c`。当前实现 V6.6 版本格式：
//!
//! - 头部 `WORDLIBHEADER`（含索引表 `index[24][24]`）
//! - 页表，每页 1024 字节，词条以变长结构存储
//!
//! 词条结构（`WORDLIBITEM`）：
//! ```text
//! u32 feature:  [effective:1][ci_length:6][syllable_length:6][freq:19]
//! u16 syllable[syllable_length]
//! u16 hz[ci_length]
//! ```

use crate::syllable::{CON_END, CON_NULL, Syllable};
use std::io::{self, Read, Write};

pub const WORDLIB_PAGE_SIZE: usize = 1024;
pub const PAGE_END: i32 = -1;
pub const WORDLIB_NAME_LENGTH: usize = 16;
pub const WORDLIB_AUTHOR_LENGTH: usize = 16;
pub const CON_NUMBER: usize = 24;
pub const WORDLIB_MAX_FREQ: u32 = (1 << 19) - 1;

pub const HYPIM_VERSION: i32 = 6;
pub const HYPIM_WORDLIB_V66_SIGNATURE: i32 = 0x14091994;
pub const HYPIM_WORDLIB_V6_SIGNATURE: i32 = 0x14081994;
pub const HYPIM_WORDLIB_V6B2_SIGNATURE: i32 = 0x14071994;
pub const HYPIM_WORDLIB_V6B1_SIGNATURE: i32 = 0x14061994;
pub const HYPIM_WORDLIB_V5_SIGNATURE: i32 = 0x19990604;

/// 页头大小（4 个 int）。
pub const PAGE_HEADER_SIZE: usize = 4 * 4;
/// 每页数据长度。
pub const PAGE_DATA_LENGTH: usize = WORDLIB_PAGE_SIZE - PAGE_HEADER_SIZE;

/// 词库头部大小。
/// signature(4) + name(16×u16) + author(16×u16) + word_count(4) + page_count(4)
/// + can_be_edit(4) + pim_version(4) + index[24][24]×u32
pub const HEADER_SIZE: usize = 4 + 2 * WORDLIB_NAME_LENGTH + 2 * WORDLIB_AUTHOR_LENGTH + 4 * 4
    + CON_NUMBER * CON_NUMBER * 4;

/// 头部占用页数（与原 C `header_data[sizeof(HEADER)/PAGE_SIZE + 1]` 一致）。
pub const HEADER_PAGE_COUNT: usize = HEADER_SIZE / WORDLIB_PAGE_SIZE + 1;
/// 页数据的起始偏移（头部按页对齐）。
pub const PAGE_OFFSET: usize = HEADER_PAGE_COUNT * WORDLIB_PAGE_SIZE;

    /// 词库文件的最小大小（头部按页对齐）。
    pub const MIN_WORDLIB_SIZE: usize = PAGE_OFFSET;

/// 词条头部（feature）大小，即一个 u32。
pub const WORDLIB_FEATURE_LENGTH: usize = 4;

/// 词库签名。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordLibVersion {
    V66,
    V6,
    V6B2,
    V6B1,
    V5,
    Wrong,
}

impl WordLibVersion {
    pub fn from_signature(sig: i32) -> Self {
        match sig {
            HYPIM_WORDLIB_V66_SIGNATURE => WordLibVersion::V66,
            HYPIM_WORDLIB_V6_SIGNATURE => WordLibVersion::V6,
            HYPIM_WORDLIB_V6B2_SIGNATURE => WordLibVersion::V6B2,
            HYPIM_WORDLIB_V6B1_SIGNATURE => WordLibVersion::V6B1,
            HYPIM_WORDLIB_V5_SIGNATURE => WordLibVersion::V5,
            _ => WordLibVersion::Wrong,
        }
    }

    pub fn signature(&self) -> i32 {
        match self {
            WordLibVersion::V66 => HYPIM_WORDLIB_V66_SIGNATURE,
            WordLibVersion::V6 => HYPIM_WORDLIB_V6_SIGNATURE,
            WordLibVersion::V6B2 => HYPIM_WORDLIB_V6B2_SIGNATURE,
            WordLibVersion::V6B1 => HYPIM_WORDLIB_V6B1_SIGNATURE,
            WordLibVersion::V5 => HYPIM_WORDLIB_V5_SIGNATURE,
            WordLibVersion::Wrong => 0,
        }
    }
}

/// 词库头部。
#[derive(Debug, Clone)]
pub struct WordLibHeader {
    pub signature: i32,
    pub name: String,
    pub author_name: String,
    pub word_count: i32,
    pub page_count: i32,
    pub can_be_edit: i32,
    pub pim_version: i32,
    pub index: [[i32; CON_NUMBER]; CON_NUMBER],
}

impl Default for WordLibHeader {
    fn default() -> Self {
        Self {
            signature: HYPIM_WORDLIB_V66_SIGNATURE,
            name: String::new(),
            author_name: String::new(),
            word_count: 0,
            page_count: 0,
            can_be_edit: 1,
            pim_version: HYPIM_VERSION,
            index: [[PAGE_END; CON_NUMBER]; CON_NUMBER],
        }
    }
}

impl WordLibHeader {
    fn read_from_bytes(data: &[u8]) -> io::Result<Self> {
        if data.len() < HEADER_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "wordlib header too short",
            ));
        }
        let mut rd = data;
        let signature = read_i32(&mut rd);
        let name = read_utf16(&mut rd, WORDLIB_NAME_LENGTH);
        let author_name = read_utf16(&mut rd, WORDLIB_AUTHOR_LENGTH);
        let word_count = read_i32(&mut rd);
        let page_count = read_i32(&mut rd);
        let can_be_edit = read_i32(&mut rd);
        let pim_version = read_i32(&mut rd);
        let mut index = [[PAGE_END; CON_NUMBER]; CON_NUMBER];
        for row in index.iter_mut() {
            for v in row.iter_mut() {
                *v = read_i32(&mut rd);
            }
        }
        Ok(WordLibHeader {
            signature,
            name,
            author_name,
            word_count,
            page_count,
            can_be_edit,
            pim_version,
            index,
        })
    }

    fn write_to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEADER_SIZE);
        write_i32(&mut out, self.signature);
        write_utf16(&mut out, &self.name, WORDLIB_NAME_LENGTH);
        write_utf16(&mut out, &self.author_name, WORDLIB_AUTHOR_LENGTH);
        write_i32(&mut out, self.word_count);
        write_i32(&mut out, self.page_count);
        write_i32(&mut out, self.can_be_edit);
        write_i32(&mut out, self.pim_version);
        for row in &self.index {
            for v in row {
                write_i32(&mut out, *v);
            }
        }
        out
    }
}

/// 词条。
#[derive(Debug, Clone)]
pub struct WordItem {
    pub effective: bool,
    pub ci_length: usize,
    pub syllable_length: usize,
    pub freq: u32,
    pub syllables: Vec<Syllable>,
    pub hz: Vec<u16>,
}

/// 词条变长长度（字节）。
pub fn item_length(hz_length: usize, syllable_length: usize) -> usize {
    WORDLIB_FEATURE_LENGTH + 2 * hz_length + 2 * syllable_length
}

impl WordItem {
    /// 从字节流中解析一条词（已知音节长度与词长度）。
    pub fn read(data: &[u8]) -> io::Result<WordItem> {
        if data.len() < WORDLIB_FEATURE_LENGTH {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "word item too short",
            ));
        }
        let feature = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let effective = feature & 1 != 0;
        let ci_length = ((feature >> 1) & 0x3F) as usize;
        let syllable_length = ((feature >> 7) & 0x3F) as usize;
        let freq = (feature >> 13) & 0x7FFFF;

        let needed = item_length(ci_length, syllable_length);
        if data.len() < needed {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "word item data truncated",
            ));
        }

        let mut syllables = Vec::with_capacity(syllable_length);
        let mut off = WORDLIB_FEATURE_LENGTH;
        for _ in 0..syllable_length {
            let raw = u16::from_le_bytes([data[off], data[off + 1]]);
            syllables.push(Syllable(raw));
            off += 2;
        }
        let mut hz = Vec::with_capacity(ci_length);
        for _ in 0..ci_length {
            let raw = u16::from_le_bytes([data[off], data[off + 1]]);
            hz.push(raw);
            off += 2;
        }
        Ok(WordItem {
            effective,
            ci_length,
            syllable_length,
            freq,
            syllables,
            hz,
        })
    }

    pub fn write(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(item_length(self.ci_length, self.syllable_length));
        let feature = ((self.effective as u32) & 1)
            | (((self.ci_length as u32) & 0x3F) << 1)
            | (((self.syllable_length as u32) & 0x3F) << 7)
            | ((self.freq & 0x7FFFF) << 13);
        out.extend_from_slice(&feature.to_le_bytes());
        for s in &self.syllables {
            out.extend_from_slice(&s.0.to_le_bytes());
        }
        for h in &self.hz {
            out.extend_from_slice(&h.to_le_bytes());
        }
        out
    }
}

/// 一页词库数据。
#[derive(Debug, Clone, Default)]
pub struct Page {
    pub page_no: i32,
    pub next_page_no: i32,
    pub length_flag: u32,
    pub data_length: usize,
    pub data: Vec<u8>,
}

impl Page {
    fn from_bytes(data: &[u8]) -> io::Result<Page> {
        if data.len() < PAGE_HEADER_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "page header too short",
            ));
        }
        let mut rd = data;
        let page_no = read_i32(&mut rd);
        let next_page_no = read_i32(&mut rd);
        let length_flag = read_u32(&mut rd);
        let data_length = read_i32(&mut rd) as usize;
        if data_length > PAGE_DATA_LENGTH {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "page data length overflow",
            ));
        }
        let body = data[PAGE_HEADER_SIZE..PAGE_HEADER_SIZE + data_length].to_vec();
        Ok(Page {
            page_no,
            next_page_no,
            length_flag,
            data_length,
            data: body,
        })
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(WORDLIB_PAGE_SIZE);
        write_i32(&mut out, self.page_no);
        write_i32(&mut out, self.next_page_no);
        write_u32(&mut out, self.length_flag);
        write_i32(&mut out, self.data_length as i32);
        out.extend_from_slice(&self.data);
        out
    }

    /// 遍历页中的词条。
    fn iter_items(&self) -> Vec<WordItem> {
        let mut items = Vec::new();
        let mut off = 0usize;
        while off + WORDLIB_FEATURE_LENGTH <= self.data.len() {
            let feature =
                u32::from_le_bytes([self.data[off], self.data[off + 1], self.data[off + 2], self.data[off + 3]]);
            let ci_length = ((feature >> 1) & 0x3F) as usize;
            let syllable_length = ((feature >> 7) & 0x3F) as usize;
            let len = item_length(ci_length, syllable_length);
            if off + len > self.data.len() {
                break;
            }
            if let Ok(item) = WordItem::read(&self.data[off..off + len]) {
                items.push(item);
            }
            off += len;
        }
        items
    }
}

/// 词库。
#[derive(Debug, Clone)]
pub struct WordLib {
    pub header: WordLibHeader,
    pub pages: Vec<Page>,
    pub file_size: usize,
}

impl WordLib {
    /// 从字节加载词库。
    pub fn from_bytes(data: &[u8]) -> io::Result<WordLib> {
        let header = WordLibHeader::read_from_bytes(data)?;
        let version = WordLibVersion::from_signature(header.signature);
        if !matches!(version, WordLibVersion::V66) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unsupported wordlib version {:?}", version),
            ));
        }

        let page_count = header.page_count as usize;
        let mut pages = Vec::with_capacity(page_count);
        let mut off = PAGE_OFFSET;
        for _ in 0..page_count {
            if off + WORDLIB_PAGE_SIZE > data.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "wordlib page data truncated",
                ));
            }
            let page = Page::from_bytes(&data[off..off + WORDLIB_PAGE_SIZE])?;
            pages.push(page);
            off += WORDLIB_PAGE_SIZE;
        }
        Ok(WordLib {
            header,
            pages,
            file_size: data.len(),
        })
    }

    /// 从文件加载词库。
    pub fn from_file(path: &str) -> io::Result<WordLib> {
        let data = std::fs::read(path)?;
        Self::from_bytes(&data)
    }

    /// 序列化为字节。
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = self.header.write_to_bytes();
        // 头部按页对齐（与原 C 结构 header_data 一致）
        out.resize(PAGE_OFFSET, 0);
        for page in &self.pages {
            let mut page_bytes = page.to_bytes();
            // 补零到整页
            page_bytes.resize(WORDLIB_PAGE_SIZE, 0);
            out.extend_from_slice(&page_bytes);
        }
        out
    }

    /// 保存到文件。
    pub fn to_file(&self, path: &str) -> io::Result<()> {
        std::fs::write(path, self.to_bytes())
    }

    /// 创建新的空词库。
    pub fn create_empty(name: &str, author: &str, can_be_edit: i32) -> WordLib {
        WordLib {
            header: WordLibHeader {
                name: name.to_string(),
                author_name: author.to_string(),
                can_be_edit,
                ..Default::default()
            },
            pages: Vec::new(),
            file_size: HEADER_SIZE,
        }
    }

    /// 在词库中查找词条（按汉字+音节）。
    pub fn find_ci(&self, hz: &[u16], syllables: &[Syllable]) -> Option<WordItem> {
        let ci_length = hz.len();
        for page in &self.pages {
            for item in page.iter_items() {
                if item.ci_length == ci_length
                    && item.syllables == syllables
                    && item.hz == hz
                    && item.effective
                {
                    return Some(item);
                }
            }
        }
        None
    }

    /// 新增一个词条。
    pub fn add_ci(
        &mut self,
        hz: &[u16],
        syllables: &[Syllable],
        freq: u32,
        can_grow: bool,
    ) -> io::Result<bool> {
        let hz_length = hz.len();
        let syllable_length = syllables.len();
        if !(2..=32).contains(&syllable_length)
            || !(2..=32).contains(&hz_length)
            || hz_length != syllable_length
        {
            return Ok(false);
        }
        let freq = freq.min(WORDLIB_MAX_FREQ);

        // 已存在则增加词频
        if let Some(existing) = self.find_ci(hz, syllables) {
            let _ = existing;
            // 需要更新：简单处理——定位并修改
            self.update_ci_freq(hz, syllables, freq);
            return Ok(true);
        }

        let item_len = item_length(hz_length, syllable_length);
        let con0 = syllables[0].con() as usize;
        let con1 = syllables[1].con() as usize;
        if con0 >= CON_NUMBER || con1 >= CON_NUMBER {
            return Ok(false);
        }

        let mut page_no = self.header.index[con0][con1];
        if page_no == PAGE_END {
            // 新建页
            let new_page_no = self.new_page()?;
            self.header.index[con0][con1] = new_page_no;
            page_no = new_page_no;
        }

        // 遍历到最后一页
        let mut current = page_no as usize;
        loop {
            let next = self.pages[current].next_page_no;
            if next == PAGE_END {
                break;
            }
            current = next as usize;
        }

        // 若当前页放不下，新建页
        if self.pages[current].data_length + item_len > PAGE_DATA_LENGTH {
            let new_page_no = self.new_page()?;
            self.pages[current].next_page_no = new_page_no;
            current = new_page_no as usize;
        }

        // 写词条
        let item = WordItem {
            effective: true,
            ci_length: hz_length,
            syllable_length,
            freq,
            syllables: syllables.to_vec(),
            hz: hz.to_vec(),
        };
        self.pages[current].length_flag |= 1 << syllable_length;
        let item_bytes = item.write();
        self.pages[current].data.extend_from_slice(&item_bytes);
        self.pages[current].data_length = self.pages[current].data.len();
        self.header.word_count += 1;
        let _ = can_grow;
        Ok(true)
    }

    fn update_ci_freq(&mut self, hz: &[u16], syllables: &[Syllable], freq: u32) {
        for page in &mut self.pages {
            let mut off = 0usize;
            while off + WORDLIB_FEATURE_LENGTH <= page.data.len() {
                let feature = u32::from_le_bytes([
                    page.data[off],
                    page.data[off + 1],
                    page.data[off + 2],
                    page.data[off + 3],
                ]);
                let ci_length = ((feature >> 1) & 0x3F) as usize;
                let syllable_length = ((feature >> 7) & 0x3F) as usize;
                let len = item_length(ci_length, syllable_length);
                if off + len > page.data.len() {
                    break;
                }
                if let Ok(item) = WordItem::read(&page.data[off..off + len])
                    && item.syllables == syllables && item.hz == hz {
                        let new_freq = freq.max(item.freq).min(WORDLIB_MAX_FREQ);
                        let new_feature = (page.data[off] as u32 & 1)
                            | (((ci_length as u32) & 0x3F) << 1)
                            | (((syllable_length as u32) & 0x3F) << 7)
                            | ((new_freq & 0x7FFFF) << 13);
                        page.data[off..off + 4].copy_from_slice(&new_feature.to_le_bytes());
                        break;
                    }
                off += len;
            }
        }
    }

    fn new_page(&mut self) -> io::Result<i32> {
        if !self.can_grow() {
            return Err(io::Error::other(
                "wordlib full, cannot allocate page",
            ));
        }
        let page_no = self.header.page_count;
        self.pages.push(Page {
            page_no,
            next_page_no: PAGE_END,
            length_flag: 0,
            data_length: 0,
            data: Vec::new(),
        });
        self.header.page_count += 1;
        Ok(page_no)
    }

    fn can_grow(&self) -> bool {
        true
    }

    /// 遍历所有有效词条。
    pub fn iter_ci(&self) -> Vec<WordItem> {
        let mut out = Vec::new();
        for page in &self.pages {
            for item in page.iter_items() {
                if item.effective {
                    out.push(item);
                }
            }
        }
        out
    }

    /// 所有词条（含无效）。
    pub fn iter_all_ci(&self) -> Vec<WordItem> {
        let mut out = Vec::new();
        for page in &self.pages {
            for item in page.iter_items() {
                out.push(item);
            }
        }
        out
    }

    /// 删除词条。
    pub fn delete_ci(&mut self, hz: &[u16], syllables: &[Syllable]) -> bool {
        for page in &mut self.pages {
            let mut off = 0usize;
            while off + WORDLIB_FEATURE_LENGTH <= page.data.len() {
                let feature = u32::from_le_bytes([
                    page.data[off],
                    page.data[off + 1],
                    page.data[off + 2],
                    page.data[off + 3],
                ]);
                let ci_length = ((feature >> 1) & 0x3F) as usize;
                let syllable_length = ((feature >> 7) & 0x3F) as usize;
                let len = item_length(ci_length, syllable_length);
                if off + len > page.data.len() {
                    break;
                }
                if let Ok(item) = WordItem::read(&page.data[off..off + len])
                    && item.syllables == syllables && item.hz == hz {
                        // 置 effective = 0
                        let new_feature = page.data[off] as u32 & !1u32;
                        page.data[off] = (new_feature & 0xFF) as u8;
                        // 只修改最低字节的低位即可（little endian）
                        if self.header.word_count > 0 && item.effective {
                            self.header.word_count -= 1;
                        }
                        return true;
                    }
                off += len;
            }
        }
        false
    }
}

// ---- 读取辅助 ----

fn read_i32<R: Read>(r: &mut R) -> i32 {
    let mut buf = [0u8; 4];
    let _ = r.read_exact(&mut buf);
    i32::from_le_bytes(buf)
}

fn read_u32<R: Read>(r: &mut R) -> u32 {
    let mut buf = [0u8; 4];
    let _ = r.read_exact(&mut buf);
    u32::from_le_bytes(buf)
}

fn read_utf16<R: Read>(r: &mut R, max_len: usize) -> String {
    let mut units = Vec::with_capacity(max_len);
    for _ in 0..max_len {
        let mut buf = [0u8; 2];
        if r.read_exact(&mut buf).is_err() {
            break;
        }
        units.push(u16::from_le_bytes(buf));
    }
    // 去掉末尾 0
    let end = units.iter().position(|&u| u == 0).unwrap_or(units.len());
    String::from_utf16_lossy(&units[..end])
}

fn write_i32<W: Write>(w: &mut W, v: i32) {
    let _ = w.write_all(&v.to_le_bytes());
}

fn write_u32<W: Write>(w: &mut W, v: u32) {
    let _ = w.write_all(&v.to_le_bytes());
}

fn write_utf16<W: Write>(w: &mut W, s: &str, max_len: usize) {
    let mut units: Vec<u16> = s.encode_utf16().take(max_len).collect();
    while units.len() < max_len {
        units.push(0);
    }
    let mut bytes = Vec::with_capacity(units.len() * 2);
    for u in units {
        bytes.extend_from_slice(&u.to_le_bytes());
    }
    let _ = w.write_all(&bytes);
}

/// 判断字节是否为合法 V66 词库。
pub fn is_v66_wordlib(data: &[u8]) -> bool {
    if data.len() < HEADER_SIZE {
        return false;
    }
    let sig = i32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    sig == HYPIM_WORDLIB_V66_SIGNATURE
}

// 保留引用，防止未用告警
#[allow(dead_code)]
fn _use(_: u8) {
    let _ = CON_END;
    let _ = CON_NULL;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zhong_guo_syl() -> (Vec<u16>, Vec<Syllable>) {
        use crate::syllable::{CON_ZH, VOW_ONG, CON_G, VOW_UO};
        let hz = vec![0x4E2D, 0x56FD]; // 中国
        let syl = vec![Syllable::new(CON_ZH, VOW_ONG, 0), Syllable::new(CON_G, VOW_UO, 0)];
        (hz, syl)
    }

    #[test]
    fn test_create_and_add() {
        let mut wl = WordLib::create_empty("测试词库", "test", 1);
        let (hz, syl) = zhong_guo_syl();
        assert!(wl.add_ci(&hz, &syl, 100, true).unwrap());
        assert_eq!(wl.header.word_count, 1);
        assert_eq!(wl.header.page_count, 1);

        let found = wl.find_ci(&hz, &syl);
        assert!(found.is_some());
        assert_eq!(found.unwrap().freq, 100);
    }

    #[test]
    fn test_roundtrip_bytes() {
        let mut wl = WordLib::create_empty("roundtrip", "author", 1);
        let (hz, syl) = zhong_guo_syl();
        wl.add_ci(&hz, &syl, 5, true).unwrap();

        let bytes = wl.to_bytes();
        let loaded = WordLib::from_bytes(&bytes).unwrap();
        assert_eq!(loaded.header.word_count, 1);
        let found = loaded.find_ci(&hz, &syl);
        assert!(found.is_some());
        assert_eq!(found.unwrap().freq, 5);
    }

    #[test]
    fn test_header_size() {
        // 校验头部大小与 C 结构一致
        let h = WordLibHeader::default();
        let bytes = h.write_to_bytes();
        assert_eq!(bytes.len(), HEADER_SIZE);
    }

    #[test]
    fn test_item_feature_layout() {
        // 验证 feature 位域布局
        let item = WordItem {
            effective: true,
            ci_length: 2,
            syllable_length: 2,
            freq: 100,
            syllables: vec![Syllable(1), Syllable(2)],
            hz: vec![0x4E2D, 0x56FD],
        };
        let bytes = item.write();
        let feature = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(feature & 1, 1); // effective
        assert_eq!((feature >> 1) & 0x3F, 2); // ci_length
        assert_eq!((feature >> 7) & 0x3F, 2); // syllable_length
        assert_eq!((feature >> 13) & 0x7FFFF, 100); // freq
    }

    #[test]
    fn test_syllable_bit_layout() {
        // 与原 C 位域一致：con 低 5 位，vow 中间 6 位，tone 高 5 位
        let s = Syllable::new(23, 22, 0); // zhong
        assert_eq!(s.0, (23) | (22 << 5));
        assert_eq!(s.con(), 23);
        assert_eq!(s.vow(), 22);
        assert_eq!(s.tone(), 0);

        let t = Syllable::new(8, 18, 1 << 3); // jing 4声
        assert_eq!(t.0, (8) | (18 << 5) | ((1u16 << 3) << 11));
        assert_eq!(t.con(), 8);
        assert_eq!(t.vow(), 18);
        assert_eq!(t.tone(), 1 << 3);
    }
}
