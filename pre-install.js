#!/usr/bin/env node

const fs = require("fs");
const path = require("path");
const { exec } = require("child_process");
const { homedir } = require("os");

const cargoDir = path.join(homedir(), ".cargo");

// check if directory exists
if (fs.existsSync(cargoDir)) {
  console.log("Cargo found.");
} else {
  console.error("ERROR: Rust toolchain not found!");
  console.error("Please install Rust from https://rustup.rs/");
  process.exit(1);
}

const features = process.env.npm_config_features ? `--features ${process.env.npm_config_features.replace(",", " ")}` : ""; 

console.log(`Installing ngx_translate_lint_rs 0.1.0 ${features}...`);
exec(`cargo install ngx_translate_lint_rs --vers 0.1.0 ${features}`, (error, stdout, stderr) => {
  console.log(stdout);
  if (error || stderr) {
    console.log(error || stderr);
  } else {
    console.log(" Installation finished!");
  }
});
