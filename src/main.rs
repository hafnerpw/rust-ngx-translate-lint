use std::{collections::BTreeMap, path::PathBuf, time::Instant};

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use ngx_translate_lint_core::config::{load_config, merge_config, parse_ignore};
use ngx_translate_lint_core::engine::LintEngine;
use ngx_translate_lint_core::models::{LintConfig, LintError, LintSummary};
use ngx_translate_lint_core::npm;

#[derive(Parser, Debug)]
#[command(name = "ngx-translate-lint-rs", about = "High-performance ngx-translate lint rewritten in Rust")]
struct CliArgs {
    #[arg(short = 'p', long = "project", value_name = "GLOB")]
    project: Option<String>,
    #[arg(short = 'l', long = "languages", value_name = "GLOB")]
    languages: Option<String>,
    #[arg(short = 'i', long = "ignore", value_name = "PATH", action = clap::ArgAction::Append)]
    ignore: Vec<String>,
    #[arg(long = "config", value_name = "FILE")]
    config: Option<PathBuf>,
    #[arg(long = "fix-zombies-keys")]
    fix_zombies_keys: bool,
    #[arg(long = "max-warning")]
    max_warning: Option<usize>,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Build node addon and npm package
    BuildNode,
}

fn main() -> Result<()> {
    let args = CliArgs::parse();

    // Print version
    println!("{}", format!("ngx-translate-lint v{}", env!("CARGO_PKG_VERSION")).bright_cyan());
    println!();

    let file_cfg = load_config(args.config.as_deref())?;
    let cli_cfg = build_cli_config(&args);
    let merged_cfg = merge_config(&file_cfg, &cli_cfg);

    if let Some(Commands::BuildNode) = args.command {
        npm::build_npm_package()?;
        return Ok(());
    }

    let start_time = Instant::now();
    let engine = LintEngine::new(merged_cfg);
    let summary = engine.run()?;
    let elapsed = start_time.elapsed();

    print_summary(&summary, elapsed);

    std::process::exit(summary.exit_code());
}

fn build_cli_config(args: &CliArgs) -> LintConfig {
    let mut cfg = LintConfig::default();
    if let Some(project) = &args.project {
        cfg.views = project.clone();
    }
    if let Some(languages) = &args.languages {
        cfg.languages = languages.clone();
    }
    if !args.ignore.is_empty() {
        cfg.ignore = parse_ignore(&args.ignore);
    }
    if args.fix_zombies_keys {
        cfg.fix_zombies_keys = true;
    }
    if let Some(max_warning) = args.max_warning {
        cfg.rules.max_warning = max_warning;
    }
    cfg
}

fn print_summary(summary: &LintSummary, elapsed: std::time::Duration) {
    if summary.errors.is_empty() {
        println!("{}", "✓ No issues found.".green().bold());
        println!("\n{}", format!("Completed in {:.2}s", elapsed.as_secs_f64()).dimmed());
        return;
    }

    let mut grouped: BTreeMap<String, Vec<&LintError>> = BTreeMap::new();
    for err in &summary.errors {
        let key = err.current_path.clone().unwrap_or_else(|| "<unknown>".to_string());
        grouped.entry(key).or_default().push(err);
    }

    for (path, errs) in grouped {
        let normalized_path = normalize_path(&path);
        println!("\n{}", normalized_path.bright_white().underline());
        for err in errs {
            let severity_label = if err.is_error() {
                "Error".red().bold()
            } else {
                "Warning".yellow().bold()
            };
            let error_type = format!("({})", err.error_flow.description()).dimmed();
            println!("  [{}] {} {}", severity_label, err.key.cyan(), error_type);
            if !err.missing_paths.is_empty() {
                let missing_str = err.missing_paths.iter().map(|p| normalize_path(p)).collect::<Vec<_>>().join(", ");
                println!("      {} {}", "missing:".dimmed(), missing_str);
            }
            if !err.suggestions.is_empty() {
                println!("      {} {}", "suggestions:".dimmed(), err.suggestions.join(", ").bright_green());
            }
        }
    }

    println!();
    let error_text = if summary.error_count > 0 {
        format!("{} error(s)", summary.error_count).red().bold()
    } else {
        format!("{} error(s)", summary.error_count).normal()
    };
    let warning_text = if summary.warning_count > 0 {
        format!("{} warning(s)", summary.warning_count).yellow().bold()
    } else {
        format!("{} warning(s)", summary.warning_count).normal()
    };
    println!("{} {}, {}", "Summary:".bold(), error_text, warning_text);

    if summary.exceeded_warning_limit {
        println!("{}", "⚠ Warning budget exceeded; treated as errors.".yellow().bold());
    }

    println!("{}", format!("Completed in {:.2}s", elapsed.as_secs_f64()).dimmed());
}

/// Normalize Windows UNC paths (\\?\C:\...) to regular paths (C:\...)
fn normalize_path(path: &str) -> String {
    if path.starts_with("\\\\?\\") {
        path[4..].to_string()
    } else {
        path.to_string()
    }
}
