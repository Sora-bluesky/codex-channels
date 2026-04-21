# /remotty-status

Inspect the local bridge state for this repo.

## Workflow

1. Work from the package or repo root that contains `bridge.toml`.
2. Run `remotty service status`.
3. Also run `remotty telegram policy allowlist --config bridge.toml`.
4. Summarize the current service state and allowed senders.

If the `remotty` command is unavailable in a source checkout, fall back to
the same commands through `cargo run --`.

## Output requirements

- Report service state.
- Report whether allowlist has at least one sender.
- Keep the summary short.
