# /remotty-configure

Configure the local Telegram bot token for this repo without printing the token.

## Workflow

1. Work from the package or repo root that contains `bridge.toml`.
2. Run `remotty telegram configure --config bridge.toml`.
3. Let the command prompt for the token with hidden input.
4. Confirm that the token was stored under the configured `token_secret_ref`.

If the `remotty` command is unavailable in a source checkout, fall back to
`cargo run -- telegram configure --config bridge.toml`.

This command must run in an interactive terminal with hidden-input support.

## Output requirements

- State which `token_secret_ref` was updated.
- Do not print the token.
- If setup fails, explain the next recovery step without asking the user to paste the token.
