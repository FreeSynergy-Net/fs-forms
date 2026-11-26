//! The records of a form (the board itself is read by `fs-components-kanban`, its tests are there): sample data, a row by channel.
//!
//! Pattern: none (tests).

use super::*;
use fs_forms_bind::{Basic, FormRef};

fn declared() -> ModuleForm {
    ModuleForm {
        data_type: String::new(),
        form: FormRef {
            name: "cards-in-columns".into(),
            major: 1,
            minor: 0,
        },
        map: vec![("facts".into(), vec!["due".into(), "mine".into()])],
        fields: vec![("due".into(), Basic::Date), ("mine".into(), Basic::Bool)],
    }
}

#[test]
fn the_input_of_a_component_has_the_channels_with_their_fields_and_the_named_columns() {
    let data = FormData {
        source: 0,
        data_type: String::new(),
        declared: declared(),
        rows: vec![FormRow::of("a", &[("title", "One")])],
        may: Vec::new(),
        commands: Vec::new(),
        columns: vec![("todo".into(), "To do".into())],
        type_description: None,
    };
    let input = input_of(&data);
    assert_eq!(input.form, "cards-in-columns@1.0.0");
    let facts = input.channel("facts").expect("channel facts");
    let blocks: Vec<_> = facts.fields.iter().map(|f| f.block.as_str()).collect();
    assert_eq!(blocks, ["field-date", "field-flag"]);
    assert_eq!(
        input.channel("column"),
        None,
        "only channels the module maps are listed"
    );
    assert_eq!(input.rows.len(), 1);
}

#[test]
fn only_the_provider_fake_is_sample_data() {
    let mut r = FormRecords {
        provider: "fake".into(),
        ..FormRecords::default()
    };
    assert!(r.is_sample());
    r.provider = "tasks".into();
    assert!(!r.is_sample());
}

#[test]
fn a_value_is_read_by_channel_and_an_empty_one_is_none() {
    let row = FormRow::of("k", &[("label", "x"), ("parent", "")]);
    assert_eq!(row.get("label"), Some("x"));
    assert_eq!(row.get("parent"), None);
    assert_eq!(row.all("nope").len(), 0);
}

#[test]
fn the_block_of_a_field_is_the_choice_of_the_catalog_and_empty_when_the_catalog_has_none() {
    let mut declared = declared();
    declared.map = vec![
        ("preview".into(), vec!["photo".into()]),
        ("facts".into(), vec!["mail".into()]),
    ];
    declared.fields = vec![
        ("photo".into(), Basic::Image),
        ("mail".into(), Basic::Email),
    ];
    let data = FormData {
        source: 0,
        data_type: String::new(),
        declared,
        rows: Vec::new(),
        may: Vec::new(),
        commands: Vec::new(),
        columns: Vec::new(),
        type_description: None,
    };
    let input = input_of(&data);
    let block = |channel: &str| {
        input.channel(channel).expect("channel").fields[0]
            .block
            .clone()
    };
    // the catalog has no block for an image (the tile and the sheet draw the face): EMPTY, never a guessed `field-text`
    assert_eq!(block("preview"), "");
    assert_eq!(block("facts"), "field-email");
}
