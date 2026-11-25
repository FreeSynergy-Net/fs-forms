//! P9: a value the module repeats on one path (`assignee` with two people) reaches the card whole: no second entry is lost.
//!
//! Pattern: none (tests).

use fs_forms_bind::*;

fn record(values: &[(&str, &str)]) -> RecordView {
    RecordView {
        id: "card-1".into(),
        values: values
            .iter()
            .map(|(p, v)| ((*p).to_owned(), (*v).to_owned()))
            .collect(),
        ..RecordView::default()
    }
}

fn fill(path: &str) -> FieldFill {
    let field = FieldDesc {
        path: vec![path.into()],
        basic: Basic::Text,
        target: None,
        read_only: false,
    };
    FieldFill {
        path: field.path.clone(),
        role: Role::Display,
        basic: field.basic,
        field,
        block: "field-text".into(),
        schemes: None,
        input: InputState::NotInput,
    }
}

#[test]
fn every_assignee_of_the_card_is_kept_in_order() {
    let r = record(&[
        ("assignee", "ada"),
        ("title", "T"),
        ("assignee", "bob"),
        ("assignee", "cy"),
    ]);
    assert_eq!(
        field_shown(&fill("assignee"), &r),
        FieldShown::Values(vec!["ada".into(), "bob".into(), "cy".into()])
    );
}

#[test]
fn one_value_stays_a_value_and_empty_entries_are_no_values() {
    let one = record(&[("assignee", "ada"), ("assignee", "")]);
    assert_eq!(
        field_shown(&fill("assignee"), &one),
        FieldShown::Value("ada".into())
    );
    let none = record(&[("assignee", "")]);
    assert_eq!(field_shown(&fill("assignee"), &none), FieldShown::Empty);
}
