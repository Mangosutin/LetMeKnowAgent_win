//! デーモン用ファイルロガー。
//! コンソールウィンドウを持たない（non-console / DETACHED_PROCESS）実行でも
//! ログを exe 隣の `daemon.log` にタイムスタンプ付きで残す。

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

static LOGGER: OnceLock<Mutex<File>> = OnceLock::new();

/// ログファイルを初期化する。`path` が書き込めない場合は無視して silent にする。
pub fn init_log(path: &Path) {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path);
    match file {
        Ok(f) => {
            let _ = LOGGER.set(Mutex::new(f));
        }
        Err(e) => {
            // 最後の手段として stderr に出力（non-console では無視される）
            let _ = writeln!(std::io::stderr(), "[winletmeknow] ログファイルオープン失敗: {e}");
        }
    }
}

/// ログ出力（info レベル）
pub fn log_info(msg: &str) {
    write_log("INFO", msg);
}

/// ログ出力（error レベル）
pub fn log_error(msg: &str) {
    write_log("ERROR", msg);
}

/// ログ出力（warn レベル）
pub fn log_warn(msg: &str) {
    write_log("WARN", msg);
}

fn write_log(level: &str, msg: &str) {
    let ts = timestamp();
    let line = format!("[{ts}] [{level}] {msg}\n");
    if let Some(logger) = LOGGER.get() {
        if let Ok(mut f) = logger.lock() {
            let _ = f.write_all(line.as_bytes());
            let _ = f.flush();
        }
    }
}

/// ログファイルパス（exe 隣の daemon.log）
pub fn log_path() -> PathBuf {
    crate::config::exe_dir().join("daemon.log")
}

/// 現在時刻を UNIX エポック秒（文字列）で取得
fn timestamp() -> String {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_secs().to_string(),
        Err(_) => "0".to_string(),
    }
}
