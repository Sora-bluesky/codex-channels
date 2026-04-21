#!/usr/bin/env node

const { spawnSync } = require("node:child_process");
const path = require("node:path");

const executable = path.join(__dirname, "remotty.exe");
const result = spawnSync(executable, process.argv.slice(2), {
  stdio: "inherit",
  windowsHide: false,
});

if (result.error) {
  console.error(`failed to start remotty: ${result.error.message}`);
  process.exit(1);
}

if (result.signal) {
  console.error(`remotty stopped by signal ${result.signal}`);
  process.exit(1);
}

process.exit(result.status ?? 0);
