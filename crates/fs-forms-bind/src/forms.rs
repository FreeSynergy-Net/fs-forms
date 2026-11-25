//! The data of the form match (ENTSCHEID-form §3): what a form is, what a module offers, what a component draws.
//!
//! A form is a named, versioned list of channels (`tree@1.0`: nodes, parent, label …). The module maps channels to its own
//! fields and declares the form in a version; the component declares which forms it draws; the match puts them together
//! ([`crate::match_form`]). These are plain values: bind reads them from the form catalog (`fs-interfaces`), from the module
//! catalog and from the block catalog (`forms=` of `fs-components`) and does no I/O.
//!
//! Pattern: Value objects (anti-corruption mirrors, like `types.rs`; replaced by the catalog readers when they are pinned).

use crate::types::Basic;

/// The base type of a channel (a channel names a set of them): what kind of value the module must put there.
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
}

/// How many fields a channel takes: from `min` to `max` (`None` = unbounded). The form of `Count` in `fs-interfaces`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Count {
    /// Fewest fields.
    pub min: u32,
    /// Most fields.
    pub max: Option<u32>,
}

impl Count {
    /// Does a module that maps `n` fields on the channel keep the count? Mirror of `Count::allows` of `fs-interfaces`.
    #[must_use]
    pub fn allows(self, n: usize) -> bool {
        u32::try_from(n).is_ok_and(|n| n >= self.min && self.max.is_none_or(|m| n <= m))
    }
}

/// One base type a channel accepts (an `Art` of the form catalog) and the field types that fill it. The field types come from the
/// ONE table of the catalog (`Art::allowed_basics` of `fs-interfaces`); bind holds no table of its own. Empty for `records` and
/// `command`: they name a record type or a command, not a field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Kind {
    /// The base type (its name for messages, and whether it names a field at all).
    pub grundtyp: Grundtyp,
    /// The basic types of a field that fits.
    pub basics: Vec<Basic>,
}

/// One channel of a form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Channel {
    /// Channel name (`label`, `parent` …).
    pub name: String,
    /// The allowed kinds: a SET (`text|number|time|flag` for `facts`); a field fits when its basic type is in one of them.
    pub kinds: Vec<Kind>,
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
    /// The data type this entry serves (forms.json `type`); `""` = not declared (old callers).
    pub data_type: String,
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
