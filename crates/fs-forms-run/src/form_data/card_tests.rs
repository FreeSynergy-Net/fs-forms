//! Mangel 5(b) (SW-4a): the column is its KEY with its name from `column-label`, a column without a card is drawn, the card
//! carries its bar and its facts. Mutants (by hand, all red): the key slugged again; the label dropped; `progress` not read;
//! the facts not hung on the card; the declared columns dropped.
//!
//! Pattern: none (tests).
use super::*;
use crate::write::fixture::data;

fn board(extra: FormData) -> FormRecords {
    FormRecords {
        forms: vec![extra],
        ..FormRecords::default()
    }
}

#[test]
fn the_key_of_a_column_is_its_value_and_its_name_is_the_label() {
    let mut d = data();
    d.rows[0] = FormRow::of(
        "a",
        &[
            ("title", "A"),
            ("column", "To-Do_1"),
            ("column-label", "To do"),
        ],
    );
    let cols = board(d).columns();
    assert_eq!(cols[0].key, "To-Do_1", "no slug: a move names this key");
    assert_eq!(cols[0].title, "To do");
    assert_eq!(cols[1].title, "In progress");
}

#[test]
fn a_column_the_module_names_is_drawn_without_a_card_and_in_its_place() {
    let mut d = data();
    d.columns = vec![
        ("todo".into(), "To do".into()),
        ("doing".into(), "In progress".into()),
        ("done".into(), "Done".into()),
        ("closed".into(), "Closed".into()),
    ];
    let cols = board(d).columns();
    let keys: Vec<_> = cols.iter().map(|c| c.key.as_str()).collect();
    assert_eq!(keys, ["todo", "doing", "done", "closed"]);
    assert!(cols[3].cards.is_empty() && cols[3].title == "Closed");
    assert_eq!(
        cols[0].cards.len(),
        1,
        "the cards still land in their column"
    );
}

#[test]
fn a_card_has_its_bar_and_its_facts_in_the_order_of_the_map() {
    let mut d = data();
    d.rows[0]
        .values
        .push(("progress".into(), vec!["40".into()]));
    d.rows[0].values.push((
        "facts".into(),
        vec!["2026-11-26T12:00:00".into(), "high".into()],
    ));
    d.rows[1]
        .values
        .push(("progress".into(), vec!["250".into()]));
    d.rows[1]
        .values
        .push(("facts".into(), vec![String::new(), "low".into()]));
    let cols = board(d).columns();
    let a = &cols[0].cards[0];
    assert_eq!(a.progress, Some(40));
    let facts: Vec<_> = a
        .facts()
        .iter()
        .map(|f| (f.block.as_str(), f.value.as_str()))
        .collect();
    assert_eq!(
        facts,
        [
            ("field-date-time", "2026-11-26T12:00:00"),
            ("field-text", "high")
        ]
    );
    let b = &cols[1].cards[0];
    assert_eq!(b.progress, Some(100), "more than 100 is cut to 100");
    assert_eq!(b.facts().len(), 2, "a fact without a value keeps its place");
    assert_eq!(b.shown_facts().count(), 1, "but it is not drawn");
    assert_eq!(cols[0].cards[0].title, "A".to_owned().replace('A', "a"));
}
