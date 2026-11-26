//! The fixed people (`person-directory.sample`): the same on every run, the same as the directory says, a JSContact card per person,
//! never a raw identifier.

mod fixed_support;

use fixed_support::golden;
use fs_forms_testdata::{
    Anchor, Period, SampleBuilder, SampleCode, SampleRow, SampleSet, SampleSpec, SetWish,
    standard_fixed,
};
use fs_person_directory::{
    EntryState, ExcerptField, ExcerptValue, PersonArt, PersonDirectory, Resolved, sample,
    sample_card, sample_name, sample_ref,
};
use fs_types::{BasicValue, Zone};

const SOURCE: &str = "person-directory.sample";
const LIST: &str = "rows@1.0.0";
const SINGLE: &str = "single-record@1.2.0";
const TILES: &str = "tiles@1.1.0";

fn wish(form: &str) -> SetWish {
    SetWish::new(form, "Profile", 0).fixed(SOURCE)
}

fn spec(sets: Vec<SetWish>) -> SampleSpec {
    SampleSpec {
        seed: 1,
        anchor: Anchor {
            period: Period::Month(2026, 11),
            zone: Zone::iana("Europe/Berlin").unwrap(),
        },
        sets,
        forbid: Vec::new(),
    }
}

/// The three forms of the three frontends in one set.
fn set() -> SampleSet {
    let sets = ["rows@^1.0.0", "single-record@^1.2.0", "tiles@^1.0.0"]
        .map(wish)
        .to_vec();
    SampleBuilder::new(spec(sets)).build().expect("builds")
}

fn rows(form: &str) -> Vec<SampleRow> {
    set().rows(form).expect("rows")
}

fn values<'a>(row: &'a SampleRow, path: &str) -> &'a [String] {
    row.values
        .iter()
        .find(|(p, _)| p == path)
        .map_or(&[], |(_, v)| v.as_slice())
}

#[test]
fn the_same_description_twice_gives_the_same_set() {
    assert_eq!(set().render(), set().render());
}

#[test]
fn golden_three_forms() {
    golden("person-directory-sample", &set().render());
}

#[test]
fn the_three_forms_are_the_ones_of_the_frontends() {
    let forms: Vec<String> = set().slices.iter().map(|s| s.form.clone()).collect();
    assert_eq!(forms, [LIST, SINGLE, TILES]);
}

/// The people the directory says exist: `(number, display)` of every entry that is a confirmed Person and resolves to itself (derived).
async fn people_of_the_directory() -> Vec<(u8, String, String)> {
    let directory = sample();
    let mut out = Vec::new();
    for n in 1..=12u8 {
        let asked = sample_ref(n).expect("ref");
        let Resolved::Found(view) = directory.resolve(&asked).await.expect("resolve") else {
            continue;
        };
        if view.person.id == asked.id
            && view.person.art == PersonArt::Person
            && view.state == EntryState::Confirmed
        {
            out.push((n, view.person.id.canonical(), view.display));
        }
    }
    out
}

#[tokio::test(flavor = "current_thread")]
async fn the_records_are_the_people_of_the_directory_and_nobody_else() {
    let want = people_of_the_directory().await;
    assert!(want.len() >= 3, "the sample has people: {want:?}");
    for form in [LIST, SINGLE, TILES] {
        let got = rows(form);
        let mut keys: Vec<&str> = got.iter().map(|r| r.key.as_str()).collect();
        let mut want_keys: Vec<&str> = want.iter().map(|w| w.1.as_str()).collect();
        keys.sort_unstable();
        want_keys.sort_unstable();
        assert_eq!(keys, want_keys, "{form}");
        for (_, iri, display) in &want {
            let row = got.iter().find(|r| &r.key == iri).expect("row");
            assert_eq!(
                row.get("name"),
                Some(display.as_str()),
                "{form} {iri}: the display name"
            );
        }
    }
}

#[tokio::test(flavor = "current_thread")]
async fn a_profile_a_group_a_deleted_a_hidden_and_a_merged_entry_are_no_records() {
    let have: Vec<String> = rows(LIST).into_iter().map(|r| r.key).collect();
    let directory = sample();
    for n in 1..=12u8 {
        let asked = sample_ref(n).expect("ref");
        let kept = have.contains(&asked.id.canonical());
        let is_person = matches!(directory.resolve(&asked).await.expect("resolve"), Resolved::Found(v) if v.person.id == asked.id
            && v.person.art == PersonArt::Person && v.state == EntryState::Confirmed);
        assert_eq!(kept, is_person, "entry {n} {:?}", sample_name(n));
    }
}

#[test]
fn one_person_with_two_mails_and_two_phones_is_one_record() {
    let card = sample_card(1);
    let count = |f: ExcerptField| card.iter().filter(|v| v.field() == f).count();
    let (mails, phones) = (count(ExcerptField::Email), count(ExcerptField::Phone));
    assert_eq!((mails, phones), (2, 2), "the sample card has two of each");
    let all = rows(SINGLE);
    let amara = all
        .iter()
        .find(|r| r.get("name") == Some("Amara Zolenko"))
        .expect("Amara");
    assert_eq!(values(amara, "emails").len(), mails);
    assert_eq!(values(amara, "phones").len(), phones);
    assert_eq!(values(amara, "addresses"), ["Musterweg 12\n10115 Berlin"]);
    let first_mail = card.iter().find_map(|v| match v {
        ExcerptValue::Email(e) => Some(e.canonical()),
        _ => None,
    });
    assert_eq!(
        values(amara, "emails").first(),
        first_mail.as_ref(),
        "the order of the card"
    );
    assert_eq!(
        all.iter()
            .filter(|r| r.get("name") == Some("Amara Zolenko"))
            .count(),
        1,
        "not twice"
    );
}

#[test]
fn the_list_has_the_short_fields_and_the_tile_its_facts() {
    let set = set();
    let declared = |form: &str| {
        set.declared
            .iter()
            .find(|d| format!("{}@{}.{}.0", d.form.name, d.form.major, d.form.minor) == form)
            .expect("declared")
    };
    let columns: Vec<&String> = declared(LIST)
        .map
        .iter()
        .find(|(c, _)| c == "columns")
        .map(|(_, f)| f.iter().collect())
        .expect("columns");
    assert_eq!(
        columns,
        ["organizations", "titles", "emails", "phones", "nicknames"]
    );
    let facts = declared(TILES)
        .map
        .iter()
        .find(|(c, _)| c == "facts")
        .map(|(_, f)| f.len())
        .expect("facts");
    assert_eq!(facts, 4, "facts takes four fields");
    let single = declared(SINGLE)
        .map
        .iter()
        .find(|(c, _)| c == "fields")
        .expect("fields");
    assert!(single.1.contains(&"emails".to_owned()));
}

#[test]
fn the_object_is_a_jscontact_card_with_the_fields_the_directory_has() {
    let set = set();
    let slice = set.slices.iter().find(|s| s.form == SINGLE).expect("slice");
    let objects = &set.objects[slice.first..slice.first + slice.len];
    let amara = objects
        .iter()
        .find(|o| o.envelope.name == "Amara Zolenko")
        .expect("Amara");
    assert_eq!(amara.content_type, "application/jscontact+json");
    let json = &amara.content;
    assert_eq!(json["@type"], "Card");
    assert_eq!(json["version"], "1.0");
    assert_eq!(json["kind"], "individual");
    assert_eq!(json["emails"].as_object().expect("emails").len(), 2);
    assert_eq!(json["phones"].as_object().expect("phones").len(), 2);
    assert_eq!(json["addresses"]["a01"]["countryCode"], "DE");
    assert_eq!(json["media"]["m01"]["kind"], "photo");
    // Not in the directory excerpt, so not invented: no birthday, no organisation, no context.
    for missing in ["anniversaries", "organizations", "nicknames", "notes"] {
        assert!(json.get(missing).is_none(), "{missing} was invented");
    }
    assert!(json["emails"]["e01"].get("contexts").is_none());
}

/// Whether a text looks like a raw identifier: an IRI, a UUID, or the hex of an object id.
fn raw(text: &str) -> bool {
    text.starts_with("urn:")
        || text.starts_with("http://")
        || text.starts_with("https://")
        || uuid::Uuid::parse_str(text).is_ok()
        || (text.len() == 32 && text.chars().all(|c| c.is_ascii_hexdigit()))
}

#[test]
fn no_text_field_shows_a_raw_identifier() {
    for form in [LIST, SINGLE, TILES] {
        for row in rows(form) {
            for (path, texts) in row.values.iter().filter(|(p, _)| p != "photo") {
                for text in texts {
                    assert!(!raw(text), "{form} {}: {path} = {text}", row.key);
                }
            }
        }
    }
}

#[test]
fn the_photo_is_a_reference_and_only_the_asker_has_one() {
    let with: Vec<String> = rows(TILES)
        .into_iter()
        .filter(|r| !values(r, "photo").is_empty())
        .filter_map(|r| r.get("name").map(str::to_owned))
        .collect();
    assert_eq!(with, ["Amara Zolenko"]);
}

#[test]
fn the_data_have_no_time_of_their_own() {
    assert!(
        set().now.is_none(),
        "a directory of people has no fixed now"
    );
}

#[test]
fn the_type_of_the_wish_must_be_the_type_of_the_source() {
    let wrong = SetWish::new("rows@^1.0.0", "Event", 0).fixed(SOURCE);
    let err = SampleBuilder::new(spec(vec![wrong]))
        .build()
        .expect_err("type");
    assert_eq!(err.code(), SampleCode::FixedType);
}

#[test]
fn the_registry_has_the_source() {
    assert!(standard_fixed().get(SOURCE).is_ok());
}
