//! Persisted application configuration (stored by the Tauri `store` plugin as JSON).

use crate::error::{Error, Result};
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

impl Config {
    /// A configured `root` must be an absolute, existing directory and not a drive/filesystem root.
    pub fn validate(&self) -> Result<()> {
        let Some(root) = &self.root else {
            return Ok(());
        };
        let reject =
            |reason: &str| Error::not_allowed("folder_not_allowed").with_param("reason", reason);
        if !root.is_absolute() {
            return Err(reject("not_absolute"));
        }
        if !root.is_dir() {
            return Err(reject("not_a_directory"));
        }
        if root.parent().is_none() {
            return Err(reject("root_directory"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_root(root: &str) -> Config {
        Config {
            root: Some(root.into()),
            ..Config::default()
        }
    }

    #[test]
    fn validate_accepts_no_root_and_an_existing_absolute_dir() {
        assert!(Config::default().validate().is_ok());
        let t = tempfile::tempdir().unwrap();
        assert!(with_root(t.path().to_str().unwrap()).validate().is_ok());
    }

    #[test]
    fn validate_rejects_relative_missing_and_filesystem_roots() {
        assert_eq!(
            with_root("relative/dir").validate().unwrap_err().code,
            "folder_not_allowed"
        );
        let t = tempfile::tempdir().unwrap();
        let missing = t.path().join("nope");
        assert!(with_root(missing.to_str().unwrap()).validate().is_err());
        let fs_root = t.path().ancestors().last().unwrap().to_path_buf();
        assert!(with_root(fs_root.to_str().unwrap()).validate().is_err());
    }

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
