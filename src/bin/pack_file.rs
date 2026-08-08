//! pack_file：文件打包工具（pack_file.c 的 Rust 移植）。
//!
//! 将目录中的全部文件随机分配到若干个打包文件中。
//!
//! 用法：`pack_file files_dir file_prefix packed_file_number`

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const MAX_FILES: usize = 100;

fn append_file(tag_file: &Path, src_file: &Path) -> anyhow::Result<()> {
    let mut src = fs::File::open(src_file)?;
    let mut tag = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(tag_file)?;
    let mut buffer = vec![0u8; 0x100000];
    loop {
        let n = src.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        tag.write_all(&buffer[..n])?;
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        println!(
            "pack_file files_dir file_prefix packed_file_number\n  files_dir: 输入文件目录名\n  file_prefix: 打包文件名称前缀, 如:yl\n  packed_file_number: 全部打包文件的数目 1-100，如：10"
        );
        std::process::exit(-1);
    }
    let files_dir = &args[1];
    let prefix = &args[2];
    let file_number: usize = args[3].parse().unwrap_or(0);
    if file_number == 0 || file_number > MAX_FILES {
        println!("packed_file_number 必须在 1-100 之间");
        std::process::exit(-1);
    }

    let cwd = std::env::current_dir()?;
    let mut tag_names: Vec<PathBuf> = Vec::with_capacity(file_number);
    for no in 0..file_number {
        let p = cwd.join(format!("{}-{:03}.txt", prefix, no));
        let _ = fs::remove_file(&p);
        tag_names.push(p);
    }

    let mut entries: Vec<PathBuf> = Vec::new();
    let dir = Path::new(files_dir);
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            entries.push(entry.path());
        }
    }

    let mut count = 0usize;
    for src in &entries {
        let no = count % file_number;
        let tag = &tag_names[no];
        append_file(tag, src)?;
        count += 1;
    }
    println!("packed {} files into {} archives", count, file_number);
    Ok(())
}
