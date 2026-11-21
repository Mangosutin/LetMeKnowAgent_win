//! `send` / `status` / `stop` / `presets` コマンド。
//! 送信は 127.0.0.1 のローカルHTTP（POST /notify）で行う。
//! サーバー未起動の場合は `serve` を DETACHED_PROCESS で起動し、起動を待つ。

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::os::windows::process::CommandExt;
use std::time::Duration;

use windows_sys::Win32::System::Threading::{CREATE_NO_WINDOW, DETACHED_PROCESS};

use crate::config::Config;
use crate::notification::Notification;

/// JSON入力形式。ex：
/// `{"preset":"claude","title":"ビルド完了","detail":"テスト通過","duration":"short"}`
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct NotificationRequest {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub preset: Option<String>,
    #[serde(default)]
    pub duration: Option<String>, // "short" | "long"
}

impl NotificationRequest {
    /// サーバー側で Notification に変換する
    pub fn into_notification(self, cfg: &Config) -> Notification {
        let preset = cfg.preset(self.preset.as_deref());
        let duration = match self.duration.as_deref() {
            Some("long") => Some(winrt_toast::ToastDuration::Long),
            _ => Some(winrt_toast::ToastDuration::Short),
        };
        let title = if self.title.is_empty() {
            "winletmeknow 通知".to_string()
        } else {
            self.title
        };
        Notification {
            title,
            detail: self.detail.unwrap_or_default(),
            preset_name: preset.name,
            icon_path: cfg.preset_icon(self.preset.as_deref()),
            tag: format!(
                "n{:x}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0)
            ),
            duration,
        }
    }
}

pub fn cmd_send(cfg: &Config, json_input: &str, file: Option<&std::path::Path>) {
    let text = if let Some(file) = file {
        match std::fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("ファイル {} の読み込みに失敗しました: {e}", file.display());
                std::process::exit(1);
            }
        }
    } else if json_input == "-" {
        let mut s = String::new();
        if let Err(e) = std::io::stdin().read_to_string(&mut s) {
            eprintln!("標準入力の読み取りに失敗しました: {e}");
            std::process::exit(1);
        }
        s
    } else {
        json_input.to_string()
    };
    let req: NotificationRequest = match serde_json::from_str(text.trim()) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("JSONのパースに失敗しました: {e}");
            std::process::exit(1);
        }
    };
    let title = if req.title.is_empty() { "winletmeknow 通知" } else { &req.title };
    match send_to_server(cfg, &req) {
        Ok(()) => println!("送信しました: {title}"),
        Err(e) => {
            eprintln!("送信に失敗しました: {e}");
            std::process::exit(1);
        }
    }
}

/// サーバーへPOST /notify。サーバー未起動なら起動してから送信する。
fn send_to_server(cfg: &Config, req: &NotificationRequest) -> Result<(), String> {
    let addr = SocketAddr::from(([127, 0, 0, 1], cfg.port));
    let body = serde_json::to_string(req).map_err(|e| e.to_string())?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(1))
        .or_else(|_| {
            spawn_daemon();
            wait_ready(addr);
            TcpStream::connect_timeout(&addr, Duration::from_secs(2))
        })
        .map_err(|e| format!("サーバーへの接続に失敗しました: {e}"))?;
    let req_text = format!(
        "POST /notify HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        cfg.port,
        body.len(),
        body
    );
    stream.write_all(req_text.as_bytes()).map_err(|e| format!("書き込みに失敗しました: {e}"))?;
    let mut resp = Vec::new();
    let _ = stream.read_to_end(&mut resp);
    let text = String::from_utf8_lossy(&resp);
    let ok = text.starts_with("HTTP/1.1 200 ") || text.starts_with("HTTP/1.1 200\r\n");
    if ok {
        Ok(())
    } else {
        Err(format!(
            "サーバーからの不正解応答: {}",
            text.lines().next().unwrap_or("(応答なし)")
        ))
    }
}

/// `serve` を DETACHED_PROCESS で起動する（起動元が終了しても残る）
fn spawn_daemon() {
    let exe = std::env::current_exe().unwrap_or_default();
    let _ = std::process::Command::new(exe)
        .arg("serve")
        .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
        .spawn();
}

/// ポートへ接続できるようになるまで待つ（最大約4秒）
fn wait_ready(addr: SocketAddr) {
    for _ in 0..80 {
        if TcpStream::connect_timeout(&addr, Duration::from_millis(50)).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

pub fn cmd_status(cfg: &Config) {
    let addr = SocketAddr::from(([127, 0, 0, 1], cfg.port));
    match TcpStream::connect_timeout(&addr, Duration::from_millis(500)) {
        Ok(_) => println!("実行中 (port {})", cfg.port),
        Err(_) => println!("停止中"),
    }
}

pub fn cmd_stop(cfg: &Config) {
    let addr = SocketAddr::from(([127, 0, 0, 1], cfg.port));
    match TcpStream::connect_timeout(&addr, Duration::from_millis(500)) {
        Ok(mut stream) => {
            let req = "POST /shutdown HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            if stream.write_all(req.as_bytes()).is_ok() {
                let mut resp = Vec::new();
                let _ = stream.read_to_end(&mut resp);
                println!("サーバーを停止しました");
            } else {
                println!("接続に失敗しました");
            }
        }
        Err(_) => println!("サーバーは起動していません"),
    }
}

pub fn cmd_presets(cfg: &Config) {
    let mut keys: Vec<String> = cfg.presets.keys().cloned().collect();
    keys.sort();
    println!("--- プリセット (port {}) ---", cfg.port);
    for k in keys {
        let p = cfg.presets.get(&k).expect("sorted keys");
        let icon = p.icon.as_ref().map(|i| i.display().to_string()).unwrap_or_else(|| "-".into());
        println!("  {k}: {} ({icon})", p.name);
    }
}
