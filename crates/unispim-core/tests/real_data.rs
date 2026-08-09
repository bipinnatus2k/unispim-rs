//! 真实词库数据集成测试（需要仓库中的 data/unispim6/wordlib 目录）。

use unispim_core::ci::process_ci_candidates;
use unispim_core::parse::parse_pin_yin_string_reverse;
use unispim_core::wordlib::WordLib;

fn data_root() -> Option<std::path::PathBuf> {
    // 从当前目录向上查找 workspace 根目录的 data/unispim6
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let p = dir.join("data/unispim6/wordlib/sys.uwl");
        if p.exists() {
            return Some(dir.join("data/unispim6"));
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

fn assert_candidates(uwl: &str, pinyin: &str, expected: &[&str]) {
    let root = data_root().expect("缺少 data/unispim6 数据");
    let path = root.join("wordlib").join(uwl);
    let wl = WordLib::from_file(&path.to_string_lossy()).unwrap();
    let syllables = parse_pin_yin_string_reverse(pinyin, 0);
    assert!(!syllables.is_empty(), "{} 无法解析", pinyin);
    let candidates = process_ci_candidates(&wl, &syllables, 0);
    let strings: Vec<String> = candidates
        .iter()
        .take(expected.len())
        .map(|c| String::from_utf16_lossy(&c.item.hz))
        .collect();
    for exp in expected {
        assert!(
            strings.contains(&exp.to_string()),
            "{}: 候选 {:?} 中未找到 {}",
            pinyin,
            strings,
            exp
        );
    }
}

#[test]
fn test_real_sys_wordlib_query() {
    let Some(root) = data_root() else {
        eprintln!("跳过：缺少 data/unispim6/wordlib 数据");
        return;
    };
    let _ = root;
    assert_candidates("sys.uwl", "zhongguo", &["中国"]);
    assert_candidates("sys.uwl", "renmin", &["人民"]);
    assert_candidates("beijing.uwl", "beijing", &["北京"]);
    assert_candidates("sys.uwl", "chengshi", &["城市"]);
    assert_candidates("sys.uwl", "jisuanji", &["计算机"]);
}

#[test]
fn test_real_wordlib_info() {
    let Some(root) = data_root() else {
        eprintln!("跳过：缺少 data/unispim6/wordlib 数据");
        return;
    };
    let wl = WordLib::from_file(&root.join("wordlib/sys.uwl").to_string_lossy()).unwrap();
    assert_eq!(wl.header.word_count, 428536);
    assert_eq!(wl.header.page_count, 7496);
    assert_eq!(wl.header.name, "华宇输入法系统词库");
}

#[test]
fn test_syllable_string_roundtrip() {
    // 所有 460 个拼音都能被解析；注意部分拼音（如 jve/jv、lv、nv）
    // 在原表中与 jue 等别名映射到相同 con/vow，还原时取其规范拼音。
    let pinyins = [
        "a", "ai", "an", "ang", "ao", "b", "ba", "bai", "ban", "bang", "bao", "bei", "ben",
        "beng", "bi", "bian", "biao", "bie", "bin", "bing", "bo", "bu", "c", "ca", "cai", "can",
        "cang", "cao", "ce", "cen", "ceng", "ch", "cha", "chai", "chan", "chang", "chao", "che",
        "chen", "cheng", "chi", "chong", "chou", "chu", "chua", "chuai", "chuan", "chuang", "chui",
        "chun", "chuo", "ci", "cong", "cou", "cu", "cuan", "cui", "cun", "cuo", "d", "da", "dai",
        "dan", "dang", "dao", "de", "dei", "den", "deng", "di", "dia", "dian", "diao", "die",
        "ding", "diu", "dong", "dou", "du", "duan", "dui", "dun", "duo", "e", "ei", "en", "eng",
        "er", "f", "fa", "fan", "fang", "fei", "fen", "feng", "fiao", "fo", "fou", "fu", "g",
        "ga", "gai", "gan", "gang", "gao", "ge", "gei", "gen", "geng", "gong", "gou", "gu", "gua",
        "guai", "guan", "guang", "gui", "gun", "guo", "h", "ha", "hai", "han", "hang", "hao",
        "he", "hei", "hen", "heng", "hong", "hou", "hu", "hua", "huai", "huan", "huang", "hui",
        "hun", "huo", "j", "ji", "jia", "jian", "jiang", "jiao", "jie", "jin", "jing", "jiong",
        "jiu", "ju", "juan", "jue", "jun", "jv", "jve", "k", "ka", "kai", "kan", "kang", "kao",
        "ke", "kei", "ken", "keng", "kong", "kou", "ku", "kua", "kuai", "kuan", "kuang", "kui",
        "kun", "kuo", "l", "la", "lai", "lan", "lang", "lao", "le", "lei", "leng", "li", "lia",
        "lian", "liang", "liao", "lie", "lin", "ling", "liu", "lo", "long", "lou", "lu", "luan",
        "lue", "lun", "luo", "lv", "lve", "m", "ma", "mai", "man", "mang", "mao", "me", "mei",
        "men", "meng", "mi", "mian", "miao", "mie", "min", "ming", "miu", "mo", "mou", "mu",
        "n", "na", "nai", "nan", "nang", "nao", "ne", "nei", "nen", "neng", "ni", "nian",
        "niang", "niao", "nie", "nin", "ning", "niu", "nong", "nou", "nu", "nuan", "nue", "nun",
        "nuo", "nv", "nve", "o", "ou", "p", "pa", "pai", "pan", "pang", "pao", "pei", "pen",
        "peng", "pi", "pian", "piao", "pie", "pin", "ping", "po", "pou", "pu", "q", "qi", "qia",
        "qian", "qiang", "qiao", "qie", "qin", "qing", "qiong", "qiu", "qu", "quan", "que",
        "qun", "qv", "qve", "r", "ran", "rang", "rao", "re", "ren", "reng", "ri", "rong", "rou",
        "ru", "ruan", "rui", "run", "ruo", "s", "sa", "sai", "san", "sang", "sao", "se", "sen",
        "seng", "sh", "sha", "shai", "shan", "shang", "shao", "she", "shei", "shen", "sheng",
        "shi", "shou", "shu", "shua", "shuai", "shuan", "shuang", "shui", "shun", "shuo", "si",
        "song", "sou", "su", "suan", "sui", "sun", "suo", "t", "ta", "tai", "tan", "tang", "tao",
        "te", "tei", "teng", "ti", "tian", "tiao", "tie", "ting", "tong", "tou", "tu", "tuan",
        "tui", "tun", "tuo", "w", "wa", "wai", "wan", "wang", "wei", "wen", "weng", "wo", "wu",
        "x", "xi", "xia", "xian", "xiang", "xiao", "xie", "xin", "xing", "xiong", "xiu", "xu",
        "xuan", "xue", "xun", "xv", "xve", "y", "ya", "yan", "yang", "yao", "ye", "yi", "yin",
        "ying", "yo", "yong", "you", "yu", "yuan", "yue", "yun", "yv", "yve", "z", "za", "zai",
        "zan", "zang", "zao", "ze", "zei", "zen", "zeng", "zh", "zha", "zhai", "zhan", "zhang",
        "zhao", "zhe", "zhei", "zhen", "zheng", "zhi", "zhong", "zhou", "zhu", "zhua", "zhuai",
        "zhuan", "zhuang", "zhui", "zhun", "zhuo", "zi", "zong", "zou", "zu", "zuan", "zui",
        "zun", "zuo",
    ];
    for py in pinyins {
        let s = parse_pin_yin_string_reverse(py, 0);
        assert_eq!(s.len(), 1, "{} 解析失败", py);
        // 校验解析结果与拼音表的映射一致
        let entry = crate::map_entry(py);
        if let Some((con, vow)) = entry {
            assert_eq!(s[0].con(), con, "{} 声母不一致", py);
            assert_eq!(s[0].vow(), vow, "{} 韵母不一致", py);
        }
    }
}

fn map_entry(py: &str) -> Option<(u8, u8)> {
    use unispim_core::syllable_map::SYLLABLE_MAP;
    SYLLABLE_MAP
        .iter()
        .find(|e| e.py == py)
        .map(|e| (e.con, e.vow))
}
