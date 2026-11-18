//! The family `forms_*` (domain `forms`), part 1: word lists, the layout tables, the log of the description itself.
//! Source of the columns: `trait-runde/forms-f1/ENTWURF.md` section 16.2 and 16.8, changes of F1b marked `F1b:`. The domain lives in
//! the PROGRAM (pattern fs-tasks-local), not in fs-objects: only `fs-forms serve` opens this file (decision D2).
//!
//! Pattern: Value Object (const table specs, one source for DDL and column names).

use crate::spec::{
    Col,
    Logical::{Enum, Int, Iri, Ref, Text, Utc},
    Table,
};

/// `registry.rs:21` RESERVED = object, activity, person, fs, setting: `forms` is free (exact compare).
pub const PREFIX: &str = "forms";

// ---- closed technical word lists (ADR 0086 addendum d3 decision 7: `CHECK IN` only for closed format words the code knows fully).
// Names that are freely assigned identifiers (frontend, page, place, view, elem; and references into catalogs owned elsewhere:
// block, slot, command, type, capability, grouping, order) are `Text`, checked in CODE against the block catalog
// (`fs-components-catalog`) and the provider's `TypeCatalog`/`Shape`. They are NOT a vocabulary of words with labels, so ADR 0086
// addendum d3 (word tables) does not apply to them (decision D7, c0 question 56 = JA; no exception).
pub const ORIGIN: &[&str] = &["package", "person", "import"];
pub const MENU_KIND: &[&str] = &["menu", "standard", "command", "view", "separator"];
pub const MENU_STANDARD: &[&str] = &["program", "help"];
pub const ON_WHAT: &[&str] = &["always", "selected"];
pub const SCOPE_SRC: &[&str] = &["all", "selected-list"];
pub const NAVIGATION: &[&str] = &["single", "list-detail"];
/// + `condition` (filter bar -> filter tree; host plan question 73, c0 open): added BEFORE step 1 is frozen, so no rebuild later.
pub const FROM: &[&str] = &[
    "records",
    "field",
    "grouping",
    "order",
    "command",
    "lock",
    "condition",
];
pub const ROLE: &[&str] = &["input", "display", "compact", "cell"];
pub const TRIGGER: &[&str] = &["button", "menu", "key", "drop", "quick-add"];
pub const TARGET: &[&str] = &["command", "workflow"];
pub const CONFIRM: &[&str] = &["none", "confirm", "choice"];
pub const VIEW_SCOPE: &[&str] = &["all", "person"];
pub const SEARCH_KIND: &[&str] = &[
    "complete",
    "open",
    "cancellable",
    "deadline",
    "enough",
    "live",
    "relevant",
];
pub const ORDER_BY: &[&str] = &["field", "order"];
pub const FILTER_OP: &[&str] = &["and", "or", "not", "xor", "is"];
pub const FILTER_COND: &[&str] = &["equals", "any-of", "range", "is-empty", "term"];
pub const OVERLAY_SCOPE: &[&str] = &["instance", "person"];
pub const OVERLAY_STATE: &[&str] = &["applied", "conflict", "orphaned", "off"];
pub const OP: &[&str] = &["set", "remove", "insert", "move"];
/// F1b: + `binding-scheme` (the per-use scheme list, user 32.4).
pub const TARGET_KIND: &[&str] = &[
    "layout",
    "menu",
    "source",
    "page",
    "block",
    "binding",
    "binding-scheme",
    "action",
    "view",
    "view-order",
    "filter-node",
    "filter-value",
    "message",
    "requires",
];
pub const POSITION: &[&str] = &["before", "after", "first-child", "last-child"];
/// F1b: + `locked` (instance lock, question 50) and `result-invalid` (the op would break a rule of the layout, D9).
pub const CAUSE: &[&str] = &[
    "target-removed",
    "target-changed",
    "anchor-removed",
    "field-unknown",
    "type-mismatch",
    "locked",
    "result-invalid",
];
pub const RESOLUTION: &[&str] = &["open", "keep", "drop", "retarget"];
pub const LOG_ACTION: &[&str] = &[
    "create",
    "save",
    "delete",
    "import",
    "install",
    "update",
    "uninstall",
    "overlay-create",
    "overlay-ops",
    "overlay-state",
    "overlay-delete",
    "overlay-import",
    "conflict-resolve",
    "lock",
    "unlock",
];

/// The stable element id column every element table carries (section 16.8, c0 question 8 = JA). Rule in code: `ElemId` (`b_model.rs`).
pub(crate) const ELEM: Col = Col::req("elem", Text);
/// One rule for "the element id is unique in its layout" (per table: an overlay op names `target_kind` + `target_elem`).
pub(crate) const ELEM_UNIQUE: &str = "UNIQUE (layout, elem)";

/// One counter; every write transaction does `rev = rev + 1` -> `Stamp` (feed, live update of the host).
pub const REV: Table = Table {
    name: "forms_rev",
    cols: &[Col::req("slot", Int), Col::req("rev", Int)],
    constraints: &["PRIMARY KEY (slot)", "CHECK (slot = 0)", "CHECK (rev >= 0)"],
    indexes: &[],
};

/// One description (window/frontend). F1b changes vs §16.2: `frontend_key` (NULL = tombstone, frees the key; pattern
/// `settings_profile.name_key`), `deleted`; uniqueness = (`extends`, `frontend_key`) over ALL origins (D5, question 52: no shadowing);
/// owner NULL exactly for packages; package + version exactly for packages.
pub const LAYOUT: Table = Table {
    name: "forms_layout",
    cols: &[
        Col::req("id", Ref),            // UUIDv7
        Col::req("extends", Text),      // AppStream id of the program, e.g. net.freesynergy.Tasks
        Col::req("frontend", Text), // FrontendId (ADR 0094 point 6), kept on the tombstone for the log
        Col::opt("frontend_key", Text), // = frontend while alive; NULL on the tombstone
        Col::req("exec", Text),     // manifest `fs:exec`, e.g. fs-forms-run
        Col::req("title_key", Text), // MF2 key
        Col::opt("style", Text), // StyleId; the person's choice `fs.shell.style` (fs-settings) wins
        Col::req("format", Int), // exchange format it came in (1 or 2), for the report only
        Col::req("origin", Enum(ORIGIN)),
        Col::opt("package", Text),
        Col::opt("package_version", Text),
        Col::opt("owner", Iri), // the person (PersonRef IRI) for person/import
        Col::req("rev", Int),
        Col::req("created", Utc),
        Col::req("updated", Utc),
        Col::req("updated_by", Iri),
        Col::opt("deleted", Utc),
    ],
    constraints: &[
        "PRIMARY KEY (id)",
        "UNIQUE (extends, frontend_key)",
        "CHECK ((origin = 'package') AND (owner IS NULL) OR NOT (origin = 'package') AND NOT (owner IS NULL))",
        "CHECK ((origin = 'package') AND (package IS NOT NULL AND package_version IS NOT NULL) OR NOT (origin = 'package') AND NOT (package IS NOT NULL AND package_version IS NOT NULL))",
        "CHECK ((deleted IS NULL) AND (frontend_key IS NOT NULL) OR NOT (deleted IS NULL) AND NOT (frontend_key IS NOT NULL))",
        "CHECK (frontend_key IS NULL OR frontend_key = frontend)",
        "CHECK (format >= 1)",
        "CHECK (rev >= 0)",
    ],
    indexes: &[
        "CREATE INDEX IF NOT EXISTS forms_layout_package ON forms_layout (package, extends, frontend)",
    ],
};

/// `ContractRange` typed, never the text ">=0.2 <0.3".
pub const REQUIRES: Table = Table {
    name: "forms_requires",
    cols: &[
        Col::req("layout", Ref),
        Col::req("seq", Int),
        ELEM,
        Col::req("capability", Text),
        Col::req("min_major", Int),
        Col::req("min_minor", Int),
        Col::req("below_major", Int),
        Col::req("below_minor", Int),
    ],
    constraints: &[
        "PRIMARY KEY (layout, seq)",
        ELEM_UNIQUE,
        "FOREIGN KEY (layout) REFERENCES forms_layout(id)",
        "CHECK ((below_major > min_major) OR (below_major = min_major AND below_minor > min_minor))",
    ],
    indexes: &[],
};

pub const SOURCE: Table = Table {
    name: "forms_source",
    cols: &[
        Col::req("layout", Ref),
        Col::req("source", Int),
        ELEM,
        Col::req("capability", Text),
        Col::req("min_major", Int),
        Col::req("min_minor", Int),
        Col::req("below_major", Int),
        Col::req("below_minor", Int),
        Col::req("data_type", Text),
        Col::opt("scope", Enum(SCOPE_SRC)),
    ],
    constraints: &[
        "PRIMARY KEY (layout, source)",
        ELEM_UNIQUE,
        "FOREIGN KEY (layout) REFERENCES forms_layout(id)",
    ],
    indexes: &[],
};

/// The menu tree (Vorbild 1:1, rule "Menüs nachbauen"). Pair checks per `kind` as in §16.2.
pub const MENU: Table = Table {
    name: "forms_menu",
    cols: &[
        Col::req("layout", Ref),
        Col::req("node", Int),
        ELEM,
        Col::opt("parent", Int),
        Col::req("seq", Int),
        Col::req("kind", Enum(MENU_KIND)),
        Col::opt("label_key", Text),
        Col::opt("standard", Enum(MENU_STANDARD)),
        Col::opt("command", Text),
        Col::opt("source", Int),
        Col::opt("on_what", Enum(ON_WHAT)),
        Col::opt("shortcut", Text),
        Col::opt("view", Text),
    ],
    constraints: &[
        "PRIMARY KEY (layout, node)",
        ELEM_UNIQUE,
        "UNIQUE (layout, parent, seq)",
        "FOREIGN KEY (layout) REFERENCES forms_layout(id)",
        "CHECK ((kind = 'standard') AND (standard IS NOT NULL) OR NOT (kind = 'standard') AND NOT (standard IS NOT NULL))",
        "CHECK ((kind = 'command') AND (command IS NOT NULL AND source IS NOT NULL) OR NOT (kind = 'command') AND NOT (command IS NOT NULL AND source IS NOT NULL))",
        "CHECK ((kind = 'view') AND (view IS NOT NULL) OR NOT (kind = 'view') AND NOT (view IS NOT NULL))",
        "CHECK (kind <> 'separator' OR (label_key IS NULL AND command IS NULL AND view IS NULL))",
    ],
    indexes: &[],
};

pub use crate::tables_body::*;
