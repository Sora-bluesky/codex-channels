# /remotty-use-this-project

Register the current Codex App project as the default `remotty` workspace.

## Workflow

1. Resolve the user config path: `$configPath = Join-Path $env:APPDATA "remotty\bridge.toml"`.
2. Resolve the current project path from the active shell: `$projectPath = (Get-Location).Path`.
3. Run `remotty config workspace upsert --config $configPath --path $projectPath`.
4. Confirm the saved workspace id and config path.
5. Tell the user that `remotty` state stays under `%APPDATA%\remotty`.

Only for repo contributors: if the `remotty` command is unavailable in a source checkout, fall back to
`cargo run -- config workspace upsert --config $configPath --path $projectPath`.

## Output requirements

- Do not ask the user to edit `bridge.toml`.
- State the project path that was registered.
- State that no files were written into the project repository.
- If the path cannot be resolved, ask the user to open the target project in Codex App and rerun the command.
