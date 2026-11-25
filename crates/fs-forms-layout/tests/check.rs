//! K60, K61, K63, K65, K66, K68, K68a, K69: checking a raw description.
#![allow(clippy::assert_is_empty)] // tests: plain asserts read best
mod support;

use fs_forms_layout::*;
use support::*;

fn codes(r: Result<Layout, Vec<Coded<LayoutCode>>>) -> Vec<LayoutCode> {
    r.expect_err("must fail").iter().map(|c| c.code).collect()
}
fn check(d: LayoutDoc) -> Result<Layout, Vec<Coded<LayoutCode>>> {
    d.check(&OpenCatalog, &Limits::declared())
}
fn first_binding(d: &mut LayoutDoc) -> &mut BindingDoc {
    &mut d.pages[0].v.blocks[0].v.bindings[0].v
}

#[test]
fn k61_round_trip_keeps_layout_and_digest() {
    let l = check(doc("board")).unwrap();
    let again = check(l.to_doc()).unwrap();
    assert_eq!(l, again);
    assert_eq!(l.digest(), again.digest());
}

#[test]
fn k60_format_one_gets_the_same_natural_ids_twice() {
    let mut d = doc("board");
    d.format = 1;
    d.pages[0].elem = None;
    d.pages[0].v.blocks[0].elem = None;
    d.pages[0].v.blocks[0].v.bindings[0].elem = None;
    d.messages[0].elem = None;
    let (a, b) = (check(d.clone()).unwrap(), check(d).unwrap());
    assert_eq!(a, b);
    assert_eq!(
        a.doc().pages[0].v.blocks[0].elem,
        Some(elem("block:board/main"))
    );
    assert_eq!(a.bindings("board", "main").len(), 1);
}

#[test]
fn k63_format_and_elem_problems() {
    let mut d = doc("board");
    d.pages[0].v.blocks[0].elem = None;
    assert_eq!(codes(check(d)), vec![LayoutCode::ElemMissing]);
    let mut d = doc("board");
    d.pages[0].v.blocks[0].v.bindings[0].elem = d.pages[0].v.blocks[0].elem.clone(); // same elem, other kind: fine
    assert!(check(d).is_ok());
    let mut d = doc("board");
    d.messages
        .push(item("message:board-title/de", d.messages[0].v.clone()));
    assert!(codes(check(d)).contains(&LayoutCode::ElemTwice));
    let mut d = doc("board");
    d.format = 5;
    let e = d.check(&OpenCatalog, &Limits::declared()).unwrap_err();
    assert_eq!(
        (e[0].code, e[0].param("found"), e[0].param("known")),
        (LayoutCode::FormatNewer, Some("5"), Some("4"))
    );
    let mut d = doc("board");
    d.format = 0;
    assert_eq!(codes(check(d)), vec![LayoutCode::FormatInvalid]);
}

#[test]
fn all_problems_at_once() {
    let mut d = doc("board");
    d.requires.push(item(
        "requires:a",
        RequiresDoc {
            range: "tasks >0.2".into(),
        },
    ));
    d.messages[0].v.key = "A b".into();
    d.messages[0].v.lang = "de_DE".into();
    d.messages[0].v.message = "{open".into();
    let c = codes(check(d));
    assert_eq!(
        c,
        vec![
            LayoutCode::RangeInvalid,
            LayoutCode::KeyInvalid,
            LayoutCode::LangInvalid,
            LayoutCode::MessageInvalid
        ]
    );
}

struct Cat;
impl CatalogRules for Cat {
    fn block_known(&self, b: &str) -> bool {
        b == "board"
    }
    fn slot_known(&self, _: &str, s: &str) -> bool {
        s == "title"
    }
    fn slot_repeat(&self, _: &str, _: &str) -> u32 {
        1
    }
}

#[test]
fn k65_catalog_names_slots_and_repeat() {
    let mut d = doc("board");
    d.pages[0].v.blocks[0].v.block = "nope".into();
    assert_eq!(
        codes(d.check(&Cat, &Limits::declared())),
        vec![LayoutCode::BlockUnknown]
    );
    let mut d = doc("board");
    first_binding(&mut d).slot = "wrong".into();
    let e = d.check(&Cat, &Limits::declared()).unwrap_err();
    assert_eq!(
        (e[0].code, e[0].param("cause")),
        (LayoutCode::BindingInvalid, Some("slot-unknown"))
    );
    let mut d = doc("board");
    let again = d.pages[0].v.blocks[0].v.bindings[0].v.clone();
    d.pages[0].v.blocks[0].v.bindings.push(item(
        "binding:board/main/title/1",
        BindingDoc { nth: 1, ..again },
    ));
    let e = d.check(&Cat, &Limits::declared()).unwrap_err();
    assert_eq!(e[0].param("cause"), Some("bound-twice"));
}

#[test]
fn k66_scheme_lists() {
    let case = |list: &[&str]| {
        let mut d = doc("board");
        first_binding(&mut d).schemes = Some(
            list.iter()
                .enumerate()
                .map(|(i, s)| item(&format!("scheme:b/{i}"), (*s).to_owned()))
                .collect(),
        );
        check(d)
    };
    assert_eq!(codes(case(&[])), vec![LayoutCode::SchemesEmpty]);
    assert_eq!(
        codes(case(&["javascript"])),
        vec![LayoutCode::SchemeNotAllowed]
    );
    assert_eq!(codes(case(&["ht tp"])), vec![LayoutCode::SchemeBad]);
    assert_eq!(
        codes(case(&["https", "HTTPS"])),
        vec![LayoutCode::SchemeTwice]
    );
    let l = case(&["ssh", "https"]).unwrap();
    assert_eq!(
        l.bindings("board", "main")[0].schemes,
        Some(vec!["ssh".to_owned(), "https".to_owned()])
    ); // order kept
}

#[test]
fn schemes_rule_is_one_source() {
    let pol = SchemePolicy::default();
    let list = vec!["https".to_owned()];
    assert_eq!(
        schemes_rule("e", "p", true, None, &pol)[0].code,
        LayoutCode::SchemesMissing
    );
    assert_eq!(
        schemes_rule("e", "p", false, Some(&list), &pol)[0].code,
        LayoutCode::SchemesUnexpected
    );
    assert!(schemes_rule("e", "p", true, Some(&list), &pol).is_empty());
    assert_eq!(
        schemes_rule("e", "p", true, Some(&[]), &pol)[0].code,
        LayoutCode::SchemesEmpty
    );
}

#[test]
fn k68_limits_report_their_source() {
    let tight = Limits {
        elements_max: 3,
        text_bytes_max: 4,
        ..Limits::declared()
    };
    let e = doc("board").check(&OpenCatalog, &tight).unwrap_err();
    let by = |c: LayoutCode| {
        e.iter()
            .find(|x| x.code == c)
            .and_then(|x| x.param("by"))
            .map(str::to_owned)
    };
    assert_eq!(by(LayoutCode::TooMany), Some("default".to_owned()));
    assert_eq!(by(LayoutCode::TextTooLong), Some("default".to_owned()));
    let exact = Limits {
        elements_max: 4,
        ..Limits::declared()
    }; // page, block, binding, message = 4
    assert!(doc("board").check(&OpenCatalog, &exact).is_ok());
}

#[test]
fn k68_menu_depth() {
    let entry = |kids: Vec<Item<MenuDoc>>| MenuDoc {
        kind: MenuKind::Menu,
        label_key: None,
        command: None,
        on: None,
        shortcut: None,
        view: None,
        children: kids,
    };
    let mut d = doc("board");
    let l3 = entry(vec![item("menu:1.0.0", entry(vec![]))]);
    d.menu = vec![item("menu:1", entry(vec![item("menu:1.0", l3)]))];
    let two = Limits {
        menu_depth_max: 2,
        ..Limits::declared()
    };
    assert_eq!(
        d.clone().check(&OpenCatalog, &two).unwrap_err()[0].param("what"),
        Some("menu-depth")
    );
    assert!(d.check(&OpenCatalog, &Limits::declared()).is_ok());
}

#[test]
fn k68a_ceiling_clamps_and_names_itself() {
    let person = Limits {
        elements_max: 8000,
        ..Limits::declared()
    };
    let ceil = CeilingSet {
        elements_max: Some(5000),
        ..CeilingSet::default()
    };
    let eff = Limits::effective(&person, &ceil);
    assert_eq!(
        (eff.elements_max, eff.by.elements),
        (5000, LimitSource::Ceiling)
    );
    assert_eq!(eff.by.ops, LimitSource::Default);
    let low = Limits {
        elements_max: 3,
        ..eff
    };
    assert_eq!(
        doc("board").check(&OpenCatalog, &low).unwrap_err()[0].param("by"),
        Some("ceiling")
    );
}

#[test]
fn technical_nth_limit() {
    let mut d = doc("board");
    first_binding(&mut d).nth = 16;
    let e = d.check(&OpenCatalog, &Limits::declared()).unwrap_err();
    assert_eq!(
        (
            e[0].code,
            e[0].param("what"),
            e[0].param("by"),
            e[0].param("max")
        ),
        (
            LayoutCode::TooMany,
            Some("binding-nth"),
            Some("technical"),
            Some("16")
        )
    );
}
