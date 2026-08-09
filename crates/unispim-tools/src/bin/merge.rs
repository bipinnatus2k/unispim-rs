//! merge：词汇合并工具（merge.c 的 Rust 移植）。
//!
//! 从标准输入读取 `词 词频` 行（GBK 编码），合并相同词的词频并输出。
//! 每行只有词时词频计为 1。最小词频通过参数指定，低于该值的词不输出。

use std::io::{BufRead, BufReader, BufWriter, Write};

const MAX_COUNT: u64 = 4_000_000_000;

fn decode_gbk(bytes: &[u8]) -> String {
    let (cow, _, _) = encoding_rs::GBK.decode(bytes);
    cow.into_owned()
}

fn encode_gbk(s: &str) -> Vec<u8> {
    let (cow, _, _) = encoding_rs::GBK.encode(s);
    cow.into_owned()
}

fn parse_line(line: &str) -> (String, u64) {
    let mut parts = line.split_whitespace();
    let token = parts.next().unwrap_or("").to_string();
    let count = match parts.next() {
        Some(c) => c.parse::<u64>().unwrap_or(1),
        None => 1,
    };
    (token, count)
}

fn output<W: Write>(out: &mut W, token: &str, count: u64, min_freq: u64) {
    let count = count.min(MAX_COUNT);
    if count >= min_freq {
        let bytes = encode_gbk(&format!("{:<20}  {:>12}\n", token, count));
        let _ = out.write_all(&bytes);
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let min_freq = if args.len() >= 2 {
        args[1].parse::<u64>().unwrap_or(3)
    } else {
        3
    };

    let stdin = std::io::stdin();
    let reader = BufReader::new(stdin.lock());
    let stdout = std::io::stdout();
    let mut writer = BufWriter::new(stdout.lock());

    let mut lines = reader.split(b'\n');
    let first = match lines.next() {
        Some(Ok(l)) => decode_gbk(&l),
        _ => return Ok(()),
    };
    let (mut last_token, mut last_count) = parse_line(&first);

    for line in lines {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let (cur_token, cur_count) = parse_line(&decode_gbk(&line));
        if last_token == cur_token {
            let new_count = last_count + cur_count;
            last_count = if new_count < last_count {
                MAX_COUNT
            } else {
                new_count
            };
            continue;
        }
        output(&mut writer, &last_token, last_count, min_freq);
        last_token = cur_token;
        last_count = cur_count;
    }
    output(&mut writer, &last_token, last_count, min_freq);
    writer.flush()?;
    Ok(())
}
