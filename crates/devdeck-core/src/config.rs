//! Persisted application configuration (stored by the Tauri `store` plugin as JSON).

use crate::scope::Scope;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// UI language. An unknown stored value falls back to English.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Es,
    #[default]
    #[serde(other)]
    En,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    /// Single repositories root folder (optional until the user picks one).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root: Option<PathBuf>,
    pub scan_depth: u8,
    pub base_branch: String,
    pub scope: Scope,
    pub language: Language,
    pub forgotten_days: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ports_auto_refresh_secs: Option<u32>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            root: None,
            scan_depth: 2,
            base_branch: "main".into(),
            scope: Scope::Global,
            language: Language::En,
            forgotten_days: 30,
            ports_auto_refresh_secs: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_spec() {
        let c = Config::default();
        assert_eq!(c.language, Language::En);
        assert_eq!(c.base_branch, "main");
        assert_eq!(c.forgotten_days, 30);
        assert_eq!(c.scope, Scope::Global);
        assert!(c.root.is_none());
    }

    #[test]
    fn unknown_language_falls_back_to_en() {
        let c: Config = serde_json::from_str(r#"{"language":"fr"}"#).unwrap();
        assert_eq!(c.language, Language::En);
    }

    #[test]
    fn empty_object_uses_defaults_and_es_roundtrips() {
        let c: Config = serde_json::from_str("{}").unwrap();
        assert_eq!(c, Config::default());
        let es: Config = serde_json::from_str(r#"{"language":"es","root":"C:/r"}"#).unwrap();
        assert_eq!(es.language, Language::Es);
        let v = serde_json::to_value(&es).unwrap();
        assert_eq!(v["language"], "es");
        assert_eq!(v["root"], "C:/r");
        assert_eq!(v["baseBranch"], "main");
    }
}
