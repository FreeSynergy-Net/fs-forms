//! Test doubles and fixtures (feature `testing`): `TestCatalog` until G7, fake provider facts, the tasks-board layout by hand.
//!
//! The choose table is written down here INDEPENDENTLY of any code (PLAN-grundtyp-felder §7.2, H-K2), so a wrong `choose` in
//! the real catalog would not be copied. Pattern: Test Double + Object Mother.

use crate::catalog::{BlockSpec, Catalog, SlotSpec};
use crate::coded::Coded;
use crate::model::*;
use crate::providers::{Offer, Provider, ProviderFacts, SettingsFacts, SettingsHealth};
use crate::types::*;

/// Catalog with a fixed table; `unbuilt` blocks are known but not built.
pub struct TestCatalog {
    /// Known blocks.
    pub blocks: Vec<BlockSpec>,
    /// Blocks without a factory.
    pub unbuilt: Vec<String>,
}

fn slot(name: &str, min: u32, max: Option<u32>) -> SlotSpec {
    SlotSpec {
        name: name.to_owned(),
        min,
        max,
    }
}

impl TestCatalog {
    /// Blocks of the tasks-board fixture plus `probe-field`.
    #[must_use]
    pub fn new() -> Self {
        let b = |n: &str, s: Vec<SlotSpec>| BlockSpec {
            name: n.to_owned(),
            slots: s,
        };
        let blocks = vec![
            b(
                "board",
                vec![
                    slot("cards", 1, Some(1)),
                    slot("card-title", 0, None),
                    slot("card-facts", 0, None),
                    slot("card-menu", 0, None),
                ],
            ),
            b(
                "board-detail",
                vec![slot("properties", 0, None), slot("actions", 0, None)],
            ),
            b("empty-state", vec![]),
            b("notes-list", vec![slot("rows", 0, None)]),
            b("form", vec![slot("fields", 0, None)]),
            b("probe-field", vec![]),
            b("filter-bar", vec![slot("conditions", 0, None)]),
        ];
        Self {
            blocks,
            unbuilt: Vec::new(),
        }
    }
}

impl Default for TestCatalog {
    fn default() -> Self {
        Self::new()
    }
}

impl Catalog for TestCatalog {
    fn need(&self, block: &str) -> Result<&BlockSpec, Coded> {
        self.blocks
            .iter()
            .find(|b| b.name == block)
            .ok_or_else(|| Coded::new("accepts-error-block-unknown").with("block", block))
    }

    fn choose(&self, f: &FieldDesc, role: Role) -> String {
        let name = match (f.basic, f.target.as_deref(), role) {
            (Basic::Reference, Some("Person"), _) => "person-picker",
            (Basic::Reference, _, _) => "field-reference",
            (Basic::Email, _, _) => "field-email",
            (Basic::DateTime, _, _) => "field-date-time",
            (Basic::Date, _, _) => "field-date",
            (Basic::Choice, _, _) => "field-choice",
            (Basic::Keywords, _, _) => "field-keywords",
            (Basic::Number, _, _) => "field-number",
            (Basic::Bool, _, _) => "field-bool",
            (Basic::Link, _, _) => "field-link",
            (Basic::Image, _, _) => "field-image",
            (Basic::Text | Basic::Unknown, _, _) => "field-text",
        };
        name.to_owned()
    }

    fn built(&self, block: &str) -> bool {
        !self.unbuilt.iter().any(|b| b == block)
    }
}

/// Fake provider facts by capability.
pub struct FakeFacts(pub Vec<(String, Offer)>);

impl ProviderFacts for FakeFacts {
    fn offer(&self, capability: &str) -> Offer {
        self.0
            .iter()
            .find(|(c, _)| c == capability)
            .map_or(Offer::NotInstalled, |(_, o)| o.clone())
    }
}

/// A provider of one version.
#[must_use]
pub fn provider(major: u32, minor: u32) -> Provider {
    Provider {
        version: (major, minor),
    }
}

/// Facts: `tasks` 0.2 reached.
#[must_use]
pub fn tasks_facts() -> FakeFacts {
    FakeFacts(vec![(
        "tasks".to_owned(),
        Offer::Reached(vec![provider(0, 2)]),
    )])
}

/// Settings without a person style, store healthy.
#[must_use]
pub fn settings() -> SettingsFacts {
    SettingsFacts {
        style: None,
        limits: Limits::default(),
        search: None,
        health: SettingsHealth::Ok,
    }
}

/// A field description.
#[must_use]
pub fn field(path: &str, basic: Basic) -> FieldDesc {
    FieldDesc {
        path: vec![path.to_owned()],
        basic,
        target: None,
        read_only: false,
    }
}

/// A command description.
#[must_use]
pub fn cmd(name: &str, supported: bool, writes: bool, on_record: bool) -> CommandIn {
    CommandIn {
        source: 0,
        name: name.to_owned(),
        desc: CommandDesc {
            supported,
            writes,
            on_record,
        },
        target: Target::Program,
    }
}

fn bind(elem: &str, slot: &str, nth: u32, kind: BindKind) -> BindingIn {
    BindingIn {
        elem: elem.to_owned(),
        slot: slot.to_owned(),
        nth,
        kind,
    }
}

fn field_bind(slot_name: &str, nth: u32, f: FieldDesc, role: Role) -> BindingIn {
    bind(
        &format!("b-{slot_name}-{nth}"),
        slot_name,
        nth,
        BindKind::Field {
            field: f,
            role,
            schemes: None,
        },
    )
}

/// The tasks-board placements by hand (the shape F1b's import of `tasks-board.toml` gives).
#[must_use]
pub fn board_bindings() -> Vec<BindingIn> {
    let mut v = vec![bind("b-cards", "cards", 0, BindKind::Records(0))];
    v.push(field_bind(
        "card-title",
        0,
        field("title", Basic::Text),
        Role::Display,
    ));
    for (i, (n, b)) in [
        ("due", Basic::DateTime),
        ("priority", Basic::Choice),
        ("participants", Basic::Reference),
        ("keywords", Basic::Keywords),
    ]
    .into_iter()
    .enumerate()
    {
        let mut f = field(n, b);
        if b == Basic::Reference {
            f.target = Some("Person".to_owned());
        }
        v.push(field_bind(
            "card-facts",
            u32::try_from(i).unwrap_or(0),
            f,
            Role::Compact,
        ));
    }
    for (i, n) in ["claim", "release", "note", "delete"]
        .into_iter()
        .enumerate()
    {
        let c = cmd(n, true, n == "delete", n != "create");
        v.push(bind(
            &format!("b-menu-{i}"),
            "card-menu",
            u32::try_from(i).unwrap_or(0),
            BindKind::Command(c),
        ));
    }
    v
}

fn place(
    page: &str,
    name: &str,
    seq: u32,
    block: &str,
    source: Option<u32>,
    bindings: Vec<BindingIn>,
) -> PlaceIn {
    PlaceIn {
        elem: format!("pl-{page}-{name}"),
        place: name.to_owned(),
        seq,
        block: block.to_owned(),
        source,
        bindings,
    }
}

fn menu_cmd(elem: &str, name: &str, on_record: bool, shortcut: Option<&str>) -> MenuIn {
    let command = cmd(name, true, name == "delete", on_record);
    MenuIn {
        elem: elem.to_owned(),
        kind: MenuKind::Command {
            command,
            on_selected: on_record,
            shortcut: shortcut.map(str::to_owned),
        },
        children: vec![],
    }
}

/// The tasks-board layout by hand: board + detail on source 0, an empty-state page, the menu of the description.
#[must_use]
pub fn tasks_resolved() -> Resolved {
    let node = |elem: &str, kind: MenuKind, children: Vec<MenuIn>| MenuIn {
        elem: elem.to_owned(),
        kind,
        children,
    };
    let task = node(
        "m-task",
        MenuKind::Menu("menu-task".to_owned()),
        vec![
            menu_cmd("m-create", "create", false, Some("ctrl+n")),
            menu_cmd("m-claim", "claim", true, None),
            menu_cmd("m-release", "release", true, None),
            menu_cmd("m-note", "note", true, None),
            menu_cmd("m-delete", "delete", true, Some("delete")),
        ],
    );
    let view = node(
        "m-view",
        MenuKind::Menu("menu-view".to_owned()),
        vec![node("m-v1", MenuKind::View("board".to_owned()), vec![])],
    );
    Resolved {
        key: "net.freesynergy.Tasks/board".to_owned(),
        stamp: 1,
        rev: 1,
        style: None,
        sources: vec![SourceIn {
            elem: "s0".to_owned(),
            capability: "tasks".to_owned(),
            range: Range {
                min: (0, 2),
                max: (0, 3),
            },
            data_type: "Task".to_owned(),
        }],
        pages: vec![
            PageIn {
                elem: "pg-board".to_owned(),
                page: "board".to_owned(),
                seq: 0,
                frame: "workbench".to_owned(),
                places: vec![place(
                    "board",
                    "main",
                    0,
                    "board",
                    Some(0),
                    board_bindings(),
                )],
            },
            PageIn {
                elem: "pg-detail".to_owned(),
                page: "detail".to_owned(),
                seq: 1,
                frame: "detail-page".to_owned(),
                places: vec![
                    place(
                        "detail",
                        "props",
                        0,
                        "board-detail",
                        Some(0),
                        vec![field_bind(
                            "properties",
                            0,
                            field("title", Basic::Text),
                            Role::Display,
                        )],
                    ),
                    place("detail", "empty", 1, "empty-state", None, vec![]),
                ],
            },
        ],
        menu: vec![
            node("m-prog", MenuKind::Standard("program".to_owned()), vec![]),
            task,
            view,
            node("m-help", MenuKind::Standard("help".to_owned()), vec![]),
        ],
        problems: vec![],
        conflicts: vec![],
    }
}
