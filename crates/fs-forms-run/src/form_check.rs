//! Which form each block draws, and the one match of module and component: the catalog of forms (`fs_interfaces::forms`, forms.txt)
//! is read here, the rules are `fs_forms_bind::match_form` (no second checker). A red result becomes a lock of the place with its
//! code (`form-channel-unmapped` ...); the host never draws a block whose module does not fit.
//!
//! Pattern: Adapter (the catalog and the records in, the locks of the view out); the table block to form is an assignment, not a decision.

use fs_forms_bind::{
    Channel, ComponentForms, Count, FormDef, FormRef, Grundtyp, Matched, match_form,
};
use fs_interfaces::forms::{Art, Count as Cnt, catalog};

use crate::records::Records;
use crate::view::{Coded, View};

/// The form each block of this build draws (name and version as the component declares them): `tree` draws `tree@1.0`, `table`
/// the member list, `board` the cards in columns.
pub(crate) const BLOCK_FORMS: &[(&str, &str, u32, u32)] = &[
    ("tree", "tree", 1, 0),
    ("table", "members", 1, 0),
    ("board", "cards-in-columns", 1, 0),
];

/// The name of the form a block draws.
pub(crate) fn form_of(block: &str) -> Option<&'static str> {
    BLOCK_FORMS
        .iter()
        .find(|(b, ..)| *b == block)
        .map(|(_, f, ..)| *f)
}

fn grundtyp(art: Art) -> Grundtyp {
    match art {
        Art::Records => Grundtyp::Records,
        Art::Text => Grundtyp::Text,
        Art::Flag => Grundtyp::Flag,
        Art::Command => Grundtyp::Command,
        Art::Time => Grundtyp::Time,
        Art::Reference => Grundtyp::Reference,
        Art::Number => Grundtyp::Number,
    }
}

fn count(c: Cnt) -> Count {
    match c {
        Cnt::Range {
            min: 1,
            max: Some(1),
        } => Count::One,
        Cnt::Range {
            min: 0,
            max: Some(1),
        } => Count::ZeroOrOne,
        Cnt::Range { max, .. } => Count::ZeroToN(max),
        Cnt::Open => Count::ZeroToN(None),
    }
}

/// The form catalog as the match reads it.
fn forms() -> Vec<FormDef> {
    catalog()
        .iter()
        .map(|f| FormDef {
            name: f.name.clone(),
            major: f.version.major,
            minor: f.version.minor,
            channels: f
                .channels
                .iter()
                .map(|c| Channel {
                    name: c.name.clone(),
                    grundtyp: grundtyp(c.art),
                    required: c.required,
                    count: count(c.count),
                })
                .collect(),
        })
        .collect()
}

fn components() -> Vec<ComponentForms> {
    BLOCK_FORMS
        .iter()
        .map(|(block, name, major, minor)| ComponentForms {
            block: (*block).to_owned(),
            forms: vec![FormRef {
                name: (*name).to_owned(),
                major: *major,
                minor: *minor,
            }],
        })
        .collect()
}

fn coded(c: &fs_forms_bind::Coded) -> Coded {
    c.params
        .iter()
        .fold(Coded::new(c.code.clone()), |out, (k, v)| out.with(k, v))
}

/// The codes the match says for the form `block` draws, when the connected module offers it; empty = fits (or nothing is offered:
/// then the block is drawn empty by the existing notices). Yellow results (a fallback) lock nothing.
pub fn problems(block: &str, records: &dyn Records) -> Vec<Coded> {
    let Some(form) = form_of(block) else {
        return Vec::new();
    };
    let Some(data) = records
        .forms()
        .iter()
        .find(|d| d.declared.form.name == form)
    else {
        return Vec::new();
    };
    match match_form(&forms(), &data.declared, &components(), Some(block)) {
        Matched::Refused(codes) => codes.iter().map(coded).collect(),
        _ => Vec::new(),
    }
}

/// Locks every place whose block does not fit the form of the module (the lock carries the codes); true when one was locked.
pub(crate) fn lock_misfits(view: &mut View, records: &dyn Records) -> bool {
    let mut any = false;
    for place in view.pages.iter_mut().flat_map(|p| p.places.iter_mut()) {
        let codes = problems(&place.block, records);
        any |= !codes.is_empty();
        place.locks.extend(codes);
    }
    any
}
