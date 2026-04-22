[English](README.md) | [日本語](README.ja.md)

# `remotty`

![remotty: Codex と Telegram をつなぐ Windows ブリッジ](docs/assets/hero.png)

`remotty` は、Telegram から手元の Codex へ依頼する Windows 向けブリッジです。

`v0.2` の主な使い方は、保存済みの Codex スレッドへ戻ることです。
Telegram チャットをスレッドへ対応付けます。
その後、Telegram から送った文を `codex app-server` 経由で渡します。
返答は同じ Telegram チャットへ戻ります。

`remotty` は、開いている Codex App 画面へキー入力しません。
ローカルの `codex` コマンドが持つ保存済みスレッドの経路を使います。

> [!WARNING]
> **免責**
>
> 本プロジェクトは、OpenAI の支援、承認、提携を受けていません。
> `Codex`、`ChatGPT`、関連する名称は OpenAI の商標です。
> ここでは、連携先のローカルツールを説明する目的でのみ使っています。
> その他の商標は、それぞれの権利者に帰属します。

## できること

- Windows 上の Codex と Telegram bot をつなぐ
- Telegram チャットから保存済み Codex スレッドを選ぶ
- Telegram の文を `codex app-server` で選択済みスレッドへ渡す
- Codex の返答を同じ Telegram チャットへ返す
- 承認待ちを Telegram へ中継する
- bot token を Windows の保護領域へ保存する
- 実行時の状態を `%APPDATA%\remotty` に置く

従来の `exec` 経路も残っています。
これは別の `codex exec` 実行を始めるため、保存済みスレッドへ戻りません。

## 使う場面

Windows PC から離れている時に、Telegram からローカルの Codex スレッドを続けたい場合に使います。

Codex Remote connections は、プロジェクトが SSH 先にある時の機能です。
Codex App をリモートマシンへ接続します。
`remotty` は、Windows PC 上の Codex へ Telegram から届く入口です。

## 必要なもの

- Windows 10 または Windows 11
- Codex App と Codex CLI
- Node.js と `npm`
- `@BotFather` で作った Telegram bot token

ソースからビルドする場合だけ、Rust が必要です。

## はじめ方

Telegram の設定を一本道で進める場合は、
[Telegram クイックスタート](docs/telegram-quickstart.ja.md) を見てください。

Telegram bot を作る前に試す場合は、
[Fakechat デモ](docs/fakechat-demo.ja.md) を使えます。

### 1. `remotty` を入れる

```powershell
npm install -g remotty
```

このコマンドで `remotty` が入ります。
同じ版の Windows 用バイナリも取得されます。

### 2. パッケージフォルダを開く

```powershell
$remottyRoot = Join-Path (npm root -g) "remotty"
Set-Location $remottyRoot
```

ローカルプラグインを入れる時は、このフォルダを Codex App で開きます。

### 3. 設定ファイルをコピーする

```powershell
$configDir = Join-Path $env:APPDATA "remotty"
New-Item -ItemType Directory -Force -Path $configDir | Out-Null
Copy-Item -Force .\bridge.toml (Join-Path $configDir "bridge.toml")
$configPath = Join-Path $configDir "bridge.toml"
```

設定と実行時の状態は `%APPDATA%\remotty` に置かれます。

### 4. ローカルプラグインを入れる

Codex App で、インストール済みの `remotty` フォルダを開きます。
Plugins 画面で `.agents/plugins/marketplace.json` を追加します。
次に、`remotty` というプラグインを入れます。

プラグインでは、次のようなコマンドを使えます。

```text
/remotty-configure
/remotty-start
/remotty-access-pair <code>
/remotty-sessions
```

### 5. Telegram の token を保存する

`@BotFather` で bot を作り、次を実行します。

```text
/remotty-configure
```

表示に従って token を貼ります。
このコマンドは token を再表示せず、Windows の保護領域へ保存します。

### 6. 経路を選ぶ

`%APPDATA%\remotty\bridge.toml` を編集します。

保存済みスレッドへ戻る場合は、次にします。

```toml
[codex]
transport = "app_server"
```

`workspaces[0].path` と `workspaces[0].writable_roots` には、
Codex が使うプロジェクトフォルダを指定します。

別実行の従来経路だけを使う場合は、次のままにします。

```toml
[codex]
transport = "exec"
```

### 7. ブリッジを起動する

```text
/remotty-start
```

Telegram から使う間は、ブリッジを起動したままにします。

### 8. Telegram をペアリングする

Telegram で bot へ任意のメッセージを送ります。
bot はペアリングコードを返します。

Codex 側で次を実行します。

```text
/remotty-access-pair <code>
/remotty-policy-allowlist
```

許可済みの送信者だけが、依頼と承認操作を送れます。

### 9. 保存済みスレッドを選ぶ

保存済み Codex スレッドを一覧します。

```text
/remotty-sessions
```

Telegram チャットへスレッドを対応付けます。

```text
/remotty-sessions <thread_id>
```

以後の Telegram メッセージは、選択した保存済みスレッドへ届きます。

## Telegram の主なコマンド

```text
/help
/status
/stop
/approve <request_id>
/deny <request_id>
/workspace
/workspace <id>
/remotty-sessions
/remotty-sessions <thread_id>
/mode completion_checks
/mode infinite
/mode max_turns 3
```

`codex.transport = "app_server"` の時は、承認待ちが Telegram ボタンにも出ます。

## `v0.1` からの移行

`v0.1` の主な設定は `codex.transport = "exec"` でした。
この経路では、Telegram の依頼ごとに別の Codex 実行を始めます。

保存済み Codex スレッドへ戻りたい場合は、
`codex.transport = "app_server"` を使ってください。

詳しくは [v0.1 から v0.2 への移行](docs/migration-v0.1-to-v0.2.ja.md) を見てください。

## 安全な情報の扱い

- `/remotty-configure` で bot token を保護領域へ保存する
- `remotty` 専用の Telegram bot を使う
- token や `api.telegram.org/bot...` の URL を issue へ貼らない
- プロジェクトファイルと `%APPDATA%\remotty` の状態を分ける

## 関連ドキュメント

- [Telegram クイックスタート](docs/telegram-quickstart.ja.md)
- [Fakechat デモ](docs/fakechat-demo.ja.md)
- [v0.1 から v0.2 への移行](docs/migration-v0.1-to-v0.2.ja.md)
- [開発者向け情報](docs/development.ja.md)

## ライセンス

[MIT](LICENSE)
