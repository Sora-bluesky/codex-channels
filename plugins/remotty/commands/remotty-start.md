# /remotty-start

Start the local bridge for this repo.

## Workflow

1. Work from the package or repo root that contains `bridge.toml`.
2. Prefer the Windows service when it is already installed:
   `remotty service start`
3. Otherwise run the foreground bridge with `remotty --config bridge.toml`.
4. If you use the foreground path, make it clear that the command occupies that terminal until the bridge stops.
5. Confirm whether the bridge is running in the foreground or as a service.

If the `remotty` command is unavailable in a source checkout, fall back to
`cargo run -- service start` or `cargo run -- --config bridge.toml`.

## Output requirements

- State which start path you used.
- If startup fails, report the blocking error clearly.
