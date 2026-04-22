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
```

## 2. Open Your Project

Open the project you want to continue from Telegram in the Codex App.
You do not need to use the same project every time.

## 3. Install the Local Plugin

In the Codex App Plugins view:

1. Add `.agents/plugins/marketplace.json` from the `$remottyRoot` folder.
2. Install the plugin named `remotty`.
3. Confirm that `remotty` appears in the Plugins view.

## 4. Register This Project

Run this in the Codex App:

```text
/remotty-use-this-project
```

This saves the open project to the config under `%APPDATA%\remotty`.
It does not write files into your project repository.

## 5. Create a Telegram Bot

1. Open `@BotFather` in Telegram.
2. Send `/newbot`.
3. Choose a display name.
4. Choose a username ending in `bot`.
5. Copy the token that BotFather returns.

Do not post the token in chat, screenshots, issues, or pull requests.

## 6. Store the Bot Token

Run this in the Codex App:

```text
/remotty-configure
```

Paste the token when prompted.
The command stores it in Windows protected storage.
It does not print the token back.

## 7. Start the Bridge

Run this in the Codex App:

```text
/remotty-start
```

Keep the bridge running while you use Telegram.
If it stops, the bot cannot reply.

## 8. Pair Telegram

Send any message to your bot in a private Telegram chat.

The bot replies with a `remotty pairing code`.
Run this in the Codex App:

```text
/remotty-access-pair <code>
```

Then lock access to your allowlist:

```text
/remotty-policy-allowlist
```

This prevents other Telegram users from controlling your local Codex setup.

## 9. Select a Codex Thread

Run this in the Codex App:

```text
/remotty-sessions
```

Choose the thread you want Telegram to continue.
Then bind this Telegram chat to it:

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
- Run `/remotty-status` in the Codex App.
- Run `/remotty-live-env-check` in the Codex App.
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
