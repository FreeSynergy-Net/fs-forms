//! The form that shows ONE record (`single-record`) is served only with the id of that record as the scope: no scope, `all` and a scope that is
//! no UUID are a refusal `404`, a valid id nobody has is an EMPTY page, the id (`urn:uuid:<uuid>` or the bare UUID) gives exactly that record; the list and the tiles take
//! every record. The same rule as `record_page` of `fs-person-service` (fs-modules-person @6766d91b).
//!
//! Pattern: Characterization test of the Fake Object `SampleModule` (the rule of the real modules, stated once).
#![allow(dead_code)] // module_support serves several test files

mod module_support;

use fs_forms_testdata::{Anchor, Period, SampleBuilder, SampleModule, SampleSpec, SetWish};
use fs_interfaces::module::{ModuleCore, PageQuery};
use fs_types::Zone;
use module_support::{cx, module, run};

const AMARA: &str = "urn:uuid:00000000-0000-7000-8000-000000000001";

fn people() -> SampleModule {
    let sets = ["rows@^1.0.0", "single-record@^1.2.0", "tiles@^1.1.0"]
        .map(|f| SetWish::new(f, "Profile", 0).fixed("person-directory.sample"))
        .to_vec();
    let spec = SampleSpec {
        seed: 1,
        anchor: Anchor {
            period: Period::Month(2026, 11),
            zone: Zone::iana("Europe/Berlin").expect("zone"),
        },
        sets,
        forbid: Vec::new(),
    };
    module(&SampleBuilder::new(spec).build().expect("builds"))
}

fn ask(m: &SampleModule, form: &str, scope: &str) -> Result<usize, u16> {
    let mut q = PageQuery::default();
    form.clone_into(&mut q.form);
    scope.clone_into(&mut q.scope);
    run(m.page(&cx("p"), q))
        .map(|p| p.records.len())
        .map_err(|r| r.status)
}

#[test]
fn one_record_needs_the_id_of_the_record() {
    let m = people();
    assert_eq!(ask(&m, "single-record@1.2.0", AMARA), Ok(1));
    assert_eq!(
        ask(
            &m,
            "single-record@1.2.0",
            AMARA.trim_start_matches("urn:uuid:")
        ),
        Ok(1),
        "the bare UUID names the same record"
    );
    for scope in ["", "all", "no-uuid", "urn:uuid:not-a-uuid"] {
        assert_eq!(ask(&m, "single-record@1.2.0", scope), Err(404), "{scope:?}");
    }
    // a valid id nobody has: an empty page, as the module answers (not a refusal)
    assert_eq!(
        ask(
            &m,
            "single-record@1.2.0",
            "urn:uuid:00000000-0000-7000-8000-0000000000ff"
        ),
        Ok(0)
    );
}

#[test]
fn the_list_and_the_tiles_take_every_record() {
    let m = people();
    for form in ["rows@1.0.0", "tiles@1.1.0"] {
        assert_eq!(ask(&m, form, ""), Ok(4), "{form}");
    }
}
