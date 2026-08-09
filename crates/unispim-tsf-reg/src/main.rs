//! unispim-tsf-reg：TSF 文本服务注册/注销工具。
//!
//! 用法：
//! ```sh
//! unispim-tsf-reg register [dll_path] [data_dir]    # 注册 TSF 文本服务
//! unispim-tsf-reg unregister [dll_path]             # 注销 TSF 文本服务
//! unispim-tsf-reg setdata <data_dir>                # 设置输入法数据目录
//! unispim-tsf-reg cleardata                         # 清除输入法数据目录
//! ```
//!
//! `dll_path` 为 `unispim_tsf.dll` 的完整路径（可选）。
//! - 若不提供，则使用当前进程模块路径（即本工具自身，仅用于验证注册逻辑）。
//! - 部署时**必须**提供 DLL 路径，否则 InProcServer32 会指向 exe 而非 DLL。
//!
//! `data_dir` 为输入法数据目录（含 `wordlib/` 与 `zi/hzpy.dat`）。
//!
//! 注意：注册写入 HKLM/HKCR 需要管理员权限（需以管理员运行）。

use unispim_tsf::{
    clear_data_dir, register, register_with_path, set_data_dir, unregister, CLSID_TEXT_SERVICE,
};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let action = args.get(1).map(|s| s.as_str()).unwrap_or("register");

    match action {
        "register" | "reg" | "-r" => {
            let dll_path = args.get(2).map(|s| s.as_str());
            let data_dir = args.get(3).map(|s| s.as_str());
            let ok = match dll_path {
                Some(path) => register_with_path(Some(path)),
                None => register(),
            };
            if ok {
                println!("TSF 文本服务注册成功（CLSID: {}）", describe_clsid());
                if let Some(dir) = data_dir {
                    if set_data_dir(dir) {
                        println!("输入法数据目录已设置: {}", dir);
                    } else {
                        eprintln!("警告: 输入法数据目录设置失败: {}", dir);
                    }
                }
            } else {
                eprintln!("TSF 文本服务注册失败");
                std::process::exit(1);
            }
        }
        "unregister" | "unreg" | "-u" => {
            unregister();
            println!("TSF 文本服务已注销");
        }
        "setdata" | "data" => {
            match args.get(2) {
                Some(dir) => {
                    if set_data_dir(dir) {
                        println!("输入法数据目录已设置: {}", dir);
                    } else {
                        eprintln!("输入法数据目录设置失败（需要管理员权限）");
                        std::process::exit(1);
                    }
                }
                None => {
                    eprintln!("用法: unispim-tsf-reg setdata <data_dir>");
                    std::process::exit(1);
                }
            }
        }
        "cleardata" => {
            clear_data_dir();
            println!("输入法数据目录已清除");
        }
        _ => {
            println!(
                "用法:\n  unispim-tsf-reg register [dll_path] [data_dir]\n  unispim-tsf-reg unregister [dll_path]\n  unispim-tsf-reg setdata <data_dir>\n  unispim-tsf-reg cleardata"
            );
            std::process::exit(1);
        }
    }
}

fn describe_clsid() -> String {
    format!(
        "{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        CLSID_TEXT_SERVICE.data1,
        CLSID_TEXT_SERVICE.data2,
        CLSID_TEXT_SERVICE.data3,
        CLSID_TEXT_SERVICE.data4[0],
        CLSID_TEXT_SERVICE.data4[1],
        CLSID_TEXT_SERVICE.data4[2],
        CLSID_TEXT_SERVICE.data4[3],
        CLSID_TEXT_SERVICE.data4[4],
        CLSID_TEXT_SERVICE.data4[5],
        CLSID_TEXT_SERVICE.data4[6],
        CLSID_TEXT_SERVICE.data4[7]
    )
}
