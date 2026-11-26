//! Layer 1: the raw description as read from a file or the wire. Names are not yet checked against the catalog and `elem` may be
//! missing (format 1). Every element carries the line it came from (0 = wire).
//!
//! Pattern: Value Object: plain data, checked by [`LayoutDoc::check`](crate::LayoutDoc).

use crate::ids::{ElemId, FrontendKey};
use crate::shim::Reference;

/// A raw element: its optional id, its source line, its value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item<T> {
    /// Element id; mandatory from format 2 on.
    pub elem: Option<ElemId>,
    /// Line in the file, 0 for the wire.
    pub line: u32,
    /// The value.
    pub v: T,
}

/// A raw description.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutDoc {
    /// Exchange format, 1 or 2.
    pub format: u32,
    /// Program and frontend name.
    pub key: FrontendKey,
    /// The host binary that runs it.
    pub exec: String,
    /// MF2 key of the title.
    pub title_key: String,
    /// Named style.
    pub style: Option<String>,
    /// Contracts the description needs.
    pub requires: Vec<Item<RequiresDoc>>,
    /// Data sources, numbered by position starting at 1.
    pub sources: Vec<Item<SourceDoc>>,
    /// Menu tree.
    pub menu: Vec<Item<MenuDoc>>,
    /// Pages.
    pub pages: Vec<Item<PageDoc>>,
    /// Actions.
    pub actions: Vec<Item<ActionDoc>>,
    /// Views.
    pub views: Vec<Item<ViewDoc>>,
    /// Texts (MF2 source) by key and language.
    pub messages: Vec<Item<MessageDoc>>,
    /// The section `[sample]` as TOML text with the top-level key `sample` (the test data the description asks for), or `None`. The
    /// layer does NOT check it: its one reader is `fs-forms-testdata` (`read_sample`, strict). Wire and store do not carry it (`None`).
    pub sample: Option<String>,
}

/// A required contract range, e.g. `tasks@>=0.2, <0.3`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequiresDoc {
    /// The range text.
    pub range: String,
}
/// A data source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceDoc {
    /// Contract range text.
    pub range: String,
    /// Name of the data type.
    pub data_type: String,
    /// Which records.
    pub scope: Option<SourceScope>,
}
/// A menu entry; `children` make the tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuDoc {
    /// Kind of entry.
    pub kind: MenuKind,
    /// MF2 key of the label.
    pub label_key: Option<String>,
    /// (source number, command name).
    pub command: Option<(u32, String)>,
    /// When it applies.
    pub on: Option<OnWhat>,
    /// Keyboard shortcut.
    pub shortcut: Option<String>,
    /// Name of a view.
    pub view: Option<String>,
    /// Sub entries.
    pub children: Vec<Item<MenuDoc>>,
}
/// A page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageDoc {
    /// Page name, unique in the layout.
    pub page: String,
    /// Frame (window shape).
    pub frame: String,
    /// Navigation kind.
    pub navigation: Navigation,
    /// Blocks of format 2 (flat list): the exchange no longer fills it (format 3 has the tree `places`); the stores and the wire still do. Use [`PageDoc::leaves`] to see all blocks.
    pub blocks: Vec<Item<BlockDoc>>,
    /// Tree of places (format 3): blocks and frames. The exchange reads and writes it; wire and tables do not carry it yet.
    pub places: Vec<Item<crate::place::PlaceNode>>,
}
/// A block placed on a page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockDoc {
    /// Place on the page, unique per page.
    pub place: String,
    /// Block name in the component catalog.
    pub block: String,
    /// Source number: counts from 1 (`source = N` = the N-th `[[source]]`); the same numbers in `fs-forms-bind` and `fs-forms-run`.
    pub source: Option<u32>,
    /// MF2 key of the title.
    pub title_key: Option<String>,
    /// MF2 key of the text.
    pub text_key: Option<String>,
    /// Bindings of the block's slots.
    pub bindings: Vec<Item<BindingDoc>>,
    /// Attributes (`set`), unique by name; the keys must be in the `sets` of the block in the catalog (check K20, a warning).
    pub set: Vec<Item<crate::place::AttrDoc>>,
}
/// A binding of one slot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BindingDoc {
    /// Slot name.
    pub slot: String,
    /// Position among bindings of the same slot.
    pub nth: u32,
    /// What it binds to.
    pub from: BindFrom,
    /// Scheme list, in suggestion order, for fields whose type carries schemes.
    pub schemes: Option<Vec<Item<String>>>,
    /// Source number, counts from 1 (format 4; only for `grouping`): the heads of the groups come from that source instead of the block's.
    pub source: Option<u32>,
}
/// An action.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionDoc {
    /// What triggers it.
    pub trigger: Trigger,
    /// The place (element id of a `[[page.place]]` with a block) whose command slot this action binds: `trigger` is then the NAME of that slot
    /// (`drop`, `menu`, `quick-add`, `activate`). `None`: a trigger of the page (`button`, `key`, `submit`).
    pub at: Option<crate::ids::ElemId>,
    /// What it does.
    pub target: ActionTarget,
    /// Confirmation.
    pub confirm: Option<Confirm>,
}
/// A view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewDoc {
    /// View name.
    pub view: String,
    /// Owner of a personal view.
    pub owner: Option<Reference>,
    /// Grouping name.
    pub grouping: Option<String>,
    /// Orders.
    pub orders: Vec<Item<OrderDoc>>,
    /// Filter tree.
    pub filter: Option<Item<FilterDoc>>,
    /// Kind of search.
    pub search_kind: Option<String>,
}
/// One order of a view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrderDoc {
    /// By field or by named order.
    pub by: OrderBy,
    /// Field path or order name.
    pub name: String,
    /// Descending.
    pub descending: bool,
}
/// A filter tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FilterDoc {
    /// Combination.
    Op {
        /// The operator.
        op: FilterOp,
        /// Operands.
        children: Vec<Item<FilterDoc>>,
    },
    /// A condition.
    Is {
        /// The condition.
        cond: FilterCond,
        /// Field path.
        path: String,
        /// Single value.
        value: Option<String>,
        /// Range start.
        low: Option<String>,
        /// Range end.
        high: Option<String>,
        /// Value list.
        values: Vec<Item<String>>,
    },
}
/// A text in one language.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MessageDoc {
    /// MF2 key.
    pub key: String,
    /// Language (BCP 47).
    pub lang: String,
    /// MF2 source text.
    pub message: String,
}

pub use crate::doc_words::*;
