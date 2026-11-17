//! Commands: the static part (`command_plan`, in `build`) and the per-record decision (`command_state`).
//!
//! Pattern: Specification. ONE function decides for menu, context menu, button and key (H-K5).

use crate::codes::{BindCode, Part, PartLock, Severity};
use crate::model::{CommandIn, Target};
use crate::plan::CommandPlan;
use crate::providers::HostCaps;
use crate::types::RecordView;

/// How one command appears for the current selection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Shown {
    /// Not in `RecordView.may`: not shown, its key does nothing, no code.
    Hidden,
    /// `supported = false`: shown disabled, "kommt".
    Coming,
    /// Shown disabled with the reason.
    Locked(PartLock),
    /// Runs.
    Enabled,
}

fn part(c: &CommandIn) -> Part {
    Part::Command {
        source: c.source,
        name: c.name.clone(),
    }
}

/// The static decision of a command. `source_lock` is the lock of its source (same cause, visible).
#[must_use]
pub fn command_plan(c: &CommandIn, host: HostCaps, source_lock: Option<&PartLock>) -> CommandPlan {
    let fixed = if !c.desc.supported {
        Some(PartLock {
            part: part(c),
            severity: Severity::Coming,
            cause: crate::codes::Cause::from_coded(
                BindCode::Coming.coded().with("command", c.name.clone()),
            ),
        })
    } else if let Some(l) = source_lock {
        Some(PartLock {
            part: part(c),
            severity: Severity::Locked,
            cause: l.cause.clone(),
        })
    } else if matches!(c.target, Target::Workflow(_)) && !host.workflow {
        let wf = if let Target::Workflow(w) = &c.target {
            w.clone()
        } else {
            String::new()
        };
        let code = BindCode::WorkflowRuntimeMissing
            .coded()
            .with("action", c.name.clone())
            .with("workflow", wf);
        Some(PartLock::locked(part(c), code))
    } else if matches!(c.target, Target::Program) && c.desc.writes && !host.write {
        Some(PartLock::locked(
            part(c),
            BindCode::WriteLater.coded().with("command", c.name.clone()),
        ))
    } else {
        None
    };
    CommandPlan {
        source: c.source,
        name: c.name.clone(),
        description: c.desc.clone(),
        on_record: c.desc.on_record,
        fixed,
        target: c.target.clone(),
    }
}

/// Decides how a command appears. Order (decided c0, question 85: the RIGHT comes first): (1) a record command with a selection that
/// is not in `may` is `Hidden`, even when it is "coming" (hiding by right is rights management, not a silent fault); (2) unsupported
/// -> `Coming`; (3) record command without selection -> `Locked(no-selection)`; (4) other fixed lock -> `Locked`; (5) `Enabled`.
/// `may` names rights only, independent of `supported` (provider side).
#[must_use]
pub fn command_state(cmd: &CommandPlan, selection: Option<&RecordView>) -> Shown {
    if cmd.on_record
        && let Some(r) = selection
        && !r.may.iter().any(|m| *m == cmd.name)
    {
        return Shown::Hidden;
    }
    if let Some(l) = &cmd.fixed
        && l.severity == Severity::Coming
    {
        return Shown::Coming;
    }
    if cmd.on_record && selection.is_none() {
        let c = BindCode::NoSelection
            .coded()
            .with("command", cmd.name.clone());
        return Shown::Locked(PartLock::locked(
            Part::Command {
                source: cmd.source,
                name: cmd.name.clone(),
            },
            c,
        ));
    }
    match &cmd.fixed {
        Some(l) => Shown::Locked(l.clone()),
        None => Shown::Enabled,
    }
}

/// The id the window uses for a command (menu, keymap): `<source>/<name>`; stable across rebuilds.
#[must_use]
pub fn command_id(cmd: &CommandPlan) -> String {
    format!("{}/{}", cmd.source, cmd.name)
}
