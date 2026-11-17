//! `PlanSuite` (feature `conformance`): 12 rules over a plan and its inputs, each proven by one bench mutant.
//!
//! Used by the crate's own tests on [`bench::BindBench`] (real `Bind::build` with flaw switches), by the host on REAL plans
//! through [`check_plan`], and later by `fs-forms bind check`. fs-conformance joins here when it is pinned (never in the normal
//! graph). Pattern: Specification suite + mutants (as the records suite of F1).

pub mod bench;
mod rules;
mod rules_menu;

use crate::catalog::Catalog;
use crate::model::Resolved;
use crate::plan::Plan;
use crate::providers::HostCaps;

pub use bench::{BindBench, Flaw, bench_mutants};

/// What a rule needs: the inputs and the plan built from them.
pub trait PlanFixture {
    /// The layout the plan is built from.
    fn resolved(&self) -> &Resolved;
    /// The catalog.
    fn catalog(&self) -> &dyn Catalog;
    /// Host abilities.
    fn host(&self) -> HostCaps;
    /// The plan.
    fn build(&self) -> Plan;
    /// The same inputs with another style; `None` = rule skipped.
    fn with_style(&self, style: &str) -> Option<Plan>;
}

/// One verdict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verdict {
    /// Rule name, e.g. `plan.every-place`.
    pub rule: &'static str,
    /// Held?
    pub pass: bool,
    /// On failure the code `bind-suite-error-<rule>` and the reason.
    pub code: Option<(String, String)>,
}

/// Codes of the suite (prefix `bind-suite-error-`), one per rule plus `fixture-incomplete`.
pub struct SuiteCode;

impl SuiteCode {
    /// Every code name once.
    pub const ALL: [&'static str; 13] = [
        "every-place",
        "lock-coded",
        "source-lock-spreads",
        "chosen",
        "schemes",
        "commands",
        "menu-from-layout",
        "bindings-ordered",
        "style-blind",
        "deterministic",
        "state",
        "notices",
        "fixture-incomplete",
    ];
    /// Prefix.
    pub const PREFIX: &'static str = "bind-suite-error-";
}

type Check = fn(&dyn PlanFixture) -> Result<(), String>;

/// The 12 rules.
pub struct PlanSuite;

impl PlanSuite {
    /// Rule names with their check, in fixed order.
    #[must_use]
    pub fn rules() -> Vec<(&'static str, Check)> {
        vec![
            ("plan.every-place", rules::every_place as Check),
            ("plan.lock-coded", rules::lock_coded),
            ("plan.source-lock-spreads", rules::source_lock_spreads),
            ("plan.chosen", rules::chosen),
            ("plan.schemes", rules::schemes),
            ("plan.commands", rules_menu::commands),
            ("plan.menu-from-layout", rules_menu::menu_from_layout),
            ("plan.bindings-ordered", rules::bindings_ordered),
            ("plan.style-blind", rules::style_blind),
            ("plan.deterministic", rules::deterministic),
            ("plan.state", rules::state),
            ("plan.notices", rules::notices),
        ]
    }

    /// Runs every rule.
    #[must_use]
    pub fn run(fixture: &dyn PlanFixture) -> Vec<Verdict> {
        Self::rules()
            .into_iter()
            .map(|(rule, check)| match check(fixture) {
                Ok(()) => Verdict {
                    rule,
                    pass: true,
                    code: None,
                },
                Err(why) => {
                    let code = format!("{}{}", SuiteCode::PREFIX, rule.trim_start_matches("plan."));
                    Verdict {
                        rule,
                        pass: false,
                        code: Some((code, why)),
                    }
                }
            })
            .collect()
    }

    /// Runs every bench mutant and reports which rule caught it; `is_proven` needs 12 of 12 and a green base.
    #[must_use]
    pub fn prove() -> Proof {
        let base_green = Self::run(&BindBench::new(None)).iter().all(|v| v.pass);
        let mut caught = Vec::new();
        for (flaw, rule) in bench_mutants() {
            let verdicts = Self::run(&BindBench::new(Some(flaw)));
            if verdicts.iter().any(|v| v.rule == rule && !v.pass) {
                caught.push(rule);
            }
        }
        Proof { base_green, caught }
    }
}

/// Result of `prove`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proof {
    /// Rules were all green on the flawless bench.
    pub base_green: bool,
    /// Rules that caught their mutant.
    pub caught: Vec<&'static str>,
}

impl Proof {
    /// Every rule caught its mutant and the base is green.
    #[must_use]
    pub fn is_proven(&self) -> bool {
        self.base_green && self.caught.len() == 12
    }
}

/// Run time use on a real plan: the verdicts of all rules.
#[must_use]
pub fn check_plan(fixture: &dyn PlanFixture) -> Vec<Verdict> {
    PlanSuite::run(fixture)
}
