//! The three people descriptions (list, single person, cards) ask for the fixed people: read with the exchange reader, built like the host does.
//! (The layout check against the real block catalog is `fs-forms-run/tests/people_descriptions.rs`.)

mod described_support;

use described_support::{read, specs};

/// (file, the form it draws, the data of the sample it must show)
const FILES: [(&str, &str); 3] = [
    ("P-F1-liste.toml", "rows@1.0.0"),
    ("P-F2-einzel.toml", "single-record@1.2.0"),
    ("P-F3-karten.toml", "tiles@1.1.0"),
];

fn text_of(name: &str) -> String {
    let path = format!("{}/../../forms/people/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

#[test]
fn every_description_asks_for_the_fixed_people_and_shows_them() {
    for (file, form) in FILES {
        let text = text_of(file);
        let layout = read(&text);
        let got = specs(&layout, Some(&text));
        assert_eq!(got.len(), 1, "{file}: one source");
        let wish = &got[0].spec.sets[0];
        assert_eq!(
            wish.fixed.as_deref(),
            Some("person-directory.sample"),
            "{file}"
        );
        assert_eq!(wish.data_type, "Profile", "{file}");
        let set = got[0].build().unwrap_or_else(|e| panic!("{file}: {e:?}"));
        let rows = set.rows(form).unwrap_or_else(|e| panic!("{file}: {e:?}"));
        assert!(
            rows.len() >= 3,
            "{file}: the people are shown ({})",
            rows.len()
        );
    }
}

#[test]
fn the_three_frontends_show_different_forms_of_the_same_module() {
    let forms: Vec<String> = FILES
        .iter()
        .map(|(file, _)| {
            let text = text_of(file);
            let got = specs(&read(&text), Some(&text));
            got[0].spec.sets[0].form.clone()
        })
        .collect();
    let mut names: Vec<&str> = forms.iter().filter_map(|f| f.split('@').next()).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 3, "three different forms: {forms:?}");
}

#[test]
fn no_description_writes_a_command_no_service_serves() {
    for (file, _) in FILES {
        let text = text_of(file);
        assert!(!text.contains("command = "), "{file}: an invented command");
        assert!(
            !text.contains("person.create") && !text.contains("me.create"),
            "{file}"
        );
    }
}
