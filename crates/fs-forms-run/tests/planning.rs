//! B4: the window takes its records from the planning service (real data), not from the sample board. The service is the real
//! program `fs-modules-planning` (see `support/service.rs`: a socket and XDG folders in a temporary directory).
//!
//! Tests: card, move and column made through the host are in the service (a second connection sees them); rename and limit stay
//! locked; a service that is not there is a code and a notice, never an empty board without a word; a contract without a module
//! is said. The pictures with real records are in `planning_pictures.rs`.
#![cfg(unix)]

mod support;
use std::path::Path;

use fs_components_kanban::{Answer, Column, Operation};
use fs_forms_run::{Config, PlanningRecords, Records, View};
use support::service::{connect, start};

fn titles(cols: &[Column], key: &str) -> Vec<String> {
    cols.iter()
        .find(|c| c.key == key)
        .map(|c| c.cards.iter().map(|x| x.title.clone()).collect())
        .unwrap_or_default()
}

fn column_keys(cols: &[Column]) -> Vec<&str> {
    cols.iter().map(|c| c.key.as_str()).collect()
}

#[test]
fn a_card_a_move_and_a_column_made_through_the_host_are_in_the_service() {
    let service = start();
    let records = connect(&service);
    assert!(!records.is_sample());
    assert_eq!(records.unavailable(), None);
    for op in [Operation::AddCard, Operation::Move, Operation::AddColumn] {
        assert!(
            records.can(op),
            "{op:?} is unlocked by what the service reports"
        );
    }
    for op in [Operation::Rename, Operation::Limit] {
        assert!(
            !records.can(op),
            "{op:?} stays locked: the port has no call for it"
        );
    }
    assert_eq!(
        column_keys(&records.columns()),
        ["todo", "doing", "done", "closed"]
    );

    assert_eq!(records.add_card("todo", "Write the spec"), Answer::Accepted);
    records.add_card("todo", "Review the plan");
    assert_eq!(
        titles(&records.columns(), "todo"),
        ["Write the spec", "Review the plan"]
    );

    let first = records.columns()[0].cards[0].key.clone();
    assert_eq!(
        records.move_card(&first, "todo", "doing"),
        Answer::Accepted
    );
    assert_eq!(titles(&records.columns(), "todo"), ["Review the plan"]);
    assert_eq!(titles(&records.columns(), "doing"), ["Write the spec"]);

    assert_eq!(records.add_column("Review"), Answer::Accepted);
    assert_eq!(
        column_keys(&records.columns()),
        ["todo", "doing", "done", "closed", "review"]
    );

    // what a refusal of the service shows: a code the board has a sentence for
    assert_eq!(
        records.move_card(&first, "doing", "nowhere"),
        Answer::Refused("rejected".into())
    );

    // a second connection (another window) sees the same: the records are the service's, not the host's
    let other = connect(&service);
    assert_eq!(other.columns(), records.columns());
    assert_eq!(titles(&other.columns(), "doing"), ["Write the spec"]);
}

#[test]
fn a_service_that_is_not_there_is_a_code_and_a_notice_not_an_empty_board_without_a_word() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("none.sock");
    // no program file next to the "running" file and none in /usr/bin that is asked for here: nothing can be started
    let running = dir.path().join("fs-forms-run");
    let e = PlanningRecords::connect(&socket, &running)
        .err()
        .expect("nothing serves it");
    assert_eq!(e.kind, fs_modules_planning_client::Kind::Unreachable);

    let locale = fs_forms_run::session_locale();
    let texts = fs_forms_run::words(&locale);
    let view = View {
        sources: vec!["tasks >=0.2 <0.3".to_owned()],
        ..View::default()
    };
    let config = Config {
        socket: Some(socket),
        ..Config::default()
    };
    let opened = fs_forms_run::open(&config, &view, &texts, &running);
    assert!(!opened.connected);
    let why = opened.records.unavailable().expect("the window says why");
    assert!(why.contains("host-notice-data-unreachable"), "{why}");
    assert!(why.contains("contract=tasks"), "{why}");
    let notice = opened.notice.expect("a notice for the window");
    assert_eq!(notice.code, "host-notice-data-unreachable");
    assert!(opened.records.columns().is_empty());
    for op in Operation::ALL {
        assert!(
            !opened.records.can(op),
            "{op:?} stays locked without a source"
        );
    }
    assert_eq!(
        opened.records.add_card("todo", "x"),
        Answer::Refused("unsupported".into())
    );
}

#[test]
fn a_contract_this_host_has_no_module_for_is_said_and_no_source_is_not_an_error() {
    let locale = fs_forms_run::session_locale();
    let texts = fs_forms_run::words(&locale);
    let running = Path::new("/nonexistent/fs-forms-run");
    let ask = |sources: &[&str], sample: bool| {
        let view = View {
            sources: sources.iter().map(|s| (*s).to_owned()).collect(),
            ..View::default()
        };
        let config = Config {
            sample_data: sample,
            ..Config::default()
        };
        fs_forms_run::open(&config, &view, &texts, running)
    };
    let unknown = ask(&["calendars >=0.1 <0.2"], false);
    assert_eq!(
        unknown.notice.expect("said").code,
        "host-notice-source-unknown"
    );
    assert!(unknown.records.unavailable().is_some());
    let none = ask(&[], false);
    assert!(none.notice.is_none() && none.records.unavailable().is_none() && !none.connected);
    let sample = ask(&["tasks >=0.2 <0.3"], true);
    assert!(
        sample.records.is_sample(),
        "--sample-data wins, nothing is connected"
    );
}
