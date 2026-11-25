//! R: a board with two sources (the cards of `Task`, the lane list of `Lane`, both read as forms of one module). The data goes to the
//! block by (source, form, type); the lane list is the heads of the lanes slot; a lane list that cannot be used is a plan `Missing` with
//! the code that says why, never a silent plain board. One test per row of the table of failures (ENTWURF section 2.4).
mod lanes_support;

use fs_components_kanban::{LaneCard, LaneHead, LanePlan, lanes_of};
use fs_forms_run::placed;
use lanes_support::{
    all_forms, card, cards_entry, codes, feeds, lane, lanes_rows, records, rows_entry,
};

#[test]
fn lane_rows_and_task_rows_land_at_their_own_place() {
    let p = placed(&records(all_forms()), "board", &feeds());
    let keys = |i: &fs_components_core::form_read::FormInput| {
        i.rows.iter().map(|r| r.key.clone()).collect::<Vec<_>>()
    };
    assert_eq!(keys(&p.input), ["t1", "t2"], "the cards are the Task rows");
    let heads = p.heads.expect("heads");
    assert_eq!(keys(&heads), ["urn:p:anna", "urn:p:ben", "urn:p:cleo"]);
    assert!(p.notices.is_empty());
    // the cards carry NAMES, the heads are IRIs: the lane takes the name as key, the label is the name (an empty lane stays)
    let LanePlan::On { grouping, heads } = p.plan else {
        panic!("lanes are on");
    };
    assert_eq!(grouping, "assignee");
    assert_eq!(heads[0], LaneHead::new("Anna", "Anna"));
    assert_eq!(heads[2], LaneHead::new("Cleo", "Cleo"));
}

#[test]
fn cards_that_carry_the_iri_keep_the_iri_as_the_key() {
    let mut forms = all_forms();
    forms[0] = cards_entry(vec![card("t1", "urn:p:anna")]);
    let p = placed(&records(forms), "board", &feeds());
    let LanePlan::On { heads, .. } = p.plan else {
        panic!("on");
    };
    assert_eq!(heads[0], LaneHead::new("urn:p:anna", "Anna"));
    assert_eq!(heads[1], LaneHead::new("urn:p:ben", "Ben"));
}

#[test]
fn a_block_without_heads_gets_no_lanes_and_the_old_assignment_by_form_name_still_serves() {
    let mut f = feeds();
    f[0].heads = None;
    let p = placed(&records(all_forms()), "board", &f);
    assert_eq!((p.heads, p.plan), (None, LanePlan::Off));
    // old callers: source 0 and no type serve every source
    let mut old = cards_entry(vec![card("t1", "Anna")]);
    (old.source, old.data_type) = (0, String::new());
    let p = placed(&records(vec![old]), "board", &[]);
    assert_eq!(p.input.rows.len(), 1);
}

#[test]
fn failure_1_the_head_source_is_not_there_says_so_and_the_columns_stay() {
    let p = placed(&records(vec![all_forms().remove(0)]), "board", &feeds());
    assert_eq!(p.plan, LanePlan::Missing);
    assert_eq!(
        codes(&p),
        [
            "host-notice-lanes-missing",
            "host-notice-lanes-source-absent"
        ]
    );
    assert_eq!(p.input.rows.len(), 2, "the cards are still given");
    assert!(p.heads.is_none());
}

#[test]
fn failure_1b_a_shut_slot_of_the_plan_is_said_with_its_cause() {
    let mut f = feeds();
    f[0].slot_locks = vec![
        fs_forms_run::Coded::new("accepts-error-contract-missing").with("contract", "planning"),
        // the lock of another slot says nothing about lanes
        fs_forms_run::Coded::new("bind-error-locked").with("part", "order"),
    ];
    let p = placed(&records(all_forms()), "board", &f);
    assert_eq!(p.plan, LanePlan::Missing);
    assert_eq!(
        codes(&p),
        [
            "host-notice-lanes-missing",
            "accepts-error-contract-missing"
        ]
    );
}

#[test]
fn failure_2_the_module_does_not_know_the_scope() {
    let mut r = records(all_forms());
    r.problems = vec![(
        2,
        fs_forms_run::Coded::new("form-source-unknown-scope").with("scope", "lanes:nope"),
    )];
    let p = placed(&r, "board", &feeds());
    assert_eq!(p.plan, LanePlan::Missing);
    assert_eq!(
        codes(&p),
        ["host-notice-lanes-missing", "form-source-unknown-scope"]
    );
    // a problem of ANOTHER source does not touch the lanes
    r.problems = vec![(1, fs_forms_run::Coded::new("form-source-unknown-scope"))];
    assert!(matches!(
        placed(&r, "board", &feeds()).plan,
        LanePlan::On { .. }
    ));
}

#[test]
fn failure_3_the_module_declares_no_entry_with_the_type() {
    // the source 2 was read, but the module declares `rows` only for `Task`
    let forms = vec![
        cards_entry(vec![card("t1", "Anna")]),
        rows_entry(2, "Task", true, vec![lane("t1", "x")]),
    ];
    let p = placed(&records(forms), "board", &feeds());
    assert_eq!(p.plan, LanePlan::Missing);
    assert_eq!(
        codes(&p),
        ["host-notice-lanes-missing", "bind-error-heads-form-missing"]
    );
    assert_eq!(
        p.notices[1].params,
        [
            ("form".into(), "rows".into()),
            ("type".into(), "Lane".into())
        ]
    );
}

#[test]
fn failure_4_the_rows_do_not_fit_the_head_form() {
    let mut forms = all_forms();
    forms[2] = rows_entry(2, "Lane", false, lanes_rows());
    let p = placed(&records(forms), "board", &feeds());
    assert_eq!(p.plan, LanePlan::Missing);
    assert_eq!(
        codes(&p),
        ["host-notice-lanes-missing", "form-channel-unmapped"]
    );
}

#[test]
fn failure_5_a_card_value_without_a_head_gets_its_lane_at_the_end() {
    let mut forms = all_forms();
    forms[0] = cards_entry(vec![card("t1", "Anna"), card("t9", "Zoe")]);
    let p = placed(&records(forms), "board", &feeds());
    let LanePlan::On { grouping, heads } = p.plan else {
        panic!("on");
    };
    let cards: Vec<LaneCard> = p
        .input
        .rows
        .iter()
        .map(|r| {
            let v: Vec<&str> = r.all("assignee").iter().map(String::as_str).collect();
            LaneCard::new(&r.key).with(&grouping, &v)
        })
        .collect();
    let lanes = lanes_of(&heads, &cards, &grouping);
    let keys: Vec<_> = lanes.iter().map(|l| (l.key.as_str(), l.headed)).collect();
    assert_eq!(
        keys,
        [
            ("Anna", true),
            ("Ben", true),
            ("Cleo", true),
            ("Zoe", false)
        ],
        "empty lanes stay, Zoe is appended and never hidden"
    );
    assert_eq!(lanes[3].cards, ["t9"]);
}
