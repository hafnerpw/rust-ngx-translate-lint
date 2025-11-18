use std::{collections::BTreeMap, fs, path::{Path, PathBuf}};

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use anyhow::{Context, Result};
use fancy_regex::{Regex as FancyRegex, RegexBuilder as FancyRegexBuilder};
use glob::Pattern;
use globwalk::GlobWalkerBuilder;
use serde_json::Value;

use crate::models::{RuleConfig, ToggleRule};

const DIRECTIVE_PATTERN: &str = r#"<[^>]*(?:translate|TRANSLATE)[^>]*>\s*(?P<directive>[A-Za-z0-9_\-.]+)\s*<\s*/"#;
const ATTRIBUTE_PATTERN: &str = r#"(?:translate|\[translate\])\s*=\s*["'](?P<attribute>[A-Za-z0-9_\-.]+)["']"#;
const PIPE_PATTERN: &str = r#"['"](?P<pipe>[A-Za-z0-9_\-.]+)['"]\s*(?:\|\s*\w+\s*)*\|\s*translate"#;

pub fn collect_files(base_dir: &Path, pattern: &str, ignore: &[String]) -> Result<Vec<PathBuf>> {
    let prepared_pattern = normalize_glob_pattern(pattern);
    let walker = GlobWalkerBuilder::from_patterns(base_dir, &[prepared_pattern.as_str()])
        .follow_links(true)
        .case_insensitive(cfg!(windows))
        .build()
        .with_context(|| format!("Unable to resolve files for pattern '{}'", pattern))?;

    let ignore_patterns = build_ignore_patterns(ignore, base_dir)?;

    let mut files = Vec::new();
    for entry in walker {
        let entry = entry?;
        let path = entry.path().to_path_buf();
        if should_ignore(&path, &ignore_patterns) {
            continue;
        }
        files.push(path);
    }
    files.sort();
    Ok(files)
}

fn build_ignore_patterns(patterns: &[String], base_dir: &Path) -> Result<Vec<Pattern>> {
    let mut set = Vec::new();
    for raw in patterns {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let normalized = normalize_pattern(trimmed, base_dir);
        let pattern = Pattern::new(&normalized)
            .with_context(|| format!("Invalid ignore pattern '{}'", raw))?;
        set.push(pattern);
    }
    Ok(set)
}

fn normalize_pattern(raw: &str, base_dir: &Path) -> String {
    let candidate = Path::new(raw);
    let absolute = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        base_dir.join(candidate)
    };
    normalize_path(&absolute)
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn normalize_glob_pattern(pattern: &str) -> String {
    let mut normalized = pattern.replace('\\', "/");
    if Path::new(&normalized).is_absolute() {
        return normalized;
    }
    while normalized.starts_with("./") {
        normalized = normalized[2..].to_string();
    }
    normalized
}

fn should_ignore(path: &Path, patterns: &[Pattern]) -> bool {
    if patterns.is_empty() {
        return false;
    }
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let normalized = normalize_path(&canonical);
    patterns.iter().any(|pattern| pattern.matches(&normalized))
}

pub fn parse_language_file(path: &Path) -> Result<BTreeMap<String, String>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Cannot read language file: {}", path.display()))?;
    let json: Value = serde_json::from_str(&content)
        .with_context(|| format!("Invalid JSON in language file: {}", path.display()))?;
    Ok(flatten_json(&json))
}

fn flatten_json(value: &Value) -> BTreeMap<String, String> {
    let mut acc = BTreeMap::new();
    flatten_value(value, None, &mut acc);
    acc
}

fn flatten_value(value: &Value, prefix: Option<String>, out: &mut BTreeMap<String, String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let next = prefix
                    .as_ref()
                    .map_or_else(|| key.clone(), |parent| format!("{}.{}", parent, key));
                flatten_value(child, Some(next), out);
            }
        }
        Value::Array(items) => {
            for (idx, child) in items.iter().enumerate() {
                let segment = idx.to_string();
                let next = prefix
                    .as_ref()
                    .map_or_else(|| segment.clone(), |parent| format!("{}.{}", parent, segment));
                flatten_value(child, Some(next), out);
            }
        }
        Value::Null => {
            if let Some(key) = prefix {
                out.insert(key, String::new());
            }
        }
        Value::String(text) => {
            if let Some(key) = prefix {
                out.insert(key, text.clone());
            }
        }
        other => {
            if let Some(key) = prefix {
                out.insert(key, other.to_string());
            }
        }
    }
}

pub struct ViewMatchers {
    pub structural: FancyRegex,
    pub deep: Option<DeepSearchMatcher>,
}

struct DeepSearchChunk {
    automaton: AhoCorasick,
    keys: Vec<String>,
}

pub struct DeepSearchMatcher {
    chunks: Vec<DeepSearchChunk>,
}

impl DeepSearchMatcher {
    pub fn collect_matches(&self, content: &str, location: &str, out: &mut Vec<(String, String)>) {
        let bytes = content.as_bytes();
        let debug_enabled = std::env::var("NGX_LINT_DEBUG").map(|v| v != "0").unwrap_or(false);
        let debug_file = std::env::var("NGX_LINT_DEBUG_FILE").ok();
        let debug_key = std::env::var("NGX_LINT_DEBUG_KEY").ok();
        for chunk in &self.chunks {
            for mat in chunk.automaton.find_iter(content) {
                let boundary_ok = has_key_boundaries(bytes, mat.start(), mat.end());
                let index = mat.pattern().as_usize();
                if let Some(key) = chunk.keys.get(index) {
                    if !boundary_ok {
                        if debug_enabled
                            && debug_key.as_deref().map_or(false, |needle| key.contains(needle))
                        {
                            let before = mat.start().checked_sub(1).and_then(|idx| bytes.get(idx).copied());
                            let after = bytes.get(mat.end()).copied();
                            println!(
                                "Boundary rejected {} in {} (before={:?}, after={:?})",
                                key,
                                location,
                                before.map(|b| format!("0x{:02X} ({:?})", b, b as char)),
                                after.map(|b| format!("0x{:02X} ({:?})", b, b as char))
                            );
                        }
                        continue;
                    }
                    if debug_enabled
                        && (debug_file.as_deref().map_or(false, |needle| location.contains(needle))
                            || debug_key
                                .as_deref()
                                .map_or(false, |needle| key.contains(needle)))
                    {
                        println!("Deep match {} in {}", key, location);
                    }
                    out.push((key.clone(), location.to_string()));
                }
            }
        }
    }
}

pub fn build_view_matchers(rule: &RuleConfig, all_keys: &[String]) -> Result<ViewMatchers> {
    let mut segments: Vec<String> = vec![
        format!("(?:{})", DIRECTIVE_PATTERN),
        format!("(?:{})", ATTRIBUTE_PATTERN),
        format!("(?:{})", PIPE_PATTERN),
    ];

    if std::env::var("NGX_LINT_DEBUG").map(|v| v != "0").unwrap_or(false) {
        if let Ok(filter) = std::env::var("NGX_LINT_DEBUG_KEY") {
            let matches: Vec<&String> = all_keys.iter().filter(|key| key.contains(&filter)).collect();
            if !matches.is_empty() {
                println!("Deep matcher contains {} key(s) matching '{}':", matches.len(), filter);
                for key in matches.iter().take(10) {
                    println!("  key -> {}", key);
                }
            } else {
                println!("Deep matcher did not receive any key containing '{}'", filter);
            }
        }
    }

    for custom in &rule.custom_reg_exp_to_find_keys {
        let trimmed = custom.trim();
        if trimmed.is_empty() {
            continue;
        }
        segments.push(format!("(?:{trimmed})"));
    }

    let combined = segments.join("|");
    let structural = FancyRegexBuilder::new(&combined)
        .backtrack_limit(10_000_000)
        .build()
        .with_context(|| "Unable to compile view key regular expression".to_string())?;

    let deep_keys: Vec<String> = if let Ok(filter) = std::env::var("NGX_LINT_FILTER_DEEP_KEY") {
        let filtered: Vec<String> = all_keys
            .iter()
            .filter(|key| key.contains(&filter))
            .cloned()
            .collect();
        if std::env::var("NGX_LINT_DEBUG").map(|v| v != "0").unwrap_or(false) {
            println!(
                "Filtered deep-search keys to {} entries containing '{}'",
                filtered.len(),
                filter
            );
        }
        filtered
    } else {
        all_keys.to_vec()
    };

    // Sort keys by length (descending) to ensure longer keys are matched before shorter prefixes
    let mut sorted_deep_keys = deep_keys.clone();
    sorted_deep_keys.sort_by(|a, b| b.len().cmp(&a.len()));

    const CHUNK_SIZE: usize = 128;
    let deep = if rule.deep_search == ToggleRule::Enable && !sorted_deep_keys.is_empty() {
        let mut chunks = Vec::new();
        for chunk_keys in sorted_deep_keys.chunks(CHUNK_SIZE) {
            let automaton = AhoCorasickBuilder::new()
                .match_kind(MatchKind::LeftmostFirst)
                .prefilter(false)
                .build(chunk_keys)
                .with_context(|| "Unable to build deep search matcher")?;
            chunks.push(DeepSearchChunk {
                automaton,
                keys: chunk_keys.to_vec(),
            });
        }
        Some(DeepSearchMatcher { chunks })
    } else {
        None
    };

    Ok(ViewMatchers { structural, deep })
}

fn has_key_boundaries(bytes: &[u8], start: usize, end: usize) -> bool {
    (start == 0 || !is_key_byte(bytes[start - 1]))
        && (end >= bytes.len() || !is_key_byte(bytes[end]))
}

fn is_key_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{RuleConfig, ToggleRule};
    use std::path::Path;

    #[test]
    fn deep_search_matches_keys_inside_ternary() {
        let mut rules = RuleConfig::default();
        rules.deep_search = ToggleRule::Enable;
        let key = "engineering.locality.edit_component_group".to_string();
        let matchers = build_view_matchers(&rules, &[key.clone()]).expect("matchers");
        let ViewMatchers { structural: _, deep } = matchers;
        let deep = deep.expect("deep matcher");
        let mut hits = Vec::new();
        let content = "{{ condition ? 'engineering.locality.edit_component_group' : 'fallback' }}";
        deep.collect_matches(content, "test", &mut hits);
        assert!(hits.iter().any(|(k, _)| k == &key));
    }

    #[test]
    fn deep_search_reports_shorter_prefix_keys() {
        let mut rules = RuleConfig::default();
        rules.deep_search = ToggleRule::Enable;
        let keys = vec![
            "engineering.locality.edit_component_group".to_string(),
            "engineering.locality.edit_component_group.delete_title".to_string(),
        ];
        let matchers = build_view_matchers(&rules, &keys).expect("matchers");
        let ViewMatchers { structural: _, deep } = matchers;
        let deep = deep.expect("deep matcher");
        let mut hits = Vec::new();
        let content = "'engineering.locality.edit_component_group'";
        deep.collect_matches(content, "test", &mut hits);
        assert!(hits.iter().any(|(k, _)| k == "engineering.locality.edit_component_group"));
    }

    #[test]
    fn deep_search_matches_addtooltip_attribute() {
        let mut rules = RuleConfig::default();
        rules.deep_search = ToggleRule::Enable;
        let key = "engineering.locality.add_automation_station".to_string();
        let matchers = build_view_matchers(&rules, &[key.clone()]).expect("matchers");
        let ViewMatchers { structural: _, deep } = matchers;
        let deep = deep.expect("deep matcher");
        let mut hits = Vec::new();
        let content = r#"addTooltip="engineering.locality.add_automation_station""#;
        deep.collect_matches(content, "test", &mut hits);
        assert!(hits.iter().any(|(k, _)| k == &key), "Expected to find key in addTooltip attribute");
    }

    #[test]
    fn deep_search_matches_library_version_dialog_keys_in_ternary() {
        let mut rules = RuleConfig::default();
        rules.deep_search = ToggleRule::Enable;
        let keys = vec![
            "library.version_dialog.reference_version".to_string(),
            "library.version_dialog.target_version".to_string(),
        ];
        let matchers = build_view_matchers(&rules, &keys).expect("matchers");
        let ViewMatchers { structural: _, deep } = matchers;
        let deep = deep.expect("deep matcher");
        let mut hits = Vec::new();
        let content = "{{ (foo ? 'library.version_dialog.reference_version' : 'library.version_dialog.target_version') | translate }}";
        deep.collect_matches(content, "test", &mut hits);
        assert!(hits.iter().any(|(k, _)| k == "library.version_dialog.reference_version"));
        assert!(hits.iter().any(|(k, _)| k == "library.version_dialog.target_version"));
    }

    #[test]
    fn deep_search_matches_external_file_when_env_paths_provided() {
        let view_path = match std::env::var("NGX_LINT_DEBUG_VIEW_FILE") {
            Ok(path) if !path.is_empty() => path,
            _ => return,
        };
        let lang_path = match std::env::var("NGX_LINT_DEBUG_LANG_FILE") {
            Ok(path) if !path.is_empty() => path,
            _ => return,
        };
        let mut rules = RuleConfig::default();
        rules.deep_search = ToggleRule::Enable;
        let languages = parse_language_file(Path::new(&lang_path)).expect("language json");
        let keys: Vec<String> = languages.keys().cloned().collect();
        let matchers = build_view_matchers(&rules, &keys).expect("matchers");
        let deep = matchers.deep.expect("deep matcher");
        let content = fs::read_to_string(Path::new(&view_path)).expect("view content");
        let mut hits = Vec::new();
        deep.collect_matches(&content, &view_path, &mut hits);
        println!(
            "Deep matches for {} ({} total):",
            view_path,
            hits.len()
        );
        for (key, _) in &hits {
            println!("  {}", key);
        }
        if let Ok(expected) = std::env::var("NGX_LINT_DEBUG_EXPECT_KEY") {
            assert!(
                hits.iter().any(|(k, _)| k == &expected),
                "expected key '{}' not matched; got {:?}",
                expected,
                hits.iter().map(|(k, _)| k).collect::<Vec<_>>()
            );
        }
    }
}

pub fn remove_keys_from_json_file(path: &Path, keys: &[String]) -> Result<()> {
    if keys.is_empty() {
        return Ok(());
    }
    let content = fs::read_to_string(path)
        .with_context(|| format!("Unable to read JSON file: {}", path.display()))?;
    let mut json: Value = serde_json::from_str(&content)
        .with_context(|| format!("Invalid JSON file: {}", path.display()))?;

    for key in keys {
        remove_nested_key(&mut json, key);
    }

    let serialized = serde_json::to_string_pretty(&json)?;
    fs::write(path, format!("{}\n", serialized))
        .with_context(|| format!("Unable to write JSON file: {}", path.display()))?
    ;
    Ok(())
}

fn remove_nested_key(value: &mut Value, dotted_key: &str) {
    let segments: Vec<&str> = dotted_key.split('.').collect();
    if segments.is_empty() {
        return;
    }
    remove_segments(value, &segments);
}

fn remove_segments(value: &mut Value, segments: &[&str]) -> bool {
    if segments.is_empty() {
        return false;
    }
    if segments.len() == 1 {
        match value {
            Value::Object(map) => map.remove(segments[0]).is_some(),
            Value::Array(items) => {
                if let Ok(idx) = segments[0].parse::<usize>() {
                    if idx < items.len() {
                        items.remove(idx);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            _ => false,
        }
    } else {
        match value {
            Value::Object(map) => {
                if let Some(child) = map.get_mut(segments[0]) {
                    let removed = remove_segments(child, &segments[1..]);
                    if removed && child_is_empty(child) {
                        map.remove(segments[0]);
                    }
                    removed
                } else {
                    false
                }
            }
            Value::Array(items) => {
                if let Ok(idx) = segments[0].parse::<usize>() {
                    if let Some(child) = items.get_mut(idx) {
                        let removed = remove_segments(child, &segments[1..]);
                        if removed && child_is_empty(child) {
                            items.remove(idx);
                        }
                        removed
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

fn child_is_empty(value: &Value) -> bool {
    match value {
        Value::Object(map) => map.is_empty(),
        Value::Array(items) => items.is_empty(),
        Value::Null => true,
        Value::String(text) => text.is_empty(),
        _ => false,
    }
}
