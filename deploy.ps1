# 部署华宇拼音输入法（uniSpim）TSF 文本服务。
#
# 需以管理员身份运行 PowerShell：
#   powershell -ExecutionPolicy Bypass -File deploy.ps1 [register|unregister]
#
# 步骤：
#   1. 编译 release DLL
#   2. 注册 TSF 文本服务（CLSID + InProcServer32 + profile + category）
#   3. 提示在系统设置中启用该输入法

param(
    [Parameter(Position = 0)]
    [ValidateSet("register", "unregister")]
    [string]$Action = "register",

    [string]$DllPath = ""
)

$ErrorActionPreference = "Stop"

# 检查管理员权限
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
    [Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Error "需要以管理员身份运行。请右键 PowerShell -> 以管理员身份运行。"
    exit 1
}

# 确定 DLL 路径
if ($DllPath -eq "") {
    $DllPath = Join-Path (Get-Location) "target\release\unispim_tsf.dll"
}
$DllPath = [System.IO.Path]::GetFullPath($DllPath)

if (-not (Test-Path $DllPath)) {
    Write-Error "未找到 DLL: $DllPath"
    Write-Error "请先运行: cargo build --workspace --release"
    exit 1
}

$regTool = Join-Path (Get-Location) "target\release\unispim-tsf-reg.exe"

switch ($Action) {
    "register" {
        Write-Host "=== 注册华宇拼音输入法 TSF 文本服务 ===" -ForegroundColor Cyan
        Write-Host "DLL: $DllPath"
        Write-Host ""

        if (-not (Test-Path $regTool)) {
            Write-Error "未找到注册工具: $regTool"
            exit 1
        }

        # 方式一：优先使用 regsvr32（由 DLL 自身 DllRegisterServer 注册，InProcServer32 指向 DLL）
        Write-Host "[1/2] 使用 regsvr32 注册 COM 服务器..."
        $p = Start-Process regsvr32 -ArgumentList "/s `"$DllPath`"" -Wait -PassThru
        if ($p.ExitCode -ne 0) {
            Write-Warning "regsvr32 退出码: $($p.ExitCode)，尝试使用注册工具..."
            & $regTool register $DllPath
        } else {
            Write-Host "      regsvr32 注册成功。" -ForegroundColor Green
        }

        # 确保 TSF profile 与 category 已注册（regsvr32 的 DllRegisterServer 已包含）
        Write-Host "[2/2] 注册完成。"
        Write-Host ""
        Write-Host "接下来需要在 Windows 中添加该输入法：" -ForegroundColor Yellow
        Write-Host "  设置 -> 时间和语言 -> 语言 -> 中文（简体，中国）-> 键盘 -> 添加键盘"
        Write-Host "  选择【华宇拼音输入法】"
        Write-Host "  或运行: control input (控制面板 -> 区域和语言 -> 键盘和语言 -> 更改键盘)"
        Write-Host ""
        Write-Host "部署完成。若输入法未立即出现，请注销并重新登录。" -ForegroundColor Green
    }

    "unregister" {
        Write-Host "=== 注销华宇拼音输入法 TSF 文本服务 ===" -ForegroundColor Cyan
        if (Test-Path $regTool) {
            & $regTool unregister
        }
        # 通过 regsvr32 注销
        $p = Start-Process regsvr32 -ArgumentList "/s /u `"$DllPath`"" -Wait -PassThru
        Write-Host "注销完成。" -ForegroundColor Green
    }
}
