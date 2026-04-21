# /remotty-policy-allowlist

Show the active Telegram allowlist for this repo.

## Workflow

1. Work from the package or repo root that contains `bridge.toml`.
2. Run `remotty telegram policy allowlist --config bridge.toml`.
3. Summarize the allowed sender IDs.

If the `remotty` command is unavailable in a source checkout, fall back to
`cargo run -- telegram policy allowlist --config bridge.toml`.

## Output requirements

- State that allowlist mode is enforced.
- Show the allowed sender IDs only.
- If no senders are allowed yet, recommend `/remotty-pair`.
