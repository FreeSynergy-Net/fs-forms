//! L2b: the free scope token and the binding source in the database (real engine, turso 0.8.3). DB -> export -> empty DB -> import ->
//! export is byte-equal with source 2 `lanes:assignee` and binding `source = 2`; a token that breaks RFC 6749 section 3.3 is refused by the
//! tables (CHECK) and, when it arrives in the rows another way, is a store error in `from_rows` (never "no scope").
//!
//! Pattern: Round trip through the real store (Memento-like: what goes in comes out byte-equal).
use std::path::Path;

use fs_forms_exchange::{Format, read_layout, write_layout};
use fs_forms_layout::{BindFrom, Layout, LayoutCode, Limits, OpenCatalog, Origin, SourceScope};
use fs_forms_store::*;
use turso::{Connection, Value};

mod engine_support;
use engine_support::*;

fn lanes_file(scope2: &str, binding: &str) -> String {
    format!(
        "schema = 4\nkind = \"frontend\"\nid = \"board\"\nextends = \"net.example.Tasks\"\nexec = \"fs-forms-run\"\ntitle = \"t\"\n\n\
[[source]]\nelem = \"source:1\"\nn = 1\ncontract = \"planning@>=0.0.1, <0.1.0\"\ntype = \"Task\"\nscope = \"selected-list\"\n\n\
[[source]]\nelem = \"source:2\"\nn = 2\ncontract = \"planning@>=0.0.1, <0.1.0\"\ntype = \"Lane\"\nscope = \"{scope2}\"\n\n\
[[page]]\nelem = \"page:board\"\npage = \"board\"\nframe = \"app-window\"\nnavigation = \"single\"\n\n\
[[page.place]]\nelem = \"block:board/main\"\nat = \"main\"\nblock = \"cards-in-columns\"\nsource = 1\n\n\
[[page.place.bind]]\nelem = \"binding:board/main/lanes/0\"\nslot = \"lanes\"\nnth = 0\n{binding}\n\n\
[[message]]\nelem = \"message:t/de\"\nkey = \"t\"\nlang = \"de\"\ntext = \"T\"\n"
    )
}
const GROUPING: &str = "grouping = \"assignee\"\nsource = 2";

fn import(text: &str) -> Layout {
    read_layout(&mut text.as_bytes(), Path::new("t.toml"), Format::Toml)
        .expect("read")
        .check(&OpenCatalog, &Limits::declared())
        .expect("check")
}
fn export(l: &Layout) -> String {
    let mut out = Vec::new();
    write_layout(l, Format::Toml, true, &mut out).expect("write");
    String::from_utf8(out).expect("utf8")
}

/// Every row of every table, as the engine holds it (column order = table spec).
async fn dump(conn: &Connection) -> Rows {
    let mut rows = Rows::default();
    for t in &ALL {
        let mut q = conn
            .query(&format!("SELECT * FROM {}", t.name), ())
            .await
            .unwrap();
        while let Some(r) = q.next().await.unwrap() {
            let cells = (0..t.cols.len())
                .map(|i| match r.get_value(i).unwrap() {
                    Value::Null => Cell::Null,
                    Value::Integer(n) => Cell::Int(n),
                    Value::Text(s) => Cell::Text(s),
                    other => panic!("{}: unexpected cell {other:?}", t.name),
                })
                .collect();
            rows.tables.entry(t.name).or_default().push(cells);
        }
    }
    rows
}

async fn store(l: &Layout) -> Connection {
    let conn = open().await;
    let rows = to_rows(l, &meta(Origin::Package)).unwrap();
    assert_eq!(validate(&rows), Vec::<String>::new());
    insert(&conn, &rows).await.unwrap();
    conn
}

fn id() -> fs_forms_layout::LayoutId {
    meta(Origin::Package).id
}

#[test]
fn db_export_empty_db_import_export_is_byte_equal_with_source_2() {
    run(async {
        let first = import(&lanes_file("lanes:assignee", GROUPING));
        let db1 = store(&first).await;
        let rows1 = dump(&db1).await;
        let out1 = export(&from_rows(&rows1, &id()).unwrap());
        // the bytes carry both new words
        assert!(out1.contains("scope = \"lanes:assignee\""), "{out1}");
        assert!(out1.contains("source = 2"), "{out1}");
        // an EMPTY database gets the export
        let db2 = store(&import(&out1)).await;
        let out2 = export(&from_rows(&dump(&db2).await, &id()).unwrap());
        assert_eq!(out1, out2, "DB -> export -> empty DB -> import -> export");
        assert_eq!(
            out1,
            lanes_file("lanes:assignee", GROUPING),
            "and the file is the canonical one"
        );
        // the model after the way through the engine
        let l = from_rows(&dump(&db2).await, &id()).unwrap();
        let d = l.doc();
        assert!(matches!(d.sources[1].v.scope, Some(SourceScope::Module(_))));
        let b = &d.pages[0].v.leaves().next().unwrap().v.bindings[0].v;
        assert_eq!(
            (b.source, &b.from),
            (Some(2), &BindFrom::Grouping("assignee".into()))
        );
    });
}

#[test]
fn the_binding_source_is_a_column_and_zero_is_kept_apart_from_none() {
    run(async {
        // source = 0 is NOT "no source": the cell is 0, not NULL
        let l = import(&lanes_file("all", "grouping = \"assignee\""));
        let none = dump(&store(&l).await).await;
        let col = |t: &str, name: &str| {
            ALL.iter()
                .find(|x| x.name == t)
                .unwrap()
                .cols
                .iter()
                .position(|c| c.name == name)
                .unwrap()
        };
        let tables = [
            fs_forms_store::tables_body::BINDING.name,
            fs_forms_store::tables_tree::NODE_BINDING.name,
        ];
        let mut seen = 0;
        for t in tables {
            for row in none.tables.get(t).into_iter().flatten() {
                assert_eq!(
                    row[col(t, "source")],
                    Cell::Null,
                    "{t}: no source given = NULL"
                );
                seen += 1;
            }
        }
        assert!(seen > 0, "a binding row was written");
        let with = dump(&store(&import(&lanes_file("lanes:assignee", GROUPING))).await).await;
        let mut seen = 0;
        for t in tables {
            for row in with.tables.get(t).into_iter().flatten() {
                assert_eq!(row[col(t, "source")], Cell::Int(2), "{t}");
                seen += 1;
            }
        }
        assert!(seen > 0);
    });
}

async fn refused_scope(scope: &str) -> bool {
    let conn = store(&import(&lanes_file("all", "grouping = \"assignee\""))).await;
    let sql = format!(
        "UPDATE forms_source SET scope = '{}' WHERE source = 2",
        scope.replace('\'', "''")
    );
    conn.execute(&sql, ()).await.is_err()
}

#[test]
fn the_tables_refuse_a_scope_outside_the_rfc_6749_grammar() {
    run(async {
        for bad in [
            "a b",
            "x\"y",
            "x\\y",
            "",
            "caf\u{e9}",
            "tab\there",
            &"a".repeat(257),
        ] {
            assert!(refused_scope(bad).await, "must be refused: {bad:?}");
        }
        for good in [
            "lanes:assignee",
            "a]b",
            "a[b",
            "~!#$%&'()*+,-./:;<=>?@^_`{|}",
            &"a".repeat(256),
        ] {
            assert!(!refused_scope(good).await, "must be accepted: {good:?}");
        }
    });
}

#[test]
fn a_binding_source_on_a_slot_that_is_no_grouping_is_refused_by_the_tables() {
    run(async {
        let conn = store(&import(&lanes_file("all", "grouping = \"assignee\""))).await;
        let mut refused = 0;
        for t in ["forms_binding", "forms_node_binding"] {
            let mut q = conn
                .query(&format!("SELECT count(*) FROM {t}"), ())
                .await
                .unwrap();
            let rows = q.next().await.unwrap().unwrap().get_value(0).unwrap();
            if rows == Value::Integer(0) {
                continue; // this layout keeps its bindings in the other table
            }
            let n = format!("UPDATE {t} SET bind_from = 'records', name = NULL, source = 2");
            assert!(
                conn.execute(&n, ()).await.is_err(),
                "{t}: source only on grouping"
            );
            refused += 1;
        }
        assert!(refused > 0, "a binding table had rows");
    });
}

#[test]
fn a_bad_token_in_the_rows_is_a_store_error_not_no_scope() {
    // the engine would refuse it; here the rows arrive another way (a broken file, another engine): from_rows must say so
    let mut rows = to_rows(
        &import(&lanes_file("lanes:assignee", GROUPING)),
        &meta(Origin::Package),
    )
    .unwrap();
    let at = ALL
        .iter()
        .find(|t| t.name == "forms_source")
        .unwrap()
        .cols
        .iter()
        .position(|c| c.name == "scope")
        .unwrap();
    for bad in ["a b", "x\"y", "x\\y", "", "caf\u{e9}"] {
        rows.tables.get_mut("forms_source").unwrap()[1][at] = Cell::Text(bad.to_owned());
        let e = from_rows(&rows, &id()).expect_err(bad);
        assert!(
            e.iter().any(|c| c.code == LayoutCode::StoreBroken
                && c.param("detail")
                    .is_some_and(|d| d.contains("forms_source.scope"))),
            "{bad:?}: {e:?}"
        );
    }
    rows.tables.get_mut("forms_source").unwrap()[1][at] = Cell::Text("a".repeat(257));
    assert!(from_rows(&rows, &id()).is_err(), "257 bytes");
    rows.tables.get_mut("forms_source").unwrap()[1][at] = Cell::Text("lanes:assignee".to_owned());
    assert!(from_rows(&rows, &id()).is_ok(), "and the good one reads");
}
