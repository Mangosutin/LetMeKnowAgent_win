//! デーモン（トレイ常駐）本体。
//!
//! 構成:
//! - **サーバースレッド**: 127.0.0.1 ループバックHTTP（POST /notify, POST /shutdown, GET /health）
//! - **トーストスレッド**: `winrt_toast` でトースト表示（COM 初期化済み）
//! - **メインスレッド**: トレイアイコン + メッセージループ（menu 終了・exit メニュー経由でも停止可能）
//!
//! メモリ設計（アイドル時≈数MB）:
//! - ブロッキング `TcpListener::accept` + ノンブロッキング Poll
//! - Win32 メッセージループのみのブロッキング（`GetMessageW`）、ポーリングなし
//! - トーストは COM/WinRT 呼び出しのみ（ヘルパは最小限）

use std::net::TcpListener;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, TrayIconBuilder};
use windows_sys::Win32::UI::WindowsAndMessaging::{DispatchMessageW, GetMessageW, MSG, SetTimer};

use crate::config::Config;
use crate::logger;
use crate::notification::Notification;
use crate::server;
use crate::toast;

/// デーモンを起動する（Ctrl-C相当以外では永久に常駐する）
pub fn run_daemon(cfg: &Config) -> Result<(), String> {
    // ログファイル初期化（exe隣の daemon.log）
    logger::init_log(&logger::log_path());
    logger::log_info("デーモン起動");

    // AUMID登録（失敗はログ出力）
    if let Some(icon) = cfg.preset_icon(None) {
        match winrt_toast::register(&cfg.aumid, "winletmeknow", Some(&icon)) {
            Ok(()) => logger::log_info(&format!("AUMID登録: {} ({})", cfg.aumid, icon.display())),
            Err(e) => logger::log_error(&format!("AUMID登録に失敗しました: {e}")),
        }
    } else {
        logger::log_warn("アイコンが見つからないため、AUMID登録をスキップしました");
    }

    let listener = TcpListener::bind(("127.0.0.1", cfg.port))
        .map_err(|e| format!("ポート {} にバインドできません: {e}", cfg.port))?;
    let (tx, rx) = mpsc::channel::<Notification>();
    let quit = Arc::new(AtomicBool::new(false));

    // トーストスレッド
    {
        let aumid = cfg.aumid.clone();
        thread::spawn(move || {
            unsafe {
                use windows_sys::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
                CoInitializeEx(null_mut(), COINIT_APARTMENTTHREADED as u32);
            }
            let manager = winrt_toast::ToastManager::new(&aumid);
            for notif in rx {
                toast::show_toast(&manager, &notif);
            }
        });
    }

    // サーバースレッド
    {
        let cfg = cfg.clone();
        let quit = quit.clone();
        thread::spawn(move || server::run(listener, &cfg, tx, quit));
    }

    // メインスレッド: トレイアイコン + メッセージループ
    let icon = build_tray_icon(cfg)?;
    let exit_item = MenuItem::new("終了", true, None);
    let menu = Menu::new();
    menu.append(&exit_item).map_err(|e| e.to_string())?;
    let tray = TrayIconBuilder::new()
        .with_icon(icon)
        .with_tooltip("winletmeknow")
        .with_menu(Box::new(menu))
        .build()
        .map_err(|e| e.to_string())?;
    unsafe {
        SetTimer(tray.window_handle(), 1, 100, None);
    }
    loop {
        while let Ok(ev) = MenuEvent::receiver().try_recv() {
            if ev.id() == exit_item.id() {
                quit.store(true, Ordering::Relaxed);
            }
        }
        if quit.load(Ordering::Relaxed) {
            break;
        }
        let mut msg = MSG::default();
        if unsafe { GetMessageW(&mut msg, null_mut(), 0, 0) } == 0 {
            break;
        }
        unsafe { DispatchMessageW(&msg); }
    }
    drop(tray);
    Ok(())
}

/// トレイアイコンを作る（32x32 円形、プリセット色）
fn build_tray_icon(cfg: &Config) -> Result<Icon, String> {
    let color = cfg.preset(None).color.unwrap_or([128, 90, 180]);
    let mut rgba: Vec<u8> = Vec::with_capacity(32 * 32 * 4);
    for y in 0..32usize {
        for x in 0..32usize {
            let dx = x as i32 - 15;
            let dy = y as i32 - 15;
            if dx * dx + dy * dy <= 15 * 15 + 1 {
                rgba.extend_from_slice(&[color[0], color[1], color[2], 255]);
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }
    Icon::from_rgba(rgba, 32, 32).map_err(|e| e.to_string())
}
