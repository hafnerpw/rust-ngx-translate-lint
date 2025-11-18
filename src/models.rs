use std::{collections::{BTreeMap, BTreeSet}, path::PathBuf};
use serde::{de::Error as DeError, Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RuleSeverity {
    #[serde(alias = "disable")]
    Disable,
    #[serde(alias = "warning")]
    Warning,
    #[serde(alias = "error")]
    Error,
}

impl Default for RuleSeverity {
    fn default() -> Self {
        RuleSeverity::Warning
    }
}

impl RuleSeverity {
    pub fn is_enabled(self) -> bool {
        !matches!(self, RuleSeverity::Disable)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ToggleRule {
    #[serde(alias = "disable")]
    Disable,
    #[serde(alias = "enable")]
    Enable,
}

impl Default for ToggleRule {
    fn default() -> Self {
        ToggleRule::Disable
    }
}

impl ToggleRule {
    pub fn enabled(self) -> bool {
        matches!(self, ToggleRule::Enable)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleConfig {
    #[serde(default = "RuleConfig::default_zombie")]
    pub zombie_keys: RuleSeverity,
    #[serde(default = "RuleConfig::default_keys_on_views")]
    pub keys_on_views: RuleSeverity,
    #[serde(default = "RuleConfig::default_empty")]
    pub empty_keys: RuleSeverity,
    #[serde(default)]
    pub misprint_keys: RuleSeverity,
    #[serde(default)]
    pub deep_search: ToggleRule,
    #[serde(
        default = "RuleConfig::default_max_warning",
        deserialize_with = "deserialize_usize_or_string"
    )]
    pub max_warning: usize,
    #[serde(
        default = "RuleConfig::default_misprint_coeff",
        deserialize_with = "deserialize_f64_or_string"
    )]
    pub misprint_coefficient: f64,
    #[serde(default)]
    pub ignored_keys: Vec<String>,
    #[serde(default)]
    pub ignored_misprint_keys: Vec<String>,
    #[serde(default)]
    pub custom_reg_exp_to_find_keys: Vec<String>,
}

impl Default for RuleConfig {
    fn default() -> Self {
        Self {
            zombie_keys: RuleConfig::default_zombie(),
            keys_on_views: RuleConfig::default_keys_on_views(),
            empty_keys: RuleConfig::default_empty(),
            misprint_keys: RuleSeverity::Disable,
            deep_search: ToggleRule::Disable,
            max_warning: RuleConfig::default_max_warning(),
            misprint_coefficient: RuleConfig::default_misprint_coeff(),
            ignored_keys: Vec::new(),
            ignored_misprint_keys: Vec::new(),
            custom_reg_exp_to_find_keys: Vec::new(),
        }
    }
}

impl RuleConfig {
    const fn default_zombie() -> RuleSeverity { RuleSeverity::Warning }
    const fn default_keys_on_views() -> RuleSeverity { RuleSeverity::Error }
    const fn default_empty() -> RuleSeverity { RuleSeverity::Warning }
    const fn default_max_warning() -> usize { 0 }
    const fn default_misprint_coeff() -> f64 { 0.9 }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FetchConfig {
    #[serde(default)]
    pub request_query: Option<String>,
    #[serde(default)]
    pub response_query: Option<String>,
    #[serde(default)]
    pub request_headers: BTreeMap<String, String>,
    #[serde(default)]
    pub script: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LintConfig {
    #[serde(default)]
    pub project: String,
    #[serde(default)]
    pub languages: String,
    #[serde(default)]
    pub ignore: Vec<String>,
    #[serde(default)]
    pub fix_zombies_keys: bool,
    #[serde(default)]
    pub rules: RuleConfig,
    #[serde(default)]
    pub fetch: FetchConfig,
    #[serde(skip)]
    pub base_dir: PathBuf,
}

impl Default for LintConfig {
    fn default() -> Self {
        let base_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            project: "./src/app/**/*.{html,ts,resx}".to_string(),
            languages: "./src/assets/i18n/*.json".to_string(),
            ignore: Vec::new(),
            fix_zombies_keys: false,
            rules: RuleConfig::default(),
            fetch: FetchConfig::default(),
            base_dir,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TranslationKey {
    pub name: String,
    pub value: Option<String>,
    pub languages: BTreeSet<String>,
    pub views: BTreeSet<String>,
}

impl TranslationKey {
    pub fn new(name: String) -> Self {
        Self {
            name,
            value: None,
            languages: BTreeSet::new(),
            views: BTreeSet::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ErrorFlow {
    ZombieKeys,
    KeysOnViews,
    MisprintKeys,
    EmptyKeys,
}

impl ErrorFlow {
    pub fn description(&self) -> &'static str {
        match self {
            ErrorFlow::ZombieKeys => "unused translation",
            ErrorFlow::KeysOnViews => "missing translation",
            ErrorFlow::MisprintKeys => "possible typo",
            ErrorFlow::EmptyKeys => "empty value",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LintError {
    pub key: String,
    pub error_flow: ErrorFlow,
    pub severity: RuleSeverity,
    pub current_path: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub missing_paths: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub suggestions: Vec<String>,
}

impl LintError {
    pub fn new<S: Into<String>>(key: S, flow: ErrorFlow, severity: RuleSeverity) -> Self {
        Self {
            key: key.into(),
            error_flow: flow,
            severity,
            current_path: None,
            missing_paths: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    pub fn with_path(mut self, path: String) -> Self {
        self.current_path = Some(path);
        self
    }

    pub fn with_missing(mut self, paths: Vec<String>) -> Self {
        self.missing_paths = paths;
        self
    }

    pub fn with_suggestions(mut self, suggestions: Vec<String>) -> Self {
        self.suggestions = suggestions;
        self
    }

    pub fn is_warning(&self) -> bool {
        matches!(self.severity, RuleSeverity::Warning)
    }

    pub fn is_error(&self) -> bool {
        matches!(self.severity, RuleSeverity::Error)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LintSummary {
    pub errors: Vec<LintError>,
    pub warning_count: usize,
    pub error_count: usize,
    pub exceeded_warning_limit: bool,
}

impl LintSummary {
    pub fn from_errors(mut errors: Vec<LintError>, max_warning: usize) -> Self {
        let warning_count = errors.iter().filter(|e| e.is_warning()).count();
        let exceeded_warning_limit = max_warning > 0 && warning_count > max_warning;
        if exceeded_warning_limit {
            errors
                .iter_mut()
                .filter(|e| e.is_warning())
                .for_each(|e| e.severity = RuleSeverity::Error);
        }
        let error_count = errors.iter().filter(|e| e.is_error()).count();
        Self { errors, warning_count, error_count, exceeded_warning_limit }
    }

    pub fn exit_code(&self) -> i32 {
        if self.error_count > 0 || self.exceeded_warning_limit {
            1
        } else {
            0
        }
    }
}

fn deserialize_usize_or_string<'de, D>(deserializer: D) -> Result<usize, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum UsizeOrString {
        Number(usize),
        String(String),
    }

    match UsizeOrString::deserialize(deserializer)? {
        UsizeOrString::Number(value) => Ok(value),
        UsizeOrString::String(value) => value.parse().map_err(DeError::custom),
    }
}

fn deserialize_f64_or_string<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum F64OrString {
        Number(f64),
        String(String),
    }

    match F64OrString::deserialize(deserializer)? {
        F64OrString::Number(value) => Ok(value),
        F64OrString::String(value) => value.parse().map_err(DeError::custom),
    }
}
