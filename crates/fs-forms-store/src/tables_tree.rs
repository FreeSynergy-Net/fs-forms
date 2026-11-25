//! The family `forms_*`, part 1c (A5 F4): the tree of places of a page (`PageDoc.places`) and the attributes (`set`) of its nodes.
//! One row per node (block or frame), the parent is the `elem` of the node above (NULL = directly in the frame of the page), the
//! children of one parent are ordered by `seq`. Attributes are rows too (typed columns, never one JSON text).
//! No migration step: the data of the earlier cut is not rescued (rule "Daten nicht retten"); the tables are part of the first step.
//!
//! Pattern: Value Object (const table specs); the rows form an adjacency list (as `forms_menu`).

use crate::spec::{
    Col,
    Logical::{Enum, FreeText, Int, Ref, Text},
    Table,
};
use crate::tables::{ELEM, FROM, ROLE};

/// What a node of the place tree is: the leaf or the composite (`PlaceNode`).
pub const NODE_KIND: &[&str] = &["block", "frame"];
/// Whose attributes a `forms_attr` row holds: a block of the flat list (`forms_block`) or a node of the tree (`forms_node`).
pub const ATTR_OWNER: &[&str] = &["block", "node"];

/// One node of the tree of places: a block (leaf) or a frame (composite). `name` is the block or the frame name of the catalog.
pub const NODE: Table = Table {
    name: "forms_node",
    cols: &[
        Col::req("layout", Ref),
        ELEM,
        Col::req("page", Text),
        Col::opt("parent", Text), // elem of the parent node; NULL = child of the frame of the page
        Col::req("place", Text),
        Col::req("seq", Int),
        Col::req("kind", Enum(NODE_KIND)),
        Col::req("name", Text),
        Col::opt("source", Int),
        Col::opt("title_key", Text),
        Col::opt("text_key", Text),
    ],
    constraints: &[
        "PRIMARY KEY (layout, elem)",
        "UNIQUE (layout, page, parent, seq)", // = order of the children (NULL parents never collide in SQL: the reader sorts and `check` refuses doubles)
        "FOREIGN KEY (layout, page) REFERENCES forms_page(layout, page)",
        "FOREIGN KEY (layout, parent) REFERENCES forms_node(layout, elem)",
        "CHECK (kind = 'block' OR (source IS NULL AND title_key IS NULL AND text_key IS NULL))", // a frame has no block columns
        "CHECK (seq >= 0)",
    ],
    indexes: &[],
};

/// Attributes (`set`) of a block or a node: name and value as text, in the order of the description.
pub const ATTR: Table = Table {
    name: "forms_attr",
    cols: &[
        Col::req("layout", Ref),
        ELEM,
        Col::req("owner_kind", Enum(ATTR_OWNER)),
        Col::req("owner", Text), // elem of the block or the node (two tables: no foreign key, `check` and the reader hold it)
        Col::req("seq", Int),
        Col::req("name", Text),
        Col::req("value", FreeText),
    ],
    constraints: &[
        "PRIMARY KEY (layout, owner_kind, owner, name)",
        "UNIQUE (layout, elem)",
        "UNIQUE (layout, owner_kind, owner, seq)",
        "FOREIGN KEY (layout) REFERENCES forms_layout(id)",
        "CHECK (length(name) > 0 AND seq >= 0)",
    ],
    indexes: &[],
};

/// Binding of a slot of a block IN THE TREE (the flat `forms_binding` is keyed by (page, place), which is not unique in a tree).
pub const NODE_BINDING: Table = Table {
    name: "forms_node_binding",
    cols: &[
        Col::req("layout", Ref),
        Col::req("node", Text),
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
        "PRIMARY KEY (layout, node, slot, nth)",
        "UNIQUE (layout, elem)",
        "FOREIGN KEY (layout, node) REFERENCES forms_node(layout, elem)",
        "CHECK (nth >= 0 AND nth < 16)",
        "CHECK (source IS NULL OR (bind_from = 'grouping' AND source >= 0))", // L2b: a source reference only on `grouping` (the layout check says the same)
        "CHECK ((bind_from = 'field') AND (path IS NOT NULL AND role IS NOT NULL) OR NOT (bind_from = 'field') AND NOT (path IS NOT NULL AND role IS NOT NULL))",
        "CHECK ((bind_from IN ('grouping','order','command')) AND (name IS NOT NULL) OR NOT (bind_from IN ('grouping','order','command')) AND NOT (name IS NOT NULL))",
    ],
    indexes: &[],
};

/// Scheme list of a binding in the tree, in suggestion order (as `forms_binding_scheme`).
pub const NODE_SCHEME: Table = Table {
    name: "forms_node_scheme",
    cols: &[
        Col::req("layout", Ref),
        Col::req("node", Text),
        Col::req("slot", Text),
        Col::req("nth", Int),
        Col::req("seq", Int),
        ELEM,
        Col::req("scheme", Text),
    ],
    constraints: &[
        "PRIMARY KEY (layout, node, slot, nth, seq)",
        "UNIQUE (layout, elem)",
        "UNIQUE (layout, node, slot, nth, scheme)",
        "FOREIGN KEY (layout, node, slot, nth) REFERENCES forms_node_binding(layout, node, slot, nth)",
        "CHECK (scheme = lower(scheme) AND length(scheme) > 0)",
    ],
    indexes: &[],
};
