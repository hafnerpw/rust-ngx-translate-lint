use std::{collections::{BTreeMap, BTreeSet}, env, fs, path::{Path, PathBuf}};

use anyhow::{anyhow, Context, Result};
use fancy_regex::Captures;
use rayon::prelude::*;
use regex::{Regex as StdRegex, RegexBuilder};
use strsim::normalized_damerau_levenshtein;

use crate::{
    models::{ErrorFlow, LintConfig, LintError, LintSummary, TranslationKey},
    utils::{build_view_matchers, collect_files, parse_language_file, remove_keys_from_json_file, ViewMatchers},
};

pub struct LintEngine {
    cfg: LintConfig,
}

impl LintEngine {
    pub fn new(cfg: LintConfig) -> Self {
        Self { cfg }
    }

    pub fn run(&self) -> Result<LintSummary> {
        let language_files = collect_files(&self.cfg.base_dir, &self.cfg.languages, &self.cfg.ignore)?;
        debug_status(|| println!("Language files: {}", language_files.len()));
        if language_files.is_empty() {
            return Err(anyhow!("No language files matched pattern {}", self.cfg.languages));
        }
        let (language_map, infos) = self.load_languages(&language_files)?;
        debug_status(|| println!("Language keys: {}", language_map.len()));
        debug_status(|| println!(
            "Deep search enabled: {}",
            self.cfg.rules.deep_search.enabled()
        ));
        debug_status(|| {
            if self.cfg.rules.custom_reg_exp_to_find_keys.is_empty() {
                println!("Custom regexes: none");
            } else {
                println!("Custom regexes ({}):", self.cfg.rules.custom_reg_exp_to_find_keys.len());
                for pattern in &self.cfg.rules.custom_reg_exp_to_find_keys {
                    println!("  {pattern}");
                }
            }
        });

        let keys: Vec<String> = language_map.keys().cloned().collect();
        let view_matchers = build_view_matchers(&self.cfg.rules, &keys)?;
        let view_files = collect_files(&self.cfg.base_dir, &self.cfg.project, &self.cfg.ignore)?;
        debug_status(|| println!("View files: {}", view_files.len()));
        let view_keys = self.scan_views(&view_files, &view_matchers)?;
        debug_status(|| println!("Distinct view keys: {}", view_keys.len()));
        debug_status(|| {
            let missing = language_map
                .keys()
                .filter(|key| !view_keys.contains_key(*key))
                .count();
            println!("Language keys missing from views: {}", missing);
            for sample in [
                "engineering.locality.edit_component_group",
                "engineering.locality.edit_electrical_cabinet",
                "shared.actions.close_without_saving",
                "library.version_dialog.reference_version",
            ] {
                println!(
                    "  sample '{}' present in views: {}, languages: {}",
                    sample,
                    view_keys.contains_key(sample),
                    language_map.contains_key(sample)
                );
            }
        });

        let errors = self.run_rules(&language_map, &infos, &view_keys)?;
        let filtered = self.apply_ignored(errors)?;

        if self.cfg.fix_zombies_keys {
            self.fix_zombie_keys(&filtered)?;
        }

        Ok(LintSummary::from_errors(filtered, self.cfg.rules.max_warning))
    }

    fn load_languages(&self, files: &[PathBuf]) -> Result<(BTreeMap<String, TranslationKey>, Vec<LanguageFileInfo>)> {
        let mut map: BTreeMap<String, TranslationKey> = BTreeMap::new();
        let mut infos: Vec<LanguageFileInfo> = Vec::new();
        for path in files {
            let canonical = fs::canonicalize(path).with_context(|| format!("Cannot resolve language file: {}", path.display()))?;
            let lang_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string();
            let path_str = canonical.to_string_lossy().to_string();
            infos.push(LanguageFileInfo { name: lang_name.clone(), path: path_str.clone() });
            let values = parse_language_file(&canonical)?;
            for (key, value) in values {
                let entry = map.entry(key.clone()).or_insert_with(|| TranslationKey::new(key.clone()));
                entry.languages.insert(path_str.clone());
                entry.value = Some(value);
            }
        }
        Ok((map, infos))
    }

    fn scan_views(&self, files: &[PathBuf], matchers: &ViewMatchers) -> Result<BTreeMap<String, BTreeSet<String>>> {
        let matches: Result<Vec<Vec<(String, String)>>, _> = files
            .par_iter()
            .map(|path| -> Result<Vec<(String, String)>> {
                let content = fs::read_to_string(path).with_context(|| format!("Unable to read view file: {}", path.display()))?;
                let mut found = Vec::new();
                let location = path.display().to_string();
                let mut structural_hits = 0usize;
                let mut structural_keys: Vec<String> = Vec::new();
                let debug_enabled = env::var("NGX_LINT_DEBUG").map(|v| v != "0").unwrap_or(false);
                let debug_file = env::var("NGX_LINT_DEBUG_FILE").ok();
                let log_this_file = debug_enabled && debug_file.as_deref().map_or(false, |needle| location.contains(needle));
                for cap in matchers.structural.captures_iter(&content) {
                    let cap = cap.map_err(|err| anyhow!(err.to_string()))?;
                    if let Some(key) = extract_key_from_capture(&cap) {
                        found.push((key.to_string(), location.clone()));
                        structural_hits += 1;
                        if log_this_file {
                            structural_keys.push(key.to_string());
                        }
                    }
                }
                let deep_before = found.len();
                if let Some(deep) = &matchers.deep {
                    deep.collect_matches(&content, &location, &mut found);
                }
                let deep_hits = found.len().saturating_sub(deep_before);
                if log_this_file {
                    println!(
                        "File {} -> structural matches: {}, deep matches: {}",
                        location, structural_hits, deep_hits
                    );
                    if !structural_keys.is_empty() {
                        for key in &structural_keys {
                            println!("  structural -> {}", key);
                        }
                    }
                    if deep_hits > 0 {
                        for (key, _) in &found[deep_before..] {
                            println!("  deep -> {}", key);
                        }
                    }
                }
                Ok(found)
            })
            .collect();

        let mut key_views: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for file_matches in matches? {
            for (key, location) in file_matches {
                key_views.entry(key).or_default().insert(location.clone());
            }
        }
        Ok(key_views)
    }

    fn run_rules(
        &self,
        languages: &BTreeMap<String, TranslationKey>,
        language_infos: &[LanguageFileInfo],
        view_keys: &BTreeMap<String, BTreeSet<String>>,
    ) -> Result<Vec<LintError>> {
        let mut errors = Vec::new();

        if self.cfg.rules.zombie_keys.is_enabled() {
            errors.extend(self.zombie_rule(languages, view_keys));
        }
        if self.cfg.rules.keys_on_views.is_enabled() {
            errors.extend(self.keys_on_views_rule(languages, language_infos, view_keys));
        }
        if self.cfg.rules.misprint_keys.is_enabled() {
            errors.extend(self.misprint_rule(languages, view_keys));
        }
        if self.cfg.rules.empty_keys.is_enabled() {
            errors.extend(self.empty_keys_rule(languages));
        }

        Ok(errors)
    }

    fn zombie_rule(&self, languages: &BTreeMap<String, TranslationKey>, view_keys: &BTreeMap<String, BTreeSet<String>>) -> Vec<LintError> {
        let mut errors = Vec::new();
        // Keys that exist in languages but not used in views (unused translations)
        for (key, entry) in languages {
            if view_keys.contains_key(key) {
                continue;
            }
            for language_path in &entry.languages {
                errors.push(
                    LintError::new(key, ErrorFlow::ZombieKeys, self.cfg.rules.zombie_keys)
                        .with_path(language_path.clone()),
                );
            }
        }
        errors
    }

    fn keys_on_views_rule(&self, languages: &BTreeMap<String, TranslationKey>, language_infos: &[LanguageFileInfo], view_keys: &BTreeMap<String, BTreeSet<String>>) -> Vec<LintError> {
        let mut errors = Vec::new();
        for (key, views) in view_keys {
            let langs_covering = languages.get(key).map(|entry| entry.languages.len()).unwrap_or(0);
            if langs_covering < language_infos.len() {
                let missing: Vec<String> = language_infos
                    .iter()
                    .filter(|lang| !languages
                        .get(key)
                        .map_or(false, |entry| entry.languages.contains(&lang.path)))
                    .map(|lang| lang.name.clone())
                    .collect();
                for view in views {
                    errors.push(
                        LintError::new(key, ErrorFlow::KeysOnViews, self.cfg.rules.keys_on_views)
                            .with_path(view.clone())
                            .with_missing(missing.clone()),
                    );
                }
            }
        }
        errors
    }

    fn misprint_rule(&self, languages: &BTreeMap<String, TranslationKey>, view_keys: &BTreeMap<String, BTreeSet<String>>) -> Vec<LintError> {
        let all_lang_keys: Vec<String> = languages.keys().cloned().collect();
        let mut errors = Vec::new();
        for (view_key, locations) in view_keys {
            if languages.contains_key(view_key) {
                continue;
            }
            if self
                .cfg
                .rules
                .ignored_misprint_keys
                .iter()
                .any(|ignored| ignored.eq_ignore_ascii_case(view_key))
            {
                continue;
            }
            if let Some(best) = self.best_match(view_key, &all_lang_keys) {
                if best.score >= self.cfg.rules.misprint_coefficient {
                    if let Some(path) = locations.iter().next() {
                        errors.push(
                            LintError::new(view_key, ErrorFlow::MisprintKeys, self.cfg.rules.misprint_keys)
                                .with_path(path.clone())
                                .with_suggestions(vec![best.key.clone()]),
                        );
                    }
                }
            }
        }
        errors
    }

    fn empty_keys_rule(&self, languages: &BTreeMap<String, TranslationKey>) -> Vec<LintError> {
        let mut errors = Vec::new();
        for (key, entry) in languages {
            if entry.value.as_ref().map(|v| v.trim().is_empty()).unwrap_or(true) {
                errors.push(LintError::new(key, ErrorFlow::EmptyKeys, self.cfg.rules.empty_keys));
            }
        }
        errors
    }

    fn apply_ignored(&self, errors: Vec<LintError>) -> Result<Vec<LintError>> {
        if self.cfg.rules.ignored_keys.is_empty() {
            return Ok(errors);
        }
        let regexes: Result<Vec<StdRegex>, _> = self
            .cfg
            .rules
            .ignored_keys
            .iter()
            .map(|pattern| RegexBuilder::new(pattern).case_insensitive(true).build())
            .collect();
        let regexes = regexes?;
        Ok(errors
            .into_iter()
            .filter(|err| !regexes.iter().any(|regex| regex.is_match(&err.key)))
            .collect())
    }

    fn fix_zombie_keys(&self, errors: &[LintError]) -> Result<()> {
        let mut file_to_keys: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for err in errors {
            if err.error_flow != ErrorFlow::ZombieKeys {
                continue;
            }
            if let Some(path) = &err.current_path {
                if path.ends_with(".json") {
                    file_to_keys.entry(path.clone()).or_default().push(err.key.clone());
                }
            }
        }
        for (file, keys) in file_to_keys {
            remove_keys_from_json_file(Path::new(&file), &keys)?;
        }
        Ok(())
    }

    fn best_match<'a>(&self, needle: &str, haystack: &'a [String]) -> Option<BestMatch<'a>> {
        haystack
            .iter()
            .filter(|candidate| candidate != &needle)
            .map(|candidate| BestMatch {
                key: candidate,
                score: normalized_damerau_levenshtein(needle, candidate),
            })
            .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap())
    }
}

fn debug_status<F: FnOnce()>(f: F) {
    if std::env::var("NGX_LINT_DEBUG").map(|v| v != "0").unwrap_or(false) {
        f();
    }
}

fn extract_key_from_capture<'a>(cap: &'a Captures<'a>) -> Option<&'a str> {
    for label in ["directive", "attribute", "pipe"] {
        if let Some(mat) = cap.name(label) {
            return Some(mat.as_str());
        }
    }
    if let Some(mat) = cap.name("deep") {
        return Some(mat.as_str());
    }
    cap.get(0).map(|m| m.as_str())
}

struct BestMatch<'a> {
    key: &'a String,
    score: f64,
}

#[derive(Clone)]
struct LanguageFileInfo {
    name: String,
    path: String,
}
