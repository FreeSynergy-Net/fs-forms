//! The closed format words of a description (the enum lists of the tables) and the binding source.
//!
//! Pattern: Value Object: closed enums with a 1:1 text form in the exchange format.

use crate::shim::Reference;

/// Which records a source delivers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceScope {
    /// All.
    All,
    /// The selected list.
    SelectedList,
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
