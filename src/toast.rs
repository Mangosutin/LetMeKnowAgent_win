//! トースト通知の表示（`winrt_toast` 経由）。
//!
//! - text1 = タイトル
//! - text2 = 詳細
//! - text3 (attribution) = 送信者プリセット名
//! - image = プリセットアイコン（appLogoOverride + hint-crop circle）
//!
//! クリック時は detail をクリップボードへコピー（activated コールバック経由）。

use winrt_toast::content::image::{Image, ImageHintCrop, ImagePlacement};
use winrt_toast::{Scenario, Text, Toast, ToastManager, WinToastError};

use crate::logger;
use crate::notification::Notification;

/// トーストを表示する（UIスレッドではなく専用トーストスレッドで呼ばれる）
pub fn show_toast(manager: &ToastManager, notif: &Notification) {
    let mut toast = Toast::new();
    toast.text1(notif.title.clone());
    if !notif.detail.is_empty() {
        toast.text2(notif.detail.clone());
    }
    toast.text3(Text::new(notif.preset_name.clone()).as_attribution());
    if let Some(icon_path) = &notif.icon_path {
        if let Ok(img) = Image::new_local(icon_path) {
            toast.image(
                1,
                img.with_placement(ImagePlacement::AppLogoOverride)
                    .with_hint_crop(ImageHintCrop::Circle),
            );
        }
    }
    toast.tag(notif.tag.clone());
    toast.launch(notif.tag.clone());
    toast.scenario(Scenario::Reminder);
    if let Some(d) = notif.duration.clone() {
        toast.duration(d);
    }
    // クリック（activated）時に detail をクリップボードへコピー
    let detail = notif.detail.clone();
    if let Err(e) = manager.show_with_callbacks(
        &toast,
        Some(Box::new(move |_: Result<String, WinToastError>| {
            crate::clipboard::set_text(&detail);
        })),
        None,
        None,
    ) {
        logger::log_error(&format!("トースト表示に失敗しました: {e}"));
    } else {
        logger::log_info(&format!(
            "トースト表示: title='{}' preset='{}'",
            notif.title, notif.preset_name
        ));
    }
}
