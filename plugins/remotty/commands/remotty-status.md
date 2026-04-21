# /remotty-status

Inspect the local bridge state for this repo.

## Workflow

1. Work from the repo root.
2. Run `cargo run -- service status`.
3. Also run `cargo run -- telegram policy allowlist --config bridge.toml`.
4. Summarize the current service state and allowed senders.

## Output requirements

- Report service state.
- Report whether allowlist has at least one sender.
- Keep the summary short.
