#!/usr/bin/env node

const { spawn } = require("child_process");
const path = require("path");
const os = require("os");

// Get arguments after 'node' and 'start.js'
const args = process.argv.slice(2);

// Construct the path to the binary
const binaryName = os.platform() === "win32" ? "ngx-translate-lint-rs.exe" : "ngx-translate-lint-rs";
const binaryPath = path.join(os.homedir(), ".cargo", "bin", binaryName);

// Spawn the Rust binary with arguments (without shell)
const child = spawn(binaryPath, args, {
  stdio: "inherit",
  shell: false
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
