//! reduce：词频减少工具（reduce.c 的 Rust 移植）。
//!
//! 从标准输入读取 `词 词频` 行（GBK 编码，每字 2 字节），
//! 将词拆分为长度 2..=8 的子词并以负词频输出。
//!
//! 用法：`reduce [min] [max]`

use std::io::{BufRead, BufReader, BufWriter, Write};

const MIN_TOKEN_LENGTH: usize = 2;
const MAX_TOKEN_LENGTH: usize = 8;

fn decode_gbk(bytes: &[u8]) -> String {
    let (cow, _, _) = encoding_rs::GBK.decode(bytes);
    cow.into_owned()
}

fn encode_gbk(s: &str) -> Vec<u8> {
    let (cow, _, _) = encoding_rs::GBK.encode(s);
    cow.into_owned()
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut min_length = MIN_TOKEN_LENGTH;
    let mut max_length = MAX_TOKEN_LENGTH;
    if args.len() >= 2 {
        min_length = args[1].parse::<usize>().unwrap_or(MIN_TOKEN_LENGTH);
        max_length = min_length;
    }
    if args.len() >= 3 {
        max_length = args[2].parse::<usize>().unwrap_or(MAX_TOKEN_LENGTH);
    }

    let stdin = std::io::stdin();
    let reader = BufReader::new(stdin.lock());
    let stdout = std::io::stdout();
    let mut writer = BufWriter::new(stdout.lock());

    for line in reader.split(b'\n') {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let text = decode_gbk(&line);
        let mut parts = text.split_whitespace();
        let token = parts.next().unwrap_or("");
        let freq: i64 = parts.next().and_then(|c| c.parse().ok()).unwrap_or(0);
        if token.is_empty() {
            continue;
        }
        let hz: Vec<u16> = token.encode_utf16().collect();
        let len = hz.len();
        for i in min_length..=max_length.min(len) {
            for j in 0..=len - i {
                let sub: String = String::from_utf16_lossy(&hz[j..j + i]);
                let bytes = encode_gbk(&format!("{:<20}  {:>6}\n", sub, -freq));
                let _ = writer.write_all(&bytes);
            }
        }
    }
    writer.flush()?;
    Ok(())
}
