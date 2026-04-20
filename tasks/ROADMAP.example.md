# ロードマップ

> planning backlog から自動生成 — 手動編集禁止
> 最終同期: 2026-04-20 16:06 (+09:00)

## バージョン概要

| バージョン | タスク数 | 進捗 |
|-----------|---------|------|
| v0.1.0 | 1 | [====================] 100% (1/1) |
| v0.1.1 | 1 | [====================] 100% (1/1) |
| v0.1.2 | 1 | [====================] 100% (1/1) |
| v0.1.3 | 1 | [====================] 100% (1/1) |
| v0.1.4 | 1 | [====================] 100% (1/1) |
| v0.1.5 | 1 | [====================] 100% (1/1) |
| v0.1.6 | 1 | [====================] 100% (1/1) |
| v0.1.7 | 1 | [====================] 100% (1/1) |
| v0.1.8 | 1 | [====================] 100% (1/1) |
| v0.1.9 | 3 | [====================] 100% (3/3) |
| v0.1.10 | 4 | [====================] 100% (4/4) |
| v0.1.11 | 3 | [=============-------] 67% (2/3) |
| v0.1.12 | 5 | [========------------] 40% (2/5) |
| v0.1.13 | 3 | [=============-------] 67% (2/3) |

## タスク詳細

### v0.1.0: 基盤の立ち上げ

| | ID | Title | Priority | Repo | Status |
|-|-----|-------|----------|------|--------|
| [x] | TASK-001 | ブリッジ基盤を作成 | P0 | codex-channels | done |

### v0.1.1: 確認フローの追加

| | ID | Title | Priority | Repo | Status |
|-|-----|-------|----------|------|--------|
| [x] | TASK-002 | 確認フローと添付処理を追加 | P0 | codex-channels | done |

### v0.1.2: 運用コマンドの追加

| | ID | Title | Priority | Repo | Status |
|-|-----|-------|----------|------|--------|
| [x] | TASK-003 | Telegram 制御コマンドとサービス管理を追加 | P0 | codex-channels | done |

### v0.1.3: 計画の初期化

| | ID | Title | Priority | Repo | Status |
|-|-----|-------|----------|------|--------|
| [x] | TASK-004 | 計画の初期化と公開面の保護を追加 | P1 | codex-channels | done |

### v0.1.4: 計画の堅牢化

| | ID | Title | Priority | Repo | Status |
|-|-----|-------|----------|------|--------|
| [x] | TASK-005 | 計画検証と探索の堅牢化を追加 | P1 | codex-channels | done |

### v0.1.5: 自動継続の追加

| | ID | Title | Priority | Repo | Status |
|-|-----|-------|----------|------|--------|
| [x] | TASK-006 | 自動継続モードを追加 | P1 | codex-channels | done |

### v0.1.6: 実機スモークの追加

| | ID | Title | Priority | Repo | Status |
|-|-----|-------|----------|------|--------|
| [x] | TASK-007 | 実機スモークと optional profile を追加 | P1 | codex-channels | done |

### v0.1.7: ワークスペース切り替え

| | ID | Title | Priority | Repo | Status |
|-|-----|-------|----------|------|--------|
| [x] | TASK-008 | 会話ごとのワークスペース切り替えを追加 | P0 | codex-channels | done |

### v0.1.8: リリース自動化

| | ID | Title | Priority | Repo | Status |
|-|-----|-------|----------|------|--------|
| [x] | TASK-009 | リリース自動化と履歴 release を追加 | P0 | codex-channels | done |

### v0.1.9: 承認通知の追加

| | ID | Title | Priority | Repo | Status |
|-|-----|-------|----------|------|--------|
| [x] | TASK-010 | app-server の承認要求を保存し、要約を作る | P0 | codex-channels | done |
| [x] | TASK-011 | 要求 ID つきの承認通知を Telegram へ送る | P0 | codex-channels | done |
| [x] | TASK-012 | 承認待ち件数を lane 状態と /status へ反映する | P0 | codex-channels | done |

### v0.1.10: 承認操作の追加

| | ID | Title | Priority | Repo | Status |
|-|-----|-------|----------|------|--------|
| [x] | TASK-013 | 承認操作向けの callback query を解釈する | P0 | codex-channels | done |
| [x] | TASK-014 | /approve と /deny を予備操作として追加する | P0 | codex-channels | done |
| [x] | TASK-015 | 重複した承認操作を安全に無視する | P0 | codex-channels | done |
| [x] | TASK-016 | 権限のない送信者の承認操作を拒否する | P0 | codex-channels | done |

### v0.1.11: 承認再開の追加

| | ID | Title | Priority | Repo | Status |
|-|-----|-------|----------|------|--------|
| [x] | TASK-017 | 承認結果を原子的な状態遷移で保存する | P0 | codex-channels | done |
| [x] | TASK-018 | 承認後に app-server のターンを再開する | P0 | codex-channels | done |
| [-] | TASK-019 | 再起動後に未解決の承認要求を復元する | P0 | codex-channels | active |

### v0.1.12: 承認体験の堅牢化

| | ID | Title | Priority | Repo | Status |
|-|-----|-------|----------|------|--------|
| [x] | TASK-020 | コマンド系とファイル系で承認要約を出し分ける | P1 | codex-channels | done |
| [x] | TASK-021 | 期限切れの承認を失効させ、古い button を外す | P1 | codex-channels | done |
| [-] | TASK-022 | 通知の部分失敗でも承認履歴を保持する | P1 | codex-channels | active |
| [ ] | TASK-023 | 承認結果ごとに callback の文言を出し分ける | P1 | codex-channels | backlog |
| [ ] | TASK-024 | 承認決定経路の非同期テストを直接追加する | P1 | codex-channels | backlog |

### v0.1.13: 承認フローの実機確認

| | ID | Title | Priority | Repo | Status |
|-|-----|-------|----------|------|--------|
| [x] | TASK-025 | 承認成功の実機 E2E を追加する | P1 | codex-channels | done |
| [x] | TASK-026 | 非承認の実機 E2E を追加する | P1 | codex-channels | done |
| [ ] | TASK-027 | Telegram と Codex で承認フローの実機確認を行う | P1 | codex-channels | backlog |

## 凡例

| 記号 | 意味 |
|------|------|
| [x] | 完了 |
| [-] | 作業中 |
| [R] | レビュー中 |
| [ ] | 未着手 |

| 優先度 | 意味 |
|--------|------|
| P0 | 最重要 |
| P1 | 高 |
| P2 | 中 |
| P3 | 低 |
