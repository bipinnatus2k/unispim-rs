//! 汉字（单字）数据模块。
//!
//! 对应原 C 工程的 `zi.c` / `kernel.h` 中的 `HZDATAHEADER` / `HZITEM`，
//! 负责读取 `hzpy.dat`（汉字拼音数据文件）并提供单字候选。

use crate::syllable::{contain_syllable_with_tone, Syllable};

/// hzpy.dat 文件头。
pub const HZDATA_HEADER_SIZE: usize = 20;
/// 单条汉字项目大小（16 字节，pack(1)）。
pub const HZITEM_SIZE: usize = 16;

/// 汉字项目。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HzItem {
    /// 字（Unicode 码点，4 字节）。
    pub hz: u32,
    /// 汉字项目 ID。
    pub hz_id: u16,
    /// 音节。
    pub syllable: Syllable,
    /// 字频。
    pub freq: i32,
    /// 简体。
    pub simplified: bool,
    /// 繁体。
    pub traditional: bool,
    /// 其它（日文/韩文汉字等）。
    pub other: bool,
    /// 有效。
    pub effective: bool,
    /// 在候选窗口中显示拼音。
    pub show_syllable: bool,
    /// ICW 字（单词字）。
    pub icw_hz: bool,
}

impl HzItem {
    fn read(data: &[u8]) -> Option<HzItem> {
        if data.len() < HZITEM_SIZE {
            return None;
        }
        let hz = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let hz_id = u16::from_le_bytes([data[4], data[5]]);
        let syllable = Syllable(u16::from_le_bytes([data[6], data[7]]));
        let freq = i32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        let flags = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);
        Some(HzItem {
            hz,
            hz_id,
            syllable,
            freq,
            simplified: flags & 1 != 0,
            traditional: flags & 2 != 0,
            other: flags & 4 != 0,
            effective: flags & 8 != 0,
            show_syllable: flags & 16 != 0,
            icw_hz: flags & (1 << 31) != 0,
        })
    }

    pub fn write(&self) -> [u8; HZITEM_SIZE] {
        let mut out = [0u8; HZITEM_SIZE];
        out[0..4].copy_from_slice(&self.hz.to_le_bytes());
        out[4..6].copy_from_slice(&self.hz_id.to_le_bytes());
        out[6..8].copy_from_slice(&self.syllable.0.to_le_bytes());
        out[8..12].copy_from_slice(&self.freq.to_le_bytes());
        let mut flags = 0u32;
        if self.simplified {
            flags |= 1;
        }
        if self.traditional {
            flags |= 2;
        }
        if self.other {
            flags |= 4;
        }
        if self.effective {
            flags |= 8;
        }
        if self.show_syllable {
            flags |= 16;
        }
        if self.icw_hz {
            flags |= 1 << 31;
        }
        out[12..16].copy_from_slice(&flags.to_le_bytes());
        out
    }
}

/// 汉字数据文件（hzpy.dat）。
#[derive(Debug, Clone)]
pub struct HzData {
    /// 签名。
    pub signature: u32,
    /// 创建日期。
    pub create_date: u32,
    /// 修改日期。
    pub modify_date: u32,
    /// 校验和。
    pub check_sum: u32,
    /// 汉字项目（按 con/vow/hz 排序）。
    pub items: Vec<HzItem>,
}

impl HzData {
    /// 从字节加载。
    pub fn from_bytes(data: &[u8]) -> Option<HzData> {
        if data.len() < HZDATA_HEADER_SIZE {
            return None;
        }
        let signature = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let create_date = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let modify_date = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        let check_sum = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);
        let hz_count = u32::from_le_bytes([data[16], data[17], data[18], data[19]]) as usize;

        let mut items = Vec::with_capacity(hz_count);
        let mut off = HZDATA_HEADER_SIZE;
        for _ in 0..hz_count {
            let item = HzItem::read(&data[off..off + HZITEM_SIZE])?;
            items.push(item);
            off += HZITEM_SIZE;
        }
        Some(HzData {
            signature,
            create_date,
            modify_date,
            check_sum,
            items,
        })
    }

    /// 从文件加载。
    pub fn from_file(path: &str) -> Option<HzData> {
        let data = std::fs::read(path).ok()?;
        Self::from_bytes(&data)
    }

    /// 序列化（不含 hzpy.dat 尾部的扩展数据）。
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HZDATA_HEADER_SIZE + self.items.len() * HZITEM_SIZE);
        out.extend_from_slice(&self.signature.to_le_bytes());
        out.extend_from_slice(&self.create_date.to_le_bytes());
        out.extend_from_slice(&self.modify_date.to_le_bytes());
        out.extend_from_slice(&self.check_sum.to_le_bytes());
        out.extend_from_slice(&(self.items.len() as u32).to_le_bytes());
        for item in &self.items {
            out.extend_from_slice(&item.write());
        }
        out
    }

    /// 在数组中按音节查找第一个匹配的索引。
    fn find_first_by_syllable(&self, target: &Syllable) -> Option<usize> {
        let (tcon, tvow) = (target.con(), target.vow());
        // 由于按 (con, vow, hz) 排序，可以二分查找边界
        let lower = self
            .items
            .partition_point(|item| {
                (item.syllable.con(), item.syllable.vow()) < (tcon, tvow)
            });
        let upper = self
            .items
            .partition_point(|item| {
                (item.syllable.con(), item.syllable.vow()) <= (tcon, tvow)
            });
        if lower < upper {
            Some(lower)
        } else {
            None
        }
    }

    /// 获得指定音节的单字候选（不含模糊音）。
    pub fn get_zi_candidates(&self, syllable: &Syllable) -> Vec<HzItem> {
        let start = match self.find_first_by_syllable(syllable) {
            Some(s) => s,
            None => return Vec::new(),
        };
        let tcon = syllable.con();
        let tvow = syllable.vow();
        let mut out = Vec::new();
        for item in &self.items[start..] {
            if item.syllable.con() != tcon || item.syllable.vow() != tvow {
                break;
            }
            if item.effective {
                out.push(*item);
            }
        }
        out
    }

    /// 获得指定音节的单字候选（含模糊音与音调过滤）。
    pub fn get_zi_candidates_with_fuzzy(
        &self,
        syllable: &Syllable,
        fuzzy_mode: u32,
    ) -> Vec<HzItem> {
        let mut out = Vec::new();
        for item in &self.items {
            if !item.effective {
                continue;
            }
            if contain_syllable_with_tone(*syllable, item.syllable, fuzzy_mode) {
                out.push(*item);
            }
        }
        out
    }

    /// 检查汉字是否包含指定音调。
    pub fn zi_contain_tone(&self, hz: u32, syllable: &Syllable, tone: u16) -> bool {
        if tone == 0 {
            return true;
        }
        // 二分查找 (con, vow, hz)
        let idx = self.items.binary_search_by(|item| {
            (
                item.syllable.con(),
                item.syllable.vow(),
                item.hz,
            )
                .cmp(&(syllable.con(), syllable.vow(), hz))
        });
        match idx {
            Ok(i) => self.items[i].syllable.tone() & tone != 0,
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syllable::{CON_ZH, VOW_ONG};

    #[test]
    fn test_hzitem_roundtrip() {
        let item = HzItem {
            hz: 0x4E2D,
            hz_id: 5,
            syllable: Syllable::new(CON_ZH, VOW_ONG, 0),
            freq: 1000,
            simplified: true,
            traditional: false,
            other: false,
            effective: true,
            show_syllable: false,
            icw_hz: false,
        };
        let bytes = item.write();
        let back = HzItem::read(&bytes).unwrap();
        assert_eq!(back.hz, item.hz);
        assert_eq!(back.syllable, item.syllable);
        assert_eq!(back.freq, item.freq);
        assert!(back.simplified);
        assert!(back.effective);
    }

    #[test]
    fn test_hzdata_file() {
        // 与真实 hzpy.dat 结构兼容
        let mut data = Vec::new();
        data.extend_from_slice(&0x1A696E55u32.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&2u32.to_le_bytes());
        let a = HzItem {
            hz: 0x4E2D,
            hz_id: 0,
            syllable: Syllable::new(CON_ZH, VOW_ONG, 0),
            freq: 10,
            simplified: true,
            traditional: false,
            other: false,
            effective: true,
            show_syllable: false,
            icw_hz: false,
        };
        let b = HzItem {
            hz: 0x4E2D + 1,
            hz_id: 1,
            syllable: Syllable::new(CON_ZH, VOW_ONG, 0),
            freq: 5,
            simplified: true,
            traditional: false,
            other: false,
            effective: true,
            show_syllable: false,
            icw_hz: false,
        };
        data.extend_from_slice(&a.write());
        data.extend_from_slice(&b.write());
        let hz = HzData::from_bytes(&data).unwrap();
        assert_eq!(hz.items.len(), 2);
        let candidates = hz.get_zi_candidates(&Syllable::new(CON_ZH, VOW_ONG, 0));
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].hz, 0x4E2D);
    }
}
