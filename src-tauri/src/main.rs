#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// DSH Desktop — Tauri 2.x 桌面壳（已在本机 cargo build --release 编译并产出 exe）
// 职责：用系统 WebView2 加载本地 DeepSeek Harness Web UI (http://127.0.0.1:3080)，
//       检测 Node.js、启动/停止 DSH 服务进程、系统托盘、窗口状态 & 设置持久化。
// 说明：主窗口由 tauri.conf.json 的 app.windows[0].url 创建（指向 3080）。

use std::net::TcpStream;
use std::process::{Child, Command};
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, RunEvent, WindowEvent};

const PORT: u16 = 3080;
const NODE_DL_URL: &str = "https://nodejs.org/zh-cn/download";

// 全局子进程句柄：启动的 DSH 服务，退出时 kill
struct DshProc(Mutex<Option<Child>>);

fn port_open(port: u16) -> bool {
    // 只做 TCP 握手不够（可能劫持到非 DSH 进程）。改为 HTTP 特征探活：发送 GET / 并确认收件方返回 HTTP 响应。
    use std::io::{Read, Write};
    if let Ok(mut s) = TcpStream::connect(("127.0.0.1", port)) {
        let _ = s.set_read_timeout(Some(std::time::Duration::from_millis(800)));
        let _ = s.write_all(b"GET / HTTP/1.0\r\nHost: localhost\r\n\r\n");
        let mut buf = [0u8; 512];
        if let Ok(n) = s.read(&mut buf) {
            let text = String::from_utf8_lossy(&buf[..n]).to_lowercase();
            return text.contains("http");
        }
    }
    false
}

fn has_node() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn start_dsh() -> Result<Child, String> {
    // Windows 上 npx 是 npx.cmd（批处理），Rust Command 无法直接 CreateProcess 执行 .cmd；
    // 用 cmd /C 包装，pin 固定版本；并加 CREATE_NO_WINDOW 防止弹出黑色控制台窗口。
    #[cfg(windows)]
    use std::os::windows::process::CommandExt;
    let mut cmd = Command::new("cmd");
    cmd.args(["/C", "npx", "-y", "@deepseek-ai/dsh@0.1.1-rc.2", "--profile", "web", "--port", &PORT.to_string()]);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    cmd.spawn().map_err(|e| format!("spawn dsh failed: {e}"))
}

fn wait_port(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        for _ in 0..60 {
            if port_open(PORT) {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.eval("window.location.reload()");
                    let _ = win.show();
                }
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    });
}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "打开", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;
    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quit" => app.exit(0),
            "open" => {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
            _ => {}
        })
        .build(app)?;
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .manage(DshProc(Mutex::new(None)))
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle();
            let _ = setup_tray(handle);

            if !has_node() {
                // 无 Node：打开官网下载页（最小引导），不启服务
                let _ = tauri_plugin_opener::open_url(NODE_DL_URL, None::<&str>);
                return Ok(());
            }

            if !port_open(PORT) {
                match start_dsh() {
                    Ok(child) => *app.state::<DshProc>().0.lock().unwrap() = Some(child),
                    Err(e) => eprintln!("dsh start failed: {e}"),
                }
            }
            wait_port(handle);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // 关窗 → 最小化到托盘（不退出）；托盘"退出"才真退出
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!())
        .expect("error building tauri app")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                // 退出时杀 DSH 进程树（taskkill /T 连带杀其派生的 node 子进程），防孤儿
                if let Some(child) = app.state::<DshProc>().0.lock().unwrap().take() {
                    let pid = child.id();
                    #[cfg(windows)]
                    {
                        let _ = Command::new("taskkill")
                            .args(["/F", "/T", "/PID", &pid.to_string()])
                            .output();
                    }
                    #[cfg(not(windows))]
                    {
                        drop(child);
                    }
                }
            }
        });
}
