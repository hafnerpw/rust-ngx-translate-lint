#!/usr/bin/env node

const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");
const { homedir } = require("os");

const cargoDir = path.join(homedir(), ".cargo");

// check if directory exists
if (!fs.existsSync(cargoDir)) {
  console.error("ERROR: Rust toolchain not found!");
  console.error("Please install Rust from https://rustup.rs/");
  process.exit(1);
}

console.log("Building ngx-translate-lint-rs from source...");

try {
  // Build in release mode from the package directory
  execSync("cargo build --release --bin ngx-translate-lint-rs", { 
    stdio: "inherit",
    cwd: __dirname 
  });
  
  // Install the binary to cargo bin directory
  execSync("cargo install --path . --bin ngx-translate-lint-rs", { 
    stdio: "inherit",
    cwd: __dirname 
  });
  
  console.log(" Build and installation successful!");
} catch (error) {
  console.error(" Build failed:", error.message);
  process.exit(1);
}
