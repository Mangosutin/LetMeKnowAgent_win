# winletmeknow 仕様書

AIエージェント（Claude Code、Codex CLI、Aider 等）が PowerShell / SSH から実行できる、
Windows 10/11 向けデスクトップ通知デーモンの仕様。

---

## 1. 概要

### 1.1 目的
AIエージェントからの通知を Windows トーストとして表示し、どのエージェントからの通知かを
プリセット（アイコン＋名前）で区別する。通知遅延を防ぐためトレイ常駐とし、
アイドル時のメモリ使用量を抑える。

### 1.2 構成
- **単一バイナリ** `winletmeknow.exe` が CLI（クライアント側）と常駐デーモン（サーバー側）を兼ねる
- 通常は `send` コマンドで最初のメッセージ送信時にデーモンが自動起動される
- デーモンは `DETACHED_PROCESS | CREATE_NO_WINDOW` で起動され、コンソールウィンドウを持たない（non-console）

### 1.3 前提環境
| 項目 | 内容 |
|---|---|
| OS | Windows 10 / Windows 11 |
| アーキテクチャ | x86_64-pc-windows-gnu |
| 通知API | Windows Toast Notification (`winrt_toast` / `windows-sys`) |
| トレイ | `tray-icon` + `muda` (Win32 メッセージループ) |
| 言語 | Rust (edition 2021) |

---

## 2. システム構成

### 2.1 プロセスモデル

```
+----------------------+      HTTP (127.0.0.1:37821)      +---------------------------+
|   AIエージェント        | <----------------------------------> |   winletmeknow.exe (デーモン)    |
| (PowerShell / SSH)    |      POST /notify                  |                           |
|                      |      POST /shutdown                |  +---------------------+ |
|                      |      GET  /health                  |  | トーストスレッド       | |
|                      |                                      |  | (winrt_toast)         | |
+----------------------+                                      |  +---------------------+ |
                                                              |  +---------------------+ |
                                                              |  | サーバースレッド      | |
                                                              |  | (TcpListener)         | |
                                                              |  +---------------------+ |
                                                              |  +---------------------+ |
                                                              |  | メインスレッド        | |
                                                              |  | (トレイ + MsgLoop)    | |
                                                              |  +---------------------+ |
                                                              +---------------------------+
```

### 2.2 スレッド構成（アイドル時ほぼゼロ Wake-up）

| スレッド | 役割 | 待機手段 |
|---|---|---|
| メイン | トレイアイコン + Win32 メッセージループ | `GetMessageW` ブロック |
| サーバー | 127.0.0.1 HTTP (`accept` → JSON解析 → トーストキュー) | `TcpListener::accept` ブロック（ノンブロッキング Poll） |
| トースト | `winrt_toast` でトースト表示、クリックイベント処理 | `mpsc::recv` ブロック |

`tokio` / `tauri` / Webフレームワークは使用しない。`std::net` + `windows-sys` のみ。

### 2.3 単一インスタンス制御
デーモン起動時に Named Mutex `Local\winLetmeknow_SingleInstance` を取得。並行 `send` で複数プロセスが spawn されても、1 つだけ生存し、2 番目以降は即座に終了する。

---

## 3. ファイル構成（exe 隣）

| パス | 説明 |
|---|---|
| `winletmeknow.exe` | 本体バイナリ（リリースビルド） |
| `config.json` | 設定ファイル（未存在時はデフォルト値で自動生成） |
| `daemon.log` | デーモン実行ログ（タイムスタンプ付き、追記） |
| `assets/*.png` | トーストアイコン（claude / codex / aider） |

---

## 4. 設定ファイル仕様（`config.json`）

exe と同じディレクトリに配置。未存在時はデフォルト値で自動生成される。

```json
{
  "port": 37821,
  "aumid": "Mango.winLetmeknow",
  "default_preset": "claude",
  "presets": {
    "claude": { "name": "Claude Code", "icon": "assets/claude.png", "color": [128, 90, 180] },
    "codex":  { "name": "Codex CLI",   "icon": "assets/codex.png",  "color": [30, 140, 170] },
    "aider":  { "name": "Aider",       "icon": "assets/aider.png",  "color": [90, 150, 110] }
  }
}
```

| フィールド | 型 | 必須 | 説明 |
|---|---|---|---|
| `port` | u16 | – | HTTPサーバーのポート番号（デフォルト 37821） |
| `aumid` | string | – | Windows AppUserModelID（トースト登録に使用、デフォルト `Mango.winLetmeknow`） |
| `default_preset` | string | – | `preset` 未指定時の既定プリセット名 |
| `presets` | object | – | 送信者プリセットのマップ |

### プリセット項目

| フィールド | 型 | 必須 | 説明 |
|---|---|---|---|
| `name` | string | – | トーストに表示する送信者名（attribution） |
| `icon` | string | – | トーストアイコン（`appLogoOverride`）のパス（exe からの相対パス可） |
| `color` | [u8;3] | – | トレイアイコンの色（R,G,B） |

プリセットが見つからない場合は `claude` の組み込みデフォルトが使用される。

---

## 5. 入出力インターフェース

### 5.1 CLI コマンド

#### `send`（通知送信、通常利用）

```
winletmeknow.exe send --json '<JSON>'
winletmeknow.exe send --file <パス>
winletmeknow.exe send -           # 標準入力
winletmeknow.exe send             # デフォルトは標準入力
```

#### その他

| コマンド | 説明 |
|---|---|
| `serve` | デーモンを手動起動（通常は `send` で自動起動されるため不要） |
| `status` | デーモンの起動／停止状態を表示 |
| `stop` | デーモンを停止 |
| `presets` | 設定済みプリセット一覧を表示 |

### 5.2 HTTP API（ループバック `127.0.0.1` のみ）

| メソッド | パス | 説明 |
|---|---|---|
| `POST` | `/notify` | JSON通知を送信。ボディは `send` と同じ形式 |
| `POST` | `/shutdown` | サーバーを停止 |
| `GET` | `/health` | `ok` を返す（生存確認用） |

#### PowerShell / curl 利用例

```powershell
# PowerShell (Invoke-RestMethod)
Invoke-RestMethod -Method Post `
  -Uri http://127.0.0.1:37821/notify `
  -ContentType application/json `
  -Body '{"preset":"claude","title":"ビルド完了","detail":"テスト通過","duration":"short"}'

# SSH / PowerShell の curl
curl -s http://127.0.0.1:37821/notify `
  -H "Content-Type: application/json" `
  -d '{"preset":"codex","title":"完了","detail":"テスト通過"}'
```

### 5.3 通知 JSON 形式

```json
{
  "preset": "claude",
  "title": "ビルド完了",
  "detail": "全テスト通過 (12/12)",
  "duration": "short"
}
```

| フィールド | 型 | 必須 | 説明 |
|---|---|---|---|
| `title` | string | – | トーストのタイトル（空なら `winletmeknow 通知`） |
| `detail` | string | – | トーストの詳細（クリックでクリップボードへコピー） |
| `preset` | string | – | 送信者プリセット名（未指定時は `default_preset`） |
| `duration` | string | – | `short`（デフォルト）または `long` |

---

## 6. 通知トースト表示仕様

### 6.1 トースト構成

| 要素 | 内容 | 備考 |
|---|---|---|
| `text1` | タイトル | `NotificationRequest.title` |
| `text2` | 詳細 | `NotificationRequest.detail` |
| `text3` (attribution) | 送信者プリセット名 | 画面下部に表示、どのエージェントかを区別 |
| `image` (appLogoOverride) | プリセットアイコン | PNG、64x64、円形マスク適用 |
| `tag` | 一意タグ | エポックナノ秒ベース |
| `launch` | タグと同じ値 | クリック（activated）イベント識別に使用 |
| `scenario` | `Reminder` | 画面に留まる通知（フォーカスアシスト対策） |
| `duration` | `short` / `long` | 表示時間 |

### 6.2 クリック動作
トーストをクリック（activated）すると、**`detail` を Windows クリップボード (`CF_UNICODETEXT`) にコピー** する。長い詳細も launch 文字列の長さ制限に依存せず扱える。

---

## 7. ライフサイクル

### 7.1 デーモン起動フロー（`send` 時の自動起動）

1. `127.0.0.1:port` へ TCP connect を試行
2. 成功 → そのまま JSON を送信して終了（通常パス、遅延なし）
3. 失敗 → 自分自身を `serve` 引数で `DETACHED_PROCESS | CREATE_NO_WINDOW` 起動
4. 最大約4秒（50ms間隔 × 80回）、connect をリトライしてから送信

### 7.2 停止
- `winletmeknow.exe stop`（HTTP `POST /shutdown`）
- トレイアイコン右クリック → 「終了」

---

## 8. メモリ設計・パフォーマンス目標

| 項目 | 目標 / 実測 |
|---|---|
| アイドル時 RSS | 約 10–20 MB（実測約20MB） |
| CPU（アイドル） | ポーリングなし（ブロッキング待機のみ） |
| 起動遅延（初回 `send`） | デーモン起動＋AUMID登録 約1–3秒 |
| 2回目以降 `send` | 既存デーモンへHTTP送信のみ、ほぼ即時 |

メモリ抑制の具体策：
- `tokio` / `tauri` / Webフレームワークを使用しない
- アイコンは 64x64 PNG のみを保持
- トレイは `GetMessageW` ブロック待機、HTTP は `accept` ブロック待機

---

## 9. セキュリティ・認証

| 項目 | 方針 |
|---|---|
| 待ち受けアドレス | `127.0.0.1` のみ（ループバック） |
| 認証 | なし（ローカルのみ。他マシンからは接続不可） |
| トークン | 未導入（将来オプション検討） |

---

## 10. エラーハンドリング・ログ

### 10.1 ログ出力先
exe 隣の `daemon.log`（タイムスタンプ付き、追記モード）。

### 10.2 ログ例
```
[1795248621] [INFO] デーモン起動
[1795248621] [INFO] AUMID登録: Mango.winLetmeknow (...\assets/claude.png)
[1795248621] [INFO] トースト表示: title='デバッグ通知' preset='Claude Code'
[1795248622] [ERROR] トースト表示に失敗しました: ...
```

非表示実行（non-console / DETACHED_PROCESS）でも `eprintln!` は破棄されるため、必ずファイルログ経由で記録する。

---

## 11. ビルド・配布

```powershell
# トーストロゴ PNG の再生成（必要に応じて）
python scripts/gen_icons.py

# リリースビルド
cargo build --release
```

生成物：`target\release\winletmeknow.exe`（約5MB）

---

## 12. 制限事項・今後の課題

| 項目 | 内容 |
|---|---|
| 画面表示の視覚確認 | この仕様書作成時点では、トースト自体の画面描画は Windows 側（フォーカスアシスト・通知設定）に依存 |
| ログローテーション | 未実装（`daemon.log` は無制限に追記。必要なら追加） |
| 認証（トークン） | 未導入（将来オプション） |
| 通知履歴の永続化 | 未実装（過去通知の再表示は非対応） |
| マルチユーザー | 未対応（単一ユーザー・単一セッション前提） |

---

## 13. 検証結果（実機）

| テスト | 結果 |
|---|---|
| `send --file` による通知 | ✅ `送信しました: ...` |
| デーモン自動起動 | ✅ `send` で自動起動し HTTP 200 応答 |
| `status` / `stop` | ✅ 起動中→停止中 の遷移確認 |
| トーストAPI呼び出し | ✅ `[INFO] トースト表示: ...` ログ記録 |
| デーモン停止後 | ✅ プロセス終了確認 |
| アイドルメモリ | ✅ 約20MB（WS） |

---

以上。