//! The family `forms_*`, part 1b: pages, blocks, bindings, scheme lists, actions, views, filters, messages.
//!
//! Pattern: Value Object (const table specs).

use crate::spec::{
    Col,
    Logical::{Bool, Enum, FreeText, Int, Iri, Ref, Text},
    Table,
};
use crate::tables::{
    CONFIRM, ELEM, ELEM_UNIQUE, FILTER_COND, FILTER_OP, FROM, NAVIGATION, ORDER_BY, ROLE,
    SEARCH_KIND, TARGET, TRIGGER, VIEW_SCOPE,
};

pub const PAGE: Table = Table {
    name: "forms_page",
    cols: &[
        Col::req("layout", Ref),
        Col::req("page", Text),
        ELEM,
        Col::req("seq", Int),
        Col::req("frame", Text),
        Col::req("navigation", Enum(NAVIGATION)),
    ],
    constraints: &[
        "PRIMARY KEY (layout, page)",
        ELEM_UNIQUE,
        "UNIQUE (layout, seq)",
        "FOREIGN KEY (layout) REFERENCES forms_layout(id)",
    ],
    indexes: &[],
};

pub const BLOCK: Table = Table {
    name: "forms_block",
    cols: &[
        Col::req("layout", Ref),
        Col::req("page", Text),
        Col::req("place", Text),
        ELEM,
        Col::req("seq", Int),
        Col::req("block", Text),
        Col::opt("source", Int),
        Col::opt("title_key", Text),
        Col::opt("text_key", Text),
    ],
    constraints: &[
        "PRIMARY KEY (layout, page, place)",
        ELEM_UNIQUE,
        "UNIQUE (layout, page, seq)", // = tab order; H1 `seq-twice` only defence
        "FOREIGN KEY (layout, page) REFERENCES forms_page(layout, page)",
        "FOREIGN KEY (layout, source) REFERENCES forms_source(layout, source)",
    ],
    indexes: &[],
};

/// = `accepts::Binding` (F1 `accepts/bind.rs:52`), one step paths in F1.
pub const BINDING: Table = Table {
    name: "forms_binding",
    cols: &[
        Col::req("layout", Ref),
        Col::req("page", Text),
        Col::req("place", Text),
        Col::req("slot", Text),
        Col::req("nth", Int),
        ELEM,
        Col::req("bind_from", Enum(FROM)),
        Col::opt("path", Text),
        Col::opt("role", Enum(ROLE)),
        Col::opt("name", Text),
        Col::opt("source", Int), // L2b (format 4): the heads of a `grouping` come from this source; NULL = from the block source (0 is valid)
    ],
    constraints: &[
        "PRIMARY KEY (layout, page, place, slot, nth)",
        ELEM_UNIQUE,
        "FOREIGN KEY (layout, page, place) REFERENCES forms_block(layout, page, place)",
        "CHECK (nth >= 0 AND nth < 16)", // technical, with evidence: accepts `repeat` 1..16 (F1 ENTWURF §3.1 Z. 63)
        "CHECK (source IS NULL OR (bind_from = 'grouping' AND source >= 0))", // L2b: a source reference only on `grouping` (the layout check says the same)
        // `from` is an SQL keyword: quoted (found by running the DDL on SQLite 3.53; the sketch had it bare)
        // `bind_from`, not `from`: `from` is an SQL keyword and the fs-objects v5 renderer does not quote column names
        "CHECK ((bind_from = 'field') AND (path IS NOT NULL AND role IS NOT NULL) OR NOT (bind_from = 'field') AND NOT (path IS NOT NULL AND role IS NOT NULL))",
        "CHECK ((bind_from IN ('grouping','order','command')) AND (name IS NOT NULL) OR NOT (bind_from IN ('grouping','order','command')) AND NOT (name IS NOT NULL))",
    ],
    indexes: &[],
};

/// F1b NEW (user 32.4, PLAN-grundtyp-felder §8b.2): the COMPLETE allowed scheme list of ONE field use, in suggestion order
/// (question 38 = JA). No row for a field whose type carries schemes (webAddress, link, image) = read error `schemes-missing`.
/// Lower case only; only names of the instance's positive list pass in code (`scheme-not-allowed`; `javascript` etc. never).
pub const BINDING_SCHEME: Table = Table {
    name: "forms_binding_scheme",
    cols: &[
        Col::req("layout", Ref),
        Col::req("page", Text),
        Col::req("place", Text),
        Col::req("slot", Text),
        Col::req("nth", Int),
        Col::req("seq", Int),
        ELEM,
        Col::req("scheme", Text),
    ],
    constraints: &[
        "PRIMARY KEY (layout, page, place, slot, nth, seq)",
        ELEM_UNIQUE,
        "UNIQUE (layout, page, place, slot, nth, scheme)",
        "FOREIGN KEY (layout, page, place, slot, nth) REFERENCES forms_binding(layout, page, place, slot, nth)",
        "CHECK (scheme = lower(scheme) AND length(scheme) > 0)",
    ],
    indexes: &[],
};

/// Workflow bricks (F3/E4w); only the frame here (rule "Makros = Workflows").
pub const ACTION: Table = Table {
    name: "forms_action",
    cols: &[
        Col::req("layout", Ref),
        Col::req("action", Int),
        ELEM,
        Col::req("trigger", Enum(TRIGGER)),
        Col::req("target", Enum(TARGET)),
        Col::opt("command", Text),
        Col::opt("source", Int),
        Col::opt("workflow", Iri),
        Col::opt("confirm", Enum(CONFIRM)),
    ],
    constraints: &[
        "PRIMARY KEY (layout, action)",
        ELEM_UNIQUE,
        "FOREIGN KEY (layout) REFERENCES forms_layout(id)",
        "CHECK ((target = 'command') AND (command IS NOT NULL AND source IS NOT NULL AND workflow IS NULL) OR NOT (target = 'command') AND NOT (command IS NOT NULL AND source IS NOT NULL AND workflow IS NULL))",
        "CHECK ((target = 'workflow') AND (workflow IS NOT NULL) OR NOT (target = 'workflow') AND NOT (workflow IS NOT NULL))",
    ],
    indexes: &[],
};

/// Views; a person's saved views = rows with `owner` (question 5 of §16.7). `owner` '' for `all` (pattern `settings_value.program`).
pub const VIEW: Table = Table {
    name: "forms_view",
    cols: &[
        Col::req("layout", Ref),
        Col::req("view", Text),
        Col::req("owner", Text),
        ELEM,
        Col::req("scope", Enum(VIEW_SCOPE)),
        Col::opt("grouping", Text),
        Col::opt("filter_root", Int),
        Col::opt("search_kind", Enum(SEARCH_KIND)),
    ],
    constraints: &[
        "PRIMARY KEY (layout, view, owner)",
        ELEM_UNIQUE,
        "FOREIGN KEY (layout) REFERENCES forms_layout(id)",
        "CHECK ((scope = 'all') AND (owner = '') OR NOT (scope = 'all') AND NOT (owner = ''))",
    ],
    indexes: &[],
};

pub const VIEW_ORDER: Table = Table {
    name: "forms_view_order",
    cols: &[
        Col::req("layout", Ref),
        Col::req("view", Text),
        Col::req("owner", Text),
        Col::req("seq", Int),
        ELEM,
        Col::req("by", Enum(ORDER_BY)),
        Col::req("name", Text),
        Col::req("descending", Bool),
    ],
    constraints: &[
        "PRIMARY KEY (layout, view, owner, seq)",
        ELEM_UNIQUE,
        "FOREIGN KEY (layout, view, owner) REFERENCES forms_view(layout, view, owner)",
    ],
    indexes: &[],
};

/// The filter tree of F1 §14 as an adjacency list with typed leaves, never JSON.
pub const FILTER_NODE: Table = Table {
    name: "forms_filter_node",
    cols: &[
        Col::req("layout", Ref),
        Col::req("view", Text),
        Col::req("owner", Text),
        Col::req("node", Int),
        ELEM,
        Col::opt("parent", Int),
        Col::req("seq", Int),
        Col::req("op", Enum(FILTER_OP)),
        Col::opt("cond", Enum(FILTER_COND)),
        Col::opt("path", Text),
        Col::opt("value", FreeText),
        Col::opt("low", FreeText),
        Col::opt("high", FreeText),
    ],
    constraints: &[
        "PRIMARY KEY (layout, view, owner, node)",
        ELEM_UNIQUE,
        "FOREIGN KEY (layout, view, owner) REFERENCES forms_view(layout, view, owner)",
        "CHECK ((op = 'is') AND (cond IS NOT NULL) OR NOT (op = 'is') AND NOT (cond IS NOT NULL))",
    ],
    indexes: &[],
};

pub const FILTER_VALUE: Table = Table {
    name: "forms_filter_value",
    cols: &[
        Col::req("layout", Ref),
        Col::req("view", Text),
        Col::req("owner", Text),
        Col::req("node", Int),
        Col::req("seq", Int),
        ELEM,
        Col::req("value", FreeText),
    ],
    constraints: &[
        "PRIMARY KEY (layout, view, owner, node, seq)",
        ELEM_UNIQUE,
        "FOREIGN KEY (layout, view, owner, node) REFERENCES forms_filter_node(layout, view, owner, node)",
    ],
    indexes: &[],
};

/// MF2 sources of the description's keys (ADR 0057); language files of a package = import.
pub const MESSAGE: Table = Table {
    name: "forms_message",
    cols: &[
        Col::req("layout", Ref),
        Col::req("key", Text),
        Col::req("lang", Text),
        ELEM,
        Col::req("message", FreeText),
    ],
    constraints: &[
        "PRIMARY KEY (layout, key, lang)",
        ELEM_UNIQUE,
        "FOREIGN KEY (layout) REFERENCES forms_layout(id)",
    ],
    indexes: &[],
};
