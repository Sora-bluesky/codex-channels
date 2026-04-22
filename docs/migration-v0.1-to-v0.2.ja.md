# `v0.1` から `v0.2` への移行

`remotty` の `v0.1` は、主に別実行のブリッジでした。

`codex.transport = "exec"` の場合、Telegram の依頼ごとに別の
`codex exec` 実行を始めます。この経路は今後も使えます。

`remotty` の `v0.2` では、保存済みスレッドへ戻る経路を追加しました。

`codex.transport = "app_server"` の場合、Telegram チャットを保存済み
Codex スレッドへ対応付けられます。新しい Telegram メッセージは、
そのスレッドへ渡されます。

## 変わること

| 項目 | `v0.1` の別実行 | `v0.2` の保存済みスレッド |
| --- | --- | --- |
| 経路 | `exec` | `app_server` |
| Codex の入口 | `codex exec` | `codex app-server` |
| スレッド | 別実行を開始 | 選んだ保存済みスレッドを再開 |
| Telegram で選ぶ対象 | workspace | workspace と保存済みスレッド |
| 状態の保存先 | `%APPDATA%\remotty` | `%APPDATA%\remotty` |

## 変わらないこと

- Telegram bot は `@BotFather` で作ります。
- bot token は `/remotty-configure` で保存します。
- ペアリングは `/remotty-access-pair <code>` を使います。
- `/remotty-policy-allowlist` で送信者を限定します。
- プロジェクトのリポジトリへ `remotty` の状態を置きません。

## 移行手順

1. `remotty` を更新します。

```powershell
npm install -g remotty
```

2. `%APPDATA%\remotty\bridge.toml` を開きます。

3. 経路を変えます。

```toml
[codex]
transport = "app_server"
```

4. ブリッジを起動します。

```text
/remotty-start
```

5. 保存済みスレッドを一覧します。

```text
/remotty-sessions
```

6. Telegram チャットへスレッドを対応付けます。

```text
/remotty-sessions <thread_id>
```

## 従来の動きへ戻す場合

次のままにします。

```toml
[codex]
transport = "exec"
```

この場合は、Telegram の依頼ごとに別の Codex CLI 実行を始めます。
保存済み Codex スレッドは再開しません。

## ファイルとリポジトリの安全性

`remotty` は自分の状態を `%APPDATA%\remotty` に保存します。

Codex が作業するプロジェクトのリポジトリへ、
`remotty` の実行時ファイルは置きません。
ただし、Codex 自体は設定と承認に従ってプロジェクトを編集します。

`app_server` 経路で未保存の変更がある Git リポジトリへ作業を渡す場合、
`remotty` は処理前に警告します。
