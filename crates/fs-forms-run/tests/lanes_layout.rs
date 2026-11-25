//! R: the layout side of lanes: the scope word for word to the host, the feed of each block, the slot `lanes` of the real catalog.
mod lanes_support;

use std::path::Path;

use fs_forms_exchange::{Format, read_layout};
use fs_forms_layout::{Limits, OpenCatalog};
use fs_forms_run::{block_spec, feeds_of, module_forms, view_of, wants_of};
use lanes_support::{all_forms, records};

const FILE: &str = "schema = 4\nkind = \"frontend\"\nid = \"board\"\nextends = \"net.example.Tasks\"\nexec = \"fs-forms-run\"\ntitle = \"t\"\n\n\
[[source]]\nelem = \"source:1\"\nn = 1\ncontract = \"planning@>=0.0.1, <0.1.0\"\ntype = \"Task\"\nscope = \"selected-list\"\n\n\
[[source]]\nelem = \"source:2\"\nn = 2\ncontract = \"planning@>=0.0.1, <0.1.0\"\ntype = \"Lane\"\nscope = \"SCOPE\"\n\n\
[[page]]\nelem = \"page:board\"\npage = \"board\"\nframe = \"app-window\"\nnavigation = \"single\"\n\n\
[[page.place]]\nelem = \"block:board/main\"\nat = \"main\"\nblock = \"board\"\nsource = 1\n\n\
[[page.place.bind]]\nelem = \"binding:board/main/lanes/0\"\nslot = \"lanes\"\nnth = 0\ngrouping = \"assignee\"\nsource = 2\n\n\
[[message]]\nelem = \"message:t/de\"\nkey = \"t\"\nlang = \"de\"\ntext = \"T\"\n";

fn layout(scope: &str) -> Result<fs_forms_layout::Layout, String> {
    let text = FILE.replace("SCOPE", scope);
    read_layout(&mut text.as_bytes(), Path::new("t.toml"), Format::Toml)
        .map_err(|e| {
            format!(
                "{:?}",
                e.iter()
                    .map(|c| c.param("cause").map(str::to_owned))
                    .collect::<Vec<_>>()
            )
        })?
        .check(&OpenCatalog, &Limits::declared())
        .map_err(|e| format!("{e:?}"))
}

#[test]
fn failure_6_a_broken_scope_never_reaches_the_host() {
    let err = layout("a b").expect_err("red at reading");
    assert!(err.contains("scope-invalid"), "{err}");
}

#[test]
fn the_description_gives_the_host_one_want_per_source_with_the_scope_word_for_word() {
    let l = layout("lanes:assignee").expect("layout");
    let w = wants_of(&l);
    assert_eq!(
        w.iter()
            .map(|w| (w.n, w.data_type.as_str(), w.scope.as_str()))
            .collect::<Vec<_>>(),
        [(1, "Task", ""), (2, "Lane", "lanes:assignee")],
        "selected-list is a host word and means no scope"
    );
    let f = feeds_of(&l);
    assert_eq!(f.len(), 1);
    assert_eq!((f[0].block.as_str(), f[0].source), ("board", 1));
    let h = f[0].heads.as_ref().expect("heads");
    assert_eq!(
        (h.source, h.data_type.as_str(), h.grouping.as_str()),
        (2, "Lane", "assignee")
    );
    assert_eq!(view_of(&l, "en").wants, w);
}

#[test]
fn the_slot_lanes_of_the_real_catalog_names_its_head_form() {
    let spec = block_spec("board").expect("board");
    let lanes = spec
        .slots
        .iter()
        .find(|s| s.name == "lanes")
        .expect("lanes");
    assert_eq!(lanes.heads.len(), 1);
    assert_eq!(
        (lanes.heads[0].name.as_str(), lanes.heads[0].major),
        ("rows", 1)
    );
    assert!(
        spec.slots
            .iter()
            .filter(|s| s.name != "lanes")
            .all(|s| s.heads.is_empty())
    );
}

#[test]
fn the_module_forms_keep_both_entries_of_the_same_form_with_their_types() {
    let m = module_forms(&records(all_forms()));
    let rows: Vec<_> = m
        .iter()
        .filter(|f| f.form.name == "rows")
        .map(|f| f.data_type.as_str())
        .collect();
    assert_eq!(rows, ["Task", "Lane"]);
}
