//! What the host reads of the plan: a narrow projection of the `Plan` of `fs-forms-bind` (probe H1, `7dde60c` plus the builder's
//! unpushed work): the window state, pages with placements in `seq` order, the locks and notices as codes with parameters, the
//! menu. It carries no decision: which block to draw was chosen by the catalog inside the plan.
//!
//! Pattern: Anti-corruption mirror (value objects). Replaced by `From<&fs_forms_bind::Plan>` when that crate is pinned.

/// A code with its parameters (`bind-error-locked{part, cause}` and the codes it carries).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Coded {
    /// The full code, e.g. `bind-error-provider-unreachable`.
    pub code: String,
    /// Named parameters.
    pub params: Vec<(String, String)>,
}

impl Coded {
    /// A code without parameters.
    pub fn new(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            params: Vec::new(),
        }
    }

    /// With one more parameter.
    #[must_use]
    pub fn with(mut self, name: &str, value: impl Into<String>) -> Self {
        self.params.push((name.to_owned(), value.into()));
        self
    }
}

/// One placement of a page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaceView {
    /// The name.
    pub place: String,
    /// The position (Tab order).
    pub seq: u32,
    /// The block the catalog chose.
    pub block: String,
    /// The words of the placement's title (already in the reader's language), if the description gives one.
    pub title: Option<String>,
    /// Not drawn: every lock (the host draws a `partial-notice` with all of them). Empty = drawn.
    pub locks: Vec<Coded>,
    /// Locks of single slots (the block is still drawn).
    pub slot_locks: Vec<Coded>,
}

/// One page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageView {
    /// The name.
    pub page: String,
    /// The position among the pages.
    pub seq: u32,
    /// The frame block.
    pub frame: String,
    /// Locks of the page itself.
    pub locks: Vec<Coded>,
    /// The placements.
    pub places: Vec<PlaceView>,
}

/// What a menu entry does now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuState {
    /// Works.
    Enabled,
    /// Locked, says "coming".
    Coming,
    /// Not shown (no right; rights come before "coming").
    Hidden,
}

/// One entry of the program menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuItem {
    /// The command id.
    pub id: String,
    /// The words of the entry (already in the reader's language, from the layout's MF2).
    pub label: String,
    /// What it does now.
    pub state: MenuState,
}

/// One `[[action]]` of the description: what triggers it (`quick-add`, `drop`, `submit`, `button`, `menu`, `key`) and the command of the
/// source it calls. The host connects a gesture of a block to a record operation only through these.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionView {
    /// The trigger.
    pub trigger: String,
    /// The command name (`create`, `move`, ...); actions that start a workflow are not read here.
    pub command: String,
}

/// The plan as the host reads it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct View {
    /// The actions of the description.
    pub actions: Vec<ActionView>,
    /// The contract range of each `[[source]]` (`tasks@>=0.2, <0.3`), in order: what the host picks its records by.
    pub sources: Vec<String>,
    /// The frontend key.
    pub frontend: String,
    /// The title of the window.
    pub title: String,
    /// The style the description names (`default` or none: the style set for the program).
    pub style: Option<String>,
    /// The pages.
    pub pages: Vec<PageView>,
    /// The program menu.
    pub menu: Vec<MenuItem>,
    /// Notices of the whole window.
    pub notices: Vec<Coded>,
}
