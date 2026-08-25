# dsh-desktop-tauri — DSH Desktop（Tauri 版）

用 **Tauri 2.x（Rust + 系统 WebView2）** 重写的 DeepSeek Harness 第三方桌面启动器，把体积从 Electron 版的 ~71MB 降到 **~9.7MB**（单文件 exe）/**约2.31MB**（NSIS 安装包），内存占用更低。

> 声明：本程序为**社区自制、非官方**的 DeepSeek Harness 桌面启动器，与 DeepSeek 官方及母公司无关；仅作兼容描述，不代表官方认可。

## 结构
```
dsh-desktop-tauri/
└─ src-tauri/
   ├─ Cargo.toml                 # tauri 2.x + store/window-state/autostart/opener（无 shell）
   ├─ Cargo.lock                 # 依赖锁定
   ├─ build.rs
   ├─ tauri.conf.json            # 主窗口加载 http://127.0.0.1:3080；WebView2 downloadBootstrapper
   ├─ capabilities/default.json  # 最小能力：core:default + 窗口 hide/show/set-focus
   ├─ icons/                     # icon.png / icon.ico
   └─ src/main.rs                # 主逻辑（Rust）
```

## 已实现（对应 Electron 版 `main.js`）
- **主窗口 + 加载 Web UI**：`app.windows[0].url=http://127.0.0.1:3080`（系统 WebView2 渲染，DSH 主 UI 为纯 Web）。
- **检测 Node.js**：`main.rs::has_node()`。
- **启动 DSH 服务到 3080**：`start_dsh()` 用 `cmd /C npx -y @deepseek-ai/dsh@0.1.1-rc.2 --profile web --port 3080`（pin 版本、cmd 兼容 Windows npx.cmd、`CREATE_NO_WINDOW` 无弹窗）；`wait_port()` HTTP 特征探活后 `eval reload`/`show`。
- **退出杀进程树**：`RunEvent::Exit` 时 `taskkill /F /T`（Windows）。
- **系统托盘**：`TrayIconBuilder` + 菜单；`CloseRequested` 时 `prevent_close + hide`（最小化到托盘）。
- **窗口状态/设置/自动启**：`tauri-plugin-window-state` / `tauri-plugin-store` / `tauri-plugin-autostart`。

## 尚未实现 / 可补强
1. **Node 引导页**：当前"无 Node 时用 `opener` 打开官网下载页"（最小版）。
2. **主题/插件装配**：✅ 已实现（v0.1.1）——开箱自带内置蓝主题 `dsh-theme-mineradio`，首次启动自动装配进 `~/.dsh/profiles/web`（Windows 用 junction 目录联接）。
3. **设置页 UI 桥接**：`invoke` + `#[tauri::command]`（待做）。
4. **代码签名 / 自动更新**：发布需 `tauri-plugin-updater` + 代码签名，避免 SmartScreen 警告。

## 编译/运行（Windows，需 Rust + MSVC + WebView2）
```powershell
cd dsh-desktop-tauri\src-tauri
cargo build --release          # 首次编译较慢（拉 tauri 全家桶）
cargo tauri dev                # 开发运行（需 tauri-cli）
cargo tauri build --bundles nsis  # 出安装包；或直接分发 release exe（便携）
```

## 体积对比
| 方案 | 体积 | 说明 |
|---|---|---|
| Electron 版 | ~71MB（便携 exe） | 捆绑 Chromium 运行时 |
| **Tauri 版** | ~9.7MB（exe）/ ~2.31MB（NSIS 安装包） | 复用系统 WebView2 |

## License
MIT（见 `LICENSE`）；第三方组件许可见项目内说明。

## 安装 / 运行提示（社区套件）
- 本程序为**社区自制、未签名**。首次安装/运行可能触发 Windows SmartScreen 的「未知发布者」提示，点 **更多信息 → 仍要运行** 即可。建议只从本仓库的 Releases 下载并校验哈希。
- 需 Windows 10/11（自带 WebView2；安装包已配 WebView2 兜底）。
- 校验 SHA-256（v0.1.1 安装包 `DSH Desktop_0.1.1_x64-setup.exe`）：
  `C0E15A2DEC4DA9D72B747FEA0CC518115095C18D87FEFC94C5A5203933CC68FA`
