//! 输入法引擎 + 真实数据集成测试。

use unispim_core::hzdata::HzData;
use unispim_core::ime::{ImeAction, ImeEngine, KeyInput};
use unispim_core::wordlib::WordLib;

fn test_data_path() -> Option<(String, String)> {
    // 从当前目录向上查找 workspace 根目录的 data/unispim6
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let wl = dir.join("data/unispim6/wordlib/sys.uwl");
        let hz = dir.join("data/unispim6/zi/hzpy.dat");
        if wl.exists() && hz.exists() {
            return Some((wl.to_string_lossy().into_owned(), hz.to_string_lossy().into_owned()));
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

fn build_engine() -> (ImeEngine, Vec<String>) {
    let (wl_path, hz_path) = test_data_path().expect("缺少 data/unispim6 数据");
    let wl = WordLib::from_file(&wl_path).unwrap();
    let hz = HzData::from_file(&hz_path).unwrap();
    let engine = ImeEngine::new(vec![wl], Some(hz));
    let out = vec!["华宇输入法系统词库".to_string(), "华宇输入法系统词库".to_string()];
    (engine, out)
}

fn commits(output: &unispim_core::ime::EngineOutput) -> Vec<String> {
    output
        .actions
        .iter()
        .filter_map(|a| match a {
            ImeAction::Commit(s) => Some(s.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn test_real_word_candidates() {
    let (mut engine, _) = build_engine();
    for c in "zhongguo".chars() {
        engine.handle_key(&KeyInput::Letter(c));
    }
    assert_eq!(engine.preedit_pinyin(), "zhong'guo");
    let texts = engine.candidate_texts();
    assert!(
        texts.iter().any(|t| t == "中国"),
        "zhongguo 候选应包含 中国: {:?}",
        &texts[..texts.len().min(10)]
    );
    // 选择第一个候选
    let first = engine.candidates[0].text();
    let out = engine.handle_key(&KeyInput::Space);
    assert_eq!(commits(&out), vec![first]);
}

#[test]
fn test_real_zi_candidates() {
    // 单字候选：输入 zhong 应返回 中/忠/钟/终 等
    let (mut engine, _) = build_engine();
    for c in "zhong".chars() {
        engine.handle_key(&KeyInput::Letter(c));
    }
    assert_eq!(engine.syllables.len(), 1);
    let texts = engine.candidate_texts();
    assert!(
        texts.contains(&"中".to_string()),
        "zhong 单字候选应包含 中: {:?}",
        &texts[..texts.len().min(10)]
    );
    assert!(
        texts.contains(&"忠".to_string()),
        "zhong 单字候选应包含 忠: {:?}",
        &texts[..texts.len().min(10)]
    );
}

#[test]
fn test_real_multi_syllable_zi() {
    // 多音节逐字候选：zhongguo 的第一字应为 中
    let (mut engine, _) = build_engine();
    for c in "zhongguo".chars() {
        engine.handle_key(&KeyInput::Letter(c));
    }
    let texts = engine.candidate_texts();
    assert!(
        texts.iter().any(|t| t == "中国"),
        "zhongguo 候选应包含 中国: {:?}",
        &texts[..texts.len().min(10)]
    );
}

#[test]
fn test_real_fuzzy() {
    // 模糊音：sichuan / sichuang
    let (mut engine, _) = build_engine();
    engine.fuzzy_mode = 0;
    for c in "sichuan".chars() {
        engine.handle_key(&KeyInput::Letter(c));
    }
    let texts = engine.candidate_texts();
    assert!(
        texts.iter().any(|t| t == "四川"),
        "sichuan 应包含 四川: {:?}",
        &texts[..texts.len().min(10)]
    );
}
