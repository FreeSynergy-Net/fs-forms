//! The tasks-board layout by hand (the shape F1b's import of `tasks-board.toml` gives), for tests and the bench.
//!
//! Pattern: Object Mother.

use crate::model::*;
use crate::testing::{cmd, field};
use crate::types::*;

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
