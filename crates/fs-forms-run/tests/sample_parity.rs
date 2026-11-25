//! Package 5a, parity old <-> new: what the hand-built `SampleBoard` had (three columns, five cards, a card without an assignee,
//! labels, a due date, gestures that move and add) the wish `[sample]` of `tests/fixtures/sample/board-parity.toml` makes again, and
//! the same gestures work on it. The cases are read from the data (nothing is named here); the old board stays beside until package 4b.
//!
//! Pattern: Characterization test (the old fixture and the new source answer the same questions).

mod sample_source;

use fs_components_kanban::Answer;
use fs_forms_run::{Records, SampleBoard, form_report};
use sample_source::{FORM, add_card, board, card, column, columns, move_card, other_column, rows};

/// What a source of board data shows, by the questions the old tests asked of it.
#[derive(Debug, PartialEq, Eq)]
struct Shape {
    cards: usize,
    columns: usize,
    without_assignee: bool,
    with_label: bool,
    is_sample: bool,
    gestures: Vec<&'static str>,
}

fn shape_of(records: &dyn Records) -> Shape {
    let rows = rows(records);
    let given = |r: &fs_forms_run::FormRow| r.all("assignee").iter().any(|v| !v.is_empty());
    Shape {
        cards: rows.len(),
        columns: columns(records).len(),
        without_assignee: rows.iter().any(|r| !given(r)),
        with_label: rows
            .iter()
            .any(|r| r.all("facts").iter().any(|f| !f.is_empty())),
        is_sample: records.is_sample(),
        gestures: ["move", "create"]
            .into_iter()
            .filter(|c| records.command_of(FORM, c).is_some())
            .collect(),
    }
}

#[test]
fn the_new_source_has_the_cases_of_the_old_board() {
    let old = SampleBoard::default();
    let new = board();
    let (o, n) = (shape_of(&old), shape_of(&*new));
    // the old board: five cards in three columns, some have no assignee, labels in the facts, sample data, move and create
    assert_eq!((n.cards, n.columns), (o.cards, o.columns), "{n:?} / {o:?}");
    // NOT the same: the old board had a card with a named assignee; the builder cannot make one (an `assignee` is a reference to a record
    // of another kind, the builder only points at records of its own set): card in the report of package 5a, so no card has one here
    assert!(n.without_assignee && n.with_label, "{n:?}");
    assert_eq!(n, Shape { ..o }, "the same questions give the same answers");
}

#[test]
fn a_card_moves_to_another_column_and_a_card_is_added_by_the_bound_gestures() {
    let b = board();
    let (key, to) = (card(0), other_column(&card(0)));
    assert_eq!(move_card(&*b, &key, &to), Answer::Accepted);
    let at = |records: &dyn Records, k: &str| {
        rows(records)
            .iter()
            .find(|r| r.key == k)
            .and_then(|r| r.get("column").map(str::to_owned))
    };
    assert_eq!(at(&*b, &key), Some(to), "the card is in its new column");
    let before = rows(&*b).len();
    assert_eq!(add_card(&*b, &column(0), "a new card"), Answer::Accepted);
    assert_eq!(rows(&*b).len(), before + 1);
    assert!(
        rows(&*b)
            .iter()
            .any(|r| r.get("title") == Some("a new card"))
    );
}

#[test]
fn what_the_old_board_refused_the_new_source_refuses_too() {
    let b = board();
    let old = SampleBoard::default();
    for (key, to) in [("NOPE", "nowhere"), ("NOPE", "")] {
        assert_ne!(move_card(&*b, key, to), Answer::Accepted);
        assert_ne!(
            fs_forms_run::sample_move_card(&old, key, to),
            Answer::Accepted
        );
    }
    let to = column(0);
    assert_ne!(move_card(&*b, "NOPE", &to), Answer::Accepted);
    assert!(
        form_report(&fs_forms_run::FormRecords {
            forms: b.forms().to_vec(),
            ..fs_forms_run::FormRecords::default()
        })
        .is_empty(),
        "the form checks of the host find nothing wrong with the new data"
    );
}
