//! unispim-ime：华宇拼音输入法交互式测试工具。
//!
//! 这是一个**安全**的测试工具：从 stdin 读取按键，实时显示
//! 预编辑（拼音）、候选列表与上屏结果，**不拦截系统键盘**，
//! 不会影响其他程序的输入。
//!
//! 用法：
//! ```sh
//! unispim-ime [--wordlib-dir DIR] [--hzpy FILE] [--send]
//! ```
//!
//! - 默认：上屏结果打印到控制台（纯测试）
//! - `--send`：确认上屏时通过 SendInput 注入到前台窗口（Windows）
//!
//! 交互按键：
//! - `a`-`z`：输入拼音
//! - `0`-`9`：选择候选
//! - 空格：选第一个候选 / 上屏拼音
//! - 退格：删除一个字符
//! - 回车：直接上屏拼音
//! - `[` / `]`：上翻页 / 下翻页
//! - `Ctrl+C` 或 `q` 退出

use std::io::{self, Read};

use clap::Parser;
use unispim_core::hzdata::HzData;
use unispim_core::ime::{ImeAction, ImeEngine, ImeMode, KeyInput};
use unispim_core::symbol::{get_symbol, SymbolState};
use unispim_core::wordlib::WordLib;

#[derive(Parser)]
#[command(name = "unispim-ime", version, about = "华宇拼音输入法交互式测试工具")]
struct Cli {
    /// 词库目录
    #[arg(long, default_value = "data/unispim6/wordlib")]
    wordlib_dir: String,
    /// 汉字数据文件 hzpy.dat
    #[arg(long, default_value = "data/unispim6/zi/hzpy.dat")]
    hzpy: String,
    /// 确认上屏时注入到前台窗口（Windows，默认仅打印）
    #[arg(long)]
    send: bool,
}

/// 加载词库。
fn load_wordlibs(dir: &str) -> Vec<WordLib> {
    let mut wordlibs = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return wordlibs;
    };
    let mut paths: Vec<_> = entries.filter_map(|e| e.ok()).collect();
    paths.sort_by_key(|e| e.file_name().to_string_lossy().to_string());
    for entry in paths {
        let path = entry.path();
        if path.extension().map(|e| e == "uwl").unwrap_or(false)
            && let Ok(wl) = WordLib::from_file(&path.to_string_lossy()) {
                println!("加载词库: {} ({} 条)", path.display(), wl.header.word_count);
                wordlibs.push(wl);
            }
    }
    wordlibs
}

/// 将 stdin 字符映射为引擎按键。
fn char_to_key(ch: char) -> Option<KeyInput> {
    match ch {
        'a'..='z' => Some(KeyInput::Letter(ch)),
        'A'..='Z' => Some(KeyInput::Letter(ch.to_ascii_lowercase())),
        '0'..='9' => Some(KeyInput::Digit(ch as u8 - b'0')),
        ' ' => Some(KeyInput::Space),
        '\x08' | '\x7f' => Some(KeyInput::Backspace), // BS / DEL
        '\r' | '\n' => Some(KeyInput::Enter),
        '[' => Some(KeyInput::PageUp),
        ']' => Some(KeyInput::PageDown),
        _ => Some(KeyInput::Char(ch)),
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let wordlibs = load_wordlibs(&cli.wordlib_dir);
    if wordlibs.is_empty() {
        anyhow::bail!("未找到词库文件（{}）", cli.wordlib_dir);
    }
    let hzdata = match HzData::from_file(&cli.hzpy) {
        Some(h) => {
            println!("加载汉字数据: {} ({} 字)", cli.hzpy, h.items.len());
            Some(h)
        }
        None => {
            println!("警告: 未找到汉字数据 {}", cli.hzpy);
            None
        }
    };

    let mut engine = ImeEngine::new(wordlibs, hzdata);
    let mut symbol_state = SymbolState::default();

    println!("=== 华宇拼音输入法 交互式测试 ===");
    println!("输入拼音后：空格=选第一候选，数字=选候选，[]=翻页，回车=上屏拼音，退格=删除，q=退出");
    println!("（本工具不拦截系统键盘，仅从本窗口读取输入）");
    if cli.send {
        println!("--send 已开启：确认上屏时将注入到前台窗口");
    }
    println!();

    // 逐字符读取 stdin（行缓冲：一次读一行，逐个处理字符）
    let mut buffer = [0u8; 1];
    let stdin = io::stdin();
    let mut locked = stdin.lock();

    loop {
        // 读取一个字节（Unicode 多字节由后续 UTF-8 累积处理）
        let mut bytes = Vec::new();
        loop {
            match locked.read(&mut buffer) {
                Ok(0) => return Ok(()), // EOF
                Ok(_) => {
                    bytes.push(buffer[0]);
                    // 尝试解码为字符
                    match std::str::from_utf8(&bytes) {
                        Ok(s) => {
                            for ch in s.chars() {
                                if ch == 'q' {
                                    println!("退出");
                                    return Ok(());
                                }
                                let key = char_to_key(ch).unwrap_or(KeyInput::Ignored);
                                let output = engine.handle_key(&key);
                                for action in &output.actions {
                                    apply_action(
                                        &mut engine,
                                        &mut symbol_state,
                                        action,
                                        cli.send,
                                    );
                                }
                                render_state(&engine);
                            }
                            break;
                        }
                        Err(_) => continue, // 还需要更多字节
                    }
                }
                Err(_) => return Ok(()),
            }
        }
    }
}

/// 渲染当前引擎状态（拼音预编辑 + 当前页候选）。
fn render_state(engine: &ImeEngine) {
    if engine.composition.is_empty() {
        return;
    }
    let pinyin = if engine.syllables.is_empty() {
        engine.composition.clone()
    } else {
        unispim_core::parse::syllables_to_string(&engine.syllables)
    };
    let candidates = engine.current_page_candidates();
    let page_count = engine.page_count();
    println!("拼音: {}", pinyin);
    if !candidates.is_empty() {
        println!("候选 (第 {}/{} 页):", engine.page + 1, page_count);
        for (i, c) in candidates.iter().enumerate() {
            println!("  {}. {}", i + 1, c.text());
        }
    }
    println!();
}

/// 应用引擎动作。
fn apply_action(
    engine: &mut ImeEngine,
    symbol_state: &mut SymbolState,
    action: &ImeAction,
    send: bool,
) {
    match action {
        ImeAction::None => {}
        ImeAction::Commit(text) => {
            // 中文模式标点转换
            let mut final_text = String::new();
            for c in text.chars() {
                if engine.mode == ImeMode::Chinese {
                    match get_symbol(c, symbol_state, true, engine.last_digital) {
                        Some(sym) => final_text.push_str(&sym),
                        None => final_text.push(c),
                    }
                } else {
                    final_text.push(c);
                }
            }
            println!("上屏: {}", final_text);
            if send {
                #[cfg(target_os = "windows")]
                send_unicode_text(&final_text);
            }
        }
        ImeAction::UpdatePreedit(_) | ImeAction::ClearPreedit | ImeAction::UpdateCandidates { .. } => {
            // 状态在 render_state 中展示
        }
    }
}

/// 通过 SendInput 注入 Unicode 文本（仅 Windows，--send 时使用）。
#[cfg(target_os = "windows")]
fn send_unicode_text(text: &str) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
        VIRTUAL_KEY,
    };
    unsafe {
        for ch in text.encode_utf16() {
            let inputs = [
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(0),
                            wScan: ch,
                            dwFlags: KEYEVENTF_UNICODE,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                },
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(0),
                            wScan: ch,
                            dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                },
            ];
            let _ = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        }
    }
}
