//! The check of the gestures an `[[action]]` binds, against the form of the block it is at (`--check`, the starter): the command of the
//! action must be one the module binds to a channel of the form (else the control stays locked: yellow `action-channel-mismatch`),
//! every argument of the slot must be a channel of the form (red `action-arg-unknown`), and a channel the command must have but the
//! slot does not carry comes from the record (yellow `action-arg-from-record`, so it is visible). A slot without arguments needs no
//! channel (a menu command the form has no channel for): there is nothing to match. The description names NO arguments; they are the
//! slot's (`Slot::args`, the contract).
//!
//! Pattern: Specification (pure rules over the view and the form catalog), reporting as `Finding`s like `form_check::report`.

use fs_forms_layout::CatalogRules;

use crate::form_check::Finding;
use crate::records::Records;
use crate::view::{ActionView, Coded, View};

fn finding(code: &str, a: &ActionView, red: bool, extra: (&str, &str)) -> Finding {
    Finding {
        code: Coded::new(code)
            .with("trigger", a.trigger.clone())
            .with("command", a.command.clone())
            .with(extra.0, extra.1),
        red,
    }
}

/// The block of the place `elem` in the view.
fn block_at<'a>(view: &'a View, elem: &str) -> Option<&'a str> {
    view.pages
        .iter()
        .flat_map(|p| &p.places)
        .find(|p| !p.elem.is_empty() && p.elem == elem)
        .map(|p| p.block.as_str())
}

/// The findings for the actions of `view` that bind a gesture with arguments. Records that offer no form of the block say nothing.
#[must_use]
pub fn action_report(view: &View, records: &dyn Records) -> Vec<Finding> {
    let rules = crate::blocks::Blocks::rules();
    let offered = records.forms();
    let mut out = Vec::new();
    for a in &view.actions {
        let Some(block) = a.at.as_deref().and_then(|e| block_at(view, e)) else {
            continue;
        };
        let Some(args) = rules
            .command_args(block, &a.trigger)
            .filter(|x| !x.is_empty())
        else {
            continue;
        };
        let Some(data) = crate::blocks::forms_of(block)
            .iter()
            .find_map(|f| offered.iter().find(|d| d.declared.form.name == f.name()))
        else {
            continue;
        };
        let form = &data.declared.form.name;
        let Some(channel) = records.channel_of(form, &a.command) else {
            out.push(finding("action-channel-mismatch", a, false, ("form", form)));
            continue;
        };
        let Some(def) = crate::write::definition(data) else {
            continue;
        };
        for arg in args.iter().filter(|x| def.channel(x).is_none()) {
            out.push(finding("action-arg-unknown", a, true, ("arg", arg)));
        }
        let writes = def.channel(&channel).and_then(|c| c.writes.as_ref());
        for w in writes.into_iter().flatten().filter(|w| w.required) {
            if !args.contains(&w.channel) {
                out.push(finding(
                    "action-arg-from-record",
                    a,
                    false,
                    ("channel", &w.channel),
                ));
            }
        }
    }
    out
}
