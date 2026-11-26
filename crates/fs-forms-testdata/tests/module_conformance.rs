//! `SampleModule` through the conformance suite of `module_v1` (fs-module, the same rev as the pinned fs-interfaces): the fixture of the
//! test data is itself checked by the rules every module must keep. Level 2: change, take back, watch; two connections.

mod module_support;

use fs_forms_testdata::{Forbid, SampleSet};
use fs_interfaces::forms::Art;
use fs_interfaces::formsource_capnp::form_source;
use fs_module::conformance::run_conformance;
use fs_module::{
    CommandKind, DeclaredCommand, DeclaredForm, FormSourceFixture, Param, Sample, Verify,
};
use module_support::{
    BARS, LANE, TITLE, bars_declared, bars_with_writes, calendar, connect, connect_core,
    fixed_events, module, run,
};

/// What the suite is told about `SampleModule` on the calendar set: the names come from the set and the declaration, one place.
struct Fx {
    one: form_source::Client,
    two: form_source::Client,
    keys: Vec<String>,
    forbid: String,
    /// The fields the set declares for the data channels of the form (`ModuleForm::map`): what the module answers by.
    fields: Vec<String>,
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
        vec![DeclaredForm {
            key: BARS.into(),
            scope: String::new(),
            fields: self.fields.clone(),
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
                params: [when.clone(), vec![param(LANE, false)]].concat(),
            },
            DeclaredCommand {
                key: "create".into(),
                kind: CommandKind::Set,
                params: [when, vec![param(TITLE, true), param(LANE, false)]].concat(),
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
                arg(TITLE, "Made by the suite"),
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
                arg(LANE, "moved"),
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

/// The fields of the data channels of `bars-in-time` the set declares (the map channel -> field of the set, one place).
fn fields_of(set: &SampleSet) -> Vec<String> {
    let form = bars_with_writes();
    let data = |c: &str| {
        form.channel(c)
            .is_some_and(|c| c.arts.first() != Some(&Art::Records))
    };
    bars_declared(set)
        .flat_map(|d| d.map.iter())
        .filter(|(c, _)| data(c))
        .flat_map(|(_, f)| f.clone())
        .collect()
}

/// One suite, any set: a fixed set is a module like a generated one (no `if fixed` anywhere).
fn passes_the_rules(make: impl Fn(Vec<Forbid>) -> SampleSet) {
    let keys = keys(&make(Vec::new()));
    let forbid = keys[1].clone();
    let set = make(vec![Forbid::on("move", &forbid)]);
    let fields = fields_of(&set);
    let module = module(&set).declare(&bars_with_writes());
    let (one, two) = connect(module);
    let fx = Fx {
        one,
        two,
        keys,
        forbid,
        fields,
    };
    let report = run(run_conformance(&fx));
    eprintln!("checked {}, skipped: {:#?}", report.checked, report.skipped);
    assert!(
        report.failures.is_empty(),
        "rules the sample module fails: {:#?}",
        report.failures
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

#[test]
fn the_calendar_set_passes_the_rules_of_module_v1() {
    passes_the_rules(|forbid| calendar(false, 0, forbid));
}

#[test]
fn the_fixed_events_pass_the_rules_of_module_v1() {
    passes_the_rules(fixed_events);
}

#[test]
fn control_the_fake_of_fs_module_passes_the_rules_too() {
    use fs_module::{FakeCore, FakeFixture, Mode};
    let (one, two) = connect_core(FakeCore::new(Mode::Strict));
    let fx = FakeFixture::new(one).with_second(two);
    let report = run(run_conformance(&fx));
    assert!(
        report.failures.is_empty(),
        "the control fails: {:#?}",
        report.failures
    );
}
