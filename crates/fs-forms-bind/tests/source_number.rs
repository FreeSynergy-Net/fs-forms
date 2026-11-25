//! Source numbers count from 1 (`[[source]]` first = 1, `source = N` of the layout, `Want`/`FormData.source` of the host): a
//! command of source 2 reads the lock of the SECOND source, never of the first (the 0-based slip of review card 3).
mod common;
use common::*;
use fs_forms_bind::suite::{PlanFixture, PlanSuite};
use fs_forms_bind::testing::*;
use fs_forms_bind::*;

fn two_sources(command_source: u32) -> Resolved {
    let mut r = tasks_resolved();
    r.sources.push(SourceIn {
        elem: "source:2".into(),
        capability: "planning".into(),
        range: Range {
            min: (0, 0, 0),
            max: (0, 1, 0),
        },
        data_type: "Lane".into(),
    });
    // Only the commands ask for a source here: no placement is locked by its own source.
    for page in &mut r.pages {
        for n in &mut page.places {
            if let NodeIn::Block(p) = n {
                p.source = None;
                for b in &mut p.bindings {
                    if let BindKind::Command(c) = &mut b.kind {
                        c.source = command_source;
                    }
                }
            }
        }
    }
    set_menu_source(&mut r.menu, command_source);
    r
}

fn set_menu_source(menu: &mut [MenuIn], source: u32) {
    for m in menu {
        if let MenuKind::Command { command, .. } = &mut m.kind {
            command.source = source;
        }
        set_menu_source(&mut m.children, source);
    }
}

/// `tasks` is the first source, `planning` the second; the one named `broken` is not installed.
fn facts(broken: &str) -> FakeFacts {
    let ok = |c: &str| {
        (
            c.to_owned(),
            if c == broken {
                Offer::NotInstalled
            } else {
                Offer::Reached(vec![provider(0, if c == "tasks" { 2 } else { 0 })])
            },
        )
    };
    FakeFacts(vec![ok("tasks"), ok("planning")])
}

fn plan(r: &Resolved, broken: &str) -> Plan {
    build_with(r, &TestCatalog::new(), &facts(broken), host())
}

fn slot_commands(p: &Plan) -> Vec<&CommandPlan> {
    let PlaceStatus::Ready(r) = &p.place("board", "main").expect("place").status else {
        panic!("the board lives, no placement source is asked for");
    };
    let v: Vec<_> = r
        .slots
        .iter()
        .filter_map(|s| match &s.fill {
            Fill::Command(c) => Some(c),
            _ => None,
        })
        .collect();
    assert!(!v.is_empty(), "the fixture has slot commands");
    v
}

fn assert_all(cmds: &[&CommandPlan], locked: bool, what: &str) {
    assert!(!cmds.is_empty(), "{what}: commands exist");
    for c in cmds {
        assert_eq!(c.fixed.is_some(), locked, "{what}: command {}", c.name);
    }
}

#[test]
fn both_sources_healthy_nothing_is_locked() {
    let p = plan(&two_sources(2), "");
    assert_all(&p.menu.commands(), false, "menu");
    assert_all(&slot_commands(&p), false, "slot");
}

#[test]
fn a_command_of_source_2_is_locked_when_source_2_is_down_and_source_1_is_fine() {
    let p = plan(&two_sources(2), "planning");
    assert!(p.sources[0].status.is_ok() && p.sources[1].status.is_err());
    assert_all(&p.menu.commands(), true, "menu");
    assert_all(&slot_commands(&p), true, "slot");
}

#[test]
fn a_command_of_source_2_stays_open_when_only_source_1_is_down() {
    let p = plan(&two_sources(2), "tasks");
    assert!(p.sources[0].status.is_err() && p.sources[1].status.is_ok());
    assert_all(&p.menu.commands(), false, "menu");
    assert_all(&slot_commands(&p), false, "slot");
}

#[test]
fn a_command_of_source_1_follows_the_first_source() {
    let down = plan(&two_sources(1), "tasks");
    assert_all(&down.menu.commands(), true, "menu");
    assert_all(&slot_commands(&down), true, "slot");
    let fine = plan(&two_sources(1), "planning");
    assert_all(&fine.menu.commands(), false, "menu");
    assert_all(&slot_commands(&fine), false, "slot");
}

#[test]
fn source_0_and_a_number_past_the_end_are_no_source_and_lock_nothing() {
    for n in [0, 3] {
        let p = plan(&two_sources(n), "tasks");
        assert_all(&p.menu.commands(), false, "menu");
    }
}

/// The rule `plan.commands` of the suite reads the lock with the same numbering as the build.
struct Fixture(Resolved, TestCatalog, &'static str);

impl PlanFixture for Fixture {
    fn resolved(&self) -> &Resolved {
        &self.0
    }
    fn catalog(&self) -> &dyn fs_forms_bind::Catalog {
        &self.1
    }
    fn host(&self) -> HostCaps {
        host()
    }
    fn build(&self) -> Plan {
        plan(&self.0, self.2)
    }
    fn with_style(&self, _: &str) -> Option<Plan> {
        None
    }
}

#[test]
fn the_commands_rule_of_the_suite_counts_from_1_too() {
    for broken in ["planning", "tasks"] {
        let f = Fixture(two_sources(2), TestCatalog::new(), broken);
        let v = PlanSuite::run(&f);
        let rule = v.iter().find(|v| v.rule == "plan.commands").unwrap();
        assert!(rule.pass, "{broken}: {:?}", rule.code);
    }
}
