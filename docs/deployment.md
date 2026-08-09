# 部署与测试华宇拼音输入法

本仓库的 `unispim-tsf` crate 编译为 Windows TSF 文本服务 DLL（`unispim_tsf.dll`），
这是一个真正的 COM 文本服务，需要注册到系统后才能作为输入法使用。

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

### 2. 手动注册（备用）

```powershell
# 方式一：regsvr32（推荐，DLL 路径正确）
regsvr32 /s "D:\Project\rust\unispim-rs\target\release\unispim_tsf.dll"

# 方式二：注册工具 + 显式 DLL 路径
.\target\release\unispim-tsf-reg.exe register "D:\Project\rust\unispim-rs\target\release\unispim_tsf.dll"
```

### 3. 在系统中启用输入法

注册完成后，需要在 Windows 中添加该输入法：

- **设置 → 时间和语言 → 语言和区域 → 中文（简体，中国）→ 键盘选项 → 添加键盘**
  → 选择「华宇拼音输入法」
- 或控制面板：`控制面板 → 区域和语言 → 键盘和语言 → 更改键盘 → 添加`

如果输入法没有立即出现，请**注销并重新登录**（或重启 explorer）。

### 4. 注销

```powershell
powershell -ExecutionPolicy Bypass -File .\deploy.ps1 unregister
# 或
regsvr32 /s /u "D:\...\unispim_tsf.dll"
```

## 三、常见问题

### 1. 输入法没有出现在输入法列表

- 确认已用**管理员**权限注册。
- 确认 `InProcServer32` 默认值指向**完整的 DLL 路径**（非 exe）：
  ```
  reg query "HKCR\CLSID\{E7EA138E-69F8-11D7-A6EA-00065B84435C}\InProcServer32"
  ```
- **注销并重新登录**后再查看。

### 2. TSF 文本服务需要数字签名

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

### 3. 本测试环境存在 HookDemo 注册表重定向

在某些 IME 开发/测试环境中，可能存在注册表钩子（如本环境的 `HookDemo_x64`），
它会拦截 `HKCR\CLSID` 写入并把内容重定向到 `HookDemo_x64\HookDemo.dll` 子键下。
此时注册表面成功，但系统无法在标准位置找到文本服务。

解决：在**干净的系统/虚拟机**中部署测试，或检查并停用此类注册表钩子工具。

### 4. 候选窗口 / 预编辑不显示

- 本实现当前将候选列表输出到控制台（`unispim-ime` 的 `--debug` 模式）。
- TSF 组合（下划线拼音）通过 `ITfComposition` 实现，已在 `TextService` 中处理。

## 四、跨平台测试（不经注册表）

如果你只是想在本地验证输入法引擎逻辑（不实际作为系统输入法），可使用：

```sh
# 启动 imekit 程序（Linux Wayland/X11 或 Windows）
cargo run --release -p unispim-ime -- --wordlib-dir data/unispim6/wordlib --hzpy data/unispim6/zi/hzpy.dat

# 查询候选（验证引擎）
cargo run --release -p unispim-tools --bin wl_tool -- query data/unispim6/wordlib/sys.uwl zhongguo
```

单元/集成测试：

```sh
cargo test --workspace
```
