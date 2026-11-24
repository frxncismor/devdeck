//! Serializable error type shared with the frontend: `{kind, code, params?, detail?}`.
//!
//! `code` is a stable snake-case identifier the frontend maps to `errors.<code>`.
//! This crate never returns UI text.

use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ErrorKind {
    NotFound,
    NotAllowed,
    Git,
    Io,
    Parse,
    Cancelled,
    Unsupported,
    NotImplemented,
}

#[derive(Debug, Clone, PartialEq, Serialize, thiserror::Error)]
#[error("{kind:?}: {code}")]
pub struct Error {
    pub kind: ErrorKind,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<BTreeMap<String, serde_json::Value>>,
    /// Raw technical text (git stderr, OS error). Never the headline.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn new(kind: ErrorKind, code: &str) -> Self {
        Self {
            kind,
            code: code.to_owned(),
            params: None,
            detail: None,
        }
    }

    pub fn with_param(mut self, key: &str, value: impl Into<serde_json::Value>) -> Self {
        self.params
            .get_or_insert_with(BTreeMap::new)
            .insert(key.to_owned(), value.into());
        self
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn not_implemented() -> Self {
        Self::new(ErrorKind::NotImplemented, "not_implemented")
    }

    pub fn cancelled() -> Self {
        Self::new(ErrorKind::Cancelled, "cancelled")
    }

    pub fn not_allowed(code: &str) -> Self {
        Self::new(ErrorKind::NotAllowed, code)
    }

    pub fn not_found(code: &str) -> Self {
        Self::new(ErrorKind::NotFound, code)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        let kind = if e.kind() == std::io::ErrorKind::NotFound {
            ErrorKind::NotFound
        } else {
            ErrorKind::Io
        };
        let code = if kind == ErrorKind::NotFound {
            "not_found"
        } else {
            "io_error"
        };
        Self::new(kind, code).with_detail(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_with_stable_shape() {
        let e = Error::new(ErrorKind::Parse, "json_invalid_at_offset")
            .with_param("offset", 42)
            .with_detail("unexpected token");
        let v = serde_json::to_value(&e).unwrap();
        assert_eq!(
            v,
            serde_json::json!({
                "kind": "Parse",
                "code": "json_invalid_at_offset",
                "params": {"offset": 42},
                "detail": "unexpected token"
            })
        );
    }

    #[test]
    fn omits_empty_optionals() {
        let v = serde_json::to_value(Error::not_implemented()).unwrap();
        assert_eq!(
            v,
            serde_json::json!({"kind": "NotImplemented", "code": "not_implemented"})
        );
    }

    #[test]
    fn io_not_found_maps_to_not_found() {
        let e: Error = std::io::Error::from(std::io::ErrorKind::NotFound).into();
        assert_eq!(e.kind, ErrorKind::NotFound);
        assert_eq!(e.code, "not_found");
    }
}
