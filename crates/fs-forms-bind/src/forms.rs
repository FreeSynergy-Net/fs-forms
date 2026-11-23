//! The data of the form match (ENTSCHEID-form §3): what a form is, what a module offers, what a component draws.
//!
//! A form is a named, versioned list of channels (`tree@1.0`: nodes, parent, label …). The module maps channels to its own
//! fields and declares the form in a version; the component declares which forms it draws; the match puts them together
//! ([`crate::match_form`]). These are plain values: bind reads them from the form catalog (`fs-interfaces`), from the module
//! catalog and from the block catalog (`forms=` of `fs-components`) and does no I/O.
//!
//! Pattern: Value objects (anti-corruption mirrors, like `types.rs`; replaced by the catalog readers when they are pinned).

use crate::types::Basic;

/// The base type of a channel (every channel names exactly one): what kind of value the module must put there.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Grundtyp {
    /// A text (also a choice word, an address, a link, keywords).
    Text,
    /// A point in time (a date or a date with time).
    Time,
    /// A number.
    Number,
    /// Yes / no.
    Flag,
    /// A reference to another record.
    Reference,
    /// A whole list of records (names a type, not a field).
    Records,
    /// A command id (names a command of the module, not a field).
    Command,
}

impl Grundtyp {
    /// The word of the catalog.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Time => "time",
            Self::Number => "number",
            Self::Flag => "flag",
            Self::Reference => "reference",
            Self::Records => "records",
            Self::Command => "command",
        }
    }

    /// The type word from the catalog, `None` for any other word.
    #[must_use]
    pub fn from_name(word: &str) -> Option<Self> {
        [
            Self::Text,
            Self::Time,
            Self::Number,
            Self::Flag,
            Self::Reference,
            Self::Records,
            Self::Command,
        ]
        .into_iter()
        .find(|g| g.name() == word)
    }

    /// Does a field of this basic type fit a channel of this base type? `records` and `command` take no field value:
    /// they always fit here (the module names a type or a command, which the type catalog checks elsewhere).
    #[must_use]
    pub fn takes(self, basic: Basic) -> bool {
        match self {
            Self::Text => matches!(
                basic,
                Basic::Text | Basic::Email | Basic::Choice | Basic::Keywords | Basic::Link
            ),
            Self::Time => matches!(basic, Basic::Date | Basic::DateTime),
            Self::Number => basic == Basic::Number,
            Self::Flag => basic == Basic::Bool,
            Self::Reference => basic == Basic::Reference,
            Self::Records | Self::Command => true,
        }
    }
}

/// How many fields a channel takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Count {
    /// Exactly one.
    One,
    /// None or one.
    ZeroOrOne,
    /// None up to `n` (`None` = unbounded).
    ZeroToN(Option<u32>),
    /// From `min` to `max` (`None` = unbounded): the form of `Count` in `fs-interfaces`.
    Range {
        /// Fewest fields.
        min: u32,
        /// Most fields.
        max: Option<u32>,
    },
}

impl Count {
    /// The fewest fields: `One` needs one, the others none.
    #[must_use]
    pub fn min(self) -> usize {
        match self {
            Self::One => 1,
            Self::Range { min, .. } => min as usize,
            _ => 0,
        }
    }

    /// The most fields (`None` = unbounded). Mirror of `Count::allows` of `fs-interfaces`.
    #[must_use]
    pub fn max(self) -> Option<usize> {
        match self {
            Self::One | Self::ZeroOrOne => Some(1),
            Self::ZeroToN(max) | Self::Range { max, .. } => max.map(|m| m as usize),
        }
    }

    /// Does a module that maps `n` fields on the channel keep the count?
    #[must_use]
    pub fn allows(self, n: usize) -> bool {
        n >= self.min() && self.max().is_none_or(|m| n <= m)
    }
}

/// One channel of a form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Channel {
    /// Channel name (`label`, `parent` …).
    pub name: String,
    /// The allowed base types: a SET (`text|number|time|flag` for `facts`); a field fits when its type is in the set.
    pub grundtypen: Vec<Grundtyp>,
    /// Required (`!`): the module must map it.
    pub required: bool,
    /// How many fields.
    pub count: Count,
}

/// One form of the form catalog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormDef {
    /// Name (`tree`).
    pub name: String,
    /// Main version.
    pub major: u32,
    /// Minor version.
    pub minor: u32,
    /// The channels.
    pub channels: Vec<Channel>,
}

/// A form version `name@H.N` as a module or component declares it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormRef {
    /// Name.
    pub name: String,
    /// Main version.
    pub major: u32,
    /// Minor version.
    pub minor: u32,
}

/// What a module offers: a form, the version it keeps, and its map channel to field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleForm {
    /// The form and the version the module declares.
    pub form: FormRef,
    /// Channel to field names (a channel may take several fields; a name not in the form is ignored here).
    pub map: Vec<(String, Vec<String>)>,
    /// The basic type of each field of the module, by field name (from the type catalog).
    pub fields: Vec<(String, Basic)>,
}

/// What a block draws (the `forms=` key of the block contract).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentForms {
    /// The block name.
    pub block: String,
    /// The forms it draws.
    pub forms: Vec<FormRef>,
}
