//! The data of one block with lanes: the cards of its own source, and, when a binding names a second source for the heads of a grouping,
//! the rows of that source as the lanes (`LanePlan`: off, missing, on). Data goes to the block by (source, form, type), so the lane
//! list `rows@1.0.0` of type `Lane` is never taken for the `rows@1.0.0` of type `Task`.
//!
//! A lane list that cannot be used (not installed, scope unknown, no entry for the type, rows that do not fit the form, the slot shut by
//! the plan) is never silent: the result is `LanePlan::Missing` and the codes that say why (`host-notice-lanes-missing` first); the
//! block with its columns is still drawn.
//!
//! The key of a lane is the IRI of the head (the key of its row). The cards of a module carry the NAME of the assignee for now: a lane
//! then takes the name as its key (`align`), until the cards carry IRIs; the card is never lost, a value without a head gets a lane at the
//! end (the component's `lanes_of`).
//!
//! Pattern: Adapter (rows of two sources in, the input of the cards, the heads and the lane plan out); the checks are Specifications.

use std::collections::HashSet;

use fs_components_core::form_read::{FormInput, HeadRow, Heads};
use fs_components_kanban::{LaneHead, LanePlan};
use fs_forms_bind::SlotRole;

use crate::feed::{Feed, HeadsRef};
use crate::form_data::{FormData, empty_input, input_of};
use crate::records::Records;
use crate::view::{Coded, View};

/// What one block is given: the cards, the rows of the heads for the lanes slot, the plan for the board, the words for the notice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placed {
    /// The rows of the cards (the block's own source).
    pub input: FormInput,
    /// The rows of the heads (the source of the binding), when they are usable.
    pub heads: Option<FormInput>,
    /// What the board is told about lanes.
    pub plan: LanePlan,
    /// The codes the window shows as a `partial-notice` (empty = nothing to say).
    pub notices: Vec<Coded>,
}

/// The data for (`source`, `form`, `data_type`): the one declared for exactly that, else one that does not say (source 0, no type: old
/// callers). `None` = the module offers nothing for it.
#[must_use]
pub fn pick<'a>(
    forms: &'a [FormData],
    form: &str,
    source: u32,
    data_type: &str,
) -> Option<&'a FormData> {
    forms
        .iter()
        .filter(|d| d.declared.form.name == form)
        .filter(|d| source == 0 || d.source == 0 || d.source == source)
        .filter(|d| data_type.is_empty() || d.data_type.is_empty() || d.data_type == data_type)
        .min_by_key(|d| u8::from(d.source != source) + u8::from(d.data_type != data_type))
}

/// The codes of a lock of the plan that are about the heads (a lock of another slot says nothing about lanes): the producer set the
/// slot role, the code text is never read.
fn about_heads(c: &Coded) -> bool {
    c.slot_role == Some(SlotRole::Heads)
}

/// The form the heads of the block come as: the first the slot of the catalog names (`heads=` of the contract). A block whose catalog
/// names none is an error (`host-notice-lanes-form-undeclared`), never a default form: the binding asks for something the block cannot take.
fn heads_form(block: &str) -> Result<String, Coded> {
    crate::bind_spec::block_spec(block)
        .and_then(|b| b.slots.into_iter().find(|s| !s.heads.is_empty()))
        .and_then(|s| s.heads.into_iter().next())
        .map(|f| f.name)
        .ok_or_else(|| Coded::new("host-notice-lanes-form-undeclared").with("block", block))
}

/// The heads in the order the module gave them; the key of a head is its IRI unless the cards carry names (then all keys are names).
fn align(heads: &FormInput, cards: &FormInput, grouping: &str) -> Vec<LaneHead> {
    let used: HashSet<&str> = cards
        .rows
        .iter()
        .flat_map(|r| r.all(grouping))
        .map(String::as_str)
        .collect();
    let label =
        |r: &fs_components_core::form_read::FormRow| r.get("label").unwrap_or(&r.key).to_owned();
    // ONE decision for the whole list (an empty lane has no card to say which it is): the cards carry names, not IRIs
    let by_name = !heads.rows.iter().any(|r| used.contains(r.key.as_str()))
        && heads.rows.iter().any(|r| used.contains(label(r).as_str()));
    heads
        .rows
        .iter()
        .map(|r| {
            let name = label(r);
            LaneHead::new(if by_name { &name } else { &r.key }, &name)
        })
        .collect()
}

/// Why the heads cannot be used, else the rows of the heads.
fn heads_data<'a>(
    records: &dyn Records,
    forms: &'a [FormData],
    feed: &Feed,
    h: &HeadsRef,
    form: &str,
) -> Result<&'a FormData, Vec<Coded>> {
    let shut: Vec<Coded> = feed
        .slot_locks
        .iter()
        .filter(|c| about_heads(c))
        .cloned()
        .collect();
    let read: Vec<Coded> = records
        .source_problems()
        .into_iter()
        .filter(|(n, _)| *n == h.source)
        .map(|(_, c)| c.for_slot(SlotRole::Heads))
        .collect();
    if !shut.is_empty() || !read.is_empty() {
        return Err(shut.into_iter().chain(read).collect());
    }
    if forms.iter().all(|d| d.source != h.source) {
        return Err(vec![
            Coded::new("host-notice-lanes-source-absent").with("source", h.source.to_string()),
        ]);
    }
    let Some(data) = forms.iter().find(|d| {
        d.source == h.source && d.declared.form.name == form && d.data_type == h.data_type
    }) else {
        return Err(vec![
            Coded::new("bind-error-heads-form-missing")
                .for_slot(SlotRole::Heads)
                .with("form", form)
                .with("type", h.data_type.clone()),
        ]);
    };
    let misfit = crate::form_check::misfit(data);
    if misfit.is_empty() {
        Ok(data)
    } else {
        Err(misfit)
    }
}

/// The cards with the lanes the board is told: `Off` = none asked, `Missing` = asked, not usable, `On` = the heads `align` matched (key and
/// label as the cards carry them). The one place where the plan becomes the input of the component.
fn with_heads(mut input: FormInput, plan: &LanePlan) -> FormInput {
    input.heads = match plan {
        LanePlan::Off => None,
        LanePlan::Missing => Some(Heads::Missing),
        LanePlan::On { heads, .. } => Some(Heads::Rows(
            heads
                .iter()
                .map(|h| HeadRow {
                    key: h.key.clone(),
                    label: h.label.clone(),
                })
                .collect(),
        )),
    };
    input
}

/// The data of the block `block`: its cards and, when its feed names heads, the lanes.
#[must_use]
pub fn placed(records: &dyn Records, block: &str, feeds: &[Feed]) -> Placed {
    let name = crate::blocks::forms_of(block)
        .first()
        .map(|f| f.name().to_owned())
        .unwrap_or_default();
    let forms = records.forms();
    let feed = feeds.iter().find(|f| f.block == block);
    let (source, data_type) = feed.map_or((0, ""), |f| (f.source, f.data_type.as_str()));
    let input = pick(&forms, &name, source, data_type).map_or_else(|| empty_input(&name), input_of);
    let (Some(feed), Some(h)) = (feed, feed.and_then(|f| f.heads.as_ref())) else {
        return Placed {
            input,
            heads: None,
            plan: LanePlan::Off,
            notices: Vec::new(),
        };
    };
    let heads = heads_form(block)
        .map_err(|c| vec![c])
        .and_then(|form| heads_data(records, &forms, feed, h, &form));
    match heads {
        Ok(data) => {
            let heads = input_of(data);
            let plan = LanePlan::On {
                grouping: h.grouping.clone(),
                heads: align(&heads, &input, &h.grouping),
            };
            Placed {
                input: with_heads(input, &plan),
                heads: Some(heads),
                plan,
                notices: Vec::new(),
            }
        }
        Err(codes) => Placed {
            input: with_heads(input, &LanePlan::Missing),
            heads: None,
            plan: LanePlan::Missing,
            notices: std::iter::once(Coded::new("host-notice-lanes-missing"))
                .chain(codes)
                .collect(),
        },
    }
}

/// Puts what the lanes have to say into the window: the codes of every block with a feed of heads become locks of single slots of its
/// place (the block is still drawn; the window draws a `partial-notice`). True when a place got one.
pub(crate) fn show_notices(view: &mut View, records: &dyn Records) -> bool {
    let feeds = view.feeds_now();
    let mut any = false;
    for place in view.pages.iter_mut().flat_map(|p| p.places.iter_mut()) {
        for c in placed(records, &place.block, &feeds).notices {
            if !place.slot_locks.contains(&c) {
                place.slot_locks.push(c);
                any = true;
            }
        }
    }
    any
}
