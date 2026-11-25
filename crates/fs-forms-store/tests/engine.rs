//! The tables against a REAL SQL engine: turso 0.8.3, the engine of fs-objects (same version, same `default-features = false`).
//! Every step of the migration runs, the rows of a layout go in with foreign keys on, and what the tables forbid is refused.
//! Each test opens its own `:memory:` database (isolated, nothing on disk).
#![allow(clippy::too_many_lines, clippy::cast_possible_truncation)] // one fixture that touches every table; the generator draws tiny numbers
use fs_forms_layout::*;
use fs_forms_store::spec::Step;
use fs_forms_store::*;
use turso::Builder;

mod engine_support;
use engine_support::*;

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
        // (table, row, column, bad value). Rows: forms_binding is in (slot, nth) order, so 5 is the field binding `value`; forms_menu is
        // in pre-order, so 1 is the command entry `menu:0.0`.
        let bad: [(&str, usize, &str, Cell); 8] = [
            ("forms_binding", 0, "nth", Cell::Int(16)),
            (
                "forms_binding",
                0,
                "bind_from",
                Cell::Text("sideways".into()),
            ),
            (
                "forms_binding_scheme",
                0,
                "scheme",
                Cell::Text("SSH".into()),
            ),
            ("forms_block", 0, "source", Cell::Int(9)),
            ("forms_layout", 0, "owner", Cell::Text("someone".into())), // R4d: a package has no owner
            ("forms_binding", 5, "path", Cell::Null), // R4e: bind_from = 'field' needs path and role
            ("forms_menu", 1, "source", Cell::Null), // R4f: a command entry needs command and source
            ("forms_menu", 1, "command", Cell::Null),
        ];
        for (t, row, c, v) in bad {
            let mut r = rows.clone();
            r.tables.get_mut(t).unwrap()[row][col(t, c)] = v;
            let conn = open().await;
            assert!(
                insert(&conn, &r).await.is_err(),
                "{t}[{row}].{c} must be refused"
            );
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
        for _ in 0..800 {
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
            let word =
                ["https", "http", "HTTPS", "x-y.z+1", "1a", "javascript", ""][next(7) as usize];
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
                let at = sql.rfind(") STRICT").unwrap();
                sql.insert_str(at, &format!(", {c}"));
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

/// R4c: an overlay op carries at most one value (and one expected value); the engine refuses two.
#[test]
fn an_overlay_op_with_two_values_is_refused_by_the_engine() {
    run(async {
        let conn = open().await;
        insert(&conn, &rows_of(full_doc()).unwrap()).await.unwrap();
        let overlay = "INSERT INTO forms_overlay (id, layout, scope, author, actor, base_package, base_extends, base_frontend, \
            base_version, state, rev, created, updated) VALUES ('o1', 'layout-1', 'instance', 'a', 'a', 'net.example.pkg', \
            'net.example.Tasks', 'board', '1.0', 'applied', 1, 1, 1)";
        conn.execute(overlay, ()).await.unwrap();
        let op = |seq: u32, values: &str| {
            format!(
                "INSERT INTO forms_overlay_op (overlay, seq, op, target_kind, target_elem, field, {values}, author, actor, at) \
                 VALUES ('o1', {seq}, 'set', 'block', 'block:main', 'text_key', {}, 'a', 'a', 1)",
                if values.contains(',') {
                    "'x', 7"
                } else {
                    "'x'"
                }
            )
        };
        assert!(
            conn.execute(&op(0, "value_text"), ()).await.is_ok(),
            "one value is fine"
        );
        assert!(
            conn.execute(&op(1, "value_text, value_int"), ())
                .await
                .is_err(),
            "two values must be refused"
        );
        let expect = op(2, "expect_text, expect_int");
        assert!(
            conn.execute(&expect, ()).await.is_err(),
            "two expected values must be refused"
        );
    });
}
