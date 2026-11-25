//! `SampleModule` through the conformance suite of `module_v1` (fs-module, the same rev as the pinned fs-interfaces): the fixture of the
//! test data is itself checked by the rules every module must keep. Level 2: change, take back, watch; two connections.

mod module_support;

use fs_forms_testdata::{Forbid, SampleSet};
use fs_interfaces::formsource_capnp::form_source;
use fs_module::conformance::run_conformance;
use fs_module::{
    CommandKind, DeclaredCommand, DeclaredForm, FormSourceFixture, Param, Sample, Verify,
};
use module_support::{BARS, bars_with_writes, calendar, connect, connect_core, module, run};

/// What the suite is told about `SampleModule` on the calendar set: the names come from the set and the declaration, one place.
struct Fx {
    one: form_source::Client,
    two: form_source::Client,
    keys: Vec<String>,
    forbid: String,
}

impl FormSourceFixture for Fx {
    fn client(&self) -> form_source::Client {
        self.one.clone()
    }
    fn level(&self) -> u32 {
        2
    }
    fn second(&self) -> Option<form_source::Client> {
        Some(self.two.clone())
    }
    fn provider(&self) -> Option<&str> {
        Some(fs_forms_testdata::SAMPLE_PROVIDER)
    }
    fn forms(&self) -> Vec<DeclaredForm> {
        let fields = [
            "start", "end", "label", "lane", "state", "depends", "progress", "all-day",
        ];
        vec![DeclaredForm {
            key: BARS.into(),
            scope: String::new(),
            fields: fields.map(str::to_owned).to_vec(),
        }]
    }
    fn commands(&self) -> Vec<DeclaredCommand> {
        let param = |name: &str, required| Param {
            name: name.into(),
            required,
        };
        let when = vec![param("start", true), param("end", true)];
        vec![
            DeclaredCommand {
                key: "move".into(),
                kind: CommandKind::Record,
                params: [when.clone(), vec![param("lane", false)]].concat(),
            },
            DeclaredCommand {
                key: "create".into(),
                kind: CommandKind::Set,
                params: [when, vec![param("label", true), param("lane", false)]].concat(),
            },
        ]
    }
    fn samples(&self) -> Vec<Sample> {
        let arg = |k: &str, v: &str| (k.to_owned(), v.to_owned());
        let made = Sample {
            command: "create".into(),
            record: String::new(),
            args: vec![
                arg("start", "2026-05-30T09:00:00+02:00"),
                arg("end", "2026-05-30T09:30:00+02:00"),
                arg("label", "Made by the suite"),
            ],
            makes_record: true,
            verify: Verify::None,
        };
        let moved = Sample {
            command: "move".into(),
            record: self.keys[0].clone(),
            args: vec![
                arg("start", "2026-05-28T09:00:00+02:00"),
                arg("end", "2026-05-28T10:30:00+02:00"),
                arg("lane", "moved"),
            ],
            makes_record: true,
            verify: Verify::Value {
                form: BARS.into(),
                record: None,
                path: "start".into(),
                text: "2026-05-28T09:00:00+02:00".into(),
            },
        };
        vec![moved, made]
    }
    fn forbidden(&self) -> Option<(String, String)> {
        Some(("move".into(), self.forbid.clone()))
    }
    fn time_fields(&self) -> Vec<(String, String)> {
        ["start", "end"]
            .map(|p| (BARS.to_owned(), p.to_owned()))
            .to_vec()
    }
}

fn keys(set: &SampleSet) -> Vec<String> {
    set.rows(BARS).unwrap().into_iter().map(|r| r.key).collect()
}

#[test]
fn the_calendar_set_passes_the_rules_of_module_v1() {
    let keys = keys(&calendar(false, 0, Vec::new()));
    let forbid = keys[1].clone();
    let set = calendar(false, 0, vec![Forbid::on("move", &forbid)]);
    let module = module(&set).declare(&bars_with_writes());
    let (one, two) = connect(module);
    let fx = Fx {
        one,
        two,
        keys,
        forbid,
    };
    let report = run(run_conformance(&fx));
    eprintln!("checked {}, skipped: {:#?}", report.checked, report.skipped);
    let (wrapper, ours) = split(report.failures);
    assert!(ours.is_empty(), "rules the sample module fails: {ours:#?}");
    assert_eq!(
        kinds(&wrapper),
        WRAPPER_KINDS.into_iter().collect(),
        "the module shows the same kinds of wrapper findings as the control"
    );
    eprintln!(
        "wrapper findings (fs-rpc, same for every module): {}",
        wrapper.len()
    );
    assert!(
        report.checked > 40,
        "the suite really ran ({} checks)",
        report.checked
    );
    // Honest: nothing was passed that was not run. These rules need a fixture the sample module cannot give, and say so.
    for s in &report.skipped {
        eprintln!("skipped: {s}");
    }
}

/// Failures that belong to the wrapper `fs_rpc::Guarded` (0f639470) and not to the module behind it: (1) its problem has the kind as title,
/// rule 14 asks for text for people; (2) a cursor that is no UTF-8 text is `invalid-request` 422 there, rule 15 asks for `bad-arguments`.
/// The control test below shows that the fake of fs-module has exactly these failures behind the same wrapper. Finding for C3 (fs-rpc), not
/// worked around here.
fn wrapper_kind(f: &str) -> Option<&'static str> {
    if f.starts_with("14 refusal") && f.contains("has a title that is text for people") {
        Some("14 title is the kind")
    } else if f.starts_with("15 limits") && f.contains("garbage cursor") {
        Some("15 garbage cursor is invalid-request")
    } else if f.starts_with("14 an exception names") && f.contains("cursor is not valid") {
        Some("14 exception uri of that 422")
    } else {
        None
    }
}

/// The three kinds of the wrapper findings: the module and the control must show exactly these, no more, no fewer.
const WRAPPER_KINDS: [&str; 3] = [
    "14 exception uri of that 422",
    "14 title is the kind",
    "15 garbage cursor is invalid-request",
];

fn split(failures: Vec<String>) -> (Vec<String>, Vec<String>) {
    failures
        .into_iter()
        .partition(|f| wrapper_kind(f).is_some())
}

fn kinds(wrapper: &[String]) -> std::collections::BTreeSet<&'static str> {
    wrapper.iter().filter_map(|f| wrapper_kind(f)).collect()
}

#[test]
fn control_the_fake_of_fs_module_has_the_same_wrapper_findings() {
    use fs_module::{FakeCore, FakeFixture, Mode};
    let (one, two) = connect_core(FakeCore::new(Mode::Strict));
    let fx = FakeFixture::new(one).with_second(two);
    let report = run(run_conformance(&fx));
    let (wrapper, ours) = split(report.failures);
    assert!(
        ours.is_empty(),
        "the control fails more than the wrapper findings: {ours:#?}"
    );
    eprintln!("control wrapper findings: {}", wrapper.len());
    assert_eq!(
        kinds(&wrapper),
        WRAPPER_KINDS.into_iter().collect(),
        "the wrapper findings changed (fs-rpc fixed?): drop `split` and expect no failure ({wrapper:#?})"
    );
}
