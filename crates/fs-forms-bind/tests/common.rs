//! Shared helpers of the integration tests.
// Test helpers: not part of any API, so `must_use` and `# Panics` notes would be noise.
#![allow(dead_code, clippy::must_use_candidate, clippy::missing_panics_doc)]

use fs_forms_bind::testing::{TestCatalog, settings, standard, tasks_facts, tasks_resolved};
use fs_forms_bind::*;

pub fn build_with(
    r: &Resolved,
    cat: &TestCatalog,
    facts: &dyn ProviderFacts,
    host: HostCaps,
) -> Plan {
    let s = settings();
    Bind::build(
        r,
        cat,
        &Providers {
            records: facts,
            settings: &s,
            host,
            platform: Platform::Linux,
            standard: &standard(),
        },
    )
}

pub fn host() -> HostCaps {
    HostCaps {
        write: true,
        workflow: false,
    }
}

pub fn tasks_plan() -> Plan {
    build_with(
        &tasks_resolved(),
        &TestCatalog::new(),
        &tasks_facts(),
        host(),
    )
}

pub fn locks_of(plan: &Plan, page: &str, place: &str) -> Vec<PartLock> {
    match &plan.place(page, place).expect("place in plan").status {
        PlaceStatus::Locked(v) => v.clone(),
        PlaceStatus::Ready(_) => vec![],
    }
}

pub fn code_of(l: &PartLock) -> String {
    l.cause.coded().code.clone()
}

pub fn view(may: &[&str]) -> RecordView {
    RecordView {
        id: "r1".into(),
        may: may.iter().map(|s| (*s).to_owned()).collect(),
        ..RecordView::default()
    }
}

/// A key press from the stored text form (`Control+N`, `Delete`).
pub fn press(text: &str) -> KeyPress {
    let sc = Shortcut::parse(text).expect("a readable key");
    KeyPress::new(sc.modifiers(), sc.key()).expect("a key press")
}

/// Like `build_with` but with the settings and the standard menus of the host chosen by the test.
pub fn build_custom(r: &Resolved, s: &SettingsFacts, std_menus: &StandardMenus) -> Plan {
    Bind::build(
        r,
        &TestCatalog::new(),
        &Providers {
            records: &tasks_facts(),
            settings: s,
            host: host(),
            platform: Platform::Linux,
            standard: std_menus,
        },
    )
}

pub fn problem(elem: &str, code: &str) -> Problem {
    Problem {
        elem: Some(elem.into()),
        code: code.into(),
        params: vec![],
    }
}

/// Every code of the plan: locks and window notices.
pub fn all_codes(p: &Plan) -> Vec<String> {
    let mut v: Vec<String> = p.locks().iter().map(|l| code_of(l)).collect();
    v.extend(p.notices.iter().map(|n| n.cause.coded().code.clone()));
    v
}
