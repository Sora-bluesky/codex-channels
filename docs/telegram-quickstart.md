# Telegram Quickstart

This guide sets up `remotty` so Telegram can send messages to a Codex thread
on your Windows PC.

## How It Works

1. You run `remotty` on your Windows PC.
2. You send a message to your Telegram bot.
3. `remotty` sends that message to the Codex thread you selected.
4. Codex replies, and `remotty` sends the reply back to Telegram.

## What You Need

- Windows 10 or Windows 11
- Codex App and Codex CLI
- Node.js and `npm`
- Telegram
- A dedicated Telegram bot from `@BotFather`

## 1. Install `remotty`

Run this in PowerShell:

```powershell
npm install -g remotty
```

Find the installed package folder:

```powershell
$remottyRoot = Join-Path (npm root -g) "remotty"
$configPath = Join-Path $env:APPDATA "remotty\bridge.toml"
```

The PowerShell examples below reuse `$configPath`.

## 2. Open or Enter Your Project

Use the project you want to continue from Telegram.
You do not need to use the same project every time.

If you use Codex App, open that project in the app.

If you use Codex CLI, enter the project folder in PowerShell:

```powershell
Set-Location C:\path\to\your\project
```

## 3. Install the Local Plugin

Codex App users can use the local plugin.

In the Codex App Plugins view:

1. Add `.agents/plugins/marketplace.json` from the `$remottyRoot` folder.
2. Install the plugin named `remotty`.
3. Confirm that `remotty` appears in the Plugins view.

Codex CLI users can skip this step.
Use the PowerShell commands shown below instead.

## 4. Register This Project

Codex App users run:

```text
/remotty-use-this-project
```

Codex CLI users run this from the project folder:

```powershell
remotty config workspace upsert --config $configPath --path (Get-Location).Path
```

This saves the project to the config under `%APPDATA%\remotty`.
It does not write files into your project repository.

## 5. Create a Telegram Bot

1. Open `@BotFather` in Telegram.
2. Send `/newbot`.
3. Choose a display name.
4. Choose a username ending in `bot`.
5. Copy the token that BotFather returns.

Do not post the token in chat, screenshots, issues, or pull requests.

## 6. Store the Bot Token

Codex App users run:

```text
/remotty-configure
```

Codex CLI users run:

```powershell
remotty telegram configure --config $configPath
```

Paste the token when prompted.
The command stores it in Windows protected storage.
It does not print the token back.

## 7. Start the Bridge

Codex App users run:

```text
/remotty-start
```

Codex CLI users run:

```powershell
# Start the foreground bridge.
remotty --config $configPath
```

Keep the bridge running while you use Telegram.
If it stops, the bot cannot reply.

## 8. Pair Telegram

Send any message to your bot in a private Telegram chat.

The bot replies with a `remotty pairing code`.
Codex App users run:

```text
/remotty-access-pair <code>
```

Codex CLI users run:

```powershell
remotty telegram access-pair <code> --config $configPath
```

Then lock access to your allowlist:

```text
/remotty-policy-allowlist
```

Codex CLI users run:

```powershell
remotty telegram policy allowlist --config $configPath
```

This prevents other Telegram users from controlling your local Codex setup.

## 9. Select a Codex Thread

Codex App users run:

```text
/remotty-sessions
```

Codex CLI users run:

```powershell
remotty telegram sessions --config $configPath
```

Choose the thread you want Telegram to continue.
Then send this in the target Telegram chat:

```text
/remotty-sessions <thread_id>
```

This binding is stored under `%APPDATA%\remotty`.
It is not written into your project repository.

## 10. Send a Test Message

In Telegram, send:

```text
Summarize the current thread and suggest the next step.
```

`remotty` sends the text to the selected Codex thread.
The reply appears in Telegram.

## Approval Prompts

When Codex asks for approval, `remotty` posts the prompt to Telegram.

You can press `Approve` or `Deny`.
You can also type:

```text
/approve <request_id>
/deny <request_id>
```

The decision is returned to the same Codex turn.

## Troubleshooting

### The Bot Does Not Reply

- Confirm `/remotty-start` is still running.
- In Codex App, run `/remotty-status`.
- In Codex App, run `/remotty-live-env-check`.
- In PowerShell, run `remotty service status`.
- In PowerShell, run `remotty telegram live-env-check --config $configPath`.
- If the webhook status is `webhook-configured`, switch the bot back to polling.

### No Codex Threads Appear

- Update Codex CLI, then try again.
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

## Related

- [Fakechat Demo](fakechat-demo.md)
- [Advanced CLI Mode](exec-transport.md)
- [Upgrade Notes](upgrading.md)

Note: if your code and shell live on an SSH host, Codex Remote connections may
also be useful. `remotty` is for returning to Codex work on your Windows PC
from Telegram.
