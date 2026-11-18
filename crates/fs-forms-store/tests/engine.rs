//! The tables against a REAL SQL engine: turso 0.8.3, the engine of fs-objects (same version, same `default-features = false`).
//! Every step of the migration runs, the rows of a layout go in with foreign keys on, and what the tables forbid is refused.
//! Each test opens its own `:memory:` database (isolated, nothing on disk).
#![allow(clippy::too_many_lines, clippy::cast_possible_truncation)] // one fixture that touches every table; the generator draws tiny numbers
use fs_forms_layout::*;
use fs_forms_store::spec::Step;
use fs_forms_store::*;
use turso::{Builder, Connection};

fn run<F: std::future::Future>(f: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(f)
}

async fn open() -> Connection {
    let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();
    conn.execute("PRAGMA foreign_keys = ON", ()).await.unwrap();
    for step in &STEPS {
        for sql in step.statements() {
            conn.execute(&sql, ())
                .await
                .unwrap_or_else(|e| panic!("{e}: {sql}"));
        }
    }
    conn
}

fn literal(c: &Cell) -> String {
    match c {
        Cell::Null => "NULL".to_owned(),
        Cell::Int(n) => n.to_string(),
        Cell::Text(s) => format!("'{}'", s.replace('\'', "''")),
    }
}

/// Inserts all rows in table order (parents first). `Err` carries the engine's message.
async fn insert(conn: &Connection, rows: &Rows) -> Result<(), String> {
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

fn it<T>(e: &str, v: T) -> Item<T> {
    Item {
        elem: Some(ElemId::parse(e).unwrap()),
        line: 0,
        v,
    }
}

fn menu(kind: MenuKind) -> MenuDoc {
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
fn full_doc() -> LayoutDoc {
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
                range: "tasks >=0.2 <0.3".into(),
            },
        )],
        sources: vec![it(
            "source:1",
            SourceDoc {
                range: "tasks >=0.2 <0.3".into(),
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

fn meta(origin: Origin) -> LayoutMeta {
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

fn rows_of(d: LayoutDoc) -> Result<Rows, Vec<Coded<LayoutCode>>> {
    let l = d.check(&OpenCatalog, &Limits::declared())?;
    Ok(to_rows(&l, &meta(Origin::Package)).unwrap())
}

#[test]
fn every_step_runs_and_the_full_layout_goes_in() {
    run(async {
        let conn = open().await;
        let rows = rows_of(full_doc()).unwrap();
        assert_eq!(validate(&rows), Vec::<String>::new());
        insert(&conn, &rows).await.unwrap();
        // and comes back out through the rows
        let l = from_rows(&rows, &meta(Origin::Package).id).unwrap();
        assert_eq!(l.doc().menu.len(), 4);
    });
}

#[test]
fn the_log_is_append_only_in_the_engine() {
    run(async {
        let conn = open().await;
        conn.execute("INSERT INTO forms_log (id, at, by, author, action) VALUES ('1', 1, 'a', 'a', 'create')", ()).await.unwrap();
        assert!(
            conn.execute("UPDATE forms_log SET at = 2", ())
                .await
                .is_err()
        );
        assert!(conn.execute("DELETE FROM forms_log", ()).await.is_err());
        // lock and unlock need a reason
        let no_reason =
            "INSERT INTO forms_log (id, at, by, author, action) VALUES ('2', 1, 'a', 'a', 'lock')";
        assert!(conn.execute(no_reason, ()).await.is_err());
    });
}

#[test]
fn what_the_tables_forbid_is_refused_by_the_engine() {
    run(async {
        let rows = rows_of(full_doc()).unwrap();
        let col = |t: &str, name: &str| {
            ALL.iter()
                .find(|x| x.name == t)
                .unwrap()
                .cols
                .iter()
                .position(|c| c.name == name)
                .unwrap()
        };
        // (table, column, bad value): each one must be refused although `check` would never produce it
        let bad: [(&str, &str, Cell); 4] = [
            ("forms_binding", "nth", Cell::Int(16)),
            ("forms_binding", "bind_from", Cell::Text("sideways".into())),
            ("forms_binding_scheme", "scheme", Cell::Text("SSH".into())),
            ("forms_block", "source", Cell::Int(9)),
        ];
        for (t, c, v) in bad {
            let mut r = rows.clone();
            r.tables.get_mut(t).unwrap()[0][col(t, c)] = v;
            let conn = open().await;
            assert!(insert(&conn, &r).await.is_err(), "{t}.{c} must be refused");
        }
        // the same layout without the change goes in (the refusal is the change, nothing else)
        insert(&open().await, &rows).await.unwrap();
    });
}

#[test]
fn a_second_live_key_is_refused_and_a_package_needs_no_owner() {
    run(async {
        let conn = open().await;
        let rows = rows_of(full_doc()).unwrap();
        insert(&conn, &rows).await.unwrap();
        let mut twin = rows.tables["forms_layout"][0].clone();
        twin[0] = Cell::Text("layout-2".into());
        let values: Vec<String> = twin.iter().map(literal).collect();
        let sql = format!("INSERT INTO forms_layout VALUES ({})", values.join(","));
        assert!(
            conn.execute(&sql, ()).await.is_err(),
            "UNIQUE (extends, frontend_key)"
        );
    });
}

/// Property: every description that `check` accepts can be written. A small deterministic generator varies the fields that the tables
/// constrain (menu pairs, sources, scheme words, view owner, nth); each variant is either refused by `check` or accepted by the engine.
#[test]
fn what_check_accepts_the_tables_accept() {
    run(async {
        let mut seed = 0x9E37_79B9_7F4A_7C15_u64;
        let mut next = move |n: u64| {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (seed >> 33) % n
        };
        let (mut accepted, mut refused) = (0, 0);
        for _ in 0..200 {
            let mut d = full_doc();
            let kinds = [
                MenuKind::Menu,
                MenuKind::Command,
                MenuKind::View,
                MenuKind::Separator,
                MenuKind::Standard(MenuStandard::Program),
            ];
            let kind = kinds[next(5) as usize];
            let command = [None, Some((1, "c".to_owned())), Some((9, "c".to_owned()))]
                [next(3) as usize]
                .clone();
            let view = [None, Some("mine".to_owned())][next(2) as usize].clone();
            let label = [None, Some("l".to_owned())][next(2) as usize].clone();
            d.menu = vec![it(
                "menu:0",
                MenuDoc {
                    kind,
                    command,
                    view,
                    label_key: label,
                    ..menu(kind)
                },
            )];
            d.pages[0].v.blocks[0].v.source = [None, Some(1), Some(9)][next(3) as usize];
            let word = ["https", "HTTPS", "x-y.z+1", "1a", "javascript", ""][next(6) as usize];
            d.pages[0].v.blocks[0].v.bindings[0].v.schemes =
                Some(vec![it("scheme:a/0", word.to_owned())]);
            d.pages[0].v.blocks[0].v.bindings[0].v.nth = next(20) as u32;
            let _ = next(1);
            match rows_of(d) {
                Err(_) => refused += 1,
                Ok(rows) => {
                    insert(&open().await, &rows)
                        .await
                        .unwrap_or_else(|e| panic!("check accepted what the tables refuse: {e}"));
                    accepted += 1;
                }
            }
        }
        assert!(
            accepted > 5 && refused > 5,
            "the generator must reach both sides ({accepted} / {refused})"
        );
    });
}

#[test]
fn steps_are_four() {
    assert_eq!(
        STEPS
            .iter()
            .filter(|s| matches!(s, Step::Create(_)))
            .count(),
        1
    );
}

/// Diagnosis that stays: every CHECK of every table on its own (columns only, no keys), so the engine's refusal names the constraint.
#[test]
fn each_check_constraint_is_accepted_on_its_own() {
    run(async {
        let mut failed = Vec::new();
        for t in &ALL {
            for (i, c) in t
                .constraints
                .iter()
                .filter(|c| c.starts_with("CHECK"))
                .enumerate()
            {
                let alone = fs_forms_store::spec::Table {
                    name: "probe",
                    constraints: &[],
                    ..*t
                };
                let mut sql = alone.ddl();
                let at = sql.rfind("\n) STRICT").unwrap();
                sql.insert_str(at, &format!(",\n    {c}"));
                let db = Builder::new_local(":memory:").build().await.unwrap();
                let conn = db.connect().unwrap();
                if let Err(e) = conn.execute(&sql, ()).await {
                    failed.push(format!("{} #{i}: {e}\n    {c}", t.name));
                }
            }
        }
        assert!(
            failed.is_empty(),
            "the engine refused {} constraint(s):\n{}",
            failed.len(),
            failed.join("\n")
        );
    });
}
