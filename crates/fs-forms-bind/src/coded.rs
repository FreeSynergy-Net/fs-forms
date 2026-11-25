//! A coded value: a stable code key plus named parameters (no sentences in code; MF2 texts live in `locales/`).
//!
//! Pattern: Value Object (with typed facts). Mirror of `fs_types::Coded` until the real crate is pinned. The code string is the text
//! that goes out; every DECISION in code reads a typed field set by the producer (family, reason, slot role), never the string.

/// Producer family of a code, set by the producer when it makes the value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Family {
    /// `accepts-error-*`.
    Accepts,
    /// `layout-error-*`.
    Layout,
    /// `records-error-*`.
    Records,
    /// `bind-error-*`.
    Bind,
    /// The producer named none (kept visible, never sorted in as another family).
    #[default]
    Unknown,
}

/// The problem a code is, for the decisions that depend on WHICH problem (set by the producer).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    /// A scheme basic without its complete per-use list (`layout-error-schemes-missing`).
    SchemesMissing,
    /// A per-use scheme list on a basic that takes none (`layout-error-schemes-unexpected`).
    SchemesUnexpected,
}

/// The role of the slot a code is about (set by the producer that locks that slot).
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotRole {
    /// The heads slot of a grouping (the lanes): the code says why the heads cannot be used.
    Heads,
}

/// A code key with ordered parameters and the typed facts its producer set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Coded {
    /// Full kebab key, e.g. `accepts-error-contract-missing`.
    pub code: String,
    /// Parameters in insertion order (deterministic).
    pub params: Vec<(String, String)>,
    family: Family,
    reason: Option<Reason>,
    slot_role: Option<SlotRole>,
}

impl Coded {
    /// A code without parameters and without a family (`Family::Unknown`): the producer says more with `of`.
    #[must_use]
    pub fn new(code: &str) -> Self {
        Self {
            code: code.to_owned(),
            params: Vec::new(),
            family: Family::Unknown,
            reason: None,
            slot_role: None,
        }
    }

    /// The producer family (the producer says it, the code text is never read for it).
    #[must_use]
    pub fn of(mut self, family: Family) -> Self {
        self.family = family;
        self
    }

    /// The problem this code is.
    #[must_use]
    pub fn because(mut self, reason: Reason) -> Self {
        self.reason = Some(reason);
        self
    }

    /// The code is about the slot of this role.
    #[must_use]
    pub fn for_slot(mut self, role: SlotRole) -> Self {
        self.slot_role = Some(role);
        self
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

    /// The producer family, as the producer set it.
    #[must_use]
    pub fn family(&self) -> Family {
        self.family
    }

    /// The problem, when the producer named one.
    #[must_use]
    pub fn reason(&self) -> Option<Reason> {
        self.reason
    }

    /// The role of the slot the code is about, when the producer named one.
    #[must_use]
    pub fn slot_role(&self) -> Option<SlotRole> {
        self.slot_role
    }
}
