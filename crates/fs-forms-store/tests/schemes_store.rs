//! The scheme policy of the instance and the tables: what `check` accepted with the instance policy is read back by `from_rows`
//! (writing and reading never contradict each other); the fixed floor still holds on the way out.
//!
//! Pattern: Specification asked through the real path (`to_rows`, `from_rows`), with the instance as the only source of a wider list.
use fs_forms_layout::*;
use fs_forms_store::*;

mod engine_support;
use engine_support::*;

fn mailto_doc() -> LayoutDoc {
    let mut d = full_doc();
    d.pages[0].v.blocks[0].v.bindings[0].v.schemes = Some(vec![
        it("scheme:a/0", "https".to_owned()),
        it("scheme:a/1", "mailto".to_owned()),
    ]);
    d
}

fn instance() -> Limits {
    let ceil = CeilingSet {
        schemes: Some(SchemePolicy::listed(["https", "mailto"]).unwrap()),
        ..CeilingSet::default()
    };
    Limits::effective(&Limits::declared(), &ceil)
}

#[test]
fn what_the_instance_allows_is_read_back_from_the_rows() {
    let l = mailto_doc()
        .check(&OpenCatalog, &instance())
        .expect("check");
    let m = meta(Origin::Package);
    let rows = to_rows(&l, &m).unwrap();
    let back = from_rows(&rows, &m.id).expect("rows of an accepted layout can be read");
    assert_eq!(back, l);
    assert_eq!(back.digest(), l.digest());
}

#[test]
fn the_default_list_still_refuses_it_on_the_way_in() {
    let e = mailto_doc()
        .check(&OpenCatalog, &Limits::declared())
        .unwrap_err();
    assert!(e.iter().any(|c| c.code == LayoutCode::SchemeNotAllowed));
}

/// Rows that name a scheme of the fixed floor are broken whatever the reader asks.
#[test]
fn the_floor_holds_when_reading_rows() {
    let l = mailto_doc().check(&OpenCatalog, &instance()).unwrap();
    let m = meta(Origin::Package);
    let mut rows = to_rows(&l, &m).unwrap();
    let t = rows.tables.get_mut("forms_binding_scheme").expect("table");
    let mut hit = false;
    for r in t.iter_mut() {
        for c in r.iter_mut() {
            if matches!(c, Cell::Text(s) if s == "mailto") {
                *c = Cell::Text("javascript".into());
                hit = true;
            }
        }
    }
    assert!(hit, "the row with mailto was not found");
    let e = from_rows(&rows, &m.id).unwrap_err();
    assert!(e.iter().any(|c| c.code == LayoutCode::SchemeNotAllowed));
}
