//! 词库维护命令行工具（wl_tool 的 Rust 移植）。
//!
//! 子命令与 C 版对应：
//! - `create <wordlib> <text>`：由文本词条文件创建词库
//! - `import <wordlib> <text>`：向词库导入词条
//! - `export <wordlib> <text>`：导出词库为文本
//! - `info <wordlib>`：显示词库信息
//! - `query <wordlib> <pinyin>`：按拼音查询候选

use clap::{Parser, Subcommand};
use unispim_core::ci::process_ci_candidates;
use unispim_core::parse::{parse_pin_yin_string_reverse, syllables_to_string};
use unispim_core::syllable::Syllable;
use unispim_core::wordlib::WordLib;

#[derive(Parser)]
#[command(name = "wl_tool", version, about = "词库（.uwl）维护工具")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 由文本词条文件创建词库
    Create {
        /// 词库文件（.uwl）
        wordlib: String,
        /// 文本词条文件（UTF-16 LE 带 BOM）
        text: String,
    },
    /// 向词库导入词条
    Import {
        wordlib: String,
        text: String,
    },
    /// 导出词库为文本
    Export {
        wordlib: String,
        text: String,
    },
    /// 显示词库信息
    Info { wordlib: String },
    /// 按拼音查询候选
    Query {
        wordlib: String,
        pinyin: String,
    },
}

fn read_text_utf16(path: &str) -> anyhow::Result<String> {
    let bytes = std::fs::read(path)?;
    let bytes = if bytes.starts_with(&[0xFF, 0xFE]) {
        &bytes[2..]
    } else {
        &bytes[..]
    };
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    Ok(String::from_utf16_lossy(&units))
}

fn write_text_utf16(path: &str, content: &str) -> anyhow::Result<()> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&[0xFF, 0xFE]);
    for unit in content.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    std::fs::write(path, bytes)?;
    Ok(())
}

/// 词条行：`词`、`音节`、`词频`
type CiEntry = (Vec<u16>, Vec<Syllable>, u32);
/// 词库文本文件解析结果：名称、作者、可编辑、词条
type TextLib = (String, String, i32, Vec<CiEntry>);

/// 解析词条行：`词<TAB>拼音<TAB>词频`
fn parse_ci_line(line: &str) -> Option<CiEntry> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }
    let ci: Vec<u16> = parts[0].encode_utf16().collect();
    let syllables = parse_pin_yin_string_reverse(parts[1], 0);
    if syllables.is_empty() {
        return None;
    }
    let freq = if parts.len() >= 3 {
        parts[2].parse::<u32>().unwrap_or(0)
    } else {
        0
    };
    Some((ci, syllables, freq))
}

/// 解析词库文本文件（含 名称=/作者=/编辑= 头）。
fn parse_text_file(content: &str) -> TextLib {
    let mut name = String::new();
    let mut author = String::new();
    let mut can_be_edit = 1;
    let mut items = Vec::new();

    for line in content.lines() {
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("名称=") {
            name = rest.trim().to_string();
            continue;
        }
        if let Some(rest) = line.strip_prefix("作者=") {
            author = rest.trim().to_string();
            continue;
        }
        if let Some(rest) = line.strip_prefix("编辑=") {
            can_be_edit = if rest.trim().starts_with('1') { 1 } else { 0 };
            continue;
        }
        if let Some((ci, syl, freq)) = parse_ci_line(line) {
            items.push((ci, syl, freq));
        }
    }
    (name, author, can_be_edit, items)
}

fn hz_to_string(hz: &[u16]) -> String {
    String::from_utf16_lossy(hz)
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Create { wordlib, text } => {
            let content = read_text_utf16(&text)?;
            let (name, author, can_be_edit, items) = parse_text_file(&content);
            let mut wl = WordLib::create_empty(
                &if name.is_empty() { "未命名词库".to_string() } else { name.clone() },
                &if author.is_empty() { "Unispim".to_string() } else { author.clone() },
                can_be_edit,
            );
            let mut ok = 0usize;
            for (ci, syl, freq) in &items {
                if wl.add_ci(ci, syl, *freq, true)? {
                    ok += 1;
                }
            }
            wl.to_file(&wordlib)?;
            println!("创建词库 {} 成功，词条 {} 条（共 {} 行）", wordlib, ok, items.len());
        }
        Command::Import { wordlib, text } => {
            let content = read_text_utf16(&text)?;
            let (_, _, _, items) = parse_text_file(&content);
            let mut wl = WordLib::from_file(&wordlib)?;
            if wl.header.can_be_edit == 0 {
                anyhow::bail!("词库 {} 不允许编辑", wordlib);
            }
            let mut ok = 0usize;
            for (ci, syl, freq) in &items {
                if wl.add_ci(ci, syl, *freq, true)? {
                    ok += 1;
                }
            }
            wl.to_file(&wordlib)?;
            println!("导入 {} 成功，共导入 {} 条", wordlib, ok);
        }
        Command::Export { wordlib, text } => {
            let wl = WordLib::from_file(&wordlib)?;
            let mut content = String::new();
            content.push_str(&format!("名称={}\n", wl.header.name));
            content.push_str(&format!("作者={}\n", wl.header.author_name));
            content.push_str("编辑=1\n\n");
            for item in wl.iter_ci() {
                let py = syllables_to_string(&item.syllables);
                content.push_str(&format!(
                    "{}\t{}\t{}\n",
                    hz_to_string(&item.hz),
                    py,
                    item.freq
                ));
            }
            write_text_utf16(&text, &content)?;
            println!("导出 {} 成功，共 {} 条", wordlib, wl.header.word_count);
        }
        Command::Info { wordlib } => {
            let wl = WordLib::from_file(&wordlib)?;
            println!("文件: {}", wordlib);
            println!("名称: {}", wl.header.name);
            println!("作者: {}", wl.header.author_name);
            println!("词条数: {}", wl.header.word_count);
            println!("页数: {}", wl.header.page_count);
            println!("可编辑: {}", wl.header.can_be_edit);
            println!("版本: 6.6 (0x{:08X})", wl.header.signature as u32);
        }
        Command::Query { wordlib, pinyin } => {
            let wl = WordLib::from_file(&wordlib)?;
            let syllables = parse_pin_yin_string_reverse(&pinyin, 0);
            if syllables.is_empty() {
                println!("拼音 {} 无法解析", pinyin);
                return Ok(());
            }
            let candidates = process_ci_candidates(&wl, &syllables, 0);
            if candidates.is_empty() {
                println!("没有找到候选（{} 个音节）", syllables.len());
                return Ok(());
            }
            println!(
                "查询 {}  ->  {}  （{} 个候选）",
                pinyin,
                syllables_to_string(&syllables),
                candidates.len()
            );
            for (i, cand) in candidates.iter().enumerate().take(30) {
                println!(
                    "{:>2}. {}  [{}]",
                    i + 1,
                    hz_to_string(&cand.item.hz),
                    cand.item.freq
                );
            }
        }
    }
    Ok(())
}
