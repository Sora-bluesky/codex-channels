# /remotty-stop

Stop the local bridge for this repo.

## Workflow

1. Work from the package or repo root that contains `bridge.toml`.
2. If the Windows service is installed and running, run `remotty service stop`.
3. Otherwise explain that the foreground process must be stopped in its own terminal.

If the `remotty` command is unavailable in a source checkout, fall back to
`cargo run -- service stop`.

## Output requirements

- State whether the service was stopped.
- If only a foreground process exists, say so clearly.
