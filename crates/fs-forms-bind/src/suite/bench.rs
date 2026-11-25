//! `BindBench`: the real `Bind::build` on a rich fixture, with one flaw switch per rule (only for `prove`).
//!
//! Each flaw changes the built plan the way the named mistake would; the matching rule must turn red.
//!
//! Pattern: Test Double with flaw switches (mutants for `prove`).

use std::cell::Cell;

mod flaws;

use self::flaws::apply;
use super::PlanFixture;
use crate::build::Bind;
use crate::catalog::Catalog;
use crate::model::{
    BindKind, BindingIn, MenuIn, MenuKind, PageIn, PlaceIn, Problem, Range, Resolved, SourceIn,
};
use crate::model_tree::NodeIn;
use crate::plan::Plan;
use crate::providers::{HostCaps, Offer, Providers};
use crate::testing::{FakeFacts, TestCatalog, cmd, field, settings, tasks_facts, tasks_resolved};
use crate::types::{Basic, Role};

/// Flaw switches, one per rule.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flaw {
    /// A locked place is left out of the plan.
    PlaceDropped,
    /// Locks carry a code of no known family.
    LockUncoded,
    /// Only the first place of a locked source is locked.
    SourceLockLocal,
    /// bind picks `field-text` for everything.
    HostPicksText,
    /// A missing/own scheme list becomes `[https, http]`.
    SchemesDefaulted,
    /// Unsupported commands lose their "coming" lock.
    ComingHidden,
    /// The menu is sorted by label.
    MenuSortedByLabel,
    /// The `nth` order of bindings is ignored.
    NthIgnored,
    /// The style chooses a block.
    StylePicksBlock,
    /// Iteration order changes between builds.
    HashOrder,
    /// A partial plan says `Ready`.
    PartialAsReady,
    /// Open overlay conflicts are not reported.
    ConflictSwallowed,
}

/// The 12 mutants with the rule each must turn red.
#[must_use]
pub fn bench_mutants() -> Vec<(Flaw, &'static str)> {
    use Flaw as F;
    vec![
        (F::PlaceDropped, "plan.every-place"),
        (F::LockUncoded, "plan.lock-coded"),
        (F::SourceLockLocal, "plan.source-lock-spreads"),
        (F::HostPicksText, "plan.chosen"),
        (F::SchemesDefaulted, "plan.schemes"),
        (F::ComingHidden, "plan.commands"),
        (F::MenuSortedByLabel, "plan.menu-from-layout"),
        (F::NthIgnored, "plan.bindings-ordered"),
        (F::StylePicksBlock, "plan.style-blind"),
        (F::HashOrder, "plan.deterministic"),
        (F::PartialAsReady, "plan.state"),
        (F::ConflictSwallowed, "plan.notices"),
    ]
}

/// The fixture: tasks board + a not installed `notes` source with two places + a link field + an unsupported command + a conflict.
pub struct BindBench {
    resolved: Resolved,
    catalog: TestCatalog,
    facts: FakeFacts,
    flaw: Option<Flaw>,
    calls: Cell<u32>,
}

impl BindBench {
    /// A bench with an optional flaw.
    #[must_use]
    pub fn new(flaw: Option<Flaw>) -> Self {
        let mut r = tasks_resolved();
        r.sources.push(SourceIn {
            elem: "s1".into(),
            capability: "notes".into(),
            range: Range {
                min: (0, 1, 0),
                max: (0, 2, 0),
            },
            data_type: "Note".into(),
        });
        let note_place = |name: &str, seq: u32| PlaceIn {
            elem: format!("pl-notes-{name}"),
            place: name.into(),
            seq,
            block: "notes-list".into(),
            source: Some(2),
            bindings: vec![],
            set: vec![],
        };
        r.pages.push(PageIn {
            elem: "pg-notes".into(),
            page: "notes".into(),
            seq: 2,
            frame: "workbench".into(),
            places: vec![note_place("a", 0).into(), note_place("b", 1).into()],
        });
        let link = BindKind::Field {
            field: field("homepage", Basic::Link),
            role: Role::Display,
            schemes: Some(vec!["ssh".into(), "https".into()]),
        };
        if let NodeIn::Block(b) = &mut r.pages[1].places[0] {
            b.bindings.push(BindingIn {
                elem: "b-homepage".into(),
                slot: "properties".into(),
                nth: 1,
                kind: link,
            });
        }
        let mut set_cols = cmd("setColumns", false, false, false);
        set_cols.source = 0;
        r.menu[1].children.push(MenuIn {
            elem: "m-cols".into(),
            kind: MenuKind::Command {
                command: set_cols,
                on_selected: false,
                shortcut: None,
            },
            children: vec![],
        });
        r.conflicts.push(Problem {
            elem: None,
            code: "layout-error-overlay-conflict".into(),
            params: vec![],
            family: crate::coded::Family::Layout,
            reason: None,
        });
        let mut facts = tasks_facts();
        facts.0.push(("notes".into(), Offer::NotInstalled));
        Self {
            resolved: r,
            catalog: TestCatalog::new(),
            facts,
            flaw,
            calls: Cell::new(0),
        }
    }

    fn plan(&self, style: Option<&str>) -> Plan {
        let std = crate::testing::standard();
        let mut s = settings();
        s.style = style.map(str::to_owned);
        let providers = Providers {
            records: &self.facts,
            settings: &s,
            host: self.host(),
            platform: crate::Platform::Linux,
            standard: &std,
        };
        let mut plan = Bind::build(&self.resolved, &self.catalog, &providers);
        self.calls.set(self.calls.get() + 1);
        if let Some(f) = self.flaw {
            apply(f, &mut plan, style, self.calls.get());
        }
        plan
    }
}

impl PlanFixture for BindBench {
    fn resolved(&self) -> &Resolved {
        &self.resolved
    }
    fn catalog(&self) -> &dyn Catalog {
        &self.catalog
    }
    fn host(&self) -> HostCaps {
        HostCaps {
            write: true,
            workflow: false,
        }
    }
    fn build(&self) -> Plan {
        self.plan(None)
    }
    fn with_style(&self, style: &str) -> Option<Plan> {
        Some(self.plan(Some(style)))
    }
}
