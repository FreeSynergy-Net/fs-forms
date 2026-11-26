//! What the module `person` says of its type to the host, as the starter will read it (`catalog/types.bin`, `catalog/texts/`): the
//! description of the type with the word id of each field, and the words of the module in English and German.
//!
//! The sentences of the ids the module has (`person.emails`, `person.phones`, `person.organizations`, `person.titles`, `person.nicknames`, `person.addresses`) are copied
//! WORD FOR WORD from `texts/{en,de}.mf2` of fs-modules-person (main bfebf590). The fixed people (`fs-forms-testdata`) have the fields of the module under the names of its type catalog.
//!
//! Pattern: Test Data Builder (the description) + Adapter (`Mf2Texts` as the port `ModuleWords`).

use std::sync::Arc;

use fs_components_mf2::{Mf2Texts, Source};
use fs_components_words::{Locale, Words};
use fs_forms_run::ModuleWords;
use fs_types::BasicType;
use fs_types::describe::{FieldDescription, TypeDescription};

/// The words of the module: the ids are the word ids of its fields (verbatim from the module).
pub const WORDS: &[Source] = &[
    Source {
        language: "en",
        text: "person.emails = Email addresses\nperson.phones = Phone numbers\nperson.organizations = Organizations\nperson.titles = Titles and roles\nperson.nicknames = Nicknames\nperson.addresses = Addresses\n",
    },
    Source {
        language: "de",
        text: "person.emails = E-Mail-Adressen\nperson.phones = Telefonnummern\nperson.organizations = Organisationen\nperson.titles = Titel und Rollen\nperson.nicknames = Spitznamen\nperson.addresses = Anschriften\n",
    },
];

/// The description of `Person`: the fields the fixed people share with the module, each with its word id.
#[must_use]
pub fn person_type() -> TypeDescription {
    let mut t = TypeDescription::new("Profile", "fs-person", "RFC 9553 Card", "person");
    for (name, basic) in [
        ("emails", BasicType::Email),
        ("phones", BasicType::Phone),
        ("organizations", BasicType::Text),
        ("titles", BasicType::Text),
        ("nicknames", BasicType::Text),
        ("addresses", BasicType::Text),
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
