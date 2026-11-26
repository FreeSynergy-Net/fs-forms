//! The head of a field is a WORD when the module says what its type is (`FormData.type_description`, the word id `person.<field>`
//! of the type catalog) and its words are handed to the host; without the description it stays the name of the field.
//!
//! Pattern: Characterization test of the Port (`FormData`, `ModuleWords`) with the person fixture; the oracle is the description.

mod sample_source;

use fs_components_core::form_read::FieldInfo;
use fs_components_words::Locale;
use fs_forms_run::{FormData, ModuleWords, input_of};
use fs_forms_testdata::SourceSpec;
use sample_source::{PersonWords, person_type, records_of};

const LIST: &str = include_str!("../../../forms/people/P-F1-liste.toml");

fn list() -> FormData {
    let records = records_of(SourceSpec::from_wish_text(LIST).expect("the wish reads"));
    records.forms().first().expect("a form of persons").clone()
}

/// The fields of the channel `columns` as the component is given them.
fn columns(data: &FormData) -> Vec<FieldInfo> {
    input_of(data)
        .channels
        .into_iter()
        .find(|c| c.name == "columns")
        .expect("columns")
        .fields
}

fn said(locale: &str) -> impl Fn(&str) -> String {
    let words = PersonWords.in_locale(&Locale::parse(locale).expect("a locale"));
    move |key: &str| {
        let said = words.say(
            &fs_components_words::TextId::new(key),
            &fs_components_words::Args::new(),
        );
        said.text().to_owned()
    }
}

#[test]
fn with_the_description_the_head_is_the_word_of_the_field() {
    let data = list();
    assert!(data.type_description.is_some(), "the fixture hands it over");
    let fields = columns(&data);
    assert!(fields.len() > 1);
    for f in &fields {
        assert_eq!(f.label, format!("person.{}", f.name), "{}", f.name);
    }
    let heads_en: Vec<String> = fields.iter().map(|f| f.shown(&said("en"))).collect();
    let heads_de: Vec<String> = fields.iter().map(|f| f.shown(&said("de"))).collect();
    assert_eq!(heads_en[0], "Email addresses");
    assert_eq!(heads_de[0], "E-Mail-Adressen");
    for (f, head) in fields.iter().zip(&heads_en) {
        assert_ne!(head, &f.name, "a word, not the field name");
    }
}

#[test]
fn without_the_description_the_head_stays_the_name_of_the_field() {
    let mut data = list();
    data.type_description = None;
    for f in columns(&data) {
        assert!(f.label.is_empty(), "{}", f.name);
        assert_eq!(f.shown(&said("en")), f.name);
    }
}

#[test]
fn a_field_the_description_does_not_name_keeps_its_name() {
    let mut data = list();
    let description = data.type_description.as_mut().expect("described");
    description.fields.retain(|f| f.name != "titles");
    let fields = columns(&data);
    let titles = fields.iter().find(|f| f.name == "titles").expect("titles");
    assert_eq!(titles.label, "");
    assert_eq!(titles.shown(&said("en")), "titles");
    assert_eq!(person_type().fields.len(), 6, "the fixture is untouched");
}
