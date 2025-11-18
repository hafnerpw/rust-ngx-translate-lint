#!/usr/bin/env node

const fs = require("fs");
const path = require("path");
const os = require("os");

// Detect platform and architecture
function getPlatform() {
  const platform = os.platform();
  const arch = os.arch();

  const platformMap = {
    "linux-x64": "linux-x64",
    "linux-arm64": "linux-arm64",
    "darwin-x64": "darwin-x64",
    "darwin-arm64": "darwin-arm64",
    "win32-x64": "win32-x64",
  };

  const key = `${platform}-${arch}`;
  return platformMap[key];
}

// Install pre-built binary
function installBinary() {
  const platform = getPlatform();

  if (!platform) {
    console.error(`ERROR: Unsupported platform ${os.platform()}-${os.arch()}`);
    console.error("Supported platforms: linux-x64, linux-arm64, darwin-x64, darwin-arm64, win32-x64");
    process.exit(1);
  }

  const binaryName = platform.startsWith("win32")
    ? "ngx_translate_lint_rs-win32-x64.exe"
    : `ngx_translate_lint_rs-${platform}`;

  const sourcePath = path.join(__dirname, "bin", binaryName);
  const cargoDir = path.join(os.homedir(), ".cargo", "bin");
  const targetName = platform.startsWith("win32") ? "ngx_translate_lint_rs.exe" : "ngx_translate_lint_rs";
  const targetPath = path.join(cargoDir, targetName);

  // Check if binary exists in package
  if (!fs.existsSync(sourcePath)) {
    console.error(`ERROR: Binary not found: ${sourcePath}`);
    console.error("This might be a corrupted installation. Please try reinstalling.");
    process.exit(1);
  }

  // Create .cargo/bin directory if it doesn't exist
  if (!fs.existsSync(cargoDir)) {
    fs.mkdirSync(cargoDir, { recursive: true });
  }

  // Copy binary to cargo bin directory
  try {
    fs.copyFileSync(sourcePath, targetPath);
    fs.chmodSync(targetPath, 0o755); // Make executable
    console.log(`✓ Installed ngx_translate_lint_rs for ${platform}`);
    console.log(`  Binary location: ${targetPath}`);
  } catch (error) {
    console.error(`ERROR: Failed to install binary: ${error.message}`);
    process.exit(1);
  }
}

console.log("Installing rust-ngx-translate-lint...");
installBinary();
