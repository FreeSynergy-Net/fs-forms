//! A coded value: a stable code key plus named parameters (no sentences in code; MF2 texts live in `locales/`).
//!
//! Pattern: Value Object. Mirror of `fs_types::Coded` until the real crate is pinned.

/// Producer family of a code, read from its prefix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    /// `accepts-error-*`.
    Accepts,
    /// `layout-error-*`.
    Layout,
    /// `records-error-*`.
    Records,
    /// `bind-error-*`.
    Bind,
    /// No known prefix.
    Unknown,
}

/// A code key with ordered parameters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Coded {
    /// Full kebab key, e.g. `accepts-error-contract-missing`.
    pub code: String,
    /// Parameters in insertion order (deterministic).
    pub params: Vec<(String, String)>,
}

impl Coded {
    /// A code without parameters.
    #[must_use]
    pub fn new(code: &str) -> Self {
        Self {
            code: code.to_owned(),
            params: Vec::new(),
        }
    }

    /// Adds one parameter.
    #[must_use]
    pub fn with(mut self, key: &str, value: impl Into<String>) -> Self {
        self.params.push((key.to_owned(), value.into()));
        self
    }

    /// A parameter by name.
    #[must_use]
    pub fn param(&self, key: &str) -> Option<&str> {
        self.params
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// The producer family of the code.
    #[must_use]
    pub fn family(&self) -> Family {
        let c = self.code.as_str();
        if c.starts_with("accepts-error-") {
            Family::Accepts
        } else if c.starts_with("layout-error-") {
            Family::Layout
        } else if c.starts_with("records-error-") {
            Family::Records
        } else if c.starts_with("bind-error-") {
            Family::Bind
        } else {
            Family::Unknown
        }
    }
}
