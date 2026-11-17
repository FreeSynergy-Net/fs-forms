//! The twelve rule checks of `PlanSuite`. Each returns `Err(reason)` on the first violation.

use super::PlanFixture;
use crate::coded::Family;
use crate::codes::{Cause, Severity};
use crate::menu::{MenuPlan, standard_entries};
use crate::model::{BindKind, MenuIn, MenuKind, Target};
use crate::plan::{CommandPlan, Fill, PlaceStatus, PlanState};

type R = Result<(), String>;

fn ready_slots(plan: &crate::plan::Plan) -> Vec<&crate::plan::SlotFill> {
    let mut v = Vec::new();
    for pl in plan.pages.iter().flat_map(|p| &p.places) {
        if let PlaceStatus::Ready(r) = &pl.status {
            v.extend(r.slots.iter());
        }
    }
    v
}

pub(super) fn every_place(f: &dyn PlanFixture) -> R {
    let plan = f.build();
    for page in &f.resolved().pages {
        for pl in &page.places {
            let n = plan
                .pages
                .iter()
                .filter(|p| p.page == page.page)
                .flat_map(|p| &p.places)
                .filter(|q| q.place == pl.place)
                .count();
            if n != 1 {
                return Err(format!(
                    "place {}/{} appears {n} times",
                    page.page, pl.place
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn lock_coded(f: &dyn PlanFixture) -> R {
    let plan = f.build();
    for l in plan.locks() {
        let fam = l.cause.coded().family();
        let ok = matches!(
            (&l.cause, fam),
            (Cause::Accepts(_), Family::Accepts)
                | (Cause::Layout(_), Family::Layout)
                | (Cause::Records(_), Family::Records)
                | (Cause::Bind(_), Family::Bind)
        );
        if !ok || l.part.text().is_empty() {
            return Err(format!(
                "lock without a known code: {}",
                l.cause.coded().code
            ));
        }
    }
    for pl in plan.pages.iter().flat_map(|p| &p.places) {
        if matches!(&pl.status, PlaceStatus::Locked(v) if v.is_empty()) {
            return Err(format!("locked place {} without lock", pl.place));
        }
    }
    Ok(())
}

pub(super) fn source_lock_spreads(f: &dyn PlanFixture) -> R {
    let plan = f.build();
    for s in &plan.sources {
        let Err(lock) = &s.status else { continue };
        for page in &f.resolved().pages {
            for pl in page.places.iter().filter(|p| p.source == Some(s.idx)) {
                let got = plan.place(&page.page, &pl.place);
                let ok = matches!(got.map(|g| &g.status), Some(PlaceStatus::Locked(v)) if v.iter().any(|l| l.cause == lock.cause));
                if !ok {
                    return Err(format!(
                        "place {} of locked source {} is not locked with its cause",
                        pl.place, s.idx
                    ));
                }
            }
        }
    }
    Ok(())
}

pub(super) fn chosen(f: &dyn PlanFixture) -> R {
    let plan = f.build();
    for s in ready_slots(&plan) {
        if let Fill::Field(ff) = &s.fill
            && ff.block != f.catalog().choose(&ff.field, ff.role)
        {
            return Err(format!("field {:?} got block {}", ff.path, ff.block));
        }
    }
    Ok(())
}

pub(super) fn schemes(f: &dyn PlanFixture) -> R {
    let plan = f.build();
    for page in &f.resolved().pages {
        for pl in &page.places {
            for b in &pl.bindings {
                let BindKind::Field { field, schemes, .. } = &b.kind else {
                    continue;
                };
                if !field.basic.has_schemes() {
                    continue;
                }
                let Some(got) = plan.place(&page.page, &pl.place) else {
                    continue;
                };
                match (&got.status, schemes.as_ref().is_some_and(|s| !s.is_empty())) {
                    (PlaceStatus::Locked(_), _) => {}
                    (PlaceStatus::Ready(_), false) => {
                        return Err(format!("{} drawn without scheme list", b.elem));
                    }
                    (PlaceStatus::Ready(r), true) => {
                        let fill = r.slots.iter().find(|s| s.elem == b.elem);
                        if !matches!(fill.map(|s| &s.fill), Some(Fill::Field(ff)) if ff.schemes == *schemes)
                        {
                            return Err(format!("{} changed its scheme list", b.elem));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn plan_commands(plan: &crate::plan::Plan) -> Vec<&CommandPlan> {
    let mut v = plan.menu.commands();
    for s in ready_slots(plan) {
        if let Fill::Command(c) = &s.fill {
            v.push(c);
        }
    }
    v
}

pub(super) fn commands(f: &dyn PlanFixture) -> R {
    let plan = f.build();
    let host = f.host();
    for c in plan_commands(&plan) {
        let source_locked = plan
            .sources
            .get(c.source as usize)
            .is_some_and(|s| s.status.is_err());
        let expect_coming = !c.description.supported;
        let coming = c
            .fixed
            .as_ref()
            .is_some_and(|l| l.severity == Severity::Coming);
        if expect_coming != coming {
            return Err(format!(
                "command {} coming={coming}, expected {expect_coming}",
                c.name
            ));
        }
        if !expect_coming && !source_locked {
            let wf = matches!(c.target, Target::Workflow(_)) && !host.workflow;
            let wr = matches!(c.target, Target::Program) && c.description.writes && !host.write;
            if c.fixed.is_some() != (wf || wr) {
                return Err(format!("command {} has a wrong fixed lock", c.name));
            }
        }
    }
    Ok(())
}

fn expected_outline(nodes: &[MenuIn], depth: usize, out: &mut Vec<String>) {
    for n in nodes {
        let pad = "  ".repeat(depth);
        match &n.kind {
            MenuKind::Menu(l) => out.push(format!("{pad}{l}")),
            MenuKind::Standard(w) => {
                out.push(format!("{pad}standard:{w}"));
                for (name, key) in standard_entries(w) {
                    out.push(format!(
                        "{pad}  {name}{}",
                        key.map_or(String::new(), |k| format!(" [{k}]"))
                    ));
                }
            }
            MenuKind::Command {
                command, shortcut, ..
            } => {
                out.push(format!(
                    "{pad}{}{}",
                    command.name,
                    shortcut
                        .as_ref()
                        .map_or(String::new(), |s| format!(" [{s}]"))
                ));
            }
            MenuKind::View(v) => out.push(format!("{pad}view:{v}")),
            MenuKind::Separator => out.push(format!("{pad}-")),
        }
        expected_outline(&n.children, depth + 1, out);
    }
}

pub(super) fn menu_from_layout(f: &dyn PlanFixture) -> R {
    let mut want = Vec::new();
    expected_outline(&f.resolved().menu, 0, &mut want);
    let got = MenuPlan::outline(&f.build().menu);
    if want == got {
        Ok(())
    } else {
        Err(format!("menu differs: {got:?}"))
    }
}

pub(super) fn bindings_ordered(f: &dyn PlanFixture) -> R {
    let plan = f.build();
    for page in &f.resolved().pages {
        for pl in &page.places {
            let Some(PlaceStatus::Ready(r)) = plan.place(&page.page, &pl.place).map(|p| &p.status)
            else {
                continue;
            };
            let want: Vec<(String, u32)> = pl
                .bindings
                .iter()
                .map(|b| (b.slot.clone(), b.nth))
                .collect();
            let got: Vec<(String, u32)> =
                r.bindings().into_iter().map(|b| (b.slot, b.nth)).collect();
            if want != got {
                return Err(format!("place {} bindings out of order", pl.place));
            }
        }
    }
    Ok(())
}

pub(super) fn style_blind(f: &dyn PlanFixture) -> R {
    let base = f.build().canonical_without_style();
    for style in ["fiori", "apple", "windows", "simple"] {
        let Some(p) = f.with_style(style) else {
            continue;
        };
        if p.canonical_without_style() != base || p.style.as_deref() != Some(style) {
            return Err(format!("style {style} changes the plan"));
        }
    }
    Ok(())
}

pub(super) fn deterministic(f: &dyn PlanFixture) -> R {
    if f.build() == f.build() {
        Ok(())
    } else {
        Err("two builds differ".to_owned())
    }
}

pub(super) fn state(f: &dyn PlanFixture) -> R {
    let plan = f.build();
    let drawable = plan
        .pages
        .iter()
        .flat_map(|p| &p.places)
        .any(|p| matches!(p.status, PlaceStatus::Ready(_)));
    let clean = plan.locks().is_empty() && plan.notices.is_empty();
    let want = if !drawable {
        PlanState::Refused
    } else if clean {
        PlanState::Ready
    } else {
        PlanState::Partial
    };
    if plan.state == want {
        Ok(())
    } else {
        Err(format!("state {:?}, expected {want:?}", plan.state))
    }
}

pub(super) fn notices(f: &dyn PlanFixture) -> R {
    let plan = f.build();
    let mut codes: Vec<String> = plan
        .locks()
        .iter()
        .map(|l| l.cause.coded().code.clone())
        .collect();
    for n in &plan.notices {
        let c = n.cause.coded();
        codes.push(c.code.clone());
        codes.extend(c.param("cause").map(str::to_owned));
    }
    for p in f.resolved().problems.iter().chain(&f.resolved().conflicts) {
        if !codes.contains(&p.code) {
            return Err(format!("{} swallowed", p.code));
        }
    }
    Ok(())
}
