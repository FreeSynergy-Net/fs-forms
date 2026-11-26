//! The cell under a head belongs to the field of that head: in every people description each value of each person stands under the
//! field the module gave it, a field with two values keeps both (Amara has two e-mails), a field without a value stays empty and
//! shifts nothing (Assistent Sol has no phone, Amara no organization).
//!
//! Pattern: Property test over the fixed people (the data are the directory's `sample()`, nothing is invented here); the oracle is the
//! sample set of `fs-forms-testdata` (the values by field as the module sends them), the subject the host's rows (`FormRow::field`).

mod sample_source;

use fs_forms_testdata::SourceSpec;
use sample_source::{records_scoped, scope_of};

/// The shipped people descriptions.
const FILES: [&str; 4] = [
    include_str!("../../../forms/people/P-F1-liste.toml"),
    include_str!("../../../forms/people/P-F2-einzel.toml"),
    include_str!("../../../forms/people/P-F3-karten.toml"),
    include_str!("../../../forms/people/P-F4-kontakte.toml"),
];

/// Every `(person, channel, field)` of every description: the values the host reads equal the values the module sent for that field.
#[test]
fn every_cell_holds_the_values_of_its_own_field() {
    let mut cells = 0;
    let mut empty = 0;
    for text in FILES {
        let spec = SourceSpec::from_wish_text(text).expect("the wish reads");
        let set = spec.clone().build().expect("the wish builds");
        let records = records_scoped(spec, &scope_of(text));
        let forms = records.forms();
        let form = forms.first().expect("a form of persons");
        let key = format!(
            "{}@{}.{}.0",
            form.declared.form.name, form.declared.form.major, form.declared.form.minor
        );
        let from_module = set.rows(&key).expect("rows");
        for row in &form.rows {
            let module = from_module
                .iter()
                .find(|s| s.key == row.key)
                .expect("sent row");
            for (channel, fields) in &form.declared.map {
                for field in fields {
                    let want: Vec<&str> = module
                        .values
                        .iter()
                        .find(|(p, _)| p == field)
                        .map_or_else(Vec::new, |(_, v)| v.iter().map(String::as_str).collect());
                    assert_eq!(
                        row.field(channel, field),
                        want,
                        "{} {channel}/{field}",
                        row.key
                    );
                    cells += 1;
                    empty += usize::from(want.is_empty());
                }
            }
        }
    }
    assert!(
        cells > 0 && empty > 0,
        "the fixture has empty fields: {empty}/{cells}"
    );
}

/// The case that shifted before: a person with two e-mails AND a field left empty, on the single page (`fields`: all the contact fields of the card): the e-mails stay under the e-mail field, the empty fields stay empty.
#[test]
fn amara_with_two_mails_and_an_empty_field_shifts_nothing() {
    let spec = SourceSpec::from_wish_text(FILES[1]).expect("the wish reads");
    let records = records_scoped(spec, &scope_of(FILES[1]));
    let forms = records.forms();
    let form = &forms[0];
    let amara = form
        .rows
        .iter()
        .find(|r| r.get("label") == Some("Amara Zolenko"))
        .expect("Amara");
    let facts = &form
        .declared
        .map
        .iter()
        .find(|(c, _)| c == "fields")
        .expect("fields")
        .1;
    let mails = amara.field("fields", "emails");
    assert_eq!(mails.len(), 2, "both e-mails of Amara: {mails:?}");
    assert!(mails.iter().all(|m| m.contains('@')), "{mails:?}");
    let empty: Vec<&String> = facts
        .iter()
        .filter(|f| amara.field("fields", f).is_empty())
        .collect();
    assert!(
        !empty.is_empty(),
        "Amara has a field without a value in {facts:?}"
    );
    for f in empty {
        assert!(amara.field("fields", f).is_empty(), "{f} stays empty");
    }
    let phones = amara.field("fields", "phones");
    assert!(phones.iter().all(|p| p.starts_with('+')), "{phones:?}");
}
