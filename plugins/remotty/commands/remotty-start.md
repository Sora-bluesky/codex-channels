# /remotty-start

Start the local bridge for this repo.

## Workflow

1. Work from the repo root.
2. Prefer the Windows service when it is already installed:
   `cargo run -- service start`
3. Otherwise run the foreground bridge with `cargo run`.
4. If you use the foreground path, make it clear that the command occupies that terminal until the bridge stops.
5. Confirm whether the bridge is running in the foreground or as a service.

## Output requirements

- State which start path you used.
- If startup fails, report the blocking error clearly.
