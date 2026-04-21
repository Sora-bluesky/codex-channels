# /remotty-configure

Configure the local Telegram bot token for this repo without printing the token.

## Workflow

1. Work from the repo root.
2. Run `cargo run -- telegram configure --config bridge.toml`.
3. Let the command prompt for the token with hidden input.
4. Confirm that the token was stored under the configured `token_secret_ref`.

This command must run in an interactive terminal with hidden-input support.

## Output requirements

- State which `token_secret_ref` was updated.
- Do not print the token.
- If setup fails, explain the next recovery step without asking the user to paste the token.
