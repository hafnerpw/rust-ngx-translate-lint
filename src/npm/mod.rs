use std::{fs, path::PathBuf, process::Command};

use anyhow::{anyhow, Context, Result};
use serde::Serialize;

#[derive(Serialize)]
struct PackageJson<'a> {
    name: &'a str,
    version: &'a str,
    description: &'a str,
    main: &'a str,
    files: Vec<&'a str>,
    engines: Engines,
    scripts: Scripts,
}

#[derive(Serialize)]
struct Engines {
    node: &'static str,
}

#[derive(Serialize)]
struct Scripts {
    install: &'static str,
}

pub fn build_npm_package() -> Result<()> {
    let out_dir = PathBuf::from("node-package");
    if out_dir.exists() {
        fs::remove_dir_all(&out_dir).with_context(|| "Failed to clean existing node-package")?;
    }
    fs::create_dir_all(&out_dir)?;

    // Build napi addon
    let status = Command::new("napi")
        .arg("build")
        .status()
        .with_context(|| "Failed to run napi build")?;
    if !status.success() {
        return Err(anyhow!("napi build failed"));
    }

    let pkg = PackageJson {
        name: "ngx-translate-lint-rs",
        version: "0.1.0",
        description: "Rust-powered ngx-translate-lint",
        main: "index.js",
        files: vec!["index.js", "package.json", "README.md", "native"],
        engines: Engines { node: ">=18" },
        scripts: Scripts { install: "napi prepublish" },
    };
    let pkg_json = serde_json::to_string_pretty(&pkg)?;
    fs::write(out_dir.join("package.json"), pkg_json)?;
    fs::write(out_dir.join("index.js"), "module.exports = require('./native/index');")?;
    fs::write(out_dir.join("README.md"), "# ngx-translate-lint-rs\n")?;

    Ok(())
}
