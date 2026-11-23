//! Fixtures and helpers of the engine tests: a connection with all migration steps, row insertion by literal SQL, the full layout.
#![allow(dead_code, clippy::too_many_lines)] // one fixture that touches every table; each test file uses a part

use fs_forms_layout::*;
use fs_forms_store::*;
use turso::{Builder, Connection};

pub fn run<F: std::future::Future>(f: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(f)
}

pub async fn open() -> Connection {
    let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = ON", ()).await.unwrap();
    for step in &STEPS {
        for sql in spec::statements(step) {
            conn.execute(&sql, ())
                .await
                .unwrap_or_else(|e| panic!("{e}: {sql}"));
        }
    }
    conn
}

pub fn literal(c: &Cell) -> String {
    match c {
        Cell::Null => "NULL".to_owned(),
        Cell::Int(n) => n.to_string(),
        Cell::Text(s) => format!("'{}'", s.replace('\'', "''")),
    }
}

/// Inserts all rows in table order (parents first). `Err` carries the engine's message.
pub async fn insert(conn: &Connection, rows: &Rows) -> Result<(), String> {
    for t in &ALL {
        for row in rows.tables.get(t.name).into_iter().flatten() {
            let values: Vec<String> = row.iter().map(literal).collect();
            let sql = format!("INSERT INTO {} VALUES ({})", t.name, values.join(","));
            conn.execute(&sql, ())
                .await
                .map_err(|e| format!("{e}: {sql}"))?;
        }
    }
    Ok(())
}

pub fn it<T>(e: &str, v: T) -> Item<T> {
    Item {
        elem: Some(ElemId::parse(e).unwrap()),
        line: 0,
        v,
    }
}

pub fn menu(kind: MenuKind) -> MenuDoc {
    MenuDoc {
        kind,
        label_key: Some("m".into()),
        command: None,
        on: Some(OnWhat::Always),
        shortcut: None,
        view: None,
        children: vec![],
    }
}

/// Every optional field set, every table touched.
pub fn full_doc() -> LayoutDoc {
    let cmd = MenuDoc {
        kind: MenuKind::Command,
        command: Some((1, "create".into())),
        shortcut: Some("ctrl+n".into()),
        ..menu(MenuKind::Command)
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
    let schemes = Some(vec![
        it("scheme:a/0", "ssh".to_owned()),
        it("scheme:a/1", "https".to_owned()),
    ]);
    let block = BlockDoc {
        place: "main".into(),
        block: "board".into(),
        source: Some(1),
        title_key: Some("t".into()),
        text_key: Some("tx".into()),
        bindings: vec![
            it("binding:a", bind("value", 0, field, schemes)),
            it(
                "binding:b",
                bind("group", 0, BindFrom::Grouping("status".into()), None),
            ),
            it("binding:c", bind("rows", 0, BindFrom::Records, None)),
            it("binding:d", bind("flag", 0, BindFrom::Lock, None)),
            it("binding:e", bind("bar", 0, BindFrom::Condition, None)),
            it(
                "binding:f",
                bind("cmd", 15, BindFrom::Command("create".into()), None),
            ),
        ],
    };
    LayoutDoc {
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
                range: "tasks >=0.2, <0.3".into(),
            },
        )],
        sources: vec![it(
            "source:1",
            SourceDoc {
                range: "tasks >=0.2, <0.3".into(),
                data_type: "Task".into(),
                scope: Some(SourceScope::SelectedList),
            },
        )],
        menu: vec![
            it(
                "menu:0",
                MenuDoc {
                    children: vec![it("menu:0.0", cmd)],
                    ..menu(MenuKind::Menu)
                },
            ),
            it("menu:1", menu(MenuKind::Standard(MenuStandard::Help))),
            it(
                "menu:2",
                MenuDoc {
                    view: Some("mine".into()),
                    ..menu(MenuKind::View)
                },
            ),
            it(
                "menu:3",
                MenuDoc {
                    label_key: None,
                    on: None,
                    ..menu(MenuKind::Separator)
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
            },
        )],
        actions: vec![it(
            "action:1",
            ActionDoc {
                trigger: Trigger::QuickAdd,
                target: ActionTarget::Command {
                    source: 1,
                    command: "create".into(),
                },
                confirm: Some(Confirm::Choice),
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
        messages: vec![it(
            "message:t/de",
            MessageDoc {
                key: "t".into(),
                lang: "de".into(),
                message: "Aufgaben".into(),
            },
        )],
    }
}

pub fn meta(origin: Origin) -> LayoutMeta {
    let pkg = (origin == Origin::Package).then(|| PackageRef {
        package: "net.example.pkg".into(),
        version: "1.0".into(),
    });
    let owner = (origin != Origin::Package).then(|| Reference("https://example.org/p".into()));
    LayoutMeta {
        id: LayoutId(ObjectId("layout-1".into())),
        origin,
        package: pkg,
        owner,
        rev: Rev(1),
        created: Utc(10),
        updated: Utc(10),
        updated_by: Reference("https://example.org/p".into()),
    }
}

pub fn rows_of(d: LayoutDoc) -> Result<Rows, Vec<Coded<LayoutCode>>> {
    let l = d.check(&OpenCatalog, &Limits::declared())?;
    Ok(to_rows(&l, &meta(Origin::Package)).unwrap())
}
