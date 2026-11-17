//! Rule checks `plan.commands` and `plan.menu-from-layout`.

use super::PlanFixture;
use super::rules::{R, plan_commands};
use crate::codes::Severity;
use crate::menu::MenuPlan;
use crate::model::{MenuIn, MenuKind, Target};

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
                for e in crate::testing::standard().entries(w) {
                    let k = e
                        .shortcut
                        .as_ref()
                        .map_or(String::new(), |k| format!(" [{k}]"));
                    out.push(format!("{pad}  {}{k}", e.name));
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
                        .and_then(|s| fs_components_commands::Shortcut::parse(s).ok())
                        .map_or(String::new(), |s| format!(" [{}]", s.aria()))
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
    let got = MenuPlan::outline(&f.build().menu, &crate::testing::standard());
    if want == got {
        Ok(())
    } else {
        Err(format!("menu differs: {got:?}"))
    }
}
