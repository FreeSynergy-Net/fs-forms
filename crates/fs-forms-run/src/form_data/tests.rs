//! The board made from the channels of a form (order of appearance, labels, columns).
//!
//! Pattern: none (tests).

use super::*;
use fs_forms_bind::FormRef;

fn cards(rows: Vec<FormRow>) -> FormRecords {
    FormRecords {
        provider: "fake".into(),
        forms: vec![FormData {
            may: Vec::new(),
            commands: Vec::new(),
            columns: Vec::new(),
            declared: ModuleForm {
                form: FormRef {
                    name: "cards-in-columns".into(),
                    major: 1,
                    minor: 0,
                },
                map: Vec::new(),
                fields: Vec::new(),
            },
            rows,
        }],
        ..FormRecords::default()
    }
}

#[test]
fn the_board_is_made_from_the_channels_column_and_title_in_order_of_appearance() {
    let r = cards(vec![
        FormRow::of("a", &[("title", "One"), ("column", "To do")]),
        FormRow::of("b", &[("title", "Two"), ("column", "Doing")]),
        FormRow::of("c", &[("title", "Three"), ("column", "To do")]),
        FormRow::of("d", &[("title", "No column")]),
    ]);
    let cols = r.columns();
    let names: Vec<_> = cols.iter().map(|c| c.title.as_str()).collect();
    assert_eq!(names, ["To do", "Doing"]);
    let keys: Vec<Vec<_>> = cols
        .iter()
        .map(|c| c.cards.iter().map(|k| k.key.as_str()).collect())
        .collect();
    assert_eq!(keys, [vec!["a", "c"], vec!["b"]]);
    assert!(r.is_sample());
}

#[test]
fn only_the_provider_fake_is_sample_data_and_a_missing_form_gives_no_columns() {
    let mut r = cards(Vec::new());
    r.provider = "tasks".into();
    assert!(!r.is_sample() && r.columns().is_empty());
    assert_eq!(FormRecords::default().columns().len(), 0);
}

#[test]
fn a_value_is_read_by_channel_and_an_empty_one_is_none() {
    let row = FormRow::of("k", &[("label", "x"), ("parent", "")]);
    assert_eq!(row.get("label"), Some("x"));
    assert_eq!(row.get("parent"), None);
    assert_eq!(row.all("nope").len(), 0);
}
