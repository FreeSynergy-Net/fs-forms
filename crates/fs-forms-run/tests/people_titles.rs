//! The four people descriptions give the host rows whose channel `label` holds the NAME of the person, never the key
//! (`urn:uuid:...`), and the fixed people of the sample are all there with the attributes each frontend shows: the table, the
//! contacts workbench and the cards the e-mails, the single page the e-mails, phones and the postal address.
//!
//! Pattern: Characterization test (the facts of the fixed people, stated once) of an Adapter; the source is the `[sample]` section
//! of the shipped description itself, so a description that breaks its data breaks here.

mod sample_source;

use sample_source::{records_of_text, scope_of};

/// The shipped descriptions: name, text, the form it reads, the channel that holds the attributes it shows.
const FILES: [(&str, &str, &str, &str); 4] = [
    (
        "P-F1",
        include_str!("../../../forms/people/P-F1-liste.toml"),
        "rows",
        "columns",
    ),
    (
        "P-F2",
        include_str!("../../../forms/people/P-F2-einzel.toml"),
        "single-record",
        "fields",
    ),
    (
        "P-F3",
        include_str!("../../../forms/people/P-F3-karten.toml"),
        "tiles",
        "facts",
    ),
    (
        "P-F4",
        include_str!("../../../forms/people/P-F4-kontakte.toml"),
        "rows",
        "columns",
    ),
];

/// The people of the sample directory (display names).
const NAMES: [&str; 4] = [
    "Amara Zolenko",
    "Assistent Sol",
    "Soo-jin Marchetti",
    "Tariq Lindqvist",
];

#[test]
fn every_people_description_shows_names_not_keys_in_the_label_channel() {
    for (name, text, form, _) in FILES {
        let records = records_of_text(text);
        let forms = records.forms();
        let found = forms.first().expect("a form of persons");
        assert_eq!(found.declared.form.name, form, "{name}: the form");
        // the list, the table and the cards show every person; the page of one person shows the one its scope names
        let shown: &[&str] = if form == "single-record" {
            &NAMES[..1]
        } else {
            &NAMES
        };
        assert_eq!(
            found.rows.len(),
            shown.len(),
            "{name}: every person (of the scope) is there"
        );
        for row in &found.rows {
            let label = row.get("label").unwrap_or_default();
            assert!(!label.is_empty(), "{name} {}: the name is empty", row.key);
            assert!(
                !label.starts_with("urn:"),
                "{name}: the name is the key ({label})"
            );
            assert_ne!(label, row.key, "{name}: the name is the key");
        }
        let labels: Vec<&str> = found.rows.iter().filter_map(|r| r.get("label")).collect();
        for person in shown {
            assert!(
                labels.contains(person),
                "{name}: {person} is missing in {labels:?}"
            );
        }
    }
}

/// `single-record` is served by the module only with the id of ONE profile (`urn:uuid:<uuid>` or the bare UUID); `all` is for the list
/// and the tiles. A description that names a source with a single record and the scope `all` (or none) would get `404` from the module.
#[test]
fn single_record_has_the_id_of_a_profile_as_scope_and_never_all() {
    for (name, text, form, _) in FILES {
        let scope = scope_of(text);
        if form == "single-record" {
            let id = scope.strip_prefix("urn:uuid:").unwrap_or(&scope);
            assert!(
                id.len() == 36 && id.chars().all(|c| c.is_ascii_hexdigit() || c == '-'),
                "{name}: the scope of a single record is a profile id, not {scope:?}"
            );
            let records = records_of_text(text);
            let key = &records.forms()[0].rows[0].key;
            assert!(
                key.ends_with(id),
                "{name}: the scope names the shown person ({key})"
            );
        } else {
            assert!(
                scope.is_empty(),
                "{name}: the scope of {form} is `all`, not {scope:?}"
            );
        }
    }
}

#[test]
fn every_people_description_carries_the_mail_and_the_phone_of_the_first_person() {
    for (name, text, _, channel) in FILES {
        let records = records_of_text(text);
        let forms = records.forms();
        let row = forms[0]
            .rows
            .iter()
            .find(|r| r.get("label") == Some("Amara Zolenko"))
            .expect("the first person");
        let cells = row.all(channel);
        assert!(
            cells.iter().any(|c| c.contains("amara@example.org")),
            "{name}: no e-mail in {cells:?}"
        );
        assert!(
            cells.iter().any(|c| c.starts_with("+49")),
            "{name}: no phone in {cells:?}"
        );
    }
}
