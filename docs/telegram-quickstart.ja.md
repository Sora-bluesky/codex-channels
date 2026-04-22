# Telegram クイックスタート

この手順では、`remotty` を Telegram へつなぎます。
さらに、Telegram チャットを保存済み Codex スレッドへ対応付けます。

`v0.2` の主経路は `codex app-server` を使います。
保存済みスレッドを再開し、Telegram の文をそのスレッドへ渡します。
開いている Codex App 画面へキー入力するものではありません。

従来の別実行経路は、
[v0.1 から v0.2 への移行](migration-v0.1-to-v0.2.ja.md) を見てください。

## 必要なもの

- Windows 10 または Windows 11
- Codex App と Codex CLI
- Node.js と `npm`
- Telegram
- `@BotFather` で作った専用 bot

## 1. `remotty` を入れる

```powershell
npm install -g remotty
```

インストール先のフォルダを開きます。

```powershell
$remottyRoot = Join-Path (npm root -g) "remotty"
Set-Location $remottyRoot
```

設定ファイルをユーザー用フォルダへコピーします。

```powershell
$configDir = Join-Path $env:APPDATA "remotty"
New-Item -ItemType Directory -Force -Path $configDir | Out-Null
Copy-Item -Force .\bridge.toml (Join-Path $configDir "bridge.toml")
$configPath = Join-Path $configDir "bridge.toml"
```

## 2. Telegram bot を作る

1. Telegram で `@BotFather` を開きます。
2. `/newbot` を送ります。
3. 表示名を決めます。
4. `bot` で終わる username を決めます。
5. BotFather が返した token を控えます。

token をチャット、スクリーンショット、issue、PR に貼らないでください。

## 3. ローカルプラグインを入れる

Codex App で `remotty` のパッケージフォルダを開きます。
Plugins 画面で `.agents/plugins/marketplace.json` を追加します。
次に、`remotty` というプラグインを入れます。

Plugins 画面に `remotty` が表示されることを確認します。

## 4. bot token を保存する

次を実行します。

```text
/remotty-configure
```

表示に従って token を貼ります。
このコマンドは token を再表示せず、Windows の保護領域へ保存します。

## 5. 保存済みスレッド用の経路を設定する

`%APPDATA%\remotty\bridge.toml` を編集します。

経路を指定します。

```toml
[codex]
transport = "app_server"
```

プロジェクトフォルダを指定します。

```toml
[[workspaces]]
id = "main"
path = "C:/Users/you/Documents/project"
writable_roots = ["C:/Users/you/Documents/project"]
```

Windows のパスは `/` で書くと安全です。

従来の `exec` 経路は、別の `codex exec` 実行を始めます。
保存済み Codex スレッドを続ける場合は、`app_server` を使ってください。

## 6. ブリッジを起動する

次を実行します。

```text
/remotty-start
```

Telegram から使う間は、ブリッジを起動したままにします。
止まっていると bot は返信できません。

状態確認:

```text
/remotty-status
```

停止:

```text
/remotty-stop
```

## 7. Telegram をペアリングする

Telegram の private chat で、bot へ任意のメッセージを送ります。

bot は `remotty pairing code` を返します。
Codex 側で次を実行します。

```text
/remotty-access-pair <code>
```

次に、送信者の許可を確認します。

```text
/remotty-policy-allowlist
```

許可済みの送信者だけが、依頼と承認操作を送れます。

## 8. 保存済み Codex スレッドを選ぶ

保存済みスレッドを一覧します。

```text
/remotty-sessions
```

Telegram から続けたいスレッドを選びます。

このチャットへ対応付けます。

```text
/remotty-sessions <thread_id>
```

対応付けは `remotty` の状態フォルダへ保存します。
対象プロジェクトのリポジトリには書き込みません。

## 9. テストメッセージを送る

Telegram で次を送ります。

```text
Summarize the current thread and suggest the next step.
```

`remotty` は選択済みスレッドを再開します。
そのスレッドへ文を渡し、返答を Telegram へ戻します。

対象リポジトリに未保存の変更がある場合は、作業を渡す前に警告します。

## 10. 承認待ち

Codex が承認を求めると、`remotty` は Telegram へ中継します。

`Approve` または `Deny` を押せます。
文字コマンドも使えます。

```text
/approve <request_id>
/deny <request_id>
```

承認結果は同じ Codex の処理へ返ります。

## 任意: 手動スモーク

手動スモークは、実 Telegram bot と一時 workspace を使います。

まず環境を確認します。

```text
/remotty-live-env-check
```

次に実行します。

```text
/remotty-smoke-approval-accept
/remotty-smoke-approval-decline
```

端末の案内に従い、Telegram の承認ボタンを押してください。

## 困った時

### bot が返信しない

- `/remotty-start` が動いているか確認します。
- `/remotty-status` を実行します。
- `/remotty-live-env-check` を実行します。
- webhook 状態が `webhook-configured` なら polling へ戻します。

### 保存済みスレッドが出ない

- Codex CLI が `app-server` に対応しているか確認します。
- Codex App か Codex CLI でスレッドを作ります。
- もう一度 `/remotty-sessions` を実行します。

### pairing code が通らない

- bot との private chat で送ってください。
- 最新の code を使ってください。
- 期限切れ前に `/remotty-access-pair <code>` を実行してください。

### polling 競合が出る

同じ Telegram bot を polling できるプロセスは1つだけです。

Windows では候補を確認できます。

```powershell
Get-Process remotty, codex -ErrorAction SilentlyContinue | Select-Object Id,ProcessName,Path
```

同じ bot を読んでいるプロセスを止めます。

```powershell
Stop-Process -Id <PID>
```

## Remote connections との違い

Codex Remote connections は、Codex App を SSH 先の開発マシンへ接続します。
コードとシェルがリモートホスト上にある時に使います。

`remotty` は、Windows PC 上の Codex へ Telegram から依頼する時に使います。
