//! unispim_tsf_reg：TSF 文本服务注册/注销工具。
//!
//! 用法：
//! ```sh
//! unispim_tsf_reg register    # 注册 TSF 文本服务
//! unispim_tsf_reg unregister  # 注销 TSF 文本服务
//! ```
//!
//! 注意：注册写入 HKCR\CLSID 需要管理员权限（需以管理员运行）。

use unispim_rs::windows_tsf::{register, unregister};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let action = args.get(1).map(|s| s.as_str()).unwrap_or("register");

    match action {
        "register" | "reg" | "-r" => {
            if register() {
                println!("TSF 文本服务注册成功（CLSID: {}）", describe_clsid());
            } else {
                eprintln!("TSF 文本服务注册失败");
                std::process::exit(1);
            }
        }
        "unregister" | "unreg" | "-u" => {
            unregister();
            println!("TSF 文本服务已注销");
        }
        _ => {
            println!("用法: unispim_tsf_reg register|unregister");
            std::process::exit(1);
        }
    }
}

fn describe_clsid() -> String {
    format!("{:?}", unispim_rs::windows_tsf::CLSID_TEXT_SERVICE)
}
