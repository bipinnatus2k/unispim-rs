# 部署与测试华宇拼音输入法

本仓库的 `unispim-tsf` crate 编译为 Windows TSF 文本服务 DLL（`unispim_tsf.dll`），
这是一个真正的 COM 文本服务，需要注册到系统后才能作为输入法使用。

## 〇、重要：更新 DLL 后必须注销重登

**输入法 DLL 会被所有已运行进程（explorer、浏览器、IDE 等）加载并占用。**
更新 DLL 后若不注销重登，系统会继续使用内存中的旧 DLL，导致"加载了但无法输入"
或"修改不生效"。

更新流程：
1. 注销输入法（见下方"注销"）
2. **注销 Windows 用户并重新登录**（或重启）
3. 重新编译：`cargo build --workspace --release`
4. 重新注册

## 一、部署前提

- **Windows 10 / 11**（64 位）
- **管理员权限**（注册写入 `HKCR\CLSID` 需要）
- 已编译 release 版本：

```sh
cargo build --workspace --release
```

## 二、部署步骤

### 1. 一键部署脚本

```powershell
# 以管理员身份运行
powershell -ExecutionPolicy Bypass -File .\deploy.ps1 register
```

脚本会：
- 调用 `regsvr32 /s unispim_tsf.dll`，由 DLL 自身的 `DllRegisterServer`
  完成注册（这样 `InProcServer32` 正确指向 DLL 本身）。
- `DllRegisterServer` 内部完成三件事：
  1. **注册表**：`HKCR\CLSID\{E7EA138E-...}` + `InProcServer32`（DLL 路径 + Apartment）
  2. **TSF profile**：`ITfInputProcessorProfiles::Register` + `AddLanguageProfile`（简体中文 0x0804）
  3. **TSF 分类**：`ITfCategoryMgr::RegisterCategory`（`GUID_TFCAT_TIP_KEYBOARD`，注册为键盘输入法）

> **注意**：部署后还需设置输入法数据目录（见下方第 3 步），否则 DLL 中引擎没有
> 词库数据，无法产生候选（表现为"无法输入"）。

### 2. 手动注册（备用）

```powershell
# 方式一：regsvr32（推荐，DLL 路径正确）
regsvr32 /s "D:\Project\rust\unispim-rs\target\release\unispim_tsf.dll"

# 方式二：注册工具 + 显式 DLL 路径
.\target\release\unispim-tsf-reg.exe register "D:\Project\rust\unispim-rs\target\release\unispim_tsf.dll"
```

### 3. 手动导入 .reg 注册文件（推荐，无需编译/运行工具）

仓库提供了可直接导入的注册表文件（UTF-16 LE 编码，已实测可用）：

```
docs\unispim_tsf_register.reg      # 注册
docs\unispim_tsf_unregister.reg    # 注销
```

**使用步骤：**

1. 用记事本打开 `docs\unispim_tsf_register.reg`，若 DLL 路径与本机不同，
   将两处 `D:\\Project\\rust\\unispim-rs\\target\\release\\unispim_tsf.dll`
   替换为你的实际路径（注意 .reg 中反斜杠要写成 `\\`）。
2. **重要**：将文件末尾的 `DataDir` 值改为你的输入法数据目录
   （包含 `wordlib\` 子目录与 `zi\hzpy.dat`），DLL 运行时从这里加载词库。
   仓库默认值：`D:\Project\rust\unispim-rs\data\unispim6`。
3. 双击该文件（或 `reg import docs\unispim_tsf_register.reg`），确认导入。
4. **注销并重新登录**，然后在输入法设置中添加「华宇拼音输入法」。

**优点：** `reg import` 由 regedit 直接写入注册表，不经过进程内 API 调用，
可绕过部分注册表钩子工具（如 HookDemo）的重定向问题（本环境已验证）。

**注册文件内容说明（对应 TSFexample 的三步注册 + 数据目录）：**

| 注册表项 | 作用 |
| --- | --- |
| `HKCR\CLSID\{E7EA138E-...}\InProcServer32` | COM 服务器（DLL 路径 + Apartment） |
| `HKLM\SOFTWARE\Microsoft\CTF\TIP\{E7EA138E-...}\LanguageProfile\0x00000804\{E7EA138F-...}` | 输入法 profile（简体中文，描述/图标/Enable=1） |
| `HKLM\SOFTWARE\Microsoft\CTF\TIP\{E7EA138E-...}\Category\Item\{34745C63-...}` | 分类注册（GUID_TFCAT_TIP_KEYBOARD 键盘输入法） |
| `HKLM\SOFTWARE\uniSpim\DataDir` | 输入法数据目录（DLL 加载词库与汉字数据） |

### 4. 在系统中启用输入法

注册完成后，需要在 Windows 中添加该输入法：

- **设置 → 时间和语言 → 语言和区域 → 中文（简体，中国）→ 键盘选项 → 添加键盘**
  → 选择「华宇拼音输入法」
- 或控制面板：`控制面板 → 区域和语言 → 键盘和语言 → 更改键盘 → 添加`

如果输入法没有立即出现，请**注销并重新登录**（或重启 explorer）。

### 5. 注销

```powershell
powershell -ExecutionPolicy Bypass -File .\deploy.ps1 unregister
# 或
regsvr32 /s /u "D:\...\unispim_tsf.dll"
# 或（手动导入）
reg import docs\unispim_tsf_unregister.reg
```

## 三、常见问题

### 1. 输入法已加载但无法输入

依次检查：

1. **数据目录未设置（最常见原因）**：DLL 中的引擎需要词库数据才能产生候选。
   确认注册表中存在：
   ```
   reg query "HKLM\SOFTWARE\uniSpim"
   ```
   应看到 `DataDir` 指向包含 `wordlib\` 和 `zi\hzpy.dat` 的目录。
   若无，执行 `unispim-tsf-reg setdata <你的数据目录>` 或重新导入 .reg。

2. **使用的是旧 DLL**：DLL 被运行中的进程占用，更新后需**注销重登**才生效。
   确认 `InProcServer32` 指向最新编译的 DLL 路径。

3. **验证 DLL 可被 COM 创建**：若 DLL 加载失败，可查看事件查看器
   （应用程序日志）中的 COM 错误，或使用 Process Monitor 观察 DLL 加载。

4. **未签名文本服务被拒载**：见下方第 2 条（数字签名要求）。

### 3. 输入法没有出现在输入法列表

- 确认已用**管理员**权限注册。
- 确认 `InProcServer32` 默认值指向**完整的 DLL 路径**（非 exe）：
  ```
  reg query "HKCR\CLSID\{E7EA138E-69F8-11D7-A6EA-00065B84435C}\InProcServer32"
  ```
- **注销并重新登录**后再查看。

### 4. TSF 文本服务需要数字签名

从 Windows 8 开始，**未签名的第三方 TSF 文本服务默认不会加载**（部分系统在
开发模式下可加载，但不保证）。这是 TSF 输入法部署的最大障碍。

测试阶段解决方式：

- **测试证书签名**：生成自签名测试证书并用 `signtool` 签名，将证书导入
  「受信任的根证书颁发机构」和「受信任的发布者」。
  ```sh
  # 生成自签名测试证书（一次性）
  New-SelfSignedCertificate -Type CodeSigningCert -Subject "CN=uniSpim Test" -CertStoreLocation Cert:\CurrentUser\My
  # 导出并导入到 受信任的根/受信任的发布者
  # 使用 signtool 签名 DLL（需 Windows SDK）
  signtool sign /fd SHA256 /t http://timestamp.digicert.com /f test.pfx /p password unispim_tsf.dll
  ```
- **临时关闭签名强制**（仅限开发，不推荐生产）：在 `HKLM\SOFTWARE\Microsoft\CTF\TIP` 下
  配置 `EnableLoadUnsafeTip`，或使用 Windows 10 的
  「启用未签名输入法的开发模式」（注册表 `HKLM\SOFTWARE\Microsoft\CTF\TIP`）。

### 5. 本测试环境存在 HookDemo 注册表重定向

在某些 IME 开发/测试环境中，可能存在注册表钩子（如本环境的 `HookDemo_x64`），
它会拦截 `HKCR\CLSID` 写入并把内容重定向到 `HookDemo_x64\HookDemo.dll` 子键下。
此时注册表面成功，但系统无法在标准位置找到文本服务。

解决：在**干净的系统/虚拟机**中部署测试，或检查并停用此类注册表钩子工具。

### 6. 候选窗口 / 预编辑不显示

- 本实现当前将候选列表输出到控制台（`unispim-ime` 的 `--debug` 模式）。
- TSF 组合（下划线拼音）通过 `ITfComposition` 实现，已在 `TextService` 中处理。

## 四、本地测试输入法引擎（不经注册表，安全）

`unispim-ime` 是**安全的交互式测试工具**：从 stdin 读取按键、实时显示
预编辑/候选/上屏结果，**不拦截系统键盘**，不会影响其他程序输入。

```sh
# 启动交互式测试（输入拼音 -> 空格/数字选候选 -> 观察上屏）
cargo run --release -p unispim-ime -- --wordlib-dir data/unispim6/wordlib --hzpy data/unispim6/zi/hzpy.dat

# 交互按键：a-z 拼音，空格=选第一候选，0-9=选候选，[]=翻页，退格=删除，回车=上屏拼音，q=退出

# 也可管道输入测试：
echo "zhongguo " | cargo run --release -p unispim-ime -- --wordlib-dir data/unispim6/wordlib --hzpy data/unispim6/zi/hzpy.dat

# 查询候选（验证引擎）
cargo run --release -p unispim-tools --bin wl_tool -- query data/unispim6/wordlib/sys.uwl zhongguo
```

单元/集成测试：

```sh
cargo test --workspace
```
