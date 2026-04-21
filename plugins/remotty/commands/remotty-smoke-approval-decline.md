# /remotty-smoke-approval-decline

Run the manual approval-decline smoke against Telegram without printing any secret values.

## Workflow

1. Work from the repo root.
2. Run `/remotty-live-env-check` first and confirm all required `LIVE_*` values are set.
3. Run `cargo run -- telegram smoke approval decline --config bridge.toml`.
4. Follow the local terminal guidance and use Telegram to press `非承認` when the pending request appears.
5. Confirm that the smoke finished with a success message and no target file was created.

## Output requirements

- Never print secret values.
- If the smoke stops on a webhook check, explain how to switch back to polling.
- If another poller is already active, tell the user to stop it before retrying.
