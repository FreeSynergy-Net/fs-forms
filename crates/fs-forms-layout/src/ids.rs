//! Identifiers of the model: element ids, element kinds, layout and overlay ids, counters, the frontend key.
//!
//! Pattern: Value Object: every id is a small value with its own rule.

use crate::codes::LayoutCode;
use crate::shim::{Coded, ObjectId, Reference};

/// Stable element id: 1..=128 bytes of `[a-z0-9:/._-]`, first char `[a-z0-9]`, unique per (layout, element kind).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ElemId(String);

impl ElemId {
    /// Longest allowed id in bytes.
    pub const MAX_BYTES: usize = 128;

    /// Checks the rule.
    ///
    /// # Errors
    /// `elem-invalid{elem}` when the text breaks the rule.
    pub fn parse(text: &str) -> Result<Self, Coded<LayoutCode>> {
        let b = text.as_bytes();
        let first_ok = b
            .first()
            .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
        let rest_ok = b
            .iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b":/._-".contains(c));
        if b.len() <= Self::MAX_BYTES && first_ok && rest_ok {
            Ok(Self(text.to_owned()))
        } else {
            Err(Coded::new(LayoutCode::ElemInvalid).with("elem", text))
        }
    }

    /// Deterministic id from the natural key: `<kind>:<parts joined by '/'>`. Characters outside the rule become `-`.
    #[must_use]
    pub fn natural(kind: ElemKind, parts: &[&str]) -> Self {
        let clean: String = parts
            .join("/")
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || ":/._-".contains(c) {
                    c.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect();
        let mut id = format!("{}:{clean}", kind.as_str());
        id.truncate(Self::MAX_BYTES);
        Self(id)
    }

    /// The text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

macro_rules! kinds {
    ($($v:ident => $s:literal),* $(,)?) => {
        /// The element kinds an overlay op can target.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum ElemKind { $(
            #[doc = $s]
            $v
        ),* }
        impl ElemKind {
            /// The names of all kinds, in declaration order.
            pub const ALL_NAMES: &'static [&'static str] = &[$($s),*];
            /// Kebab-case name.
            #[must_use]
            pub fn as_str(self) -> &'static str { match self { $(Self::$v => $s),* } }
            /// Reads the name.
            #[must_use]
            pub fn parse(name: &str) -> Option<Self> { match name { $($s => Some(Self::$v),)* _ => None } }
        }
    };
}
kinds! {
    Layout => "layout", Menu => "menu", Source => "source", Page => "page", Block => "block", Binding => "binding",
    BindingScheme => "binding-scheme", Action => "action", View => "view", ViewOrder => "view-order",
    FilterNode => "filter-node", FilterValue => "filter-value", Message => "message", Requires => "requires",
    Frame => "frame", Attr => "attr",
}

/// An element: kind plus id.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ElemRef {
    /// Kind of the element.
    pub kind: ElemKind,
    /// Its id.
    pub elem: ElemId,
}

/// Id of a layout (UUIDv7).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LayoutId(pub ObjectId);
/// Id of an overlay (UUIDv7).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OverlayId(pub ObjectId);
/// Counter of one layout or overlay (compare-and-set).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rev(pub u64);
/// The store counter: equal means nothing to read again.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Stamp(pub u64);

/// (`AppStream` id of the program, frontend name): unique among the live layouts of one store. Names are free identifiers.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FrontendKey {
    /// `AppStream` id of the program.
    pub extends: String,
    /// Frontend name, freely chosen.
    pub frontend: String,
}

/// Package name and version as the package says (compared for equality only).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackageRef {
    /// Package name.
    pub package: String,
    /// Package version.
    pub version: String,
}

/// Who acts: `actor` acts, `author` is the person the work is for. A human: both equal. (Stand-in for the shared actor type.)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Who {
    /// The acting party.
    pub actor: Reference,
    /// The person it is for.
    pub author: Reference,
}
