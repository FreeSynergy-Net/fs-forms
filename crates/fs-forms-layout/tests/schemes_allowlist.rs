//! Scheme lists: a positive list, not a block list (RFC 3986 section 3.1 grammar, ASCII case-insensitive compare).
//!
//! Pattern: Specification: the rule is asked directly, with the default policy and with a widened one.

use fs_forms_layout::{
    DEFAULT_ALLOWED, ErrorCode as _, LayoutCode, SchemePolicy, scheme_names_rule,
    scheme_names_rule_with,
};

fn codes(names: &[&str]) -> Vec<LayoutCode> {
    let list: Vec<String> = names.iter().map(|s| (*s).to_owned()).collect();
    scheme_names_rule("e", &list)
        .into_iter()
        .map(|c| c.code)
        .collect()
}

#[test]
fn dangerous_and_unknown_schemes_are_refused() {
    for s in [
        "javascript",
        "vbscript",
        "data",
        "file",
        "blob",
        "foo",
        "about",
        "ftp",
    ] {
        assert_eq!(codes(&[s]), vec![LayoutCode::SchemeNotAllowed], "{s}");
    }
}

#[test]
fn case_tricks_are_refused() {
    assert_eq!(codes(&["JaVaScRiPt"]), vec![LayoutCode::SchemeNotAllowed]);
    assert_eq!(codes(&["DATA"]), vec![LayoutCode::SchemeNotAllowed]);
    // allowed name, but not canonical lower case
    assert_eq!(codes(&["HTTPS"]), vec![LayoutCode::SchemeBad]);
}

#[test]
fn control_characters_and_whitespace_are_bad_names() {
    for s in [
        "java\tscript",
        " javascript",
        "javascript ",
        "java\0script",
        "java\nscript",
        "",
        "1http",
        "ht tp",
        "https:",
        "h\u{fb01}",
    ] {
        assert_eq!(codes(&[s]), vec![LayoutCode::SchemeBad], "{s:?}");
    }
}

#[test]
fn allowed_schemes_pass_and_doubles_are_found() {
    assert_eq!(codes(DEFAULT_ALLOWED), Vec::<LayoutCode>::new());
    assert_eq!(codes(&["https", "HTTPS"]), vec![LayoutCode::SchemeTwice]);
    assert_eq!(codes(&[]), vec![LayoutCode::SchemesEmpty]);
}

#[test]
fn the_list_is_a_parameter_default_stays_closed() {
    let list = vec![
        "mailto".to_owned(),
        "tel".to_owned(),
        "javascript".to_owned(),
    ];
    let wide = SchemePolicy {
        allowed: &["mailto", "tel"],
    };
    let got: Vec<_> = scheme_names_rule_with("e", &list, wide)
        .into_iter()
        .map(|c| c.code)
        .collect();
    assert_eq!(got, vec![LayoutCode::SchemeNotAllowed]);
    assert_eq!(codes(&["mailto"]), vec![LayoutCode::SchemeNotAllowed]);
}

#[test]
fn the_refusal_names_the_scheme() {
    let list = vec!["data".to_owned()];
    let c = &scheme_names_rule("e", &list)[0];
    assert_eq!(c.code.as_str(), "scheme-not-allowed");
}
