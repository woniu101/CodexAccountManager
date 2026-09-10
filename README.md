# Codex Account Manager

<p align="center">
  <img src="src-tauri/icons/app-icon.svg" width="112" alt="Codex Account Manager 图标">
</p>

<p align="center">
  Windows 上的 Codex 多账号额度查看与手动切换工具
</p>

> [!IMPORTANT]
> 本项目是非官方辅助工具，与 OpenAI 没有隶属或背书关系。切换账号会关闭并重新启动 Codex Desktop，可能中断正在运行的任务，请先保存工作。

## 功能

- 用悬浮球同时展示当前账号的 5 小时和一周剩余额度。
- 在悬停摘要和账号面板中查看套餐、额度、重置时间与运行状态。
- 通过浏览器 OAuth 添加多个 Codex 账号，不影响当前正在使用的账号。
- 使用 Windows DPAPI（CurrentUser）加密保存非当前账号的登录态。
- 手动切换 Codex Desktop 账号，包含退出、原子替换、重启验证和失败回滚。
- 支持删除非当前账号、立即刷新以及 1/5/10/15/30 分钟自动刷新。
- 支持系统托盘、开机启动、位置记忆、多方向展开和右键拖动。
- 状态点、额度环和中心图标保持同步呼吸，并兼容“减少动态效果”设置。

## 界面预览

下图由程序的三个实际运行状态截图合成，账号名已替换为演示数据；界面细节以当前版本实际运行效果为准。

<p align="center">
  <img src="design/codex-account-manager-ui.png" width="100%" alt="Codex Account Manager 三状态运行界面">
</p>

原始界面截图保存在 `design/status1.png`、`design/status2.png` 和 `design/status3.png`。

## 系统要求

- Windows 10/11 x64。
- Microsoft Edge WebView2 Runtime。
- 已安装并登录的 Codex Desktop。

从源码构建还需要 Node.js 20+、Rust stable（MSVC toolchain）和 Visual Studio C++ Build Tools。

## 使用

### 下载和启动

1. 从 [GitHub Releases](https://github.com/woniu101/CodexAccountManager/releases) 下载 `codex-account-manager.exe`。
2. 运行 EXE。程序默认以悬浮球显示，并常驻系统托盘。
3. 首次启动会导入当前 Codex Desktop 登录的账号。

当前构建未签名，Windows SmartScreen 可能提示“未知发布者”。请只从本仓库 Releases 下载，并在发布页核对 SHA-256。

### 悬浮界面

| 操作 | 效果 |
|---|---|
| 鼠标移入悬浮球 | 展开当前账号摘要 |
| 左键单击摘要 | 展开账号管理面板 |
| 鼠标离开界面 | 收起为悬浮球 |
| 在界面任意状态按住右键拖动 | 移动并保存悬浮球位置 |

界面会优先向右、向上展开；空间不足时自动切换方向，并保持悬浮球锚点不动。

状态点含义：

- 绿色：Codex Desktop 正在运行。
- 灰色：Codex Desktop 未运行。
- 黄色：正在切换账号。
- 红色：认证、启动或切换出现异常。

### 添加账号

1. 打开账号面板，选择“添加账号”。
2. 在系统浏览器完成 Codex 授权。
3. 授权完成后程序会读取额度、加密保存登录态并刷新账号列表。

授权流程支持取消，等待超过 5 分钟会自动超时。重复添加同一账号时，会按账号 ID 更新已有记录。

### 切换账号

在目标账号左侧的大圆形按钮上单击并确认切换。程序会：

1. 检查是否存在独立运行的 Codex CLI，避免误伤命令行任务。
2. 关闭已识别的 Codex Desktop 进程。
3. 保存当前账号，替换目标账号登录态。
4. 重新启动 Codex Desktop，并验证实际登录账号。
5. 验证失败时自动恢复原账号。

### 系统托盘

托盘菜单可显示或隐藏悬浮窗、刷新全部账号、快速切换账号、添加账号、打开设置和退出程序。托盘中的账号邮箱会自动脱敏。

## 数据和安全

程序不提供云同步，也不会把账号凭据上传到自有服务器。应用数据保存在：

```text
%LOCALAPPDATA%\CodexAccountManager\
├── settings.json
├── accounts.json
├── accounts\<account-id>\credential.bin
├── switch-journal.json          # 切换期间短暂存在
└── temp\                        # 授权或查询完成后清理
```

- `accounts.json` 只保存账号摘要，不包含 token。
- 非当前账号凭据由 Windows DPAPI CurrentUser 加密，仅当前 Windows 用户可解密。
- 当前活动账号仍由 Codex 保存在 `%USERPROFILE%\.codex\auth.json`，这是 Codex 正常工作所必需的。
- 隔离授权和查询期间会短暂生成明文登录态，操作结束后立即清理；启动时也会清理超过 24 小时的残留临时目录。
- 切换日志只记录账号 ID 和事务阶段，不记录 token。

请勿提交 `%LOCALAPPDATA%\CodexAccountManager`、`.codex/auth.json`、日志、真实账号截图或其他凭据文件。

## 从源码运行

```powershell
npm install
npm run tauri dev
```

前端检查与构建：

```powershell
npm run build
```

Rust 格式和测试：

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml
```

## 生成便携 EXE

```powershell
npm run release
```

当前脚本只生成便携 EXE，不创建 MSI/NSIS 安装包。输出位置：

```text
src-tauri\target\release\codex-account-manager.exe
```

发布前计算校验值：

```powershell
Get-FileHash .\src-tauri\target\release\codex-account-manager.exe -Algorithm SHA256
```

完整发布核对项见 [docs/MVP.md](docs/MVP.md#发布清单)。

## 项目结构

```text
src/                    Vue 3 界面、状态管理和交互
src-tauri/src/          Rust 核心、账号仓库、切换器和托盘
src-tauri/icons/        应用图标资源
scripts/                Windows 构建脚本
docs/                   MVP 定义、验收状态与发布清单
design/                 三状态实机截图与脱敏产品展示图
```

## 当前限制

- 仅支持 Windows 10/11 x64。
- 只管理 Codex Desktop 的个人 ChatGPT/Codex 登录态，不统计 API 或组织账单。
- 不支持额度耗尽后自动切换，也不支持多个账号同时运行。
- 当前只发布便携 EXE，尚未提供安装包、自动更新和代码签名。
- Codex Desktop 或 App Server 的接口变化可能影响账号读取与切换。

## 参与贡献

欢迎提交 Issue 和 Pull Request。问题报告请附版本、Windows 版本、复现步骤和脱敏后的日志或截图；不要上传 token、完整 `auth.json` 或真实邮箱。

## 许可证

本项目采用 [MIT License](LICENSE)。

OpenAI、ChatGPT 和 Codex 是其各自权利人的商标，仅用于说明本工具的兼容对象。
