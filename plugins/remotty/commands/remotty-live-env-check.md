# /remotty-live-env-check

Check whether the live test environment variables are present without printing their values.

## Workflow

1. Work from the repo root.
2. Run `cargo run -- telegram live-env-check`.
3. Summarize only which variables are set or missing.

## Output requirements

- Never print secret values.
- Distinguish required and optional variables.
