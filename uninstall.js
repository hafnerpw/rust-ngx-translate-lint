#!/usr/bin/env node

const fs = require("fs");
const path = require("path");
const os = require("os");

const cargoDir = path.join(os.homedir(), ".cargo", "bin");
const binaryName = process.platform === "win32" ? "ngx_translate_lint_rs.exe" : "ngx_translate_lint_rs";
const binaryPath = path.join(cargoDir, binaryName);

if (fs.existsSync(binaryPath)) {
  try {
    fs.unlinkSync(binaryPath);
    console.log("✓ Uninstalled rust-ngx-translate-lint");
  } catch (error) {
    console.error(`Failed to uninstall: ${error.message}`);
  }
} else {
  console.log("rust-ngx-translate-lint not found, nothing to uninstall.");
}
    
    