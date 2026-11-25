//! The `Plan` (Bauplan) the host interprets: pure value objects, never a `Result`, every problem inside as a `PartLock` or `Notice`.
//!
//! Pattern: Interpreter input. Same inputs give an equal `Plan` (`Eq`), so the canonical form is equal too.

use crate::codes::{Notice, PartLock};
use crate::keys::KeyPlan;
use crate::menu::MenuPlan;
use crate::model::{ElemId, Range, Target};
use crate::types::{Basic, CommandDesc, FieldDesc, Role, SearchKind};

/// Host state right after building.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanState {
    /// No lock, no notice.
    Ready,
    /// At least one lock or notice; the rest runs.
    Partial,
    /// Nothing drawable; the host shows the notices only.
    Refused,
}

/// The whole window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    /// Frontend key.
    pub key: String,
    /// Stamp of the layout state.
    pub stamp: u64,
    /// Revision of the layout.
    pub rev: u64,
    /// The style to apply, ONLY echoed; no other field may depend on it.
    pub style: Option<String>,
    /// Sources.
    pub sources: Vec<SourcePlan>,
    /// Pages.
    pub pages: Vec<PagePlan>,
    /// Menu.
    pub menu: MenuPlan,
    /// Keys.
    pub keys: KeyPlan,
    /// Window notices.
    pub notices: Vec<Notice>,
    /// State.
    pub state: PlanState,
}

/// Position in `Resolved.sources` (0-based; the layout NUMBER of this source is `idx + 1`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourcePlan {
    /// Index.
    pub idx: u32,
    /// Element id.
    pub elem: ElemId,
    /// Accepted range.
    pub range: Range,
    /// Data type.
    pub data_type: String,
    /// The chosen provider or the lock why there is none.
    pub status: Result<SourceReady, PartLock>,
}

/// A reached source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceReady {
    /// Capability.
    pub capability: String,
    /// Highest version in range.
    pub version: (u32, u32, u32),
    /// Search kind for queries.
    pub search: SearchKind,
}

/// A page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PagePlan {
    /// Name.
    pub page: String,
    /// Element id.
    pub elem: ElemId,
    /// Position.
    pub seq: u32,
    /// Frame block.
    pub frame: String,
    /// Locks of the page itself.
    pub locks: Vec<PartLock>,
    /// The tree of placements; every level in `seq` order (= Tab order).
    pub places: Vec<crate::plan_tree::NodePlan>,
}

/// A placement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlacePlan {
    /// Name.
    pub place: String,
    /// Element id.
    pub elem: ElemId,
    /// Position.
    pub seq: u32,
    /// Block.
    pub block: String,
    /// Source number used (from 1).
    pub source: Option<u32>,
    /// Attributes (`set`).
    pub set: Vec<crate::plan_tree::AttrPlan>,
    /// Drawn or replaced by `partial-notice`.
    pub status: PlaceStatus,
}

/// Drawn or locked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlaceStatus {
    /// Drawn (possibly with locked slots).
    Ready(ReadyPlace),
    /// Not drawn; the host draws `partial-notice` with every lock.
    Locked(Vec<PartLock>),
}

/// A drawn placement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadyPlace {
    /// Slots in binding order.
    pub slots: Vec<SlotFill>,
}

/// `(slot, nth)` of one binding, for reports and H-K15.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
    /// Slot.
    pub slot: String,
    /// Occurrence.
    pub nth: u32,
}

impl ReadyPlace {
    /// The bindings this placement was built with, in order.
    #[must_use]
    pub fn bindings(&self) -> Vec<Binding> {
        self.slots
            .iter()
            .map(|s| Binding {
                slot: s.slot.clone(),
                nth: s.nth,
            })
            .collect()
    }

    /// Locks of single slots (block still drawn).
    #[must_use]
    pub fn slot_locks(&self) -> Vec<&PartLock> {
        self.slots.iter().filter_map(|s| s.lock.as_ref()).collect()
    }
}

/// One bound slot occurrence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotFill {
    /// Slot.
    pub slot: String,
    /// Occurrence.
    pub nth: u32,
    /// Element id.
    pub elem: ElemId,
    /// What it gets.
    pub fill: Fill,
    /// Slot-level lock (block drawn, this occurrence shows the lock).
    pub lock: Option<PartLock>,
}

/// What a slot gets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fill {
    /// Records of a source.
    Records {
        /// Source number (from 1, the layout number).
        source: u32,
    },
    /// A field.
    Field(FieldFill),
    /// A grouping.
    Grouping {
        /// Grouping (field) name.
        name: String,
        /// Source index of the heads (the lanes); `None` = the heads come from the cards.
        heads: Option<u32>,
    },
    /// An order.
    Order(String),
    /// A command.
    Command(CommandPlan),
    /// A filter input (slot kind `condition`, proposal).
    Condition(ConditionFill),
}

/// A field in a role with the ONE block the catalog chose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldFill {
    /// Path.
    pub path: Vec<String>,
    /// Role.
    pub role: Role,
    /// Basic type.
    pub basic: Basic,
    /// Full description.
    pub field: FieldDesc,
    /// `Catalog::choose`; the host never picks.
    pub block: String,
    /// Complete scheme list in suggestion order.
    pub schemes: Option<Vec<String>>,
    /// Input state.
    pub input: InputState,
}

/// Whether input is allowed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputState {
    /// Not an input role.
    NotInput,
    /// Editable.
    Open,
    /// Shown with the reason, never hidden.
    Locked(PartLock),
}

/// A command as bound; visibility per record is decided by `command_state`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandPlan {
    /// Source number (from 1, the layout number).
    pub source: u32,
    /// Name.
    pub name: String,
    /// Description.
    pub description: CommandDesc,
    /// Acts on one record.
    pub on_record: bool,
    /// Static part of the decision.
    pub fixed: Option<PartLock>,
    /// Program or workflow.
    pub target: Target,
}

/// Filter input bound to a field of the records slot of the same block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConditionFill {
    /// Path.
    pub path: Vec<String>,
    /// Basic type.
    pub basic: Basic,
    /// Chip display block: `choose(field, Chip)`.
    pub chip: String,
}
