//! The layout input of `Bind::build`: a narrow mirror of the F1b `Resolved` (original + overlays, open conflicts, live problems).
//!
//! Pattern: Anti-corruption mirror. Edge to F1b: replaced by `fs_forms_layout::Resolved` when its probe is pushed.

use crate::types::{Basic, CommandDesc, FieldDesc, Role};

/// Element id of the layout (`forms_*` row).
pub type ElemId = String;

/// A layout problem found by F1b. `code` is the full key (`layout-error-*`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    /// The element it concerns; `None` = the whole layout.
    pub elem: Option<ElemId>,
    /// Full code key.
    pub code: String,
    /// Parameters.
    pub params: Vec<(String, String)>,
}

/// Half-open version range `>=min <max` (`major.minor.patch`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Range {
    /// Inclusive lower bound.
    pub min: (u32, u32, u32),
    /// Exclusive upper bound.
    pub max: (u32, u32, u32),
}

impl Range {
    /// Is the version inside?
    #[must_use]
    pub fn contains(&self, v: (u32, u32, u32)) -> bool {
        v >= self.min && v < self.max
    }

    /// Text form `>=0.2.0, <0.3.0` (the one spelling of `semver::VersionReq`).
    #[must_use]
    pub fn text(&self) -> String {
        format!(
            ">={}.{}.{}, <{}.{}.{}",
            self.min.0, self.min.1, self.min.2, self.max.0, self.max.1, self.max.2
        )
    }
}

/// A data source of the description.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceIn {
    /// Element id.
    pub elem: ElemId,
    /// Capability asked for.
    pub capability: String,
    /// Accepted version range.
    pub range: Range,
    /// Data type served.
    pub data_type: String,
}

/// Where a command goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// A program command.
    Program,
    /// A workflow run in the name of the person.
    Workflow(String),
}

/// A command reference (slot, menu, action).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandIn {
    /// Source index.
    pub source: u32,
    /// Command name.
    pub name: String,
    /// Description from the type catalog.
    pub desc: CommandDesc,
    /// Program or workflow.
    pub target: Target,
}

/// What one slot occurrence is bound to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BindKind {
    /// The records of a source.
    Records(u32),
    /// A field in a role, with the per-use scheme list.
    Field {
        /// Field description.
        field: FieldDesc,
        /// Role.
        role: Role,
        /// Complete scheme list of this use (link, image).
        schemes: Option<Vec<String>>,
    },
    /// A command.
    Command(CommandIn),
    /// A grouping by name.
    Grouping(String),
    /// An order by name.
    Order(String),
    /// A filter input bound to a field (slot kind `condition`, proposal).
    Condition {
        /// Field path.
        path: Vec<String>,
        /// Basic type.
        basic: Basic,
    },
}

/// One bound slot occurrence of a placement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BindingIn {
    /// Element id.
    pub elem: ElemId,
    /// Slot name.
    pub slot: String,
    /// Occurrence index within the slot.
    pub nth: u32,
    /// What it is bound to.
    pub kind: BindKind,
}

/// A placement of a block on a page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaceIn {
    /// Element id.
    pub elem: ElemId,
    /// Placement name.
    pub place: String,
    /// Position (Tab order).
    pub seq: u32,
    /// Block name.
    pub block: String,
    /// Source index used.
    pub source: Option<u32>,
    /// Bindings in binding order.
    pub bindings: Vec<BindingIn>,
}

/// A page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageIn {
    /// Element id.
    pub elem: ElemId,
    /// Page name.
    pub page: String,
    /// Position.
    pub seq: u32,
    /// Frame block.
    pub frame: String,
    /// Placements.
    pub places: Vec<PlaceIn>,
}

/// One node of the menu tree (order = `seq`, never sorted).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuIn {
    /// Element id.
    pub elem: ElemId,
    /// What the node is.
    pub kind: MenuKind,
    /// Sub nodes.
    pub children: Vec<MenuIn>,
}

/// Kind of a menu node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuKind {
    /// A submenu with a label key.
    Menu(String),
    /// A standard menu of the shell (`program`, `help`).
    Standard(String),
    /// A command entry.
    Command {
        /// The command.
        command: CommandIn,
        /// Needs a selected record.
        on_selected: bool,
        /// Shortcut text, e.g. `ctrl+n`.
        shortcut: Option<String>,
    },
    /// A view switch.
    View(String),
    /// A separator.
    Separator,
}

/// The resolved layout (layer 3 of F1b).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved {
    /// Frontend key.
    pub key: String,
    /// Stamp of the state it was built from.
    pub stamp: u64,
    /// Revision.
    pub rev: u64,
    /// Style named by the description.
    pub style: Option<String>,
    /// Sources.
    pub sources: Vec<SourceIn>,
    /// Pages.
    pub pages: Vec<PageIn>,
    /// Menu roots.
    pub menu: Vec<MenuIn>,
    /// Locking problems found by F1b (missing or unreachable provider, mismatching range, broken elements).
    pub problems: Vec<Problem>,
    /// Warnings found by F1b (e.g. `layout-error-provider-unchecked`): shown as notices, never a lock (c0 86).
    pub warnings: Vec<Problem>,
    /// Open overlay conflicts.
    pub conflicts: Vec<Problem>,
}
