//! The declaration of a placement arrives in the `DrawCtx` the component is given.
//!
//! Pattern: none (tests).

use std::sync::Arc;

use fs_components_styles::StyleId;
use fs_components_words::Locale;

use super::{Ctx, draw_ctx, has, mount};
use crate::after_write::AfterWrite;
use crate::place_info::{PlaceInfo, from_set};
use crate::records::NoRecords;

fn ctx() -> Ctx {
    Ctx {
        style: Arc::new(|| StyleId::Craft),
        records: Arc::new(NoRecords),
        say: Arc::new(|key: &str| key.to_owned()),
        actions: Arc::default(),
        locale: Locale::english(),
        mounts: Arc::default(),
        after_write: AfterWrite::default(),
        feeds: Arc::default(),
        clock: Arc::new(crate::clock::SystemClock),
    }
}

fn place_of(info: &PlaceInfo) -> PlaceInfo {
    draw_ctx(&ctx(), "calendar", "", (StyleId::Craft, false), info).place
}

#[test]
fn week_compact_arrives_at_the_component() {
    let info = from_set([("view", "week"), ("variant", "compact")]);
    let got = place_of(&info);
    assert_eq!(got.view.as_deref(), Some("week"));
    assert_eq!(got.variant.as_deref(), Some("compact"));
}

#[test]
fn no_declaration_is_the_default() {
    assert_eq!(place_of(&from_set([])), PlaceInfo::default());
}

#[test]
fn two_placements_with_other_view_are_two_mounts() {
    let name = crate::blocks::forms_of_blocks()
        .into_iter()
        .map(|(n, _)| n)
        .find(|n| has(n))
        .expect("a block with a view");
    let ctx = ctx();
    let (week, month) = (from_set([("view", "week")]), from_set([("view", "month")]));
    assert!(mount(&ctx, &name, "", &week).is_some());
    assert!(mount(&ctx, &name, "", &month).is_some());
    assert!(mount(&ctx, &name, "", &week).is_some());
    let slots = ctx.mounts.lock().unwrap();
    assert_eq!(slots.len(), 2, "one mount per (block, look, info)");
    assert_ne!(slots[0].info, slots[1].info);
}
