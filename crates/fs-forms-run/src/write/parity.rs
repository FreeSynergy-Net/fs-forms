//! Parity of the hand-built fake module (`fixture`) and the one the generator makes (`generated`): both answer the same questions the
//! tests of the written way ask, before those tests are switched. Goes with the old side.
//!
//! Pattern: Characterization test.

use super::{fixture, generated};
use crate::form_data::FormData;

/// What the tests of the written way rely on, as values (no literal of either side).
#[derive(Debug, PartialEq, Eq)]
struct Shape {
    form: (String, u32, u32),
    commands: Vec<String>,
    page_may: Vec<String>,
    may_per_row: Vec<usize>,
    columns_per_card: Vec<bool>,
    cards_in_two_columns: bool,
    one_lane: bool,
    every_card_has: Vec<(&'static str, bool)>,
    a_card_has_a_place: bool,
    bindings: Vec<(String, Vec<String>)>,
}

fn shape(d: &FormData) -> Shape {
    let columns = |d: &FormData| {
        let mut c: Vec<_> = d.rows.iter().filter_map(|r| r.get("column")).collect();
        c.sort_unstable();
        c.dedup();
        c.len()
    };
    let lanes = {
        let mut l: Vec<_> = d.rows.iter().filter_map(|r| r.get("lane")).collect();
        l.dedup();
        l.len()
    };
    let has = |ch: &'static str| (ch, d.rows.iter().all(|r| r.get(ch).is_some()));
    Shape {
        form: (
            d.declared.form.name.clone(),
            d.declared.form.major,
            d.declared.form.minor,
        ),
        commands: d.commands.clone(),
        page_may: d.may.clone(),
        may_per_row: {
            let mut n: Vec<_> = d.rows.iter().map(|r| r.may.len()).collect();
            n.sort_unstable();
            n
        },
        columns_per_card: d.rows.iter().map(|r| r.get("column").is_some()).collect(),
        cards_in_two_columns: columns(d) == 2,
        one_lane: lanes == 1,
        every_card_has: ["title", "column-label", "lane"]
            .into_iter()
            .map(has)
            .collect(),
        a_card_has_a_place: d.rows.iter().any(|r| r.get("order").is_some()),
        bindings: ["move", "create", "remove"]
            .into_iter()
            .map(|c| {
                (
                    c.to_owned(),
                    d.declared
                        .map
                        .iter()
                        .find(|(n, _)| n == c)
                        .map(|(_, f)| f.clone())
                        .unwrap_or_default(),
                )
            })
            .collect(),
    }
}

#[test]
fn the_generated_module_answers_what_the_hand_built_one_did() {
    assert_eq!(shape(&fixture::data()), shape(&generated::data()));
}

#[test]
fn every_channel_the_hand_built_declaration_maps_is_mapped_by_the_generated_one() {
    let new = generated::declared();
    for (channel, _) in &fixture::declared().map {
        assert!(
            new.map.iter().any(|(c, _)| c == channel),
            "the generated declaration lacks the channel {channel}"
        );
    }
}

#[test]
fn the_generated_data_is_the_same_on_every_call_and_the_rights_are_the_tests_own() {
    assert_eq!(generated::data(), generated::data());
    let d = generated::data();
    let (a, b) = (generated::first(), generated::second());
    let may = |k: &str| d.rows.iter().find(|r| r.key == k).map(|r| r.may.clone());
    assert_eq!(may(&a), Some(vec![generated::MOVE.to_owned()]));
    assert_eq!(may(&b), Some(Vec::new()));
    assert_ne!(
        generated::column_of(&d, &a),
        generated::other_column(&d, &a)
    );
}
