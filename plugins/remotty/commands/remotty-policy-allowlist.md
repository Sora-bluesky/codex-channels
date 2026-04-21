# /remotty-policy-allowlist

Show the active Telegram allowlist for this repo.

## Workflow

1. Work from the repo root.
2. Run `cargo run -- telegram policy allowlist --config bridge.toml`.
3. Summarize the allowed sender IDs.

## Output requirements

- State that allowlist mode is enforced.
- Show the allowed sender IDs only.
- If no senders are allowed yet, recommend `/remotty-pair`.
