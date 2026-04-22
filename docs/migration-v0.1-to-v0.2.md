# Migration From v0.1 to v0.2

`remotty` v0.1 mainly used a separate-run bridge.

With `codex.transport = "exec"`, each Telegram request starts a separate
`codex exec` run. This remains supported.

`remotty` v0.2 adds the saved-thread relay path.

With `codex.transport = "app_server"`, a Telegram chat can bind to a saved
Codex thread. New Telegram messages are relayed to that thread.

## What Changes

| Area | v0.1 separate run | v0.2 saved thread |
| --- | --- | --- |
| Transport | `exec` | `app_server` |
| Codex entry point | `codex exec` | `codex app-server` |
| Thread behavior | Starts a separate run | Resumes a selected saved thread |
| Telegram selection | Workspace only | Workspace and saved thread |
| State location | `%APPDATA%\remotty` | `%APPDATA%\remotty` |

## What Does Not Change

- Telegram bot setup still uses `@BotFather`.
- Bot tokens should still be stored with `/remotty-configure`.
- Pairing still uses `/remotty-access-pair <code>`.
- Access should still be locked with `/remotty-policy-allowlist`.
- Project repositories should not contain `remotty` runtime state.

## Upgrade Steps

1. Update `remotty`.

```powershell
npm install -g remotty
```

2. Open `%APPDATA%\remotty\bridge.toml`.

3. Change the transport.

```toml
[codex]
transport = "app_server"
```

4. Start the bridge.

```text
/remotty-start
```

5. List saved threads.

```text
/remotty-sessions
```

6. Bind the Telegram chat to a thread.

```text
/remotty-sessions <thread_id>
```

## If You Want the Old Behavior

Keep:

```toml
[codex]
transport = "exec"
```

This starts a separate Codex CLI run for Telegram work. It does not resume a
saved Codex thread.

## Files and Repository Safety

`remotty` stores its own state under `%APPDATA%\remotty`.

It should not create runtime files inside the project repository that Codex is
working on. The selected project may still be edited by Codex itself, based on
your workspace settings and approvals.

When `app_server` relays work into a Git repository with uncommitted changes,
`remotty` warns before the turn starts.
