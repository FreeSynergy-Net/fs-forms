//! ONE rule source for the per-use scheme list of a field binding. Pure and sync; used by `check` (names, offline), by
//! `check_providers` (whole rule, live) and later by the host plan when a provider was away. Codes are `layout-error-scheme*`
//! everywhere; there is no second family. Stand-in for the rules of fs-types 0.0.4 (`describe.*`) until they land.
//!
//! Pattern: Specification: one pure rule that every caller asks.

use std::collections::HashSet;

use crate::codes::LayoutCode;
use crate::shim::Coded;

/// The name rules: not empty, no `javascript`/`vbscript`, no doubles (case-insensitive), and each name is an RFC 3986 scheme name in
/// its canonical lower case (the table refuses upper case, so the import refuses it too: `scheme-bad`).
#[must_use]
pub fn scheme_names_rule(elem: &str, list: &[String]) -> Vec<Coded<LayoutCode>> {
    let mut out = Vec::new();
    if list.is_empty() {
        out.push(Coded::new(LayoutCode::SchemesEmpty).with("elem", elem));
    }
    let mut seen = HashSet::new();
    for name in list {
        let p = |c| Coded::new(c).with("elem", elem).with("scheme", name);
        let lower = name.to_ascii_lowercase();
        if lower == "javascript" || lower == "vbscript" {
            out.push(p(LayoutCode::SchemeNever));
        } else if !seen.insert(lower) {
            out.push(p(LayoutCode::SchemeTwice));
        } else if !scheme_ok(name) {
            out.push(p(LayoutCode::SchemeBad));
        }
    }
    out
}

fn scheme_ok(s: &str) -> bool {
    let mut c = s.chars();
    c.next().is_some_and(|f| f.is_ascii_lowercase())
        && c.all(|x| x.is_ascii_lowercase() || x.is_ascii_digit() || "+-.".contains(x))
}

/// The whole rule for one field use. `carries_schemes`: the field's basic type carries schemes (webAddress, link, image);
/// `list` `None` = no list given.
#[must_use]
pub fn schemes_rule(
    elem: &str,
    path: &str,
    carries_schemes: bool,
    list: Option<&[String]>,
) -> Vec<Coded<LayoutCode>> {
    match (carries_schemes, list) {
        (true, None) => vec![
            Coded::new(LayoutCode::SchemesMissing)
                .with("elem", elem)
                .with("path", path),
        ],
        (true, Some(l)) => scheme_names_rule(elem, l),
        (false, Some(_)) => vec![
            Coded::new(LayoutCode::SchemesUnexpected)
                .with("elem", elem)
                .with("path", path),
        ],
        (false, None) => Vec::new(),
    }
}
