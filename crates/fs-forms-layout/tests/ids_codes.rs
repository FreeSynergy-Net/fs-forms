//! K59 (codes), K62 (element ids), natural ids.
mod support;

use fs_forms_layout::*;

#[test]
fn k62_elem_id_rule() {
    let ok = ["a", "block:x/y", &"a".repeat(128)];
    ok.iter()
        .for_each(|t| assert!(ElemId::parse(t).is_ok(), "{t}"));
    let bad = ["", "-a", "A", "a b", &"a".repeat(129), "\u{e4}"];
    for t in bad {
        assert_eq!(
            ElemId::parse(t).unwrap_err().code,
            LayoutCode::ElemInvalid,
            "{t}"
        );
    }
}

#[test]
fn natural_ids_are_deterministic_and_valid() {
    let a = ElemId::natural(ElemKind::Block, &["board", "main"]);
    assert_eq!(a.as_str(), "block:board/main");
    assert_eq!(a, ElemId::natural(ElemKind::Block, &["board", "main"]));
    assert!(ElemId::parse(ElemId::natural(ElemKind::Message, &["Key X", "de"]).as_str()).is_ok());
}

#[test]
fn k59_codes_are_unique_kebab_and_round_trip() {
    assert_eq!(LayoutCode::ALL.len(), 43);
    let mut seen = std::collections::HashSet::new();
    for c in LayoutCode::ALL {
        let name = c.as_str();
        assert!(seen.insert(name), "{name} twice");
        assert!(
            name.chars().all(|x| x.is_ascii_lowercase() || x == '-'),
            "{name}"
        );
        assert_eq!(c.message_key(), format!("layout-error-{name}"));
        assert_eq!(LayoutCode::parse(name), Some(*c));
    }
}

#[test]
fn coded_prints_code_and_parameters() {
    let c = Coded::new(LayoutCode::RevChanged)
        .with("id", "x")
        .with("expected", 1);
    assert_eq!(c.to_string(), "layout-error-rev-changed id=x expected=1");
}
