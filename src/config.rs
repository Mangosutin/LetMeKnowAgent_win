//! 設定の読み込み。exe と同じディレクトリにある `config.json` を使う。
//! 存在しなければデフォルト値で `config.json` を自動生成する。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// 送信者プリセット: トーストに表示するアイコンと名前の組み合わせ。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    /// トーストに表示する名前（送信者）
    pub name: String,
    /// トーストのappLogoOverrideに使うPNGのパス（exeからの相対パス可）
    #[serde(default)]
    pub icon: Option<PathBuf>,
    /// トレイアイコンの色（R,G,B）
    #[serde(default)]
    pub color: Option<[u8; 3]>,
}

impl Preset {
    fn builtin(key: &str) -> Preset {
        let (name, icon, color) = match key {
            "codex" => ("Codex CLI", "assets/codex.png", [30u8, 140, 170]),
            "aider" => ("Aider", "assets/aider.png", [90, 150, 110]),
            _ => ("Claude Code", "assets/claude.png", [128, 90, 180]),
        };
        Preset {
            name: name.to_string(),
            icon: Some(PathBuf::from(icon)),
            color: Some(color),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_aumid")]
    pub aumid: String,
    #[serde(default)]
    pub default_preset: Option<String>,
    #[serde(default)]
    pub presets: HashMap<String, Preset>,
}

fn default_port() -> u16 {
    37821
}

fn default_aumid() -> String {
    "Mango.winLetmeknow".to_string()
}

impl Config {
    /// exeと同じディレクトリの設定ファイルパス
    pub fn path() -> PathBuf {
        exe_dir().join("config.json")
    }

    /// 設定ファイルの読み込み。存在しなければデフォルト値で作成する。
    pub fn load() -> Result<Config, String> {
        let path = Self::path();
        if path.exists() {
            let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
        } else {
            let cfg = Config::default_config();
            if let Err(e) = std::fs::write(&path, serde_json::to_string_pretty(&cfg).unwrap()) {
                eprintln!("警告: {} を作成できません: {e}", path.display());
            }
            Ok(cfg)
        }
    }

    pub fn default_config() -> Config {
        Config {
            port: default_port(),
            aumid: default_aumid(),
            default_preset: Some("claude".to_string()),
            presets: builtin_presets(),
        }
    }

    /// プリセットを解決する。`name` 指定がなければ `default_preset` を、
    /// それもなければ "claude" を使う。
    pub fn preset(&self, name: Option<&str>) -> Preset {
        let key = name
            .map(str::to_string)
            .or_else(|| self.default_preset.clone())
            .unwrap_or_else(|| "claude".to_string());
        self.presets
            .get(key.as_str())
            .cloned()
            .unwrap_or_else(|| Preset::builtin("claude"))
    }

    /// プリセットのトーストアイコンパス（絶対パス。ファイルが無ければ None）
    pub fn preset_icon(&self, name: Option<&str>) -> Option<PathBuf> {
        let icon = self.preset(name).icon?;
        let path = if icon.is_absolute() { icon } else { exe_dir().join(icon) };
        path.exists().then_some(path)
    }
}

fn builtin_presets() -> HashMap<String, Preset> {
    ["claude", "codex", "aider"]
        .iter()
        .map(|k| ((*k).to_string(), Preset::builtin(k)))
        .collect()
}

/// exeのあるディレクトリ
pub fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_default()
}
