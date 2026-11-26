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
    // the fields the module describes have its word id; `address` and `surname` are test data only (the module has none of them)
    let (named, plain): (Vec<_>, Vec<_>) = fields.iter().partition(|f| !f.label.is_empty());
    assert!(named.len() >= 2 && !plain.is_empty());
    for f in &named {
        assert_eq!(f.label, format!("person.{}", f.name), "{}", f.name);
        assert_ne!(f.shown(&said("en")), f.name, "a word, not the field name");
    }
    for f in &plain {
        assert_eq!(f.shown(&said("en")), f.name, "no word id: the field name");
    }
    let heads_en: Vec<String> = fields.iter().map(|f| f.shown(&said("en"))).collect();
    let heads_de: Vec<String> = fields.iter().map(|f| f.shown(&said("de"))).collect();
    assert_eq!(heads_en[0], "Email addresses");
    assert_eq!(heads_de[0], "E-Mail-Adressen");
    assert!(heads_en.contains(&"Titles and roles".to_owned()));
    assert!(heads_de.contains(&"Organisationen".to_owned()));
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
    assert_eq!(person_type().fields.len(), 4, "the fixture is untouched");
}

/// The words of the module reach the host through the hub (`Shared` cache and `Grip`): a decorator that forgets to hand them on
/// would silently leave every head a field name (the default of `Records::module_words` is none).
#[test]
fn the_words_of_the_module_survive_the_hub() {
    let records = records_of(SourceSpec::from_wish_text(LIST).expect("the wish reads"));
    let counts = std::sync::Arc::new(fs_forms_run::HoldCounts::default());
    let module: std::sync::Arc<dyn fs_forms_run::Module> =
        fs_forms_run::FakeModule::new(counts, Box::new(move |_| records.clone()));
    let hub = fs_forms_run::RecordHub::new();
    let key = fs_forms_run::RecordKey {
        range: "net.freesynergy.People".into(),
        scope: None,
    };
    let grip = hub.attach(&key, &module).expect("attached");
    let en = Locale::english();
    for (what, records) in [("grip", grip.records()), ("cache", grip.cache())] {
        let words = records
            .module_words()
            .unwrap_or_else(|| panic!("{what}: the words are lost"))
            .in_locale(&en);
        let said = words.say(
            &fs_components_words::TextId::new("person.emails"),
            &fs_components_words::Args::new(),
        );
        assert_eq!(said.text(), "Email addresses", "{what}");
    }
}
