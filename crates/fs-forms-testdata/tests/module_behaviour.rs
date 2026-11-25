//! `SampleModule` as a `ModuleCore`, called directly (no wrapper): what the commands do, from the declaration only.
#![allow(clippy::many_single_char_names)] // a, b, m, n, p: two calls, one module, a count, a page

mod module_support;

use fs_forms_testdata::{Forbid, SAMPLE_PROVIDER, SampleModule};
use fs_interfaces::module::ProviderKind;
use fs_interfaces::module::{Ask, ChangeKind, Invocation, ModuleCore, Refusal, StepRef};
use fs_types::Utc;
use module_support::{
    BARS, bars_with_writes, board, calendar, call, cx, module, page, refused, run, text,
};

fn calendar_module(forbid: Vec<Forbid>) -> SampleModule {
    module(&calendar(true, 1, forbid)).declare(&bars_with_writes())
}

const MOVE: [(&str, &str); 3] = [
    ("start", "2026-05-12T10:00:00+02:00"),
    ("end", "2026-05-12T11:00:00+02:00"),
    ("lane", "moved"),
];

#[test]
fn the_page_shows_times_with_the_offset_of_the_zone_and_whole_days_as_dates() {
    let p = page(&calendar_module(Vec::new()), BARS);
    assert_eq!(
        p.records.len(),
        9,
        "8 events (2 of them whole days) + 1 broken row"
    );
    assert_eq!(p.total, Some(9));
    let starts: Vec<&str> = p
        .records
        .iter()
        .flat_map(|r| {
            r.values
                .iter()
                .filter(|v| v.path == "start")
                .map(|v| v.text.as_str())
        })
        .collect();
    assert!(
        starts.iter().any(|s| s.len() == 10 && !s.contains('T')),
        "a whole day is a full-date: {starts:?}"
    );
    assert!(
        starts.iter().any(|s| s.ends_with("+02:00")),
        "May in Berlin is +02:00: {starts:?}"
    );
    assert!(
        p.records
            .iter()
            .all(|r| r.may == ["move", "create"].map(String::from)[..1]),
        "move is offered on a record"
    );
    assert_eq!(p.may, ["create"], "create is offered on the set");
}

#[test]
fn move_writes_start_end_and_lane_from_the_declaration_and_revert_takes_it_back() {
    let m = calendar_module(Vec::new());
    let before = page(&m, BARS);
    let key = before.records[0].id.clone();
    let done = run(m.apply(&cx("p"), call("move", Some(&key), &MOVE))).expect("move");
    let after = page(&m, BARS);
    assert_eq!(text(&after, &key, "start"), Some(MOVE[0].1));
    assert_eq!(text(&after, &key, "end"), Some(MOVE[1].1));
    assert_eq!(text(&after, &key, "lane"), Some("moved"));
    let change = done.change.expect("a change was recorded");
    assert_eq!(change.kind, ChangeKind::Update);
    assert_eq!(
        (change.before.as_str(), change.revision.as_str()),
        ("1", "2")
    );
    let undo = run(m.revert(&cx("p"), &change.activity, StepRef::default())).expect("revert");
    assert_eq!(undo.change.as_ref().map(|c| c.kind), Some(ChangeKind::Undo));
    let back = page(&m, BARS);
    assert_eq!(
        back.records, before.records,
        "the page is as before the move"
    );
    let redo = undo.change.expect("undo change");
    run(m.revert(&cx("p"), &redo.activity, StepRef::default())).expect("redo");
    assert_eq!(
        page(&m, BARS).records,
        after.records,
        "taking back the undo makes the move again"
    );
}

#[test]
fn a_create_makes_a_record_and_taking_it_back_removes_it() {
    let m = calendar_module(Vec::new());
    let n = page(&m, BARS).records.len();
    let args = [
        ("start", "2026-05-30T09:00:00+02:00"),
        ("end", "2026-05-30T09:30:00+02:00"),
        ("label", "Made"),
    ];
    let a = run(m.apply(&cx("p"), call("create", None, &args))).expect("create");
    let b = run(m.apply(&cx("p"), call("create", None, &args))).expect("create");
    assert_ne!(
        a.record, b.record,
        "two creates, two keys (no key of a seed is reused)"
    );
    assert_eq!(page(&m, BARS).records.len(), n + 2);
    run(m.revert(&cx("p"), &b.change.unwrap().activity, StepRef::default())).expect("revert");
    let p = page(&m, BARS);
    assert_eq!(p.records.len(), n + 1);
    assert!(p.records.iter().all(|r| r.id != b.record));
}

#[test]
fn a_command_runs_from_writes_whatever_its_name_is() {
    // Cards of a board: `move` writes column! order lane, `create` writes title! column! lane (the pinned catalog), no code names them.
    let m = module(&board(Vec::new()));
    let key = "cards-in-columns@1.1.0";
    let p = page(&m, key);
    assert_eq!(p.records.len(), 6);
    let card = p.records[0].id.clone();
    let args = [("column", "done"), ("order", "3"), ("lane", "l9")];
    run(m.apply(&cx("p"), call("move", Some(&card), &args))).expect("move");
    let p = page(&m, key);
    assert_eq!(text(&p, &card, "column"), Some("done"));
    assert_eq!(text(&p, &card, "order"), Some("3"));
    let made = run(m.apply(
        &cx("p"),
        call("create", None, &[("title", "T"), ("column", "todo")]),
    ))
    .expect("create");
    assert!(page(&m, key).records.iter().any(|r| r.id == made.record));
}

#[test]
fn what_the_declaration_does_not_say_is_refused_with_a_code_never_guessed() {
    // A declaration of the form without `writes` on `move` (the real 1.1.0 has them): it cannot be run from the declaration.
    let text = "bars-in-time@1.1.0 | Bars in time | planned | without writes\n\
        > items | records | ! | 1\n> start | time | ! | 1\n> end | time | ! | 1\n> label | text | ! | 1\n> lane | text | - | 0..1\n\
        > move | command | - | 0..1\n";
    let plain = module(&calendar(false, 0, Vec::new()))
        .declare(&fs_interfaces::forms::parse(text).expect("parses").remove(0));
    let key = page(&plain, BARS).records[0].id.clone();
    let r = refused(run(plain.apply(&cx("p"), call("move", Some(&key), &MOVE))));
    assert_eq!(
        (r.kind.as_str(), r.status, r.detail.as_str()),
        ("unsupported", 501, "sample-error-writes-unknown")
    );
    // `remove` (empty writes) looks like `open`: no effect to run.
    let m = module(&board(Vec::new()));
    let card = page(&m, "cards-in-columns@1.1.0").records[0].id.clone();
    let r = refused(run(m.apply(&cx("p"), call("remove", Some(&card), &[]))));
    assert_eq!(
        (r.kind.as_str(), r.detail.as_str()),
        ("unsupported", "sample-error-effect-unknown")
    );
}

#[test]
fn bad_calls_are_refusals_with_the_kinds_of_the_contract() {
    let m = calendar_module(Vec::new());
    let key = page(&m, BARS).records[0].id.clone();
    let kind = |r: Refusal| (r.kind, r.status);
    let go = |c: Invocation| kind(refused(run(m.apply(&cx("p"), c))));
    assert_eq!(
        go(call("nothing", None, &[])),
        ("unknown-command".into(), 404)
    );
    assert_eq!(
        go(call("move", Some("urn:none"), &MOVE)),
        ("unknown-record".into(), 404)
    );
    assert_eq!(
        go(call("move", Some(&key), &[("zz", "x")])),
        ("bad-arguments".into(), 400)
    );
    assert_eq!(
        go(call("move", Some(&key), &[MOVE[0], MOVE[0], MOVE[1]])),
        ("bad-arguments".into(), 400)
    );
    assert_eq!(
        go(call("move", Some(&key), &[MOVE[0]])),
        ("bad-arguments".into(), 400),
        "end is required"
    );
    assert_eq!(
        go(call("move", Some(&key), &[("start", "tomorrow"), MOVE[1]])),
        ("bad-arguments".into(), 400)
    );
    assert_eq!(
        go(call("move", None, &MOVE)),
        ("bad-arguments".into(), 400),
        "move needs a record"
    );
    assert_eq!(
        go(call("create", Some(&key), &MOVE)),
        ("bad-arguments".into(), 400),
        "create names no record"
    );
    assert_eq!(
        page(&m, BARS).records.len(),
        9,
        "nothing was changed by the refusals"
    );
}

#[test]
fn a_stale_expect_is_a_conflict_and_only_the_latest_change_of_a_record_is_taken_back() {
    let m = calendar_module(Vec::new());
    let key = page(&m, BARS).records[0].id.clone();
    let first = run(m.apply(&cx("p"), call("move", Some(&key), &MOVE)))
        .unwrap()
        .change
        .unwrap();
    let mut stale = call("move", Some(&key), &MOVE);
    stale.expect = Some("1".into());
    let r = refused(run(m.apply(&cx("p"), stale)));
    assert_eq!((r.kind.as_str(), r.status), ("conflict", 409));
    assert_eq!(r.conflict.expect("typed").by.activity, first.activity);
    let second = [
        ("start", "2026-05-13T10:00:00+02:00"),
        ("end", "2026-05-13T11:00:00+02:00"),
    ];
    run(m.apply(&cx("p"), call("move", Some(&key), &second))).unwrap();
    let r = refused(run(m.revert(&cx("p"), &first.activity, StepRef::default())));
    assert_eq!(
        (r.kind.as_str(), r.status),
        ("conflict", 409),
        "a later change is in between"
    );
    assert_eq!(
        refused(run(m.revert(&cx("p"), "nonsense", StepRef::default()))).kind,
        "unknown-change"
    );
}

#[test]
fn rights_are_everything_except_what_the_spec_forbids() {
    let set = calendar(false, 0, Vec::new());
    let keys: Vec<String> = set.rows(BARS).unwrap().into_iter().map(|r| r.key).collect();
    let m = module(&calendar(
        false,
        0,
        vec![Forbid::on("move", &keys[1]), Forbid::everywhere("create")],
    ))
    .declare(&bars_with_writes());
    let who = cx("p");
    let may = |cmd: &str, rec: Option<&str>| {
        m.rules()
            .may(&Ask::new(&who, cmd, rec, Utc::from_millis(0)))
            .is_allowed()
    };
    assert!(!may("move", Some(&keys[1])));
    assert!(may("move", Some(&keys[0])));
    assert!(!may("create", None) && !may("create", Some(&keys[0])));
    assert!(may("change.revert", Some(&keys[1])) && may("anything.else", None));
    assert!(m.rules().hidden(&who, &keys[0], "start").is_none());
}

#[test]
fn the_provider_says_test_data() {
    let m = calendar_module(Vec::new());
    assert_eq!(m.provider(), SAMPLE_PROVIDER);
    assert_eq!(
        ProviderKind::of(m.provider()),
        ProviderKind::Sample,
        "a host recognises example data by the provider text"
    );
    assert_eq!(
        ProviderKind::of("net.freesynergy.Planning"),
        ProviderKind::Module
    );
    assert_eq!(m.level(), 2);
    let r = refused(run(m.blob(&cx("p"), "x", "image")));
    assert_eq!(r.kind, "unsupported");
}
