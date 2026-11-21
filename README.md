# winletmeknow — AIエージェント向け Windows 通知デーモン

AIエージェント（Claude Code、Codex CLI、Aider等）が PowerShell / SSH から実行できる、
Windows 10/11 向けデスクトップ通知サーバーです。

## 特徴

- **トレイ常駐**: `send` で最初のメッセージを送ると自動起動。アイドル時のメモリ使用量は低く（実測目安約20MB）維持します。
- **JSON通知**: `{"preset":"claude","title":"ビルド完了","detail":"全テスト通過","duration":"short"}` 形式。
- **送信者プリセット**: プリセットごとにトーストにアイコン（appLogoOverride）と名前（attribution）を表示します。
- **クリック → クリップボード**: トーストをクリックすると `detail` がクリップボードへコピーされます。
- **ローカルAPI**: `http://127.0.0.1:37821/notify` （PowerShell / curl でも直接利用可能）。
- **ループバックのみ**: 127.0.0.1 バインドのみ。他のマシンからは接続できません。
- **ファイルログ**: デーモン（`serve`）のログは exe 隣の `daemon.log` にタイムスタンプ付きで書き出されます。コンソールウィンドウを持たない非表示実行でもログが残ります。

## 使い方

### 起動（通常は自動）

```powershell
winletmeknow.exe serve          # 手動起動（通常は不要）
winletmeknow.exe stop           # 停止
winletmeknow.exe status         # 起動中 / 停止中
winletmeknow.exe presets        # プリセット一覧
```

### 通知送信

```powershell
# PowerShell で exe 経由
winletmeknow.exe send --json '{"preset":"claude","title":"ビルド完了","detail":"全テスト通過","duration":"short"}'

# PowerShell から直接 HTTP POST（Invoke-RestMethod）
Invoke-RestMethod -Method Post `
  -Uri http://127.0.0.1:37821/notify `
  -ContentType application/json `
  -Body '{"preset":"claude","title":"完了","detail":"..."}'

# SSH / PowerShell の curl
curl -s http://127.0.0.1:37821/notify `
  -H "Content-Type: application/json" `
  -d '{"preset":"codex","title":"完了","detail":"テスト通過"}'

# 標準入力
echo '{"title":"hello","detail":"world"}' | winletmeknow.exe send -
```

### JSONフィールド

| フィールド | 必須 | 説明 |
|---|---|---|
| `title` | – | トーストのタイトル（空なら `winletmeknow 通知`） |
| `detail` | – | トーストの詳細 |
| `preset` | – | 送信者プリセット（`claude`/`codex`/`aider`...）。省略時は `default_preset` |
| `duration` | – | `short`（デフォルト）/ `long` |

### トレイ

- トレイアイコン右クリック → `終了` でもサーバーを停止できます
- `winletmeknow.exe stop` でも停止できます

## 設定 (exeと同じディレクトリの config.json)

`config.example.json` を参照してください。`presets` は `name`（通知に表示する送信者名）と `icon`（トーストアイコンのPNGパス）、`color`（トレイアイコンの色）を持ちます。ポートは `port`（デフォルト 37821）で変更できます。

## ビルド・アイコン生成

```powershell
# トーストロゴ PNG の再生成
python scripts/gen_icons.py

# リリースビルド
cargo build --release
```
