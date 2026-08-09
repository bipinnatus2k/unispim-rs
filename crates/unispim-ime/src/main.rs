//! unispim-ime：华宇拼音输入法 imekit 集成程序。
//!
//! 使用 [imekit] 注册为系统输入法：
//! - Linux: Wayland（zwp_input_method_v2）/ X11（XIM），处理按键事件
//! - Windows: TSF / SendInput 上屏
//!
//! 用法：
//! ```sh
//! unispim-ime [--wordlib-dir DIR] [--hzpy FILE] [--debug]
//! ```

use clap::Parser;
use imekit::{InputMethod, InputMethodEvent, KeyState};
use unispim_core::hzdata::HzData;
use unispim_core::ime::{ImeAction, ImeEngine, ImeMode, KeyInput};
use unispim_core::symbol::{get_symbol, SymbolState};
use unispim_core::wordlib::WordLib;

#[derive(Parser)]
#[command(name = "unispim-ime", version, about = "华宇拼音输入法（基于 imekit）")]
struct Cli {
    /// 词库目录
    #[arg(long, default_value = "data/unispim6/wordlib")]
    wordlib_dir: String,
    /// 汉字数据文件 hzpy.dat
    #[arg(long, default_value = "data/unispim6/zi/hzpy.dat")]
    hzpy: String,
    /// 调试模式（打印事件）
    #[arg(long)]
    debug: bool,
}

/// 按键 -> 抽象按键。
fn keysym_to_key(keysym: u32) -> KeyInput {
    match keysym {
        // a-z
        0x61..=0x7a => KeyInput::Letter(char::from_u32(keysym).unwrap_or('?')),
        // 0-9
        0x30..=0x39 => KeyInput::Digit((keysym - 0x30) as u8),
        // 空格
        0x20 => KeyInput::Space,
        // BackSpace
        0xff08 => KeyInput::Backspace,
        // Return / KP_Enter
        0xff0d | 0xff8d => KeyInput::Enter,
        // PageUp / PageDown
        0xff55 => KeyInput::PageUp,
        0xff56 => KeyInput::PageDown,
        // 可打印字符（标点等）
        _ => {
            if let Some(c) = char::from_u32(keysym) {
                if c.is_ascii() && !c.is_ascii_control() {
                    KeyInput::Char(c)
                } else {
                    KeyInput::Ignored
                }
            } else {
                KeyInput::Ignored
            }
        }
    }
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

    let mut im = InputMethod::new()?;
    println!("华宇拼音输入法已启动，等待输入...");
    println!("按 Ctrl+C 退出");

    loop {
        while let Some(event) = im.next_event() {
            match event {
                InputMethodEvent::Activate { serial } => {
                    println!("输入法激活 (serial={})", serial);
                }
                InputMethodEvent::Deactivate => {
                    println!("输入法取消激活");
                }
                InputMethodEvent::Unavailable => {
                    println!("输入法协议不可用");
                    return Ok(());
                }
                InputMethodEvent::KeyEvent {
                    keysym,
                    state,
                    ..
                } => {
                    if cli.debug {
                        println!("KeyEvent: keysym=0x{:x} state={:?}", keysym, state);
                    }
                    if state != KeyState::Pressed {
                        continue;
                    }
                    let key = keysym_to_key(keysym);
                    if matches!(key, KeyInput::Ignored) {
                        continue;
                    }
                    if cli.debug {
                        println!("  -> {:?}", key);
                    }
                    let output = engine.handle_key(&key);
                    for action in output.actions {
                        apply_action(&im, &mut engine, &mut symbol_state, &action, cli.debug);
                    }
                }
                InputMethodEvent::SurroundingText { text, cursor, .. }
                    if cli.debug => {
                        println!("SurroundingText: {:?} cursor={}", text, cursor);
                    }
                _ => {}
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

/// 应用引擎动作到 imekit。
fn apply_action(
    im: &InputMethod,
    engine: &mut ImeEngine,
    symbol_state: &mut SymbolState,
    action: &ImeAction,
    debug: bool,
) {
    match action {
        ImeAction::None => {}
        ImeAction::Commit(text) => {
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
            if debug {
                println!("Commit: {:?}", final_text);
            }
            let _ = im.commit_string(&final_text);
        }
        ImeAction::UpdatePreedit(text) => {
            if debug {
                println!("Preedit: {:?}", text);
            }
            let _ = im.set_preedit_string(text, text.len() as i32, text.len() as i32);
        }
        ImeAction::ClearPreedit => {
            if debug {
                println!("ClearPreedit");
            }
            let _ = im.set_preedit_string("", 0, 0);
        }
        ImeAction::UpdateCandidates {
            candidates,
            page,
            page_count,
        } => {
            if debug {
                println!(
                    "Candidates (page {}/{}): {:?}",
                    page + 1,
                    page_count,
                    candidates
                );
            }
        }
    }
}
