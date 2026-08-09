//! reduce_freq：词频文件减少工具（reduce_freq.c 的 Rust 移植）。
//!
//! 用法：`reduce_freq in_file out_file min_freq`
//! 依据词频是否达到最小词频决定是否保留词条。

use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};

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
    if args.len() != 4 {
        println!("reduce_freq in_file out_file min_freq");
        std::process::exit(-1);
    }
    let in_name = &args[1];
    let out_name = &args[2];
    let min_freq: u32 = args[3].parse().unwrap_or(2);

    let fr = File::open(in_name)
        .map_err(|e| anyhow::anyhow!("无法打开 <{}>: {}", in_name, e))?;
    let fw = File::create(out_name)
        .map_err(|e| anyhow::anyhow!("无法创建 <{}>: {}", out_name, e))?;

    let reader = BufReader::new(fr);
    let mut writer = BufWriter::new(fw);

    let mut count = 0usize;
    for line in reader.split(b'\n') {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        count += 1;
        if count & 0x3ff == 0 {
            println!("{}", count);
        }
        let text = decode_gbk(&line);
        let mut parts = text.split_whitespace();
        let token = parts.next().unwrap_or("");
        let freq: i64 = parts.next().and_then(|c| c.parse().ok()).unwrap_or(-1);
        if token.is_empty() {
            continue;
        }
        if freq >= min_freq as i64 {
            let bytes = encode_gbk(&format!("{}\t{}\n", token, freq));
            let _ = writer.write_all(&bytes);
        }
    }
    writer.flush()?;
    Ok(())
}
