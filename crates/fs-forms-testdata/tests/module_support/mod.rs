//! Helpers of the `SampleModule` tests: the sets, the contexts, the runtime and the wrapper (`Guarded`, in process, no socket).
//!
//! Pattern: Test double helpers.

#![allow(dead_code)]

use std::future::Future;

use capnp::capability::FromClientHook;
use fs_forms_testdata::{
    Anchor, Case, Forbid, IntervalCase, Period, Relation, SampleBuilder, SampleModule, SampleSet,
    SampleSpec, SetWish,
};
use fs_interfaces::forms::{self, Form};
use fs_interfaces::formsource_capnp::form_source;
use fs_interfaces::module::{
    Actor, Caller, Claimed, Context, Invocation, ModuleCore, Page, PageQuery, Refusal, Value,
};
use fs_interfaces::owner::fake::person;
use fs_rpc::{Guarded, PerConnection, Ports};
use fs_types::Zone;

pub const BARS: &str = "bars-in-time@1.1.0";
pub const PERSON: &str = "urn:uuid:018f3a52-7c1e-7b3a-9d4e-5a6b7c8d9e01";

/// The real declaration `bars-in-time@1.1.0` of the pinned catalog: `move` writes `start! end! lane`, `create` writes `start! end! label! lane`.
pub fn bars_with_writes() -> Form {
    let version = forms::parse_version("1.1.0").expect("version");
    forms::find("bars-in-time", &version)
        .expect("bars-in-time@1.1.0 is in the pinned fs-interfaces")
        .clone()
}

pub fn anchor() -> Anchor {
    Anchor {
        period: Period::Month(2026, 5),
        zone: Zone::iana("Europe/Berlin").unwrap(),
    }
}

/// The calendar set of the first use (package 2): 8 events, two lanes; `all_day` adds the two whole-day events.
pub fn calendar(all_day: bool, invalid: u32, forbid: Vec<Forbid>) -> SampleSet {
    let mut wish = SetWish::new("bars-in-time@^1.0.0", "Event", 8)
        .distinct("lane", 2)
        .case(Case::Interval(IntervalCase::MultiDay(3), 1))
        .case(Case::Interval(IntervalCase::SameDay(4), 1))
        .case(Case::Interval(IntervalCase::CrossPeriod, 1))
        .invalid(invalid);
    if all_day {
        wish = wish.case(Case::Interval(IntervalCase::AllDay, 2));
    }
    SampleBuilder::new(SampleSpec {
        seed: 1,
        anchor: anchor(),
        sets: vec![wish],
        forbid,
    })
    .relation(Relation::order("start", "end", Some("all-day")))
    .build()
    .expect("builds")
}

pub fn board(forbid: Vec<Forbid>) -> SampleSet {
    SampleBuilder::new(SampleSpec {
        seed: 4,
        anchor: anchor(),
        sets: vec![
            SetWish::new("cards-in-columns@=1.1.0", "Task", 6)
                .distinct("column", 3)
                .distinct("lane", 2),
        ],
        forbid,
    })
    .build()
    .expect("builds")
}

pub fn module(set: &SampleSet) -> SampleModule {
    SampleModule::new(set).expect("module")
}

pub fn cx(program: &str) -> Context {
    let caller = Caller::Connection {
        system_id: "1000".into(),
        system_name: None,
    };
    Context::from_connection(caller, Actor::Person, person(PERSON))
        .with_claimed(Claimed::new(program, "i1", "d1"))
}

pub fn run<F: Future>(f: F) -> F::Output {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    tokio::task::LocalSet::new().block_on(&rt, f)
}

/// Two connections to ONE module behind the wrapper: they stand for two programs.
pub fn connect(module: SampleModule) -> (form_source::Client, form_source::Client) {
    connect_core(module)
}

/// The same for any `ModuleCore` (the fake of fs-module as control).
pub fn connect_core<M: fs_interfaces::module::ModuleCore>(
    module: M,
) -> (form_source::Client, form_source::Client) {
    let ports = Ports::new(
        |_: &fs_rpc::Caller| Actor::Person,
        |_: &fs_rpc::Caller| Some(person(PERSON)),
    );
    let mut g = Guarded::new(module, ports);
    let mut one = |who: &str| -> form_source::Client {
        FromClientHook::new(g.bootstrap(&fs_rpc::Caller::new(who, None)).hook)
    };
    (one("a"), one("b"))
}

pub fn call(command: &str, record: Option<&str>, args: &[(&str, &str)]) -> Invocation {
    let mut i = Invocation::default();
    command.clone_into(&mut i.command);
    i.record = record.map(str::to_owned);
    i.args = args.iter().map(|(p, t)| Value::new(p, t)).collect();
    i
}

pub fn page(m: &SampleModule, form: &str) -> Page {
    let mut q = PageQuery::default();
    form.clone_into(&mut q.form);
    run(m.page(&cx("p"), q)).expect("page")
}

pub fn text<'a>(page: &'a Page, record: &str, path: &str) -> Option<&'a str> {
    let r = page.records.iter().find(|r| r.id == record)?;
    r.values
        .iter()
        .find(|v| v.path == path)
        .map(|v| v.text.as_str())
}

pub fn refused<T>(r: Result<T, Refusal>) -> Refusal {
    match r {
        Err(e) => e,
        Ok(_) => panic!("expected a refusal"),
    }
}

/// The fixed events of the sample of the contract `events` for a day of November 2026 without whole-day events (the same form, the same
/// declaration as `calendar`). Why a day without whole-day events: the rule "time field has a value" of the suite wants an RFC 3339
/// `date-time` for every `start`/`end`, while `bars-in-time@1.1.0` says `full-date` when `all-day` is set (forms.txt); card for C3.
pub fn fixed_events(forbid: Vec<Forbid>) -> SampleSet {
    SampleBuilder::new(SampleSpec {
        seed: 1,
        anchor: Anchor {
            period: Period::Day(2026, 11, 17),
            zone: Zone::iana("Europe/Berlin").unwrap(),
        },
        sets: vec![SetWish::new("bars-in-time@^1.1.0", "Event", 0).fixed("events.sample")],
        forbid,
    })
    .relation(Relation::order("start", "end", Some("all-day")))
    .build()
    .expect("builds")
}
