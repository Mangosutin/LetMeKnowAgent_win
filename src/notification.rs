//! サーバー（UI側）で扱う通知構造体。

use std::path::PathBuf;

use winrt_toast::ToastDuration;

#[derive(Debug, Clone)]
pub struct Notification {
    /// トーストのタイトル
    pub title: String,
    /// トーストの詳細
    pub detail: String,
    /// 送信者プリセット名（attributionに表示）
    pub preset_name: String,
    /// トーストアイコン（appLogoOverride、絶対パス）
    pub icon_path: Option<PathBuf>,
    /// 一意タグ（更新・トレース用）
    pub tag: String,
    /// トースト表示時間
    pub duration: Option<ToastDuration>,
}
