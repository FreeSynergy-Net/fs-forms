//! The family `forms_*`, part 2: overlays, conflicts, instance locks, the append-only log, the migration steps.
//!
//! Pattern: Value Object (const table specs) with append-only migration steps.

use crate::spec::{
    Col,
    Logical::{Bool, Enum, FreeText, Int, Iri, Ref, Text, Utc},
    Step, Table,
};
use crate::tables::{
    ACTION, BINDING, BINDING_SCHEME, BLOCK, CAUSE, FILTER_NODE, FILTER_VALUE, LAYOUT, LOG_ACTION,
    MENU, MESSAGE, OP, OVERLAY_SCOPE, OVERLAY_STATE, PAGE, POSITION, REQUIRES, RESOLUTION, REV,
    SOURCE, TARGET_KIND, VIEW, VIEW_ORDER,
};

/// §16.8; F1b: `base_extends`/`base_frontend` (re-attach after uninstall + reinstall, D6), `deleted` (tombstone, never reused id).
pub const OVERLAY: Table = Table {
    name: "forms_overlay",
    cols: &[
        Col::req("id", Ref),
        Col::req("layout", Ref),
        Col::req("scope", Enum(OVERLAY_SCOPE)),
        Col::opt("owner", Iri),
        Col::opt("title_key", Text),
        Col::opt("lang", Text),
        Col::req("author", Iri),
        Col::req("actor", Iri),
        Col::req("base_package", Text),
        Col::req("base_extends", Text),
        Col::req("base_frontend", Text),
        Col::req("base_version", Text),
        Col::req("state", Enum(OVERLAY_STATE)),
        Col::req("rev", Int),
        Col::req("created", Utc),
        Col::req("updated", Utc),
        Col::opt("deleted", Utc),
    ],
    constraints: &[
        "PRIMARY KEY (id)",
        "FOREIGN KEY (layout) REFERENCES forms_layout(id)",
        "CHECK ((scope = 'person') = (owner IS NOT NULL))",
        "CHECK (rev >= 0)",
    ],
    indexes: &[
        "CREATE INDEX IF NOT EXISTS forms_overlay_layout ON forms_overlay (layout, scope, state)",
    ],
};

pub const OVERLAY_OP: Table = Table {
    name: "forms_overlay_op",
    cols: &[
        Col::req("overlay", Ref),
        Col::req("seq", Int),
        Col::req("op", Enum(OP)),
        Col::req("target_kind", Enum(TARGET_KIND)),
        Col::req("target_elem", Text),
        Col::opt("field", Text),
        Col::opt("value_text", FreeText),
        Col::opt("value_int", Int),
        Col::opt("value_bool", Bool),
        Col::opt("expect_text", FreeText),
        Col::opt("expect_int", Int),
        Col::opt("expect_bool", Bool),
        Col::opt("anchor_elem", Text),
        Col::opt("position", Enum(POSITION)),
        Col::opt("new_elem", Text),
        Col::req("author", Iri),
        Col::req("actor", Iri),
        Col::req("at", Utc),
    ],
    constraints: &[
        "PRIMARY KEY (overlay, seq)",
        "FOREIGN KEY (overlay) REFERENCES forms_overlay(id)",
        // set <=> field + at most one value (none = set to empty, only for optional fields; checked in code against the model)
        "CHECK ((op = 'set') = (field IS NOT NULL))",
        "CHECK ((value_text IS NOT NULL) + (value_int IS NOT NULL) + (value_bool IS NOT NULL) <= 1)",
        "CHECK ((expect_text IS NOT NULL) + (expect_int IS NOT NULL) + (expect_bool IS NOT NULL) <= 1)",
        "CHECK (op <> 'remove' OR (field IS NULL AND anchor_elem IS NULL AND position IS NULL AND new_elem IS NULL))",
        "CHECK ((op = 'insert') = (new_elem IS NOT NULL))",
        "CHECK ((op IN ('insert','move')) = (anchor_elem IS NOT NULL AND position IS NOT NULL))",
    ],
    indexes: &[],
};

/// Never deleted automatically; `drop` only by a person (logged).
pub const OVERLAY_CONFLICT: Table = Table {
    name: "forms_overlay_conflict",
    cols: &[
        Col::req("overlay", Ref),
        Col::req("seq", Int),
        Col::req("base_version", Text),
        Col::req("cause", Enum(CAUSE)),
        Col::opt("found_text", FreeText),
        Col::req("at", Utc),
        Col::req("resolution", Enum(RESOLUTION)),
        Col::opt("resolved_by", Iri),
        Col::opt("resolved_at", Utc),
    ],
    constraints: &[
        "PRIMARY KEY (overlay, seq, base_version)",
        "FOREIGN KEY (overlay, seq) REFERENCES forms_overlay_op(overlay, seq)",
        "CHECK ((resolution = 'open') = (resolved_by IS NULL AND resolved_at IS NULL))",
    ],
    indexes: &[],
};

/// F1b NEW (question 50): an instance admin locks one element (or `elem = ''` = the whole layout) against person overlays.
/// Pattern `settings_instance_lock`: reason mandatory, by + at set by the service. Empty in a person DB until the instance service.
pub const LOCK: Table = Table {
    name: "forms_lock",
    cols: &[
        Col::req("layout", Ref),
        Col::req("elem", Text),
        Col::req("reason", FreeText),
        Col::req("locked_by", Iri),
        Col::req("locked_at", Utc),
    ],
    constraints: &[
        "PRIMARY KEY (layout, elem)",
        "FOREIGN KEY (layout) REFERENCES forms_layout(id)",
        "CHECK (length(trim(reason)) > 0)",
    ],
    indexes: &[],
};

/// F1b NEW: append-only log of every write (triggers, steps 2-3, pattern `settings_instance_log`). `by` = acting actor (an AI is
/// its own actor, ADR 0054/0091), `author` = the person the work is for; time + `os_user` by the service, never the caller.
pub const LOG: Table = Table {
    name: "forms_log",
    cols: &[
        Col::req("id", Ref),
        Col::req("at", Utc),
        Col::req("by", Iri),
        Col::req("author", Iri),
        Col::opt("os_user", Text),
        Col::req("action", Enum(LOG_ACTION)),
        Col::opt("layout", Ref),
        Col::opt("overlay", Ref),
        Col::opt("elem", Text),
        Col::opt("rev_before", Int),
        Col::opt("rev_after", Int),
        Col::opt("package", Text),
        Col::opt("version", Text),
        Col::opt("count", Int),
        Col::opt("reason", FreeText),
    ],
    constraints: &[
        "PRIMARY KEY (id)",
        "CHECK (action NOT IN ('lock','unlock') OR (reason IS NOT NULL AND length(trim(reason)) > 0))",
        "CHECK (action NOT IN ('overlay-create','overlay-ops','overlay-state','overlay-delete','overlay-import','conflict-resolve') OR overlay IS NOT NULL)",
    ],
    indexes: &["CREATE INDEX IF NOT EXISTS forms_log_at ON forms_log (at)"],
};

pub const ALL: [Table; 20] = [
    REV,
    LAYOUT,
    REQUIRES,
    SOURCE,
    MENU,
    PAGE,
    BLOCK,
    BINDING,
    BINDING_SCHEME,
    ACTION,
    VIEW,
    VIEW_ORDER,
    FILTER_NODE,
    FILTER_VALUE,
    MESSAGE,
    OVERLAY,
    OVERLAY_OP,
    OVERLAY_CONFLICT,
    LOCK,
    LOG,
];

/// Schema version = number of steps (`registry.rs:4`) = 4. Later: append `Step::Sql("ALTER TABLE ... ADD COLUMN ...")` and update
/// the `Table` above; never RENAME (ADR 0086 addendum). Planned additive steps: `forms_binding_path` (multi-step paths), dialogs,
/// key maps, page context (O10 of F1), editor drafts (E5), old-state history (question 53).
pub const STEPS: [Step; 4] = [
    Step::Create(&ALL),
    Step::Sql(
        "CREATE TRIGGER forms_log_no_update BEFORE UPDATE ON forms_log BEGIN SELECT RAISE(ABORT, 'the forms log is append-only'); END",
    ),
    Step::Sql(
        "CREATE TRIGGER forms_log_no_delete BEFORE DELETE ON forms_log BEGIN SELECT RAISE(ABORT, 'the forms log is append-only'); END",
    ),
    Step::Sql("INSERT INTO forms_rev (slot, rev) VALUES (0, 0)"),
];
