//! Overlays: changes a person or the instance lays over a package original without touching it. Ops name elements by id.
//!
//! Pattern: Command: an op is a stored, replayable change request; `apply` replays them over the original.

use crate::ids::{ElemId, ElemKind, ElemRef, LayoutId, OverlayId, PackageRef, Rev, Who};
use crate::shim::{Reference, Utc};

/// Whose overlay it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OverlayScope {
    /// The instance (admin).
    Instance,
    /// One person.
    Person(Reference),
}
/// State of an overlay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayState {
    /// Acts.
    Applied,
    /// At least one op conflicts.
    Conflict,
    /// Its original is gone (uninstalled).
    Orphaned,
    /// Switched off.
    Off,
}
/// An overlay with its ops.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Overlay {
    /// Id.
    pub id: OverlayId,
    /// The layout it lies over.
    pub layout: LayoutId,
    /// Whose it is.
    pub scope: OverlayScope,
    /// MF2 key of its title.
    pub title_key: Option<String>,
    /// Language.
    pub lang: Option<String>,
    /// Who made it.
    pub who: Who,
    /// The package version it was made against.
    pub base: PackageRef,
    /// State.
    pub state: OverlayState,
    /// Counter.
    pub rev: Rev,
    /// The ops by `seq`.
    pub ops: Vec<Op>,
}
/// One op, numbered and stamped by the service.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Op {
    /// Position, from 0.
    pub seq: u32,
    /// What it does.
    pub kind: OpKind,
    /// Who made it.
    pub who: Who,
    /// When (set by the service).
    pub at: Utc,
}
/// What an op does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpKind {
    /// Sets a field; `expect` is the value the original had (filled by the store, never trusted from the caller).
    Set {
        /// Element.
        target: ElemRef,
        /// Field.
        field: Field,
        /// New value.
        value: Value,
        /// Value the author saw.
        expect: Value,
    },
    /// Removes an element.
    Remove {
        /// Element.
        target: ElemRef,
    },
    /// Inserts a new element (its fields follow as `Set` ops). Not applied yet in this cut.
    Insert {
        /// Kind of the new element.
        kind: ElemKind,
        /// Its id.
        new_elem: ElemId,
        /// Neighbour.
        anchor: ElemId,
        /// Where, relative to the anchor.
        position: Position,
    },
    /// Moves an element. Not applied yet in this cut.
    Move {
        /// Element.
        target: ElemRef,
        /// Neighbour.
        anchor: ElemId,
        /// Where, relative to the anchor.
        position: Position,
    },
}
/// Position relative to an anchor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Position {
    /// Before it.
    Before,
    /// After it.
    After,
    /// First child of it.
    FirstChild,
    /// Last child of it.
    LastChild,
}
/// A value an op sets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    /// Unset.
    None,
    /// Text.
    Text(String),
    /// Integer.
    Int(i64),
    /// Boolean.
    Bool(bool),
}

macro_rules! fields {
    ($($v:ident => [$($k:ident),*]),* $(,)?) => {
        /// The closed list of settable fields (keys of the model, not column names).
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Field { $(
            #[doc = stringify!($v)]
            $v
        ),* }
        impl Field {
            /// Can this field be set on elements of this kind?
            #[must_use]
            pub fn applies_to(self, kind: ElemKind) -> bool {
                match self { $(Self::$v => matches!(kind, $(ElemKind::$k)|*)),* }
            }
        }
    };
}
fields! {
    TitleKey => [Layout, Block], Style => [Layout], LabelKey => [Menu], Shortcut => [Menu], OnWhat => [Menu],
    Command => [Menu, Action], View => [Menu], TextKey => [Block], Frame => [Page], Navigation => [Page],
    Block => [Block], Source => [Block], Path => [Binding], Role => [Binding], Name => [ViewOrder],
    Scheme => [BindingScheme], Message => [Message], Grouping => [View], SearchKind => [View], Descending => [ViewOrder],
    Value => [FilterNode, FilterValue], Low => [FilterNode], High => [FilterNode], Trigger => [Action], Confirm => [Action],
    Workflow => [Action], Range => [Requires, Source], DataType => [Source],
}

/// Why an op does not act.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause {
    /// The target is gone.
    TargetRemoved,
    /// The target's value changed since the author saw it.
    TargetChanged,
    /// The anchor is gone.
    AnchorRemoved,
    /// The field is not settable on this kind (or not supported yet).
    FieldUnknown,
    /// The value has the wrong type.
    TypeMismatch,
    /// A lock forbids it.
    Locked,
    /// The result would break a rule of the layout.
    ResultInvalid,
}
impl Cause {
    /// Kebab-case word used as code parameter.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TargetRemoved => "target-removed",
            Self::TargetChanged => "target-changed",
            Self::AnchorRemoved => "anchor-removed",
            Self::FieldUnknown => "field-unknown",
            Self::TypeMismatch => "type-mismatch",
            Self::Locked => "locked",
            Self::ResultInvalid => "result-invalid",
        }
    }
}
/// How a person resolves a conflict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolution {
    /// Keep the op against the new value.
    Keep,
    /// Drop the op.
    Drop,
    /// Aim the op at another element of the same kind.
    Retarget(ElemId),
}
/// A stored conflict of one op; never deleted automatically.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conflict {
    /// The overlay.
    pub overlay: OverlayId,
    /// The op.
    pub seq: u32,
    /// Package version at the time.
    pub base_version: String,
    /// Why.
    pub cause: Cause,
    /// The value found.
    pub found: Value,
    /// When.
    pub at: Utc,
    /// Still open.
    pub open: bool,
}
/// One instance lock; `target = None` locks the whole layout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ElemLock {
    /// What is locked.
    pub target: Option<ElemRef>,
    /// Why (mandatory).
    pub reason: String,
    /// By whom.
    pub locked_by: Reference,
    /// When.
    pub locked_at: Utc,
}
