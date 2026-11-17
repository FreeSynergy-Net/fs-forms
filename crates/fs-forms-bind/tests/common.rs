//! Shared helpers of the integration tests.
#![allow(dead_code)]

use fs_forms_bind::testing::{TestCatalog, settings, tasks_facts, tasks_resolved};
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
