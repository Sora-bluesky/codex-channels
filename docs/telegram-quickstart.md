# Telegram Quickstart

This guide connects `remotty` to Telegram and binds a Telegram chat to a saved
Codex thread.

The v0.2 path uses `codex app-server`. It resumes a saved thread and sends your
Telegram message to that thread.
It does not type into the open Codex App window.

For the older separate-run path, see
[Migration From v0.1](migration-v0.1-to-v0.2.md).

## What You Need

- Windows 10 or Windows 11
- Codex App and Codex CLI
- Node.js and `npm`
- Telegram
- A dedicated Telegram bot from `@BotFather`

## 1. Install `remotty`

```powershell
npm install -g remotty
```

Open the installed package folder:

```powershell
$remottyRoot = Join-Path (npm root -g) "remotty"
Set-Location $remottyRoot
```

Copy the starter config to your user config folder:

```powershell
$configDir = Join-Path $env:APPDATA "remotty"
New-Item -ItemType Directory -Force -Path $configDir | Out-Null
Copy-Item -Force .\bridge.toml (Join-Path $configDir "bridge.toml")
$configPath = Join-Path $configDir "bridge.toml"
```

## 2. Create a Telegram Bot

1. Open `@BotFather` in Telegram.
2. Send `/newbot`.
3. Choose a display name.
4. Choose a username ending in `bot`.
5. Copy the token that BotFather returns.

Do not post the token in chat, screenshots, issues, or pull requests.

## 3. Install the Local Plugin

Open the `remotty` package folder in the Codex App.
In the Plugins view, add `.agents/plugins/marketplace.json`.
Then install the plugin named `remotty`.

Confirm that `remotty` appears in the Plugins view.

## 4. Store the Bot Token

Run:

```text
/remotty-configure
```

Paste the token when prompted. The command stores it in Windows protected
storage and does not print it back.

## 5. Configure the Saved-Thread Transport

Edit `%APPDATA%\remotty\bridge.toml`.

Set the transport:

```toml
[codex]
transport = "app_server"
```

Set the project folder:

```toml
[[workspaces]]
id = "main"
path = "C:/Users/you/Documents/project"
writable_roots = ["C:/Users/you/Documents/project"]
```

Use forward slashes in Windows paths.

The older `exec` transport starts a separate `codex exec` run. Use
`app_server` when you want to continue a saved Codex thread.

## 6. Start the Bridge

Run:

```text
/remotty-start
```

Keep the bridge running while you use Telegram. If it stops, the bot cannot
reply.

Check status:

```text
/remotty-status
```

Stop it:

```text
/remotty-stop
```

## 7. Pair Telegram

Send any message to your bot in a private Telegram chat.

The bot replies with a `remotty pairing code`. In Codex, run:

```text
/remotty-access-pair <code>
```

Then lock access to the allowlist:

```text
/remotty-policy-allowlist
```

Only allowlisted senders can send work and approval decisions.

## 8. Select a Saved Codex Thread

List saved threads:

```text
/remotty-sessions
```

Pick the thread that you want Telegram to continue.

Bind this Telegram chat to it:

```text
/remotty-sessions <thread_id>
```

The binding is stored under the configured `remotty` state directory.
It is not written into the target project repository.

## 9. Send a Test Message

In Telegram, send:

```text
Summarize the current thread and suggest the next step.
```

`remotty` resumes the selected saved thread, sends the text, and returns the
reply to Telegram.

If the project Git tree has uncommitted changes, `remotty` warns before
relaying work into that repository.

## 10. Approval Prompts

When Codex asks for approval, `remotty` posts the prompt to Telegram.

You can press `Approve` or `Deny`. You can also use:

```text
/approve <request_id>
/deny <request_id>
```

The decision is returned to the same Codex turn.

## Optional: Manual Smoke Checks

Manual smoke checks use a real Telegram bot and a local temporary workspace.

Check the environment first:

```text
/remotty-live-env-check
```

Then run:

```text
/remotty-smoke-approval-accept
/remotty-smoke-approval-decline
```

Follow the terminal guidance and press the Telegram approval button when asked.

## Troubleshooting

### The Bot Does Not Reply

- Confirm `/remotty-start` is still running.
- Run `/remotty-status`.
- Run `/remotty-live-env-check`.
- If the webhook status is `webhook-configured`, switch the bot back to polling.

### No Saved Threads Appear

- Confirm Codex CLI supports `app-server`.
- Start at least one Codex App or Codex CLI thread.
- Run `/remotty-sessions` again.

### Pairing Code Does Not Work

- Send the message in a private chat with the bot.
- Use the newest code.
- Run `/remotty-access-pair <code>` before the code expires.

### Polling Conflict

Only one process can poll the same Telegram bot.

On Windows, list likely processes:

```powershell
Get-Process remotty, codex -ErrorAction SilentlyContinue | Select-Object Id,ProcessName,Path
```

Stop the process that reads the same bot:

```powershell
Stop-Process -Id <PID>
```

## Remote Connections

Codex Remote connections connect the Codex App to an SSH development machine.
Use them when the code and shell live on a remote host.

Use `remotty` when the Codex setup is on your Windows PC and Telegram should
send work to it.
