//! Flaw application for `BindBench`: how each switch changes a built plan.
//!
//! Pattern: Strategy – one mutation per `Flaw`, picked by `apply` (the mutants for `prove`).
use super::Flaw;
use crate::coded::Coded;
use crate::codes::{Cause, Severity};
use crate::menu::{MenuEntry, MenuNode};
use crate::plan::{CommandPlan, Fill, PlaceStatus, Plan, PlanState, ReadyPlace};
use crate::plan_tree::NodePlan;

fn each_command(plan: &mut Plan, mut f: impl FnMut(&mut CommandPlan)) {
    fn walk(n: &mut MenuNode, f: &mut dyn FnMut(&mut CommandPlan)) {
        if let MenuEntry::Command { command, .. } = &mut n.entry {
            f(command);
        }
        n.children.iter_mut().for_each(|c| walk(c, f));
    }
    plan.menu.roots.iter_mut().for_each(|n| walk(n, &mut f));
}

fn each_field(plan: &mut Plan, mut f: impl FnMut(&mut crate::plan::FieldFill)) {
    for pl in plan
        .pages
        .iter_mut()
        .flat_map(|p| crate::plan_tree::leaves_of_mut(&mut p.places))
    {
        if let PlaceStatus::Ready(r) = &mut pl.status {
            for s in &mut r.slots {
                if let Fill::Field(ff) = &mut s.fill {
                    f(ff);
                }
            }
        }
    }
}

fn label(n: &MenuNode) -> String {
    match &n.entry {
        MenuEntry::Menu { label_key } => label_key.clone(),
        MenuEntry::Standard { which } => format!("standard:{which}"),
        _ => String::new(),
    }
}

pub(super) fn apply(flaw: Flaw, plan: &mut Plan, style: Option<&str>, call: u32) {
    match flaw {
        Flaw::PlaceDropped => {
            if let Some(p) = plan.pages.iter_mut().find(|p| {
                p.leaves()
                    .iter()
                    .any(|q| matches!(q.status, PlaceStatus::Locked(_)))
            }) {
                p.places
                    .retain(|q| !matches!(q, NodePlan::Block(b) if matches!(b.status, PlaceStatus::Locked(_))));
            }
        }
        Flaw::LockUncoded => {
            for pl in plan
                .pages
                .iter_mut()
                .flat_map(|p| crate::plan_tree::leaves_of_mut(&mut p.places))
            {
                if let PlaceStatus::Locked(v) = &mut pl.status {
                    for l in v.iter_mut() {
                        l.cause = Cause::Bind(Coded::new("oops"));
                    }
                }
            }
        }
        Flaw::SourceLockLocal => {
            let mut first = true;
            for pl in plan
                .pages
                .iter_mut()
                .flat_map(|p| crate::plan_tree::leaves_of_mut(&mut p.places))
            {
                if matches!(pl.status, PlaceStatus::Locked(_)) {
                    if !first {
                        pl.status = PlaceStatus::Ready(ReadyPlace { slots: vec![] });
                    }
                    first = false;
                }
            }
        }
        Flaw::HostPicksText => each_field(plan, |f| f.block = "field-text".into()),
        Flaw::SchemesDefaulted => each_field(plan, |f| {
            if f.basic.has_schemes() {
                f.schemes = Some(vec!["https".into(), "http".into()]);
            }
        }),
        Flaw::ComingHidden => each_command(plan, |c| {
            if c.fixed
                .as_ref()
                .is_some_and(|l| l.severity == Severity::Coming)
            {
                c.fixed = None;
            }
        }),
        Flaw::MenuSortedByLabel => plan.menu.roots.sort_by_key(label),
        Flaw::NthIgnored => {
            for pl in plan
                .pages
                .iter_mut()
                .flat_map(|p| crate::plan_tree::leaves_of_mut(&mut p.places))
            {
                if let PlaceStatus::Ready(r) = &mut pl.status {
                    r.slots.sort_by_key(|s| std::cmp::Reverse(s.nth));
                }
            }
        }
        Flaw::StylePicksBlock => {
            if let Some(s) = style {
                each_field(plan, |f| f.block = format!("{}-{s}", f.block));
            }
        }
        Flaw::HashOrder => {
            if call.is_multiple_of(2) {
                plan.pages.reverse();
            }
        }
        Flaw::PartialAsReady => plan.state = PlanState::Ready,
        Flaw::ConflictSwallowed => plan
            .notices
            .retain(|n| n.cause.coded().code != "layout-error-overlay-conflict"),
    }
}
