//! `BindCode`, `Part`, `Cause`, `PartLock`, `Notice`: every problem of the plan is a `PartLock` on the smallest part (or a window `Notice`).
//!
//! Pattern: Value Objects + closed code list (append only, never renamed). Texts are MF2 keys `bind-error-<code>`.

use crate::coded::{Coded, Family};

macro_rules! bind_codes {
    ($($variant:ident = $name:literal),+ $(,)?) => {
        /// Codes of bind (prefix `bind-error-`). Parameters are named at each use.
        #[non_exhaustive]
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum BindCode { $(
            #[doc = concat!("`bind-error-", $name, "`.")]
            $variant,
        )+ }
        impl BindCode {
            /// Every code once.
            pub const ALL: &'static [BindCode] = &[$(BindCode::$variant),+];
            /// The kebab name without prefix.
            #[must_use]
            pub fn name(self) -> &'static str { match self { $(BindCode::$variant => $name),+ } }
        }
    };
}

bind_codes! {
    Locked = "locked", ProviderUnreachable = "provider-unreachable", BlockNotBuilt = "block-not-built",
    FieldComponentMissing = "field-component-missing", WriteLater = "write-later",
    WorkflowRuntimeMissing = "workflow-runtime-missing", Coming = "coming", ShortcutConflict = "shortcut-conflict",
    ShortcutInvalid = "shortcut-invalid", KeyReserved = "key-reserved", StandardTwice = "standard-twice",
    SeqTwice = "seq-twice", PageEmpty = "page-empty", PageRefused = "page-refused",
    FieldHidden = "field-hidden", SettingsDefaults = "settings-defaults", NoSelection = "no-selection",
    ElemOrphan = "elem-orphan",
}

impl BindCode {
    /// Prefix of every key of this family.
    pub const PREFIX: &'static str = "bind-error-";

    /// The full key, e.g. `bind-error-locked`.
    #[must_use]
    pub fn key(self) -> String {
        format!("{}{}", Self::PREFIX, self.name())
    }

    /// A coded value of this code (parameters added with `Coded::with`).
    #[must_use]
    pub fn coded(self) -> Coded {
        Coded::new(&self.key())
    }
}

/// The smallest part a lock concerns.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Part {
    /// The whole window.
    Window,
    /// A source by index.
    Source(u32),
    /// A page.
    Page(String),
    /// A placement.
    Place {
        /// Page name.
        page: String,
        /// Placement name.
        place: String,
    },
    /// One bound slot occurrence.
    Slot {
        /// Page name.
        page: String,
        /// Placement name.
        place: String,
        /// Slot name.
        slot: String,
        /// Occurrence index.
        nth: u32,
    },
    /// A menu entry by element id.
    Menu(String),
    /// A command by source index and name.
    Command {
        /// Source index.
        source: u32,
        /// Command name.
        name: String,
    },
}

impl Part {
    /// Stable text form (`place:<page>/<place>` ...), used in codes, CLI and tests.
    #[must_use]
    pub fn text(&self) -> String {
        match self {
            Part::Window => "window".to_owned(),
            Part::Source(n) => format!("source:{n}"),
            Part::Page(p) => format!("page:{p}"),
            Part::Place { page, place } => format!("place:{page}/{place}"),
            Part::Slot {
                page,
                place,
                slot,
                nth,
            } => format!("slot:{page}/{place}/{slot}/{nth}"),
            Part::Menu(e) => format!("menu:{e}"),
            Part::Command { source, name } => format!("command:{source}/{name}"),
        }
    }
}

/// The producer family of a cause, passed through unchanged (never re-coded).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cause {
    /// `accepts-error-*`.
    Accepts(Coded),
    /// `layout-error-*`.
    Layout(Coded),
    /// `records-error-*`.
    Records(Coded),
    /// `bind-error-*` (also the home of codes without a known prefix, so the suite can see them).
    Bind(Coded),
}

impl Cause {
    /// Sorts a coded value into its family variant.
    #[must_use]
    pub fn from_coded(c: Coded) -> Self {
        match c.family() {
            Family::Accepts => Cause::Accepts(c),
            Family::Layout => Cause::Layout(c),
            Family::Records => Cause::Records(c),
            Family::Bind | Family::Unknown => Cause::Bind(c),
        }
    }

    /// The code inside.
    #[must_use]
    pub fn coded(&self) -> &Coded {
        match self {
            Cause::Accepts(c) | Cause::Layout(c) | Cause::Records(c) | Cause::Bind(c) => c,
        }
    }
}

/// How a locked part appears. There is deliberately no "hidden": a command without right is not a lock and has no code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    /// Replaced by `partial-notice` (placement) or shown with the lock text (slot, entry).
    Locked,
    /// Entry shown, disabled, text "kommt".
    Coming,
}

/// One lock on a part: where, how, why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartLock {
    /// The smallest part concerned.
    pub part: Part,
    /// How it appears.
    pub severity: Severity,
    /// The cause, unchanged from its producer.
    pub cause: Cause,
}

impl PartLock {
    /// A locked part with a cause.
    #[must_use]
    pub fn locked(part: Part, cause: Coded) -> Self {
        Self {
            part,
            severity: Severity::Locked,
            cause: Cause::from_coded(cause),
        }
    }

    /// `bind-error-locked{part, cause, cause-*}`: the one coded value the window renders.
    #[must_use]
    pub fn coded(&self) -> Coded {
        let cause = self.cause.coded();
        let mut c = BindCode::Locked
            .coded()
            .with("part", self.part.text())
            .with("cause", cause.code.clone());
        for (k, v) in &cause.params {
            c = c.with(&format!("cause-{k}"), v.clone());
        }
        c
    }
}

/// A whole-window notice (status line).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notice {
    /// The cause.
    pub cause: Cause,
}

impl Notice {
    /// A notice from a coded value.
    #[must_use]
    pub fn new(c: Coded) -> Self {
        Self {
            cause: Cause::from_coded(c),
        }
    }
}
