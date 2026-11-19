//! K67: the live check against the providers fills `Resolved.problems` (blocking) and `Resolved.warnings` (notice only).
#![allow(clippy::assert_is_empty, clippy::many_single_char_names)] // tests: short names and plain asserts read best
mod support;

use std::rc::Rc;

use fs_forms_layout::*;
use support::*;

struct Fake(Result<Option<bool>, ()>);
impl ProviderCatalogs for Fake {
    fn field_has_schemes<'a>(
        &'a self,
        _: &'a str,
        _: &'a str,
        _: &'a str,
    ) -> Pending<'a, Option<bool>> {
        let r = self
            .0
            .map_err(|()| Coded::new(LayoutCode::Unreachable).with("socket", "tasks"));
        Box::pin(std::future::ready(r))
    }
}

fn with_source() -> LayoutDoc {
    let mut d = doc("board");
    let src = SourceDoc {
        range: "tasks >=0.2 <0.3".into(),
        data_type: "Task".into(),
        scope: None,
    };
    d.sources = vec![item("source:1", src)];
    d.pages[0].v.blocks[0].v.source = Some(1);
    d
}

fn resolved(p: Fake) -> Resolved {
    let s = store().with_providers(Rc::new(p));
    install(&s, with_source(), "1.0");
    block_on(s.resolve(&key("board"))).unwrap().unwrap()
}

#[test]
fn missing_list_blocks() {
    let r = resolved(Fake(Ok(Some(true))));
    assert_eq!(r.problems[0].code, LayoutCode::SchemesMissing);
    assert!(r.warnings.is_empty());
}

#[test]
fn unexpected_list_blocks_and_plain_field_is_clean() {
    assert!(resolved(Fake(Ok(Some(false)))).problems.is_empty());
    // the provider does not know the field: said as a warning, never silent
    let unknown = resolved(Fake(Ok(None)));
    assert!(unknown.problems.is_empty());
    assert_eq!(unknown.warnings[0].code, LayoutCode::ProviderUnchecked);
}

#[test]
fn provider_away_is_a_warning_never_a_lock() {
    let r = resolved(Fake(Err(())));
    assert!(r.problems.is_empty());
    assert_eq!(r.warnings[0].code, LayoutCode::ProviderUnchecked);
    assert_eq!(r.warnings[0].param("capability"), Some("tasks"));
}
