use std::fmt;
use std::path::Path;

use crate::ruleset::RulesetManifest;

#[derive(Debug)]
pub enum RulesetLoadError {
    Io(std::io::Error),
    Yaml(serde_yaml::Error),
    Json(serde_json::Error),
    UnknownExtension(String),
}

impl fmt::Display for RulesetLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "cannot read ruleset file: {e}"),
            Self::Yaml(e) => write!(f, "invalid ruleset YAML: {e}"),
            Self::Json(e) => write!(f, "invalid ruleset JSON: {e}"),
            Self::UnknownExtension(ext) => {
                write!(
                    f,
                    "unrecognized ruleset file extension: {ext} (expected .yaml/.yml/.json)"
                )
            }
        }
    }
}

impl std::error::Error for RulesetLoadError {}

pub fn load_from_yaml_str(input: &str) -> Result<RulesetManifest, RulesetLoadError> {
    serde_yaml::from_str(input).map_err(RulesetLoadError::Yaml)
}

pub fn load_from_json_str(input: &str) -> Result<RulesetManifest, RulesetLoadError> {
    serde_json::from_str(input).map_err(RulesetLoadError::Json)
}

/// Load a ruleset from disk, dispatching on file extension. Used by
/// `ofm_core`/`albion_server` to read `data/rulesets/*.yaml` without every
/// caller needing to know the on-disk format.
pub fn load_from_path(path: &Path) -> Result<RulesetManifest, RulesetLoadError> {
    let content = std::fs::read_to_string(path).map_err(RulesetLoadError::Io)?;
    match path.extension().and_then(|e| e.to_str()) {
        Some("yaml") | Some("yml") => load_from_yaml_str(&content),
        Some("json") => load_from_json_str(&content),
        other => Err(RulesetLoadError::UnknownExtension(
            other.unwrap_or("<none>").to_string(),
        )),
    }
}
