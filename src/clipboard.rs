//! Windowsクリップボードにテキストを設定する（CF_UNICODETEXT）。

use windows_sys::Win32::Foundation::GlobalFree;
use windows_sys::Win32::System::DataExchange::{CloseClipboard, OpenClipboard, SetClipboardData};
use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows_sys::Win32::System::Ole::CF_UNICODETEXT;

/// クリップボードにUTF-16テキストを設定する（戻り値は無視）
pub fn set_text(text: &str) {
    if text.is_empty() {
        return;
    }
    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return;
        }
        let buf: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let bytes = buf.len() * 2;
        let h = GlobalAlloc(GMEM_MOVEABLE, bytes);
        if !h.is_null() {
            let p = GlobalLock(h) as *mut u16;
            if !p.is_null() {
                std::ptr::copy_nonoverlapping(buf.as_ptr(), p, buf.len());
                GlobalUnlock(h);
                // 成功時はクリップボードがハンドルを受け取るため GlobalFree しない
                let ok = SetClipboardData(CF_UNICODETEXT as u32, h) != std::ptr::null_mut();
                if !ok {
                    let _ = GlobalFree(h);
                }
            } else {
                let _ = GlobalFree(h);
            }
        }
        CloseClipboard();
    }
}
