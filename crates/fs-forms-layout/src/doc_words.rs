//! The closed format words of a description (the enum lists of the tables) and the binding source.
//!
//! Pattern: Value Object: closed enums with a 1:1 text form in the exchange format.

use crate::scope_token::ScopeToken;
use crate::shim::Reference;

/// Which records a source delivers. `All` and `SelectedList` are words of the host; any other valid token goes unchanged to the module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceScope {
    /// All.
    All,
    /// The selected list.
    SelectedList,
    /// A word of the module (never `all` or `selected-list`: those are read as the host words).
    Module(ScopeToken),
}
impl SourceScope {
    /// The word of the exchange file.
    #[must_use]
    pub fn text(&self) -> &str {
        match self {
            Self::All => "all",
            Self::SelectedList => "selected-list",
            Self::Module(t) => t.as_str(),
        }
    }
    /// The scope of a word: a host word, or a valid [`ScopeToken`]; `None` when the grammar is broken.
    #[must_use]
    pub fn from_text(w: &str) -> Option<Self> {
        match w {
            "all" => Some(Self::All),
            "selected-list" => Some(Self::SelectedList),
            _ => ScopeToken::parse(w).map(Self::Module),
        }
    }
}
/// Kind of a menu entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuKind {
    /// A menu with children.
    Menu,
    /// A standard menu.
    Standard(MenuStandard),
    /// A command.
    Command,
    /// A view.
    View,
    /// A separator.
    Separator,
}
/// The standard menus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuStandard {
    /// Program menu.
    Program,
    /// Help menu.
    Help,
}
/// When a menu entry applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnWhat {
    /// Always.
    Always,
    /// On a selection.
    Selected,
}
/// Navigation kind of a page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Navigation {
    /// One view.
    Single,
    /// List and detail.
    ListDetail,
}
/// What a binding binds to (the catalog's source minus the contract).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BindFrom {
    /// The records.
    Records,
    /// A field; `role` is the catalog's role word.
    Field {
        /// Field path.
        path: String,
        /// Role word.
        role: String,
    },
    /// A grouping.
    Grouping(String),
    /// An order.
    Order(String),
    /// A command.
    Command(String),
    /// The lock state.
    Lock,
    /// A condition (filter bar).
    Condition,
}
/// What triggers an action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trigger {
    /// Button.
    Button,
    /// Menu.
    Menu,
    /// Key.
    Key,
    /// Drop.
    Drop,
    /// Quick add.
    QuickAdd,
    /// A form (fields) sent: the values of the fields go to a command channel of the form (SW-3).
    Submit,
}
/// What an action does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionTarget {
    /// A command of a source.
    Command {
        /// Source number.
        source: u32,
        /// Command name.
        command: String,
    },
    /// A workflow.
    Workflow(Reference),
    /// Opens the frontend node with this id (`PlaceNode::Frontend`) in its own `mode`.
    Open {
        /// Element id of the frontend node (checked: it must name one).
        elem: crate::ids::ElemId,
    },
}
/// How an embedded frontend appears; the parent node decides, the child frontend is the same in both cases.
///
/// Pattern: Strategy (the parent chooses the way); "Tab" is a panel in a frame `tabs`, no mode of its own.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Placement {
    /// Inside the frame of the parent, on its place.
    #[default]
    Panel,
    /// In a window of its own.
    Window,
}
/// When an embedded frontend is opened.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Shown {
    /// With the parent.
    #[default]
    Start,
    /// Only by an action `open`.
    OnOpen,
}
/// Confirmation of an action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Confirm {
    /// None.
    None,
    /// Ask.
    Confirm,
    /// Offer a choice.
    Choice,
}
/// What an order sorts by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderBy {
    /// A field.
    Field,
    /// A named order.
    Order,
}
/// Filter operators.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterOp {
    /// And.
    And,
    /// Or.
    Or,
    /// Not.
    Not,
    /// Xor.
    Xor,
    /// A single condition.
    Is,
}
/// Filter conditions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterCond {
    /// Equals.
    Equals,
    /// Any of.
    AnyOf,
    /// In a range.
    Range,
    /// Is empty.
    IsEmpty,
    /// Matches a term.
    Term,
}
