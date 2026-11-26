//! Fixtures of the lane tests: a module that declares `rows@1.0.0` twice (for `Task` and for `Lane`).
#![allow(dead_code)]

use fs_forms_bind::{Basic, FormRef, ModuleForm};
use fs_forms_run::{Feed, FormData, FormRecords, FormRow, HeadsRef};

pub fn pair(c: &str, f: &str) -> (String, Vec<String>) {
    (c.to_owned(), vec![f.to_owned()])
}

pub fn cards_entry(rows: Vec<FormRow>) -> FormData {
    FormData {
        source: 1,
        data_type: "Task".into(),
        declared: ModuleForm {
            form: FormRef {
                name: "cards-in-columns".into(),
                major: 1,
                minor: 4,
            },
            data_type: "Task".into(),
            map: vec![
                pair("cards", "Task"),
                pair("column", "column"),
                pair("title", "title"),
                pair("assignee", "assignee"),
            ],
            fields: vec![("title".into(), Basic::Text)],
        },
        rows,
        may: Vec::new(),
        commands: Vec::new(),
        columns: Vec::new(),
        type_description: None,
    }
}

/// `rows@1.0.0` as the module declares it: the same form twice, for `Task` and for `Lane`; `label_mapped = false` leaves the required
/// channel `label` without a field.
pub fn rows_entry(
    source: u32,
    data_type: &str,
    label_mapped: bool,
    rows: Vec<FormRow>,
) -> FormData {
    let mut map = vec![pair("items", data_type)];
    if label_mapped {
        map.push(pair("label", "name"));
    }
    FormData {
        source,
        data_type: data_type.into(),
        declared: ModuleForm {
            form: FormRef {
                name: "rows".into(),
                major: 1,
                minor: 0,
            },
            data_type: data_type.into(),
            map,
            fields: vec![("name".into(), Basic::Text)],
        },
        rows,
        may: Vec::new(),
        commands: Vec::new(),
        columns: Vec::new(),
        type_description: None,
    }
}

pub fn card(key: &str, assignee: &str) -> FormRow {
    FormRow::of(
        key,
        &[("title", key), ("column", "todo"), ("assignee", assignee)],
    )
}
pub fn lane(iri: &str, name: &str) -> FormRow {
    FormRow::of(iri, &[("label", name)])
}

pub fn lanes_rows() -> Vec<FormRow> {
    vec![
        lane("urn:p:anna", "Anna"),
        lane("urn:p:ben", "Ben"),
        lane("urn:p:cleo", "Cleo"),
    ]
}

pub fn records(forms: Vec<FormData>) -> FormRecords {
    FormRecords {
        provider: "fake".into(),
        forms,
        ..FormRecords::default()
    }
}

pub fn feeds() -> Vec<Feed> {
    vec![Feed {
        block: "board".into(),
        source: 1,
        data_type: "Task".into(),
        heads: Some(HeadsRef {
            source: 2,
            data_type: "Lane".into(),
            grouping: "assignee".into(),
        }),
        slot_locks: Vec::new(),
    }]
}

pub fn all_forms() -> Vec<FormData> {
    vec![
        cards_entry(vec![card("t1", "Anna"), card("t2", "Ben")]),
        // the module declares `rows@1.0.0` twice: the entry of the tasks comes FIRST, the lane list second
        rows_entry(1, "Task", true, vec![lane("t1", "Task one")]),
        rows_entry(2, "Lane", true, lanes_rows()),
    ]
}

pub fn codes(p: &fs_forms_run::Placed) -> Vec<&str> {
    p.notices.iter().map(|c| c.code.as_str()).collect()
}
