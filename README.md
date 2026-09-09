# Codex Account Manager

Windows 桌面端 Codex 多账号用量查看与手动切换工具。界面采用悬浮球、悬停摘要和账号面板三种状态，分别展示 5 小时与一周额度。

> 当前为 MVP 开发版本。切换账号会关闭并重新启动 Codex Desktop，操作前请先保存正在进行的工作。

## 已实现

- 通过官方 Codex App Server 读取账号、Plus/Pro 套餐与额度窗口。
- 导入当前账号，并通过浏览器 OAuth 添加其他账号。
- 使用 Windows DPAPI 加密保存各账号登录态。
- 串行刷新多个账号，失败时保留最近一次可用数据。
- 手动切换账号，包含进程退出、原子替换、重启验证和失败回滚。
- 系统托盘、可配置自动刷新、开机启动、位置记忆和三状态悬浮界面。
- 精确额度圆环、轻微 4 秒呼吸动效，以及运行/停止/切换/异常状态点。

## 开发环境

- Windows 10/11 与 Microsoft Edge WebView2 Runtime
- Node.js 20 或更高版本
- Rust stable（MSVC toolchain）
- Visual Studio C++ Build Tools
- 已安装并登录的 Codex Desktop

## 本地开发

```powershell
npm install
npm run tauri dev
```

## 生成便携 EXE

```powershell
npm run release
```

脚本只生成未打包的开发发行版，不创建 MSI/NSIS 安装包，也不会在项目根目录创建 `release` 文件夹。输出文件为 `src-tauri/target/release/codex-account-manager.exe`。

## 拖动悬浮球

在圆球状态直接按住鼠标左键拖动；如果悬停摘要已经展开，也可以按住摘要左侧的圆球拖动。移动超过 4 px 即进入原生窗口拖动，不会被当作单击；松开后会按当前显示器和 DPI 保存位置。

本项目是非官方辅助工具，与 OpenAI 没有隶属或背书关系。OpenAI/ChatGPT 标记归 OpenAI 所有，并仅用于表明本工具直接服务于 Codex/ChatGPT 账号管理。

完整需求、数据安全与切换事务设计见 [MVP.md](MVP.md)，确认过的 V3 设计图见 [design/codex-account-manager-ui.png](design/codex-account-manager-ui.png)。
