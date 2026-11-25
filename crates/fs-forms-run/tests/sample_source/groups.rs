//! The test data of the groups module (a tree of two depths and a member list), made from the wish
//! `tests/fixtures/sample/groups-parity.toml` and read through `SampleModule::page` the way the host reads a module. It replaces
//! `sample_groups()` of the host (the old hand-built one) in the tests that need a groups module: no test names a node or a member.
//!
//! Pattern: Adapter (`Page` -> `FormData`, the function `data_of` of the parent) over a Fake Object (`SampleModule`).

use fs_forms_run::{FormData, FormRow};
use fs_forms_testdata::{Relation, SourceSpec};
use fs_interfaces::module::{ModuleCore, PageQuery};

use super::{context, data_of};

const WISH: &str = include_str!("../fixtures/sample/groups-parity.toml");

/// The relation that makes a tree of the nodes (the catalog does not carry it yet at the pinned `fs-interfaces`; once it does the
/// spec has it and this is not added a second time).
fn parent() -> Relation {
    Relation::Parent {
        child: "parent".to_owned(),
    }
}

/// The forms of the sample groups module: the tree and the member list, in the order of the wish.
#[must_use]
pub fn groups() -> Vec<FormData> {
    let mut source = SourceSpec::from_wish_text(WISH).expect("the wish reads");
    if !source.relations.contains(&parent()) {
        source.relations.push(parent());
    }
    let set = source.build().expect("the wish builds");
    let module = fs_forms_testdata::SampleModule::new(&set).expect("the module");
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("runtime");
    let cx = context();
    set.slices
        .iter()
        .zip(&set.declared)
        .map(|(slice, declared)| {
            let mut q = PageQuery::default();
            q.form.clone_from(&slice.form);
            q.limit = 500;
            let page = rt.block_on(module.page(&cx, q)).expect("page");
            data_of(&page, declared)
        })
        .collect()
}

/// The rows of the form `name` in `forms`.
#[must_use]
pub fn rows_of<'a>(forms: &'a [FormData], name: &str) -> &'a [FormRow] {
    forms
        .iter()
        .find(|f| f.declared.form.name == name)
        .map_or(&[], |f| f.rows.as_slice())
}
