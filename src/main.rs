//! winletmeknow — AIエージェント向け Windows デスクトップ通知デーモン。
//!
//! - `serve`  : トレイ常駐サーバー（通常は `send` で自動起動される）
//! - `send`   : JSON 通知をサーバーへ送信（`--json -` なら標準入力）
//! - `status` : サーバーの状態（起動中 / 停止中）
//! - `stop`   : サーバーを停止する
//! - `presets`: プリセット一覧を表示する

mod clipboard;
mod client;
mod config;
mod logger;
mod notification;
mod server;
mod toast;
mod tray;

use clap::{Parser, Subcommand};
use config::Config;

#[derive(Parser)]
#[command(name = "winletmeknow", about = "AIエージェント向けWindowsデスクトップ通知デーモン", version, author)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// デーモン起動（通常は `send` で自動起動）
    Serve,
    /// JSON通知をサーバーへ送信（"-" なら標準入力）
    Send(SendArgs),
    /// サーバーの状態（起動中 / 停止中）
    Status,
    /// サーバーを停止する
    Stop,
    /// プリセット一覧を表示する
    Presets,
}

#[derive(clap::Args)]
struct SendArgs {
    /// JSON文字列、または "-" （標準入力）。
    /// 形式: {"preset":"claude","title":"ビルド完了","detail":"テスト通過","duration":"short"}
    #[arg(long, default_value = "-")]
    json: String,
    /// JSONファイルのパス（`--json` と排他）
    #[arg(long)]
    file: Option<std::path::PathBuf>,
}

fn main() {
    let cli = Cli::parse();
    let cfg = Config::load().unwrap_or_else(|e| {
        eprintln!("設定の読み込みに失敗しました: {e}");
        std::process::exit(1);
    });
    match cli.cmd {
        Some(Cmd::Serve) => {
            if let Err(e) = tray::run_daemon(&cfg) {
                eprintln!("サーバー起動に失敗しました: {e}");
                std::process::exit(1);
            }
        }
        Some(Cmd::Send(a)) => client::cmd_send(&cfg, &a.json, a.file.as_deref()),
        Some(Cmd::Status) => client::cmd_status(&cfg),
        Some(Cmd::Stop) => client::cmd_stop(&cfg),
        Some(Cmd::Presets) => client::cmd_presets(&cfg),
        None => client::cmd_send(&cfg, "-", None),
    }
}
