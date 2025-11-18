use std::{fs, path::{Path, PathBuf}, process::Command};

use anyhow::{anyhow, Context, Result};
use serde_json::Value;

use crate::models::LintConfig;

pub fn load_config(path: Option<&Path>) -> Result<LintConfig> {
    match path {
        None => Ok(LintConfig::default()),
        Some(file_path) => {
            let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or_default();
            match ext {
                "json" => load_json_config(file_path),
                "js" | "cjs" | "mjs" => load_js_config(file_path),
                _ => Err(anyhow!("Unsupported config extension: {}", ext)),
            }
        }
    }
}

fn load_json_config(path: &Path) -> Result<LintConfig> {
    let data = fs::read_to_string(path).with_context(|| format!("Unable to read config: {}", path.display()))?;
    let mut cfg: LintConfig = serde_json::from_str(&data).with_context(|| "Invalid JSON config")?;
    cfg.base_dir = resolve_base_dir(path);
    Ok(cfg)
}

fn load_js_config(path: &Path) -> Result<LintConfig> {
    let abs_path = fs::canonicalize(path).with_context(|| format!("Cannot resolve config path: {}", path.display()))?;
    let script = format!(
        "(async () => {{ const cfg = await import('file://{0}'); const data = cfg.default ?? cfg; console.log(JSON.stringify(data)); }})().catch(err => {{ console.error(err); process.exit(1); }});",
        abs_path.display()
    );
    let output = Command::new("node").arg("-e").arg(script).output().with_context(|| "Failed to execute node to load config")?;
    if !output.status.success() {
        return Err(anyhow!("Node reported error while loading config: {}", String::from_utf8_lossy(&output.stderr)));
    }
    let stdout = String::from_utf8(output.stdout)?;
    let json: Value = serde_json::from_str(stdout.trim()).with_context(|| "Config JS output was not valid JSON")?;
    let mut cfg: LintConfig = serde_json::from_value(json)?;
    cfg.base_dir = resolve_base_dir(path);
    Ok(cfg)
}

pub fn merge_config(base: &LintConfig, cli: &LintConfig) -> LintConfig {
    let mut merged = base.clone();
    let default_rules = crate::models::RuleConfig::default();

    if !cli.views.is_empty() { merged.views = cli.views.clone(); }
    if !cli.languages.is_empty() { merged.languages = cli.languages.clone(); }
    if !cli.ignore.is_empty() { merged.ignore = cli.ignore.clone(); }
    if cli.fix_zombies_keys { merged.fix_zombies_keys = true; }

    // merge rules
    merged.rules.custom_reg_exp_to_find_keys = if cli.rules.custom_reg_exp_to_find_keys.is_empty() {
        base.rules.custom_reg_exp_to_find_keys.clone()
    } else {
        cli.rules.custom_reg_exp_to_find_keys.clone()
    };

    if !cli.rules.ignored_keys.is_empty() { merged.rules.ignored_keys = cli.rules.ignored_keys.clone(); }
    if !cli.rules.ignored_misprint_keys.is_empty() { merged.rules.ignored_misprint_keys = cli.rules.ignored_misprint_keys.clone(); }

    if cli.rules.max_warning != 0 { merged.rules.max_warning = cli.rules.max_warning; }
    if cli.rules.misprint_coefficient != default_rules.misprint_coefficient {
        merged.rules.misprint_coefficient = cli.rules.misprint_coefficient;
    }

    if cli.rules.deep_search != default_rules.deep_search {
        merged.rules.deep_search = cli.rules.deep_search;
    }
    if cli.rules.empty_keys != default_rules.empty_keys {
        merged.rules.empty_keys = cli.rules.empty_keys;
    }
    if cli.rules.keys_on_views != default_rules.keys_on_views {
        merged.rules.keys_on_views = cli.rules.keys_on_views;
    }
    if cli.rules.zombie_keys != default_rules.zombie_keys {
        merged.rules.zombie_keys = cli.rules.zombie_keys;
    }
    if cli.rules.misprint_keys != default_rules.misprint_keys {
        merged.rules.misprint_keys = cli.rules.misprint_keys;
    }

    merged
}

pub fn parse_ignore(input: &[String]) -> Vec<String> {
    input
        .iter()
        .flat_map(|entry| entry.split(',').map(|s| s.trim().to_string()).collect::<Vec<_>>())
        .filter(|s| !s.is_empty())
        .collect()
}

fn resolve_base_dir(path: &Path) -> PathBuf {
    let directory = path.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
    fs::canonicalize(&directory).unwrap_or(directory)
}
