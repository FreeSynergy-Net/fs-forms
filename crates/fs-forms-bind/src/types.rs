//! Narrow mirrors of the values bind reads from the catalog (accepts), the type catalog and the records service.
//!
//! Pattern: Anti-corruption mirror. Replaced by the real types when `forms_f1_v1` is set (edge noted in the report).

/// How a field is used in a block.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Role {
    /// Editable.
    Input,
    /// Read-only, full.
    Display,
    /// Short form (card).
    Compact,
    /// Chip (filter values).
    Chip,
}

/// Basic type of a field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Basic {
    /// Text.
    Text,
    /// E-mail address.
    Email,
    /// Number.
    Number,
    /// Yes / no.
    Bool,
    /// Date.
    Date,
    /// Date and time.
    DateTime,
    /// One of a list.
    Choice,
    /// Reference to another record (`target`).
    Reference,
    /// Keywords.
    Keywords,
    /// Link (needs a scheme list per use).
    Link,
    /// Image (needs a scheme list per use).
    Image,
    /// A basic this version does not know.
    Unknown,
}

impl Basic {
    /// Basics that carry a per-use list of allowed address schemes.
    #[must_use]
    pub fn has_schemes(self) -> bool {
        matches!(self, Basic::Link | Basic::Image)
    }
}

/// A field as described by the type catalog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldDesc {
    /// Field path.
    pub path: Vec<String>,
    /// Basic type.
    pub basic: Basic,
    /// For references: the target type.
    pub target: Option<String>,
    /// Never editable.
    pub read_only: bool,
}

/// A command as described by the type catalog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandDesc {
    /// `false`: shown as "coming".
    pub supported: bool,
    /// Changes data.
    pub writes: bool,
    /// Acts on one record (needs a selection).
    pub on_record: bool,
    /// Type-level right (`Shape.may`, c0 86) for a type command such as `create`: `Some(false)` = no right = hidden;
    /// `None` = the provider does not say, the command is shown (the provider answers `forbidden` visibly).
    pub type_may: Option<bool>,
}

/// Why a field is hidden for this person.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HiddenReason {
    /// No right.
    NoRight,
    /// Sensitive.
    Sensitive,
}

/// One record as the records service shows it to the asking person.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecordView {
    /// Record id.
    pub id: String,
    /// Commands this person may run on it.
    pub may: Vec<String>,
    /// Hidden fields (dotted path, reason).
    pub hidden: Vec<(String, HiddenReason)>,
    /// Values by dotted path.
    pub values: Vec<(String, String)>,
}

/// A page of records.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecordPage {
    /// The records.
    pub records: Vec<RecordView>,
    /// Exact total.
    pub total: Option<u64>,
    /// Estimated total.
    pub total_estimated: Option<u64>,
}

/// Limits a page must keep.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Largest allowed value in bytes (the limit itself is allowed).
    pub value_max: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self { value_max: 65_536 }
    }
}

/// Search kind of a query.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SearchKind {
    /// Prefix search.
    #[default]
    Prefix,
    /// Whole words.
    Words,
}

/// The rule "a scheme basic needs its complete per-use list". Mirror of F1b's `schemes_rule` (the one rule source, question 81);
/// replaced by the real function when `fs-forms-layout` is pinned.
#[must_use]
pub fn schemes_rule(
    basic: Basic,
    list: Option<&[String]>,
    elem: &str,
    path: &str,
) -> Option<crate::coded::Coded> {
    if basic.has_schemes() && list.is_none_or(<[String]>::is_empty) {
        Some(
            crate::coded::Coded::new("layout-error-schemes-missing")
                .with("elem", elem)
                .with("path", path),
        )
    } else {
        None
    }
}
