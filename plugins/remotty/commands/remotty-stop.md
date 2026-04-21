# /remotty-stop

Stop the local bridge for this repo.

## Workflow

1. Work from the repo root.
2. If the Windows service is installed and running, run `cargo run -- service stop`.
3. Otherwise explain that the foreground process must be stopped in its own terminal.

## Output requirements

- State whether the service was stopped.
- If only a foreground process exists, say so clearly.
