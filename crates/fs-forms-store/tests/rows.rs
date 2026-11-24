//! A layout to rows and back (the proof that the table columns hold everything the model says), and the rows against the specs.
#![allow(clippy::too_many_lines)] // one fixture that touches every table
use fs_forms_layout::*;
use fs_forms_store::*;

fn elem(s: &str) -> ElemId {
    ElemId::parse(s).unwrap()
}
fn it<T>(e: &str, v: T) -> Item<T> {
    Item {
        elem: Some(elem(e)),
        line: 0,
        v,
    }
}
fn menu(kind: MenuKind, kids: Vec<Item<MenuDoc>>) -> MenuDoc {
    MenuDoc {
        kind,
        label_key: Some("m".into()),
        command: None,
        on: Some(OnWhat::Always),
        shortcut: None,
        view: None,
        children: kids,
    }
}

fn layout() -> Layout {
    let cmd = MenuDoc {
        kind: MenuKind::Command,
        label_key: None,
        command: Some((1, "create".into())),
        on: None,
        shortcut: Some("ctrl+n".into()),
        view: None,
        children: vec![],
    };
    let bind = |slot: &str, nth, from, schemes| BindingDoc {
        slot: slot.into(),
        nth,
        from,
        schemes,
    };
    let field = BindFrom::Field {
        path: "homepage".into(),
        role: "input".into(),
    };
    let block = BlockDoc {
        place: "main".into(),
        block: "board".into(),
        source: Some(1),
        title_key: Some("t".into()),
        text_key: Some("tx".into()),
        bindings: vec![
            it(
                "binding:a",
                bind(
                    "value",
                    0,
                    field,
                    Some(vec![
                        it("scheme:a/0", "ssh".to_owned()),
                        it("scheme:a/1", "https".to_owned()),
                    ]),
                ),
            ),
            it(
                "binding:b",
                bind("group", 0, BindFrom::Grouping("status".into()), None),
            ),
            it("binding:c", bind("rows", 0, BindFrom::Records, None)),
        ],
    };
    let doc = LayoutDoc {
        format: 2,
        key: FrontendKey {
            extends: "net.example.Tasks".into(),
            frontend: "board".into(),
        },
        exec: "fs-forms-run".into(),
        title_key: "board-title".into(),
        style: Some("default".into()),
        requires: vec![it(
            "requires:t",
            RequiresDoc {
                range: "tasks@>=0.2.0, <0.3.0".into(),
            },
        )],
        sources: vec![it(
            "source:1",
            SourceDoc {
                range: "tasks@>=0.0.1, <0.1.0".into(),
                data_type: "Task".into(),
                scope: Some(SourceScope::SelectedList),
            },
        )],
        menu: vec![
            it("menu:0", menu(MenuKind::Menu, vec![it("menu:0.0", cmd)])),
            it(
                "menu:1",
                menu(MenuKind::Standard(MenuStandard::Help), vec![]),
            ),
            it(
                "menu:2",
                MenuDoc {
                    view: Some("mine".into()),
                    ..menu(MenuKind::View, vec![])
                },
            ),
            it(
                "menu:3",
                MenuDoc {
                    label_key: None,
                    ..menu(MenuKind::Separator, vec![])
                },
            ),
        ],
        pages: vec![it(
            "page:board",
            PageDoc {
                page: "board".into(),
                frame: "workbench".into(),
                navigation: Navigation::ListDetail,
                blocks: vec![it("block:main", block)],
                places: vec![],
            },
        )],
        actions: vec![it(
            "action:1",
            ActionDoc {
                trigger: Trigger::Button,
                target: ActionTarget::Command {
                    source: 1,
                    command: "create".into(),
                },
                confirm: Some(Confirm::None),
            },
        )],
        views: vec![it(
            "view:mine",
            ViewDoc {
                view: "mine".into(),
                owner: Some(Reference("https://example.org/p".into())),
                grouping: Some("status".into()),
                orders: vec![it(
                    "order:1",
                    OrderDoc {
                        by: OrderBy::Field,
                        name: "due".into(),
                        descending: true,
                    },
                )],
                filter: None,
                search_kind: Some("live".into()),
            },
        )],
        messages: vec![
            it(
                "message:board-title/de",
                MessageDoc {
                    key: "board-title".into(),
                    lang: "de".into(),
                    message: "Aufgaben".into(),
                },
            ),
            it(
                "message:board-title/en",
                MessageDoc {
                    key: "board-title".into(),
                    lang: "en".into(),
                    message: "Tasks".into(),
                },
            ),
        ],
    };
    doc.check(&OpenCatalog, &Limits::declared()).unwrap()
}

fn meta() -> LayoutMeta {
    LayoutMeta {
        id: LayoutId(ObjectId("layout-1".into())),
        origin: Origin::Package,
        package: Some(PackageRef {
            package: "net.example.pkg".into(),
            version: "1.0".into(),
        }),
        owner: None,
        rev: Rev(1),
        created: Utc(10),
        updated: Utc(10),
        updated_by: Reference("https://example.org/p".into()),
    }
}

#[test]
fn rows_fit_the_table_specs() {
    let rows = to_rows(&layout(), &meta()).unwrap();
    assert_eq!(validate(&rows), Vec::<String>::new());
    for t in [
        "forms_layout",
        "forms_requires",
        "forms_source",
        "forms_menu",
        "forms_page",
        "forms_block",
        "forms_binding",
        "forms_binding_scheme",
        "forms_action",
        "forms_view",
        "forms_view_order",
        "forms_message",
    ] {
        assert!(rows.tables.contains_key(t), "{t}");
    }
    assert_eq!(rows.tables["forms_binding_scheme"].len(), 2);
    assert_eq!(rows.tables["forms_menu"].len(), 5);
}

#[test]
fn layout_to_rows_and_back_is_the_same_layout() {
    let l = layout();
    let rows = to_rows(&l, &meta()).unwrap();
    let back = from_rows(&rows, &meta().id).unwrap();
    assert_eq!(back, l);
    assert_eq!(back.digest(), l.digest());
}

#[test]
fn unknown_id_and_broken_rows_are_reported() {
    let rows = to_rows(&layout(), &meta()).unwrap();
    let e = from_rows(&rows, &LayoutId(ObjectId("nope".into()))).unwrap_err();
    assert_eq!(e[0].code, LayoutCode::Unknown);
    let mut bad = rows.clone();
    bad.tables.get_mut("forms_page").unwrap()[0][5] = Cell::Text("sideways".into());
    assert!(
        validate(&bad)
            .iter()
            .any(|p| p.contains("not in the word list"))
    );
    let mut nulled = rows.clone();
    nulled.tables.get_mut("forms_page").unwrap()[0][4] = Cell::Null;
    assert!(validate(&nulled).iter().any(|p| p.contains("NOT NULL")));
    // N6c: a row with a cell too few is reported
    let mut short = rows.clone();
    short.tables.get_mut("forms_page").unwrap()[0].pop();
    assert!(validate(&short).iter().any(|p| p.contains("cells for")));
    // an unknown word is a code, never a substitute value
    let e = from_rows(&bad, &meta().id).unwrap_err();
    assert_eq!(
        (e[0].code, e[0].param("cause")),
        (LayoutCode::StoreBroken, Some("rows"))
    );
    assert!(e[0].param("detail").unwrap().contains("sideways"));
    let mut dead_elem = rows;
    dead_elem.tables.get_mut("forms_page").unwrap()[0][2] = Cell::Text("Not Valid".into());
    assert_eq!(
        from_rows(&dead_elem, &meta().id).unwrap_err()[0].code,
        LayoutCode::ElemMissing
    );
}
