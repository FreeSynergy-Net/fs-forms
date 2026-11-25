//! ONE rule source for the per-use scheme list of a field binding. Pure and sync; used by `check` (names, offline), by
//! `check_providers` (whole rule, live) and later by the host plan when a provider was away. Codes are `layout-error-scheme*`
//! everywhere; there is no second family. Stand-in for the rules of fs-types 0.0.4 (`describe.*`) until they land.
//!
//! Pattern: Specification: one pure rule that every caller asks; the allowed set is a `SchemePolicy` (a validated Value Object,
//! Strategy parameter), default = the closed positive list `DEFAULT_ALLOWED`. No block list: URI schemes are an open set, so only
//! named ones pass.

use std::collections::HashSet;
use std::sync::Arc;

use crate::codes::LayoutCode;
use crate::shim::Coded;

/// The schemes a layout may list by default (positive list; only what the descriptions and tests really use). Everything else is
/// `scheme-not-allowed` until the instance gives a wider `SchemePolicy`.
pub const DEFAULT_ALLOWED: &[&str] = &["https", "http", "ssh"];

/// The schemes no policy may ever allow, whatever the instance says: they run code or read local data when opened
/// (`javascript`, `vbscript`: script in the page; `data`, `blob`: inline or in-memory content that bypasses the address; `file`:
/// local files). Fixed floor under every `SchemePolicy`, answered with `scheme-not-allowed`; compared ASCII case-insensitively.
pub const NEVER_ALLOWED: &[&str] = &["javascript", "vbscript", "data", "file", "blob"];

#[derive(Clone, Debug, PartialEq, Eq)]
enum Allowed {
    /// Exactly these names (canonical lower case, no doubles).
    Listed(Arc<[Box<str>]>),
    /// Every name that is a valid scheme and not on the floor: for readers of data a writer already checked.
    AnyValid,
}

/// Which schemes a list may name. A value: cheap to clone (shared list), validated and lower-cased at construction, so a policy
/// can be changed at run time by building a new one (nothing leaks). Default: `DEFAULT_ALLOWED`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchemePolicy(Allowed);

impl Default for SchemePolicy {
    fn default() -> Self {
        Self(Allowed::Listed(
            DEFAULT_ALLOWED.iter().map(|s| Box::from(*s)).collect(),
        ))
    }
}

impl SchemePolicy {
    /// A policy of exactly these names (ASCII case-insensitive, stored lower case, doubles dropped).
    ///
    /// # Errors
    /// `scheme-bad{scheme}` for the first name that is not an RFC 3986 scheme.
    pub fn listed<I, S>(names: I) -> Result<Self, Coded<LayoutCode>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut seen = HashSet::new();
        let mut out: Vec<Box<str>> = Vec::new();
        for n in names {
            let n = n.as_ref();
            if !scheme_grammar(n) {
                return Err(Coded::new(LayoutCode::SchemeBad).with("scheme", n));
            }
            let lower = n.to_ascii_lowercase();
            if seen.insert(lower.clone()) {
                out.push(lower.into());
            }
        }
        Ok(Self(Allowed::Listed(out.into())))
    }

    /// The policy of a reader: the writer (instance, service) already decided which schemes are listed, so every valid name passes
    /// except the fixed floor. Grammar and floor still hold; only the list is not asked again.
    #[must_use]
    pub fn any_valid() -> Self {
        Self(Allowed::AnyValid)
    }

    /// Is this name (any case) allowed? The floor `NEVER_ALLOWED` is always no.
    #[must_use]
    pub fn allows(&self, name: &str) -> bool {
        let lower = name.to_ascii_lowercase();
        if NEVER_ALLOWED.contains(&lower.as_str()) {
            return false;
        }
        match &self.0 {
            Allowed::Listed(l) => l.iter().any(|s| **s == *lower),
            Allowed::AnyValid => scheme_grammar(name),
        }
    }
}

/// The name rules with the default policy, see `scheme_names_rule_with`.
#[must_use]
pub fn scheme_names_rule(elem: &str, list: &[String]) -> Vec<Coded<LayoutCode>> {
    scheme_names_rule_with(elem, list, &SchemePolicy::default())
}

/// The name rules: not empty, no doubles (ASCII case-insensitive), each name an RFC 3986 scheme (`scheme-bad`, so also no
/// whitespace/control characters), only names of the policy and not of the fixed floor `NEVER_ALLOWED` (`scheme-not-allowed`), and in canonical lower case (`scheme-bad`).
#[must_use]
pub fn scheme_names_rule_with(
    elem: &str,
    list: &[String],
    policy: &SchemePolicy,
) -> Vec<Coded<LayoutCode>> {
    let mut out = Vec::new();
    if list.is_empty() {
        out.push(Coded::new(LayoutCode::SchemesEmpty).with("elem", elem));
    }
    let mut seen = HashSet::new();
    for name in list {
        let p = |c| Coded::new(c).with("elem", elem).with("scheme", name);
        if !seen.insert(name.to_ascii_lowercase()) {
            out.push(p(LayoutCode::SchemeTwice));
        } else if !scheme_grammar(name) {
            out.push(p(LayoutCode::SchemeBad));
        } else if !policy.allows(name) {
            out.push(p(LayoutCode::SchemeNotAllowed));
        } else if name.bytes().any(|b| b.is_ascii_uppercase()) {
            out.push(p(LayoutCode::SchemeBad)); // canonical lower case only
        }
    }
    out
}

/// RFC 3986 section 3.1: `ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )`, compared ASCII case-insensitively. Own small function: the
/// url/iri-string crates in the lock file are not dependencies of this crate and parse whole URIs, not a bare name (§8).
fn scheme_grammar(s: &str) -> bool {
    let mut c = s.chars();
    c.next().is_some_and(|f| f.is_ascii_alphabetic())
        && c.all(|x| x.is_ascii_alphanumeric() || "+-.".contains(x))
}

/// The whole rule for one field use. `carries_schemes`: the field's basic type carries schemes (webAddress, link, image);
/// `list` `None` = no list given.
#[must_use]
pub fn schemes_rule(
    elem: &str,
    path: &str,
    carries_schemes: bool,
    list: Option<&[String]>,
    policy: &SchemePolicy,
) -> Vec<Coded<LayoutCode>> {
    match (carries_schemes, list) {
        (true, None) => vec![
            Coded::new(LayoutCode::SchemesMissing)
                .with("elem", elem)
                .with("path", path),
        ],
        (true, Some(l)) => scheme_names_rule_with(elem, l, policy),
        (false, Some(_)) => vec![
            Coded::new(LayoutCode::SchemesUnexpected)
                .with("elem", elem)
                .with("path", path),
        ],
        (false, None) => Vec::new(),
    }
}
