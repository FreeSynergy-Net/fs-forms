//! L1: format 4 (free scope word of a source, source number of a `grouping` binding). Reading 3 and 4, writing 4, the round trips,
//! and the counter-tests: each must go red when its rule is broken (grammar, length, unknown source, source on a non-grouping slot).
use std::path::Path;

use fs_forms_exchange::{Format, read_layout, write_layout};
use fs_forms_layout::{
    BindFrom, Coded, Layout, LayoutCode, LayoutDoc, Limits, OpenCatalog, SCOPE_TOKEN_MAX,
    ScopeToken, SourceScope,
};

/// A canonical format 4 file: two sources (the second with a module scope), a grouping binding that takes its heads from source 2.
fn lanes_file(schema: u32, scope2: &str, binding: &str) -> String {
    format!(
        "schema = {schema}\nkind = \"frontend\"\nid = \"board\"\nextends = \"net.example.Tasks\"\nexec = \"fs-forms-run\"\ntitle = \"t\"\n\n\
[[source]]\nelem = \"source:1\"\nn = 1\ncontract = \"planning@>=0.0.1, <0.1.0\"\ntype = \"Task\"\nscope = \"selected-list\"\n\n\
[[source]]\nelem = \"source:2\"\nn = 2\ncontract = \"planning@>=0.0.1, <0.1.0\"\ntype = \"Lane\"\nscope = \"{scope2}\"\n\n\
[[page]]\nelem = \"page:board\"\npage = \"board\"\nframe = \"app-window\"\nnavigation = \"single\"\n\n\
[[page.place]]\nelem = \"block:board/main\"\nat = \"main\"\nblock = \"cards-in-columns\"\nsource = 1\n\n\
[[page.place.bind]]\nelem = \"binding:board/main/lanes/0\"\nslot = \"lanes\"\nnth = 0\n{binding}\n\n\
[[message]]\nelem = \"message:t/de\"\nkey = \"t\"\nlang = \"de\"\ntext = \"T\"\n"
    )
}
const GROUPING: &str = "grouping = \"assignee\"\nsource = 2";

fn read(text: &str) -> Result<LayoutDoc, Vec<Coded<LayoutCode>>> {
    read_layout(&mut text.as_bytes(), Path::new("t.toml"), Format::Toml)
}
fn checked(text: &str) -> Result<Layout, Vec<Coded<LayoutCode>>> {
    read(text)?.check(&OpenCatalog, &Limits::declared())
}
fn layout(text: &str) -> Layout {
    checked(text).expect("read and check")
}
fn export(l: &Layout) -> String {
    let mut out = Vec::new();
    write_layout(l, Format::Toml, true, &mut out).expect("write");
    String::from_utf8(out).expect("utf8")
}
fn causes(text: &str) -> Vec<String> {
    checked(text)
        .expect_err("must fail")
        .iter()
        .map(|c| c.param("cause").unwrap_or("-").to_owned())
        .collect()
}

#[test]
fn format_4_round_trip_is_byte_equal_and_keeps_scope_and_source() {
    let f4 = lanes_file(4, "lanes:assignee", GROUPING);
    let l = layout(&f4);
    assert_eq!(export(&l), f4, "4 to 4: the same bytes");
    let d = l.doc();
    assert_eq!(d.sources[0].v.scope, Some(SourceScope::SelectedList));
    assert_eq!(
        d.sources[1].v.scope,
        ScopeToken::parse("lanes:assignee").map(SourceScope::Module)
    );
    let b = &d.pages[0].v.leaves().next().expect("leaf").v.bindings[0].v;
    assert_eq!(
        (b.source, &b.from),
        (Some(2), &BindFrom::Grouping("assignee".into()))
    );
}

#[test]
fn format_3_reads_and_is_written_as_4() {
    // a format 3 file (no module scope, no binding source) is a subset: same reading path, the writer says 4
    let f3 = lanes_file(3, "all", "grouping = \"assignee\"");
    let l = layout(&f3);
    assert_eq!(
        export(&l),
        lanes_file(4, "all", "grouping = \"assignee\""),
        "3 to 4: only the schema line differs"
    );
    assert_eq!(export(&layout(&export(&l))), export(&l), "and 4 stays 4");
}

#[test]
fn host_words_stay_host_words_and_any_other_token_goes_unchanged() {
    for (w, want) in [
        ("all", SourceScope::All),
        ("selected-list", SourceScope::SelectedList),
    ] {
        assert_eq!(SourceScope::from_text(w), Some(want));
    }
    for w in [
        "lanes:assignee",
        "lanes:assignee:board-1",
        "x",
        "A~z!#[]^_`{|}",
    ] {
        let s = SourceScope::from_text(w).expect(w);
        assert!(matches!(s, SourceScope::Module(_)), "{w}");
        assert_eq!(s.text(), w, "unchanged");
    }
}

#[test]
fn a_broken_scope_grammar_is_red_at_reading() {
    let long = "a".repeat(SCOPE_TOKEN_MAX + 1);
    for (name, w) in [
        ("space", "a b".to_owned()),
        ("quote", "x\\\"y".to_owned()),
        ("backslash", "x\\\\y".to_owned()),
        ("empty", String::new()),
        ("too long: 257 bytes", long),
        ("non-ASCII", "lanes:\u{e4}".to_owned()),
    ] {
        let e = read(&lanes_file(4, &w, GROUPING)).expect_err(name);
        let c: Vec<_> = e
            .iter()
            .map(|c| c.param("cause").unwrap_or("-").to_owned())
            .collect();
        assert!(c.iter().any(|c| c == "scope-invalid"), "{name}: {c:?}");
    }
    let max = "a".repeat(SCOPE_TOKEN_MAX);
    assert!(
        read(&lanes_file(4, &max, GROUPING)).is_ok(),
        "256 bytes is still fine"
    );
}

#[test]
fn the_grammar_of_the_token_is_the_rfc_6749_one() {
    for ok in ["a", "lanes:assignee", "!", "~", "#", "[", "]", "all-but"] {
        assert!(ScopeToken::parse(ok).is_some(), "{ok}");
    }
    for bad in ["", " ", "a b", "\"", "\\", "\t", "a\nb", "\u{7f}", "\u{e4}"] {
        assert!(ScopeToken::parse(bad).is_none(), "{bad:?}");
    }
    assert!(ScopeToken::parse(&"x".repeat(256)).is_some());
    assert!(ScopeToken::parse(&"x".repeat(257)).is_none());
}

#[test]
fn a_binding_source_must_exist() {
    let c = causes(&lanes_file(
        4,
        "lanes:assignee",
        "grouping = \"assignee\"\nsource = 9",
    ));
    assert_eq!(c, ["bind-source-unknown"]);
    let c = causes(&lanes_file(
        4,
        "lanes:assignee",
        "grouping = \"assignee\"\nsource = 0",
    ));
    assert_eq!(c, ["bind-source-unknown"], "numbers count from 1");
}

#[test]
fn a_binding_source_is_only_allowed_at_grouping() {
    let field = "field = \"title\"\nrole = \"display\"\nsource = 2";
    assert_eq!(
        causes(&lanes_file(4, "lanes:assignee", field)),
        ["bind-source-kind"]
    );
    let e = checked(&lanes_file(
        4,
        "lanes:assignee",
        "order = \"o\"\nsource = 2",
    ))
    .expect_err("order");
    assert_eq!(e[0].param("kind"), Some("order"));
}

#[test]
fn a_newer_format_is_refused_by_name() {
    let e = checked(&lanes_file(5, "all", GROUPING)).expect_err("5");
    assert_eq!(e[0].code, LayoutCode::FormatNewer);
    assert_eq!(
        (e[0].param("found"), e[0].param("known")),
        (Some("5"), Some("4"))
    );
}
