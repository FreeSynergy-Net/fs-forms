//! The four people descriptions (P-F1 table, P-F2 single, P-F3 cards, P-F4 contacts) read with the exchange reader and checked against the REAL block
//! catalog of this build (places, takes, counts): exit 0 of the check, no problem list.
//!
//! Pattern: none (tests).

use std::path::Path;

use fs_forms_exchange::{Format, read_layout};
use fs_forms_layout::Limits;
use fs_forms_run::Blocks;

fn dir() -> String {
    format!("{}/../../forms/people", env!("CARGO_MANIFEST_DIR"))
}

/// The file names of the descriptions (sorted).
fn files() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir())
        .expect("forms/people")
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| Path::new(n).extension().is_some_and(|x| x == "toml"))
        .collect();
    names.sort();
    names
}

#[test]
fn there_are_four_people_descriptions() {
    assert_eq!(files().len(), 4, "{:?}", files());
}

#[test]
fn every_description_reads_and_passes_the_layout_check_of_the_real_catalog() {
    for name in files() {
        let path = format!("{}/{name}", dir());
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
        let doc = read_layout(&mut text.as_bytes(), Path::new(&name), Format::Toml)
            .unwrap_or_else(|e| panic!("{name}: the exchange reader says {e:?}"));
        if let Err(problems) = doc.check(&Blocks::rules(), &Limits::declared()) {
            panic!("{name}: the layout check says {problems:?}");
        }
    }
}
