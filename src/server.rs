//! 127.0.0.1 ループバックHTTPサーバー（自前実装、フレームワークなし）。
//!
//! - `POST /notify`   : JSON通知 → トーストスレッドへ
//! - `POST /shutdown` : サーバー停止フラグを立てる
//! - `GET  /health`   : "ok"
//!
//! その他は 404。`Content-Length` 対応の最小HTTP/1.1パーサ。

use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

use crate::client::NotificationRequest;
use crate::config::Config;
use crate::notification::Notification;

/// サーバーを起動する。`quit` が true になったらループを抜けて終了する。
pub fn run(listener: TcpListener, cfg: &Config, tx: mpsc::Sender<Notification>, quit: Arc<AtomicBool>) {
    let _ = listener.set_nonblocking(true);
    loop {
        if quit.load(Ordering::Relaxed) {
            return;
        }
        match listener.accept() {
            Ok((stream, _)) => {
                let tx = tx.clone();
                let cfg = cfg.clone();
                let quit = quit.clone();
                thread::spawn(move || handle(stream, &cfg, tx, quit));
            }
            Err(e) => {
                if e.kind() != ErrorKind::WouldBlock {
                    eprintln!("accept エラー: {e}");
                }
                thread::sleep(Duration::from_millis(20));
            }
        }
    }
}

struct Request {
    method: String,
    path: String,
    body: Vec<u8>,
}

fn handle(mut stream: TcpStream, cfg: &Config, tx: mpsc::Sender<Notification>, quit: Arc<AtomicBool>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let req = match read_request(&mut stream) {
        Ok(r) => r,
        Err(_) => return,
    };
    let (status, body): (&str, String) = match (req.method.as_str(), req.path.as_str()) {
        ("POST", "/notify") => match serde_json::from_slice::<NotificationRequest>(&req.body) {
            Ok(r) => {
                let notif = r.into_notification(cfg);
                if tx.send(notif).is_ok() {
                    ("200 OK", "{\"status\":\"ok\"}".to_string())
                } else {
                    ("500 Internal Server Error", "{\"status\":\"send-failed\"}".to_string())
                }
            }
            Err(_) => ("400 Bad Request", "{\"status\":\"bad-json\"}".to_string()),
        },
        ("POST", "/shutdown") => {
            quit.store(true, Ordering::Relaxed);
            ("200 OK", "{\"status\":\"shutdown\"}".to_string())
        }
        ("GET", "/health") => ("200 OK", "ok".to_string()),
        _ => ("404 Not Found", "{\"status\":\"not-found\"}".to_string()),
    };
    let resp = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(resp.as_bytes());
}

/// 小さいHTTPリクエスト（Request-Line + ヘッダー + Content-Lengthで読むボディ）を読む
fn read_request(stream: &mut TcpStream) -> std::io::Result<Request> {
    let mut buf: Vec<u8> = Vec::with_capacity(2048);
    let mut chunk = [0u8; 2048];
    let header_end;
    loop {
        if let Some(pos) = find(&buf, b"\r\n\r\n") {
            header_end = pos + 4;
            break;
        }
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            return Err(std::io::Error::new(ErrorKind::UnexpectedEof, "eof before headers"));
        }
        buf.extend_from_slice(&chunk[..n]);
        if buf.len() > 65536 {
            return Err(std::io::Error::new(ErrorKind::InvalidData, "headers too large"));
        }
    }
    let head = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let mut lines = head.split("\r\n");
    let request_line = lines.next().unwrap_or_default().to_string();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts
        .next()
        .unwrap_or_default()
        .split('?')
        .next()
        .unwrap_or_default()
        .to_string();
    let mut content_length = 0usize;
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            if k.trim().eq_ignore_ascii_case("content-length") {
                content_length = v.trim().parse().unwrap_or(0);
            }
        }
    }
    let mut body = buf[header_end..].to_vec();
    while body.len() < content_length {
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
    }
    body.truncate(content_length);
    Ok(Request { method, path, body })
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}
