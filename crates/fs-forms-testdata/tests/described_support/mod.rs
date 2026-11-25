//! Shared by the tests of descriptions: the table of blocks (a stand-in for the block catalog) and small readers.

#![allow(dead_code)]

use std::fmt::Write as _;
use std::path::Path;

use fs_forms_bind::{BlockSpec, FormRef, SlotSpec};
use fs_forms_exchange::{Format, read_layout};
use fs_forms_layout::{Layout, Limits, OpenCatalog};
use fs_forms_testdata::{BlockForms, SampleCode, SampleSpec, SourceSpec};

pub fn form(name: &str, major: u32, minor: u32) -> FormRef {
    FormRef {
        name: name.to_owned(),
        major,
        minor,
    }
}

/// The blocks of the catalog the descriptions below use, with `forms=` and the heads of a grouping slot (read from the block
/// contract of fs-components 81367489: widgets.toml, general.toml).
pub struct Table;

impl BlockForms for Table {
    fn forms(&self, block: &str) -> Vec<FormRef> {
        match block {
            "board" => vec![form("cards-in-columns", 1, 0)],
            "tree" => vec![form("tree", 1, 0)],
            "table" => vec![form("rows", 1, 0)],
            "calendar-grid" => vec![form("bars-in-time", 1, 0)],
            _ => Vec::new(),
        }
    }

    fn spec(&self, block: &str) -> Option<BlockSpec> {
        (block == "board").then(|| BlockSpec {
            name: "board".to_owned(),
            slots: vec![SlotSpec {
                name: "lanes".to_owned(),
                min: 0,
                max: Some(1),
                heads: vec![form("rows", 1, 0)],
            }],
        })
    }
}

pub fn read(text: &str) -> Layout {
    read_layout(&mut text.as_bytes(), Path::new("t.toml"), Format::Toml)
        .expect("reads")
        .check(&OpenCatalog, &Limits::declared())
        .expect("checks")
}

pub fn file(path: &str) -> (Layout, String) {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    (read(&text), text)
}

pub const FORMS: &str = "/srv/FreeSynergy/.scratch/c2/wave2/fs-tasks/forms";

pub fn here(name: &str) -> String {
    format!(
        "{}/../fs-forms-run/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// A one-source description with the given blocks (`block = ...` lines of one page).
pub fn describe(sources: &[&str], places: &[(&str, u32)]) -> String {
    let mut t = String::from(
        "schema = 4\nkind = \"frontend\"\nid = \"t\"\nextends = \"net.example.T\"\nexec = \"fs-forms-run\"\ntitle = \"t\"\n",
    );
    for (i, ty) in sources.iter().enumerate() {
        let n = i + 1;
        let _ = write!(
            t,
            "\n[[source]]\nelem = \"source:{n}\"\nn = {n}\ncontract = \"x@>=0.1, <0.2\"\ntype = \"{ty}\"\nscope = \"all\"\n"
        );
    }
    t += "\n[[page]]\nelem = \"page:p\"\npage = \"p\"\nframe = \"app-window\"\nnavigation = \"single\"\n";
    for (i, (block, source)) in places.iter().enumerate() {
        let _ = write!(
            t,
            "\n[[page.place]]\nelem = \"block:p/{i}\"\nat = \"main{i}\"\nblock = \"{block}\"\nsource = {source}\n"
        );
    }
    t += "\n[[message]]\nelem = \"message:t/de\"\nkey = \"t\"\nlang = \"de\"\ntext = \"T\"\n";
    t
}

pub fn specs(layout: &Layout, sample: Option<&str>) -> Vec<SourceSpec> {
    SampleSpec::from_description(layout, &Table, sample).expect("from description")
}

pub fn code(layout: &Layout, text: &str) -> SampleCode {
    SampleSpec::from_description(layout, &Table, Some(text))
        .expect_err("must fail")
        .code()
}
