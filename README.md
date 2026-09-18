# unispim-rs

华宇拼音输入法（uniSpim）核心引擎与词库工具链的 Rust 重写（Cargo workspace）。

原项目位于 `unispim/`（C/C++，LGPL 2.1），本仓库以 Rust 重新实现其可移植的核心部分：
音节（拼音）引擎、`.uwl` 词库文件格式、词汇候选检索、汉字数据（hzpy.dat）、
符号转换、输入法引擎状态机，并通过 [imekit](https://crates.io/crates/imekit)
与 `windows` crate 集成到系统输入法框架（Wayland / X11 / Windows TSF）。

## Workspace 结构

| crate | 类型 | 说明 |
| --- | --- | --- |
| `unispim-core` | 库 | 核心引擎（平台无关）：音节/拼音、词库、候选、汉字数据、符号、IME 状态机 |
| `unispim-tools` | 二进制 | 词库与语料工具（`wl_tool`、`merge`、`reduce`、`reduce_freq`、`pack_file`） |
| `unispim-ime` | 二进制 | 交互式测试工具（安全，不拦截系统键盘） |
| `unispim-tsf` | cdylib | Windows TSF 文本服务 DLL（`unispim_tsf.dll`，导出 COM 服务器） |
| `unispim-tsf-reg` | 二进制 | Windows TSF 文本服务注册/注销工具 |

### unispim-core 模块

| 模块 | 对应原工程 | 说明 |
| --- | --- | --- |
| `syllable` | `syllable.h/.c` | 音节编码（声母/韵母/音调，16 位位域）、常量、模糊音 |
| `syllable_map` | `share_segment.c` | 拼音 -> 音节转换表（460 条，与原表逐条一致） |
| `parse` | `syllable.c` | 拼音串解析（`GetSyllable` / `ParsePinYinString[Reverse]`） |
| `wordlib` | `wordlib.h/.c` | `.uwl` V6.6 词库格式：头部、索引表、1024 字节页、变长词条 |
| `ci` | `ci.c` | 词汇候选检索、排重与排序 |
| `hzdata` | `zi.c` / `kernel.h` | 汉字数据（hzpy.dat）读取与单字候选 |
| `symbol` | `symbol.c` | 中英文符号转换（中文标点、全角、引号配对） |
| `ime` | `editor.c` / `kernel.c` | 输入法引擎状态机（纯逻辑，平台无关） |

## 词库（.uwl）二进制格式

与 C 版完全兼容（小端、`#pragma pack(1)` 语义）：

- **头部** `WORDLIBHEADER`（2388 字节）：签名 `0x14091994`、名称(16)、作者(16)、
  词条数、页数、可编辑、版本、索引表 `index[24][24]`；按页对齐（3 页共 3072 字节）
- **页**（1024 字节）：页号、下一页号、长度标志、数据长度 + 最多 1008 字节词条
- **词条** `WORDLIBITEM`：4 字节 feature（`effective:1, ci_length:6, syllable_length:6, freq:19`）
  + 音节数组（每项 2 字节）+ 汉字数组（每项 2 字节，UTF-16LE）

## 构建与测试

```sh
cargo build --workspace --release
cargo test --workspace
```

## Windows TSF 文本服务 DLL

`unispim-tsf` crate 编译为 `unispim_tsf.dll`，参考 `TSFexample/`
（1BasicTextService ~ 7Composition）导出标准 COM 服务器：

- `DllGetClassObject`：通过 `IClassFactory` 创建 `TextService` COM 对象
- `DllRegisterServer` / `DllUnregisterServer`：注册 / 注销
- `DllCanUnloadNow`：引用计数控制卸载

`TextService` 实现 5 个 TSF 接口：`ITfTextInputProcessor` + `ITfThreadMgrEventSink`
+ `ITfKeyEventSink` + `ITfTextEditSink` + `ITfCompositionSink`。

### 部署与测试

```sh
cargo build --workspace --release
powershell -ExecutionPolicy Bypass -File .\deploy.ps1 register   # 管理员运行
```

详细步骤、常见问题（数字签名要求、HookDemo 注册表重定向等）见
[`docs/deployment.md`](docs/deployment.md)。

注册/注销（需管理员权限）：

```sh
cargo run --release -p unispim-tsf-reg -- register     # 注册 TSF 文本服务
cargo run --release -p unispim-tsf-reg -- unregister   # 注销
```

## 交互式测试工具（unispim-ime）

**安全的输入法引擎测试工具**：从 stdin 读取按键，实时显示预编辑、候选与上屏结果，
**不拦截系统键盘**，不影响其他程序输入。

```sh
cargo run --release -p unispim-ime -- --wordlib-dir data/unispim6/wordlib --hzpy data/unispim6/zi/hzpy.dat
```

交互按键：
- `a`-`z`：输入拼音；空格：选第一候选；`0`-`9`：选候选
- `[`/`]`：翻页；退格：删除；回车：上屏拼音；`q` 或 Ctrl+C：退出
- `--send`：确认上屏时通过 SendInput 注入到前台窗口（Windows，默认仅打印）

功能：
- 拼音输入 -> 词汇候选（多词库自动去重）+ 单字候选（按词频排序）
- 空格选首个候选、数字键选择、翻页、退格、回车上屏拼音
- 标点自动转换为中文符号

## 工具（unispim-tools）

### wl_tool（词库维护）

```
wl_tool create <wordlib.uwl> <text.txt>   # 由文本词条文件创建词库
wl_tool import <wordlib.uwl> <text.txt>   # 向词库导入词条
wl_tool export <wordlib.uwl> <text.txt>   # 导出词库为文本
wl_tool info   <wordlib.uwl>              # 显示词库信息
wl_tool query  <wordlib.uwl> <pinyin>     # 按拼音查询候选
```

文本词条文件格式（UTF-16LE 带 BOM，与 C 版一致）：

```
名称=测试词库
作者=unispim-rs
编辑=1

中国	zhongguo	100
人民	renmin	90
```

### 语料处理工具（对应 `source/tools/`）

```
merge       <min_freq>      # 合并相同词的词频（stdin -> stdout，GBK）
reduce      [min] [max]     # 将词拆分为 2..8 字子词并输出负词频
reduce_freq in out min_freq # 依据词频下限筛选词条
pack_file   dir prefix n    # 将目录中的文件随机打包到 n 个文件
```

## 许可

核心代码移植自 LGPL 2.1 的华宇拼音输入法源码，版权归北京华宇软件股份有限公司所有。

