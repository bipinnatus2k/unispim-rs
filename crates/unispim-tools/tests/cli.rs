//! wl_tool CLI 集成测试：创建 -> 查询 -> 导出 -> 重新导入 全链路。

use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_wl_tool")
}

fn tmp(name: &str) -> String {
    let dir = std::env::temp_dir().join("unispim-rs-test");
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name).to_string_lossy().to_string()
}

fn write_utf16(path: &str, content: &str) {
    let mut bytes = vec![0xFF, 0xFE];
    for unit in content.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    std::fs::write(path, bytes).unwrap();
}

fn read_utf16(path: &str) -> String {
    let bytes = std::fs::read(path).unwrap();
    let bytes = &bytes[2..];
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    String::from_utf16_lossy(&units)
}

const DICT: &str = "名称=集成测试\n作者=rs\n编辑=1\n\n中国\tzhongguo\t100\n人民\trenmin\t90\n电脑\tdiannao\t80\n";

#[test]
fn test_full_roundtrip() {
    let dict = tmp("it_dict.txt");
    let uwl = tmp("it.uwl");
    let out = tmp("it_out.txt");

    write_utf16(&dict, DICT);

    // create
    let status = Command::new(bin())
        .args(["create", &uwl, &dict])
        .status()
        .unwrap();
    assert!(status.success());

    // info
    let info = Command::new(bin()).args(["info", &uwl]).output().unwrap();
    let info = String::from_utf8_lossy(&info.stdout).to_string();
    assert!(info.contains("词条数: 3"), "info: {}", info);
    assert!(info.contains("集成测试"));

    // query
    let q = Command::new(bin())
        .args(["query", &uwl, "zhongguo"])
        .output()
        .unwrap();
    let q = String::from_utf8_lossy(&q.stdout).to_string();
    assert!(q.contains("中国"), "query: {}", q);

    let q2 = Command::new(bin())
        .args(["query", &uwl, "renmin"])
        .output()
        .unwrap();
    let q2 = String::from_utf8_lossy(&q2.stdout).to_string();
    assert!(q2.contains("人民"), "query2: {}", q2);

    // export
    let status = Command::new(bin())
        .args(["export", &uwl, &out])
        .status()
        .unwrap();
    assert!(status.success());
    let exported = read_utf16(&out);
    assert!(exported.contains("名称=集成测试"));
    assert!(exported.contains("中国\tzhong'guo\t100"));
    assert!(exported.contains("人民\tren'min\t90"));
    assert!(exported.contains("电脑\tdian'nao\t80"));

    // import back (duplicates -> no new entries)
    let status = Command::new(bin())
        .args(["import", &uwl, &out])
        .status()
        .unwrap();
    assert!(status.success());
    let info2 = Command::new(bin()).args(["info", &uwl]).output().unwrap();
    let info2 = String::from_utf8_lossy(&info2.stdout).to_string();
    assert!(info2.contains("词条数: 3"), "after import: {}", info2);
}

#[test]
fn test_query_no_result() {
    let dict = tmp("it2_dict.txt");
    let uwl = tmp("it2.uwl");
    write_utf16(&dict, DICT);
    Command::new(bin())
        .args(["create", &uwl, &dict])
        .status()
        .unwrap();

    let q = Command::new(bin())
        .args(["query", &uwl, "abcdefgh"])
        .output()
        .unwrap();
    let q = String::from_utf8_lossy(&q.stdout).to_string();
    assert!(q.contains("没有找到候选") || q.contains("无法解析"));
}

#[test]
fn test_invalid_file() {
    let uwl = tmp("it_missing.uwl");
    let _ = std::fs::remove_file(&uwl);
    let out = Command::new(bin()).args(["info", &uwl]).output().unwrap();
    assert!(!out.status.success());
}
