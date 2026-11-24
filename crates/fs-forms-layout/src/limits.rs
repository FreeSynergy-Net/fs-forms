//! The limits of one description. They are settings (user decision O7, c0 question 57): a person's value, clamped by the
//! instance ceiling (`...-ceiling`, set and locked by an admin in fs-settings). The values here are only the declared defaults
//! and the clamping rule; the caller passes the limits in. There is no fixed ceiling in code except the technical ones below.
//!
//! Pattern: Value Object with a default.

/// Where a limit came from; carried as parameter `by` in every `too-many` and `text-too-long` code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LimitSource {
    /// The declared default.
    Default,
    /// The person's own setting.
    Person,
    /// The instance ceiling clamped it.
    Ceiling,
    /// The technology forces it (with evidence).
    Technical,
}
impl LimitSource {
    /// Word used as the code parameter.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Person => "person",
            Self::Ceiling => "ceiling",
            Self::Technical => "technical",
        }
    }
}

/// The source of each limit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LimitSources {
    /// Source of `elements_max`.
    pub elements: LimitSource,
    /// Source of `menu_depth_max`.
    pub menu_depth: LimitSource,
    /// Source of `place_depth_max`.
    pub place_depth: LimitSource,
    /// Source of `text_bytes_max`.
    pub text_bytes: LimitSource,
    /// Source of `ops_max`.
    pub ops: LimitSource,
    /// Source of `overlays_max`.
    pub overlays: LimitSource,
}

/// Limits applied while checking a description and storing overlays. Values are not measured yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// All elements of one layout.
    pub elements_max: u32,
    /// Depth of the menu tree.
    pub menu_depth_max: u32,
    /// Depth of the tree of places (frames inside frames; a block on the page root = 1).
    pub place_depth_max: u32,
    /// Bytes of one key, name or message.
    pub text_bytes_max: u32,
    /// Ops of one overlay.
    pub ops_max: u32,
    /// Live overlays on one layout.
    pub overlays_max: u32,
    /// Where each value came from.
    pub by: LimitSources,
}

/// Instance ceilings; `None` = the admin set nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CeilingSet {
    /// Ceiling of `elements_max`.
    pub elements_max: Option<u32>,
    /// Ceiling of `menu_depth_max`.
    pub menu_depth_max: Option<u32>,
    /// Ceiling of `place_depth_max`.
    pub place_depth_max: Option<u32>,
    /// Ceiling of `text_bytes_max`.
    pub text_bytes_max: Option<u32>,
    /// Ceiling of `ops_max`.
    pub ops_max: Option<u32>,
    /// Ceiling of `overlays_max`.
    pub overlays_max: Option<u32>,
}

/// Technical limits, each with evidence; everything else is a setting.
pub struct Technical;
impl Technical {
    /// `nth` of a binding must be below this: the catalog's `repeat` is 1..=16 (F1 ENTWURF section 3.1, line 63).
    pub const NTH_MAX: u32 = 16;
    /// capnp-rust 0.27.3 `DEFAULT_READER_OPTIONS`: traversal limit 8 * 1024 * 1024 words (64 MiB), nesting limit 64
    /// (`capnp-0.27.3/src/message.rs:117-120`). A wire message of a description stays far below it at the declared defaults;
    /// whether fs-rpc changes the options is not read yet.
    pub const WIRE_TRAVERSAL_WORDS: u64 = 8 * 1024 * 1024;
    /// capnp-rust 0.27.3 nesting limit of one wire message (same source line).
    pub const WIRE_NESTING: i32 = 64;
}

fn clamp(person: u32, ceiling: Option<u32>, by: LimitSource) -> (u32, LimitSource) {
    match ceiling {
        Some(c) if person > c => (c, LimitSource::Ceiling),
        _ => (person, by),
    }
}

impl Limits {
    /// The declared defaults (= the fs-settings declarations of fs-forms).
    #[must_use]
    pub fn declared() -> Self {
        let d = LimitSource::Default;
        Self {
            elements_max: 4096,
            menu_depth_max: 4,
            place_depth_max: 8,
            text_bytes_max: 16 * 1024,
            ops_max: 1024,
            overlays_max: 64,
            by: LimitSources {
                elements: d,
                menu_depth: d,
                place_depth: d,
                text_bytes: d,
                ops: d,
                overlays: d,
            },
        }
    }

    /// The person's limits clamped by the instance ceilings: effective = min(value, ceiling), the source says who decided.
    #[must_use]
    pub fn effective(person: &Limits, ceiling: &CeilingSet) -> Self {
        let (elements_max, elements) = clamp(
            person.elements_max,
            ceiling.elements_max,
            person.by.elements,
        );
        let (menu_depth_max, menu_depth) = clamp(
            person.menu_depth_max,
            ceiling.menu_depth_max,
            person.by.menu_depth,
        );
        let (place_depth_max, place_depth) = clamp(
            person.place_depth_max,
            ceiling.place_depth_max,
            person.by.place_depth,
        );
        let (text_bytes_max, text_bytes) = clamp(
            person.text_bytes_max,
            ceiling.text_bytes_max,
            person.by.text_bytes,
        );
        let (ops_max, ops) = clamp(person.ops_max, ceiling.ops_max, person.by.ops);
        let (overlays_max, overlays) = clamp(
            person.overlays_max,
            ceiling.overlays_max,
            person.by.overlays,
        );
        Self {
            elements_max,
            menu_depth_max,
            place_depth_max,
            text_bytes_max,
            ops_max,
            overlays_max,
            by: LimitSources {
                elements,
                menu_depth,
                place_depth,
                text_bytes,
                ops,
                overlays,
            },
        }
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self::declared()
    }
}
