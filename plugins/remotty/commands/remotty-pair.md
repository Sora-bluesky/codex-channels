# /remotty-pair

Pair the latest Telegram sender with the local allowlist for this repo.

## Workflow

1. Confirm the bridge is not already running. If it is running, stop it before pairing.
2. Run `remotty telegram pair --config bridge.toml`.
3. Read the one-time pairing code shown in the local terminal.
4. Ask the user to send `/pair <code>` to the bot from Telegram.
5. Wait for the local terminal to show the matched `sender_id` and `chat_id`.
6. Confirm that the sender was added to the allowlist.

If the `remotty` command is unavailable in a source checkout, fall back to
`cargo run -- telegram pair --config bridge.toml`.

## Output requirements

- Report the paired sender ID after success.
- Do not print the bot token.
- If pairing fails because another poller is active, tell the user to stop the running bridge and retry.
- If no matching Telegram message exists, tell the user to send `/pair <code>` and retry.
