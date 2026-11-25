//! Test doubles of the form match (feature `testing`): a small catalog and a tree module and component by hand.
//!
//! The real catalog is `fs_interfaces::forms` (read by the host, `fs-forms-run`, which tests the match against it); bind reads no
//! form file. The field types a channel takes are written out here because this is a fixture of the matcher, not a second table:
//! the host fills them from `Art::allowed_basics` of `fs-interfaces`.
//!
//! Pattern: Test double (no I/O).

use crate::forms::{Channel, ComponentForms, Count, FormDef, FormRef, Grundtyp, Kind, ModuleForm};
use crate::types::Basic;

fn kind(grundtyp: Grundtyp, basics: &[Basic]) -> Kind {
    Kind {
        grundtyp,
        basics: basics.to_vec(),
    }
}

fn channel(name: &str, kinds: Vec<Kind>, required: bool, min: u32, max: Option<u32>) -> Channel {
    Channel {
        name: name.to_owned(),
        kinds,
        required,
        count: Count { min, max },
    }
}

/// The fixture catalog: `tree@1.0` and `tree@1.1` (one optional channel more).
#[must_use]
pub fn fixture_catalog() -> Vec<FormDef> {
    let nodes = || channel("nodes", vec![kind(Grundtyp::Records, &[])], true, 0, None);
    let parent = || {
        channel(
            "parent",
            vec![kind(Grundtyp::Reference, &[Basic::Reference])],
            true,
            0,
            None,
        )
    };
    let text = [
        Basic::Text,
        Basic::Email,
        Basic::Choice,
        Basic::Keywords,
        Basic::Link,
    ];
    let label = || channel("label", vec![kind(Grundtyp::Text, &text)], true, 1, Some(1));
    let facts = channel(
        "facts",
        vec![
            kind(Grundtyp::Text, &text),
            kind(Grundtyp::Number, &[Basic::Number]),
            kind(Grundtyp::Time, &[Basic::Date, Basic::DateTime]),
            kind(Grundtyp::Flag, &[Basic::Bool]),
        ],
        false,
        0,
        Some(4),
    );
    let edge_kind = channel(
        "edge-kind",
        vec![kind(Grundtyp::Text, &text)],
        false,
        0,
        None,
    );
    let mark = channel(
        "mark",
        vec![kind(Grundtyp::Flag, &[Basic::Bool])],
        false,
        0,
        Some(1),
    );
    vec![
        FormDef {
            name: "tree".into(),
            major: 1,
            minor: 0,
            channels: vec![nodes(), parent(), label(), edge_kind, facts],
        },
        FormDef {
            name: "tree".into(),
            major: 1,
            minor: 1,
            channels: vec![nodes(), parent(), label(), mark],
        },
    ]
}

/// A form reference `name@H.N`.
#[must_use]
pub fn form_ref(name: &str, major: u32, minor: u32) -> FormRef {
    FormRef {
        name: name.to_owned(),
        major,
        minor,
    }
}

/// A module that maps `tree@1.0` completely (nodes, parent, label required; edge-kind optional left out).
#[must_use]
pub fn tree_module() -> ModuleForm {
    let map = |c: &str, f: &str| (c.to_owned(), vec![f.to_owned()]);
    ModuleForm {
        form: form_ref("tree", 1, 0),
        data_type: "Group".into(),
        map: vec![
            map("nodes", "Group"),
            map("parent", "parent"),
            map("label", "name"),
        ],
        fields: vec![
            ("parent".into(), Basic::Reference),
            ("name".into(), Basic::Text),
        ],
    }
}

/// The component of the block `tree`, drawing `tree@1.0`.
#[must_use]
pub fn tree_component() -> ComponentForms {
    ComponentForms {
        block: "tree".into(),
        forms: vec![form_ref("tree", 1, 0)],
    }
}
