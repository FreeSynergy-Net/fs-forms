//! Stand-ins for `fs-types` 0.0.4 (`Coded`, `ErrorCode`, `Utc`, `Reference`, `ObjectId`), which has not landed yet.
//! Replaced by the real types at the lift; nothing else in the crate depends on how they are built.
//!
//! Pattern: Value Object: small immutable values compared by content.

use std::fmt;

/// A code of kind `C` with its parameters; never a sentence (MF2 keys carry the text).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Coded<C> {
    /// The code.
    pub code: C,
    /// Parameters in the order they were given.
    pub params: Vec<(String, String)>,
}

/// A closed list of codes with a fixed prefix.
pub trait ErrorCode: Copy {
    /// Prefix of the printed form and of the MF2 key.
    const PREFIX: &'static str;
    /// The kebab-case name without prefix.
    fn as_str(self) -> &'static str;
    /// MF2 message key: prefix plus name.
    fn message_key(self) -> String {
        format!("{}{}", Self::PREFIX, self.as_str())
    }
}

impl<C: ErrorCode> Coded<C> {
    /// A code without parameters.
    #[must_use]
    pub fn new(code: C) -> Self {
        Self {
            code,
            params: Vec::new(),
        }
    }
    /// Adds a parameter.
    #[must_use]
    #[allow(clippy::needless_pass_by_value)] // callers hand over temporaries; a reference would not be shorter
    pub fn with(mut self, key: &str, value: impl ToString) -> Self {
        self.params.push((key.to_owned(), value.to_string()));
        self
    }
    /// The value of a parameter.
    #[must_use]
    pub fn param(&self, key: &str) -> Option<&str> {
        self.params
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

impl<C: ErrorCode> fmt::Display for Coded<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code.message_key())?;
        self.params
            .iter()
            .try_for_each(|(k, v)| write!(f, " {k}={v}"))
    }
}

/// UTC time in milliseconds since the epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Utc(pub i64);

/// An IRI that points at a person or an actor; never a copy of a name.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Reference(pub String);

/// A UUIDv7 as text.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId(pub String);
