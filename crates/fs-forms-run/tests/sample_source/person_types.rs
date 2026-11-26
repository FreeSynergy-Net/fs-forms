//! What the module `person` says of its type to the host, as the starter will read it (`catalog/types.bin`, `catalog/texts/`): the
//! description of the type with the word id of each field (`person.<field>`) and the words of the module in German and English.
//! The fields are the ones of the fixed people (`fs-forms-testdata`); the ids and sentences follow the catalog of the module.
//!
//! Pattern: Test Data Builder (the description) + Adapter (`Mf2Texts` as the port `ModuleWords`).

use std::sync::Arc;

use fs_components_mf2::{Mf2Texts, Source};
use fs_components_words::{Locale, Words};
use fs_forms_run::ModuleWords;
use fs_types::BasicType;
use fs_types::describe::{FieldDescription, TypeDescription};

/// The words of the module: the ids are the word ids of the fields.
pub const WORDS: &[Source] = &[
    Source {
        language: "en",
        text: "person.emails = Email addresses\nperson.phones = Phone numbers\nperson.address = Postal address\nperson.surname = Family name\nperson.organizations = Organization\nperson.titles = Job title\n",
    },
    Source {
        language: "de",
        text: "person.emails = E-Mail-Adressen\nperson.phones = Telefonnummern\nperson.address = Postanschrift\nperson.surname = Familienname\nperson.organizations = Organisation\nperson.titles = Funktion\n",
    },
];

/// The description of `Person`: the fields of the fixed people, each with its word id.
#[must_use]
pub fn person_type() -> TypeDescription {
    let mut t = TypeDescription::new("Person", "fs-person", "RFC 9553 Card", "person");
    for (name, basic) in [
        ("emails", BasicType::Email),
        ("phones", BasicType::Phone),
        ("address", BasicType::Address),
        ("surname", BasicType::Name),
        ("organizations", BasicType::Text),
        ("titles", BasicType::Text),
    ] {
        t.fields.push(
            FieldDescription::new(name, basic)
                .multiple()
                .label(&format!("person.{name}")),
        );
    }
    t
}

/// The words of the module as the port the host takes.
#[derive(Debug)]
pub struct PersonWords;

impl ModuleWords for PersonWords {
    fn in_locale(&self, locale: &Locale) -> Arc<dyn Words> {
        Arc::new(Mf2Texts::new(WORDS, locale))
    }
}
