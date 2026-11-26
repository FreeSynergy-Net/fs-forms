//! The claim of package 3: a command runs from the `writes` column of its declaration, no command name is compared anywhere.
//! The declarations here use names no code could know, and one that is a trap (`create` that only changes).

mod module_support;

use fs_interfaces::forms::{self, Form};
use fs_interfaces::module::ModuleCore;
use module_support::{BARS, LANE, TITLE, calendar, call, cx, module, page, refused, run, text};

const WHEN: [(&str, &str); 2] = [
    ("start", "2026-05-12T10:00:00+02:00"),
    ("end", "2026-05-12T11:00:00+02:00"),
];

/// `bars-in-time` with strange command names: `shift` changes, `add-one` makes a record, and `create` only changes the lane.
fn strange() -> Form {
    let text = "bars-in-time@1.1.0 | Bars in time | planned | stand-in with writes\n\
        > items | records | ! | 1\n> start | time | ! | 1\n> end | time | ! | 1\n> label | text | ! | 1\n> lane | text | - | 0..1\n\
        > shift | command | - | 0..1 | start! end!\n> add-one | command | - | 0..1 | start! end! label!\n\
        > create | command | - | 0..1 | lane!\n";
    forms::parse(text).expect("parses").remove(0)
}

fn strange_module() -> fs_forms_testdata::SampleModule {
    module(&calendar(false, 0, Vec::new())).declare(&strange())
}

#[test]
fn a_command_under_a_strange_name_changes_a_record_from_its_writes() {
    let m = strange_module();
    let key = page(&m, BARS).records[0].id.clone();
    run(m.apply(&cx("p"), call("shift", Some(&key), &WHEN))).expect("shift");
    assert_eq!(text(&page(&m, BARS), &key, "start"), Some(WHEN[0].1));
}

#[test]
fn a_command_under_a_strange_name_makes_a_record_because_its_writes_cover_the_required_channels() {
    let m = strange_module();
    let n = page(&m, BARS).records.len();
    let args = [WHEN[0], WHEN[1], (TITLE, "Made")];
    let made = run(m.apply(&cx("p"), call("add-one", None, &args))).expect("add-one");
    let p = page(&m, BARS);
    assert_eq!(p.records.len(), n + 1);
    assert_eq!(text(&p, &made.record, TITLE), Some("Made"));
}

#[test]
fn the_name_create_decides_nothing_a_create_that_writes_only_the_lane_changes_a_record() {
    let m = strange_module();
    let key = page(&m, BARS).records[0].id.clone();
    run(m.apply(&cx("p"), call("create", Some(&key), &[(LANE, "x")]))).expect("changes");
    assert_eq!(text(&page(&m, BARS), &key, LANE), Some("x"));
    // Without a record it is refused: it does not make one just for its name.
    let r = refused(run(m.apply(&cx("p"), call("create", None, &[(LANE, "x")]))));
    assert_eq!(r.kind, "bad-arguments");
}

#[test]
fn an_argument_for_a_channel_that_is_not_in_writes_is_refused_even_if_the_channel_exists() {
    let m = strange_module();
    let key = page(&m, BARS).records[0].id.clone();
    let before = page(&m, BARS);
    // `label` is a data channel of the form, but `shift` writes only `start! end!`.
    let args = [WHEN[0], WHEN[1], (TITLE, "sneaked")];
    let r = refused(run(m.apply(&cx("p"), call("shift", Some(&key), &args))));
    assert_eq!((r.kind.as_str(), r.status), ("bad-arguments", 400));
    assert_eq!(
        text(&page(&m, BARS), &key, TITLE),
        text(&before, &key, TITLE),
        "nothing was written"
    );
}
