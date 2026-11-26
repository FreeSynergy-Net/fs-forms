//! The people frontends P-F2 (one person, `detail-page`) and P-F3 (cards, `tile-grid`) are DRAWN by the views of `fs-components`
//! from the data of the host: the sheet and every card carry the NAME of the person (never the key), every fact stands under the word
//! of its own field (two e-mails under ONE label, a missing field shifts nothing) and the card of a person without a photo is a face.
//! The host also draws the block `detail-page`, which stays the frame of other descriptions (`fs-tasks` `list.toml`).
//!
//! Pattern: Characterization test of the Adapter chain (records -> `input_of` -> view model); the source is the `[sample]` section of
//! the shipped description, so a description or a catalog that breaks its data breaks here.

mod sample_source;

use fs_components_core::form_read::FormInput;
use fs_components_widgets::detail_page::form_view::sheet;
use fs_components_widgets::tile_grid::form_view::tiles;
use fs_components_words::{Args, Locale, TextId};
use fs_forms_run::{Blocks, ModuleWords, input_of};
use fs_forms_testdata::SourceSpec;
use sample_source::{PersonWords, records_of};

const SINGLE: &str = include_str!("../../../forms/people/P-F2-einzel.toml");
const CARDS: &str = include_str!("../../../forms/people/P-F3-karten.toml");

fn input(text: &str) -> FormInput {
    let records = records_of(SourceSpec::from_wish_text(text).expect("the wish reads"));
    input_of(records.forms().first().expect("a form of persons"))
}

fn say(key: &str) -> String {
    let words = PersonWords.in_locale(&Locale::parse("en").expect("a locale"));
    words.say(&TextId::new(key), &Args::new()).text().to_owned()
}

#[test]
fn the_card_of_a_person_carries_the_name_the_face_and_the_facts_under_their_words() {
    let input = input(CARDS);
    assert_eq!(input.form, "tiles@1.1.0");
    let cards = tiles(&input, &say);
    let titles: Vec<&str> = cards.iter().map(|c| c.title.as_str()).collect();
    for name in [
        "Amara Zolenko",
        "Assistent Sol",
        "Soo-jin Marchetti",
        "Tariq Lindqvist",
    ] {
        assert!(titles.contains(&name), "{name} has no card: {titles:?}");
    }
    assert!(
        titles.iter().all(|t| !t.starts_with("urn:")),
        "a card is titled with a key: {titles:?}"
    );
    let amara = cards
        .iter()
        .find(|c| c.title == "Amara Zolenko")
        .expect("Amara");
    assert!(amara.face, "no block for an image: the card draws the face");
    let emails = amara
        .facts
        .iter()
        .find(|f| f.label == "Email addresses")
        .expect("the fact has the word of its field");
    assert!(
        emails.values.len() >= 2,
        "two e-mails stay two: {:?}",
        emails.values
    );
    assert!(emails.values.iter().any(|v| v == "amara@example.org"));
}

#[test]
fn the_sheet_of_a_person_carries_the_name_and_every_value_under_its_field() {
    let input = input(SINGLE);
    assert_eq!(input.form, "single-record@1.2.0");
    let amara = input
        .rows
        .iter()
        .find(|r| r.get("label") == Some("Amara Zolenko"))
        .expect("Amara");
    let sheet = sheet(&input, amara, &say);
    assert_eq!(sheet.title, "Amara Zolenko");
    let fields: Vec<_> = sheet.sections.iter().flat_map(|s| &s.fields).collect();
    let emails = fields
        .iter()
        .find(|f| f.label == "Email addresses")
        .expect("e-mails");
    assert!(emails.values.len() >= 2, "{:?}", emails.values);
    let phones = fields
        .iter()
        .find(|f| f.label == "Phone numbers")
        .expect("phones");
    assert!(
        phones.values.iter().all(|v| v.starts_with('+')),
        "a phone slid under another label: {:?}",
        phones.values
    );
    assert!(
        fields.iter().all(|f| f.shown().next().is_some()),
        "a field without a value is not drawn"
    );
}

#[test]
fn the_host_draws_the_page_and_the_cards_and_keeps_the_page_as_a_frame() {
    use fs_forms_run::Catalog;
    for block in ["detail-page", "tile-grid"] {
        assert!(Blocks.draws(block), "{block} has no view in the host");
        assert!(Blocks.built(block), "{block} is not built");
    }
    assert!(
        Blocks::is_frame("detail-page"),
        "the frame of fs-tasks is gone"
    );
    assert!(
        !Blocks::is_frame("tile-grid"),
        "tile-grid is an element, not a frame"
    );
}
