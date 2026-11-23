//! `order` is read as the sequence of the cards of a column (r2 review MUST 1). Mutants (by hand, all red): the sort removed; an
//! unstable sort; cards without `order` put in front.
//!
//! Pattern: none (tests).
use std::sync::Arc;

use super::*;
use crate::events::{Ask, Wiring};
use crate::view::ActionView;
use crate::write::fixture::{FakeWriter, data, records};
use fs_components_kanban::{Answer, BoardEvents, BoardSource};
use fs_components_widgets::dialogs::Labels;

fn card(key: &str, column: &str, order: Option<&str>) -> FormRow {
    let mut pairs = vec![("title", key), ("column", column)];
    if let Some(o) = order {
        pairs.push(("order", o));
    }
    FormRow::of(key, &pairs)
}

fn keys(r: &FormRecords, column: usize) -> Vec<String> {
    r.columns()[column]
        .cards
        .iter()
        .map(|c| c.key.clone())
        .collect()
}

fn board(rows: Vec<FormRow>) -> FormRecords {
    let mut d = data();
    d.rows = rows;
    FormRecords {
        forms: vec![d],
        ..FormRecords::default()
    }
}

#[test]
fn a_module_that_delivers_unsorted_is_drawn_in_the_sequence_of_order() {
    let r = board(vec![
        card("c", "todo", Some("2")),
        card("a", "todo", Some("0")),
        card("b", "todo", Some("1")),
        card("x", "doing", Some("10")),
        card("y", "doing", Some("9")),
    ]);
    assert_eq!(keys(&r, 0), ["a", "b", "c"]);
    assert_eq!(keys(&r, 1), ["y", "x"]);
}

#[test]
fn cards_without_a_number_stay_in_the_order_they_came_and_stand_behind_the_numbered_ones() {
    let r = board(vec![
        card("n1", "todo", None),
        card("b", "todo", Some("5")),
        card("n2", "todo", Some("soon")),
        card("a", "todo", Some("5")),
        card("n3", "todo", Some("")),
    ]);
    // equal numbers keep the delivery (stable): b before a
    assert_eq!(keys(&r, 0), ["b", "a", "n1", "n2", "n3"]);
    let plain = board(vec![card("z", "todo", None), card("m", "todo", None)]);
    assert_eq!(keys(&plain, 0), ["z", "m"], "no order at all: the delivery");
}

#[test]
fn the_drop_index_counts_in_the_shown_sequence_and_goes_to_the_module_unchanged() {
    let w = Arc::new(FakeWriter::default());
    let mut d = data();
    d.rows = vec![
        card("c", "doing", Some("2")).allowing(&[]),
        card("a", "doing", Some("0")),
        card("m", "todo", Some("7")).allowing(&["card.move"]),
    ];
    let on = Wiring {
        actions: Arc::new(vec![ActionView {
            trigger: "drop".into(),
            command: "card.move".into(),
        }]),
        records: Arc::new(records(&w, d)),
        ask: Arc::new(Ask::default()),
        say: Arc::new(|k: &str| k.to_owned()),
        labels: Labels::default(),
    };
    let cols = on.columns();
    let doing = cols.iter().find(|c| c.key == "doing").unwrap();
    let shown: Vec<_> = doing.cards.iter().map(|c| c.key.clone()).collect();
    assert_eq!(shown, ["a", "c"], "the target column is shown sorted");
    assert_eq!(
        on.move_card_at("m", "todo", "doing", Some(1)),
        Answer::Accepted
    );
    let calls = w.calls.lock().unwrap();
    assert!(
        calls[0]
            .args
            .contains(&("index".to_owned(), "1".to_owned()))
    );
}

#[test]
fn many_cards_with_the_same_number_keep_the_delivery_the_sort_is_stable() {
    let num = |i: usize| (i * 7 + i / 3) % 5;
    let rows: Vec<_> = (0..200)
        .map(|i| card(&format!("k{i:03}"), "todo", Some(&num(i).to_string())))
        .collect();
    let mut want: Vec<usize> = (0..200).collect();
    want.sort_by_key(|i| (num(*i), *i)); // by number, then by delivery
    let want: Vec<String> = want.iter().map(|i| format!("k{i:03}")).collect();
    assert_eq!(keys(&board(rows), 0), want);
}
