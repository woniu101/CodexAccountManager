# Codex Account Manager — MVP 定义与交付状态

> 版本：0.1.0
>
> 状态：核心功能完成，准备公开仓库与发布 Windows 便携 EXE
>
> 最后更新：2026-09-10

## 产品目标

Codex Account Manager 是一款 Windows 桌面辅助工具，用来：

1. 在不反复登录的情况下查看多个个人 Codex 账号的 5 小时和一周额度。
2. 安全、可回滚地切换 Codex Desktop 当前使用的账号。
3. 用低干扰的悬浮球和系统托盘提供常驻入口。

MVP 优先保证凭据安全、数据准确、切换可恢复和日常操作顺畅，不追求自动切号或跨平台。

## MVP 边界

### 已包含

- 导入 Codex Desktop 当前登录账号。
- 通过浏览器 OAuth 添加账号，支持取消、超时和重复账号更新。
- 使用 Windows DPAPI CurrentUser 加密保存非当前账号登录态。
- 串行刷新所有账号；单个账号失败时保留其他账号及最近可用数据。
- 展示套餐、5 小时额度、一周额度和重置时间。
- 三状态悬浮界面、自适应展开方向、右键拖动和位置记忆。
- 手动切换账号，包含关闭 Codex、事务日志、原子替换、重启验证与失败回滚。
- 删除非当前账号，并在操作失败时保持索引一致。
- 系统托盘、立即刷新、自动刷新、开机启动和设置页。

### 暂不包含

- 额度耗尽后自动切换账号。
- 同时运行多个不同账号的 Codex Desktop。
- ChatGPT API、组织账单或团队用量统计。
- 长期趋势、消耗预测和云端同步。
- macOS、Linux、Microsoft Store 发布。
- 安装包、自动更新和代码签名。

## 已交付用户流程

### 查看额度

1. 启动后导入或读取当前 Codex 账号。
2. 常驻悬浮球用青色内环表示 5 小时剩余量，用紫色外环表示一周剩余量。
3. 鼠标移入后展开摘要，显示百分比和重置时间。
4. 单击摘要进入账号管理器，查看所有账号。
5. 面板打开且缓存超过 60 秒时触发后台刷新。

缺失额度显示暂无数据，不会误报为 0%。未识别的额度窗口会保存到 `extraLimits`，当前界面不展示。

### 添加账号

1. 程序为新登录创建隔离的临时 `CODEX_HOME`。
2. 调用 Codex App Server 启动 ChatGPT 浏览器 OAuth。
3. 用户在系统浏览器完成授权。
4. 程序读取账号身份和额度，以账号 ID 去重。
5. 登录态经 DPAPI 加密后写入账号仓库，随后刷新界面。

单次授权最长等待 5 分钟；用户可取消。操作完成、取消或失败后都会清理内嵌进度状态和临时目录。

### 切换账号

```text
获取全局切换锁
    ↓
检查 Codex Desktop 与独立 CLI 进程
    ↓
关闭已验证的 Codex Desktop 进程树
    ↓
加密保存当前账号的最新 auth.json
    ↓
写入不含凭据的切换事务日志
    ↓
解密目标凭据并原子替换 auth.json
    ↓
重新启动 Codex Desktop
    ↓
通过 account/read 验证目标 account_id
    ↓
成功：提交并清理日志／失败：恢复原账号并重启
```

如果检测到独立运行的 Codex CLI，切换会被阻止，避免中断命令行任务。程序从 Codex 进程树启动时会通过 Explorer 脱离父进程，避免关闭 Codex 后自身一并退出。

### 删除账号

- 仅允许删除非当前账号。
- 删除前显示账号信息和凭据影响范围。
- 删除的是本机保存的加密凭据，不会删除 OpenAI 账号。
- 元数据更新失败时保持原索引，避免留下半完成状态。

## 界面与交互

### 展示资产

- [`design/status1.png`](../design/status1.png)、[`design/status2.png`](../design/status2.png) 和 [`design/status3.png`](../design/status3.png) 是当前版本的三状态实机截图。
- [`design/codex-account-manager-ui.png`](../design/codex-account-manager-ui.png) 由上述截图合成为 README 产品总览。
- 总览中的账号使用 `alex.chen@example.com` 和 `backup@example.com` 演示，不对应真实用户。
- 合成图仅用于说明交互层级和视觉关系，验收应以实际 EXE 运行为准。

### 三种状态

| 状态 | 用途 | 交互 |
|---|---|---|
| 悬浮球 | 常驻查看当前额度与运行状态 | 移入展开摘要 |
| 摘要 | 查看当前账号的完整额度信息 | 单击展开管理器 |
| 管理器 | 添加、刷新、设置、切换和删除账号 | 离开界面收起 |

- 悬浮球视觉尺寸约 72 × 72 px，收起窗口有效区域为 88 × 88 px。
- 摘要窗口画布为 424 × 420 px，通过 Win32 Region 只保留当前可交互表面。
- 展开方向优先向右、向上；可用空间不足时自动改为向左或向下。
- 状态变化以悬浮球为锚点，切换状态时不改变圆球位置。
- 向上展开保持底边固定，向下展开时操作栏位于面板底部。
- 所有外角使用一致圆角；单账号和多账号按内容紧凑布局。
- 状态点、两个额度环和中心图标使用同一 4 秒呼吸节奏；系统启用减少动态效果时禁用动画。

### 系统托盘

- 左键单击托盘图标切换悬浮窗显示或隐藏。
- 动态展示当前运行状态、脱敏邮箱、套餐和额度。
- 提供账号子菜单，可识别当前账号并直接选择其他账号。
- 提供立即刷新、添加账号、设置和退出。

## 技术实现

### 技术栈

- Tauri 2
- Vue 3 + TypeScript + Pinia
- Rust 2024 edition
- Codex App Server JSON-RPC
- Windows DPAPI 与 Win32 窗口/进程 API

### 代码结构

```text
src/
├── App.vue                    悬浮窗状态与几何编排
├── components/               悬浮球、摘要、账号行和设置界面
├── stores/accounts.ts        前端账号摘要与操作状态
├── types/account.ts          前端数据类型
└── utils/plan.ts             套餐显示规范化

src-tauri/src/
├── app_server.rs             JSON-RPC、OAuth 与额度读取
├── vault.rs                  账号仓库、DPAPI 和临时目录
├── process_manager.rs        Codex 进程识别、退出与启动
├── commands.rs               Tauri 命令和切换事务
├── settings.rs               设置与位置持久化
├── tray_menu.rs              动态系统托盘菜单
├── models.rs                 后端数据模型
└── lib.rs                    应用初始化与事件注册
```

### App Server 查询

- 使用 `account/read` 获取账号身份和套餐。
- 使用 `account/rateLimits/read` 获取额度窗口。
- 单次请求超时为 12 秒。
- 300 分钟窗口映射为 5 小时额度，10080 分钟窗口映射为一周额度。
- 多账号使用全局队列串行查询，避免同时创建大量子进程。
- 自动刷新可选 1、5、10、15 或 30 分钟，默认 5 分钟。

## 数据与安全边界

应用数据目录：

```text
%LOCALAPPDATA%\CodexAccountManager\
├── settings.json
├── accounts.json
├── accounts\<account-id>\credential.bin
├── switch-journal.json          # 仅切换期间存在
└── temp\                        # 隔离授权与查询目录
```

- `accounts.json` 不保存 access token 或 refresh token。
- `credential.bin` 使用 Windows DPAPI CurrentUser 加密，绑定当前 Windows 用户。
- 当前活动账号必须继续以 Codex 可读取的形式保存在 `%USERPROFILE%\.codex\auth.json`。
- 前端只接收账号摘要，不接收完整凭据。
- 日志和切换事务不写入 token。
- 临时隔离目录在操作后删除，启动时清理超过 24 小时的残留。
- MVP 依赖 Windows 用户目录的默认访问控制，尚未额外收紧临时目录 ACL。

## 设置默认值

| 设置 | 默认值 | 可选值 |
|---|---:|---|
| 自动刷新 | 5 分钟 | 1/5/10/15/30 分钟 |
| 开机启动 | 关闭 | 开启/关闭 |
| 记住悬浮球位置 | 开启 | 开启/关闭 |

## 验收状态

| 验收项 | 状态 |
|---|---|
| 导入并展示当前账号 | 已完成 |
| 隔离 OAuth 添加账号，支持取消和超时 | 已完成 |
| DPAPI 加密多账号凭据 | 已完成 |
| 串行刷新与单账号失败隔离 | 已完成 |
| 三状态界面和四方向展开 | 已完成 |
| 位置保存与重启恢复 | 已完成 |
| 事务式切换、验证和回滚 | 已完成 |
| 删除非当前账号 | 已完成 |
| 动态托盘菜单 | 已完成 |
| 前端生产构建 | 已通过 |
| Rust 格式检查与单元测试 | 已通过，当前 12 项测试 |
| Windows 10/11 干净环境回归 | 发布前待完成 |
| 125%/150%/200% DPI 和多显示器回归 | 发布前待完成 |
| 关闭、写入、启动和验证阶段故障注入 | 建议发布前补充 |
| 安装包、签名与自动更新 | 后续版本 |

## 已知限制

- Codex App Server 是外部依赖，其接口或返回字段变化可能导致查询失败。
- 当前只针对 Windows x64 和 Codex Desktop 个人账号流程进行实现。
- 便携 EXE 未签名时可能触发 Windows SmartScreen 提示。
- 账号切换会重启 Codex Desktop，不能保证运行中的任务继续执行。
- 不自动切换账号，也不会绕过套餐、额度或平台限制。
- 当前活动账号仍受 Codex 自身明文 `auth.json` 存储方式约束。

## 发布清单

### 仓库检查

- [x] README 与当前功能同步。
- [x] MVP 范围、实现状态和限制已整理。
- [x] 添加 MIT License。
- [x] README 使用由三状态实机截图合成的脱敏展示图。
- [x] 展示图账号已替换为演示数据，原始截图未包含可读的完整邮箱。
- [x] 当前工作区及现有 4 个提交已完成高置信度凭据模式与敏感文件名扫描，未发现异常。
- [x] `.gitignore` 已覆盖 `node_modules`、`dist`、缓存和 `src-tauri/target`。
- [x] `package.json`、`package-lock.json`、`Cargo.toml` 和 `tauri.conf.json` 版本号均为 `0.1.0`。

### 构建与验证

- [x] `npm ci`（2026-09-10 已通过）
- [x] `npm run build`（2026-09-10 已通过）
- [x] `cargo fmt --manifest-path src-tauri/Cargo.toml --check`（2026-09-10 已通过）
- [x] `cargo test --manifest-path src-tauri/Cargo.toml`（2026-09-10：12 项通过）
- [x] `npm run release`（2026-09-10 已生成 0.1.0 便携 EXE）
- [x] 本机发行构建启动检查通过，包括经 Explorer 脱离父进程后的常驻进程。
- [ ] 在未安装开发环境的 Windows 10/11 x64 机器启动 EXE。
- [ ] 验证首次导入、添加取消/超时/成功、刷新、切换成功/回滚和删除。
- [ ] 验证多显示器、任务栏位置和 100%/125%/150%/200% DPI。
- [ ] 检查退出后无遗留 App Server、临时目录或切换事务。

### GitHub Release

- [ ] 创建 `v0.1.0` 标签。
- [ ] 上传 `codex-account-manager.exe`。
- [ ] 计算并发布 SHA-256：

```powershell
Get-FileHash .\src-tauri\target\release\codex-account-manager.exe -Algorithm SHA256
```

- [ ] 发布说明包含系统要求、主要功能、已知限制和升级注意事项。
- [ ] 明确标注未签名便携版，提醒用户只从官方 Releases 下载。

## 后续路线

### 0.1.x

- 补齐干净环境、故障注入、多显示器和高 DPI 自动/手工回归。
- 改进诊断日志导出，并确保自动脱敏。
- 根据真实用户反馈继续优化动画、方向排版和授权异常提示。

### 0.2

- 评估 NSIS/MSI 安装包、代码签名和自动更新。
- 提供账号别名、排序和更完整的额度详情。
- 评估更严格的临时目录 ACL 与诊断工具。

### 长期方向

- 在 Codex 官方接口和平台政策允许的前提下评估 macOS/Linux。
- 评估本地历史趋势和可选通知；不默认上传任何账号数据。
