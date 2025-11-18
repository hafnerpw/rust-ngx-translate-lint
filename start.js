#!/usr/bin/env node

const { spawn } = require("child_process");

// Get arguments after 'node' and 'start.js'
const args = process.argv.slice(2);

// Spawn the Rust binary with arguments
const child = spawn("ngx-translate-lint-rs", args, {
  stdio: "inherit",
  shell: true
});

child.on("exit", (code) => {
  process.exit(code || 0);
});

process.on("SIGTERM", () => {
  child.kill("SIGTERM");
});

process.on("SIGINT", () => {
  child.kill("SIGINT");
});
