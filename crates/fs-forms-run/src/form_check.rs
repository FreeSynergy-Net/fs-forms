//! Which form each block draws, and the one match of module and component: the catalog of forms (`fs_interfaces::forms`, forms.txt)
//! is read here, the rules are `fs_forms_bind::match_form` (no second checker). A red result becomes a lock of the place with its
//! code (`form-channel-unmapped` ...); the host never draws a block whose module does not fit.
//!
//! Pattern: Adapter (the catalog and the records in, the locks of the view out); the table block to form is an assignment, not a decision.

use fs_forms_bind::{
    Basic, Channel, ComponentForms, Count, FormDef, FormRef, Grundtyp, Kind, Matched, match_form,
};
use fs_interfaces::forms::{Art, Count as Cnt, catalog};
use fs_types::interface::InterfaceVersion;

use crate::records::Records;
use crate::view::{Coded, View};

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

/// The field types of a channel of this art: the ONE table is `Art::allowed_basics` of `fs-interfaces` (bind has none of its own).
/// A word the table names but bind does not know becomes `Basic::Unknown`. That is a field type like any other (a field of that type would fit); it is kept out only by the equality test against the real table (no `Unknown` there) and by the starter, which reads an unknown field type as an error.
fn kind(art: Art) -> Kind {
    Kind {
        grundtyp: grundtyp(art),
        basics: art.allowed_basics().iter().map(|w| basic(w)).collect(),
    }
}

fn basic(word: &str) -> Basic {
    match word {
        "Text" => Basic::Text,
        "Email" => Basic::Email,
        "Number" => Basic::Number,
        "Bool" => Basic::Bool,
        "Date" => Basic::Date,
        "DateTime" => Basic::DateTime,
        "Choice" => Basic::Choice,
        "Reference" => Basic::Reference,
        "Keywords" => Basic::Keywords,
        "Link" => Basic::Link,
        "Image" => Basic::Image,
        _ => Basic::Unknown,
    }
}

fn count(c: Cnt) -> Count {
    Count {
        min: c.min,
        max: c.max,
    }
}

/// The form catalog as the match reads it.
fn forms() -> Vec<FormDef> {
    catalog()
        .iter()
        .map(|f| FormDef {
            name: f.name.clone(),
            major: small(f.version.major),
            minor: small(f.version.minor),
            channels: f
                .channels
                .iter()
                .map(|c| Channel {
                    name: c.name.clone(),
                    kinds: c.arts.iter().copied().map(kind).collect(),
                    required: c.required,
                    count: count(c.count),
                })
                .collect(),
        })
        .collect()
}

/// A number of a form version as bind reads it (`u32`); a version of that size does not exist, the largest value then never fits.
fn small(n: u64) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// What each block draws, as the COMPONENT declares it (`forms=` of the block contract, read by the catalog): the host holds no
/// table of its own. A component declares a NEED (`tree@^1.0.0`), bind compares the version up to which the component draws: that is
/// the version of the catalog when the need draws it, else `0.0` (the form is named, but no version of the catalog is drawn: the match
/// says `form-version-mismatch`). A form the catalog does not know is not listed.
fn components() -> Vec<ComponentForms> {
    let catalog = catalog();
    crate::blocks::forms_of_blocks()
        .into_iter()
        .map(|(block, forms)| ComponentForms {
            block,
            forms: forms
                .iter()
                .filter_map(|need| {
                    let form = catalog.iter().find(|f| f.name == need.name())?;
                    let offered: InterfaceVersion =
                        format!("{}@{}", form.name, form.version).parse().ok()?;
                    let (major, minor) = if need.draws(&offered) {
                        (small(form.version.major), small(form.version.minor))
                    } else {
                        (0, 0)
                    };
                    Some(FormRef {
                        name: form.name.clone(),
                        major,
                        minor,
                    })
                })
                .collect(),
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
    // the forms the block draws, from the component; the module offers one of them or there is nothing to match
    let drawn = crate::blocks::forms_of(block);
    let forms_of_module = records.forms();
    let Some(data) = forms_of_module
        .iter()
        .find(|d| drawn.iter().any(|f| f.name() == d.declared.form.name))
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

/// One result of the match for `--check`: the code and whether it is red (a block locked, exit 3) or yellow (a message, exit 0).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    /// The code with the form it is about.
    pub code: Coded,
    /// Red = the module does not fit (rc 3); yellow = a fallback shows the form.
    pub red: bool,
}

/// The match of every form the module declares (`records.forms()`), by the SAME function the window uses (`match_form`, no second
/// checker): the findings for `--check`. Green forms give none. The block is not named here (any block of the component catalog).
#[must_use]
pub fn report(records: &dyn Records) -> Vec<Finding> {
    let (catalog, components) = (forms(), components());
    let offered = records.forms();
    offered
        .iter()
        .flat_map(|d| {
            let said = |c: &fs_forms_bind::Coded, red| Finding {
                code: coded(c),
                red,
            };
            match match_form(&catalog, &d.declared, &components, None) {
                Matched::Blocks(_) => Vec::new(),
                Matched::Fallback(c) => vec![said(&c, false)],
                Matched::Refused(codes) => codes.iter().map(|c| said(c, true)).collect(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{count, forms};
    use fs_interfaces::forms::catalog;

    #[test]
    fn the_counts_of_the_real_catalog_allow_the_same_numbers_after_the_conversion() {
        let mut seen = 0;
        for (real, mine) in catalog().iter().zip(forms()) {
            assert_eq!(real.name, mine.name);
            for (r, m) in real.channels.iter().zip(&mine.channels) {
                for n in 0..7 {
                    assert_eq!(
                        count(r.count).allows(n),
                        r.count.allows(n),
                        "{}.{} with {n}",
                        real.name,
                        r.name
                    );
                    seen += 1;
                }
                assert_eq!(m.required, r.required);
            }
        }
        assert!(seen > 100, "the whole catalog is read, {seen}");
    }
}

#[cfg(test)]
mod catalog_tests;

/// The channel of the form `cards-in-columns` a gesture of the description is for (the table of the schnitt: a drop moves, the quick
/// row and a sent form create); the other triggers (`button`, `menu`, `key`) call any command and are not checked.
fn channel_of(trigger: &str) -> Option<&'static str> {
    match trigger {
        "drop" => Some("move"),
        "quick-add" | "submit" => Some("create"),
        _ => None,
    }
}

/// The actions of the description whose command is not the one the module binds to the channel their gesture is for: the warning
/// `action-channel-mismatch{trigger, command}` (yellow, nothing is locked here: the control simply stays locked, because the key
/// is not what the module bound). Only for records that bind commands at all (a module without writing says nothing).
#[must_use]
pub fn action_report(actions: &[crate::view::ActionView], records: &dyn Records) -> Vec<Finding> {
    let bound = |ch: &str| records.command_of("cards-in-columns", ch);
    actions
        .iter()
        .filter_map(|a| {
            let channel = channel_of(&a.trigger)?;
            let key = bound(channel)?;
            (key != a.command).then(|| Finding {
                code: Coded::new("action-channel-mismatch")
                    .with("trigger", a.trigger.clone())
                    .with("command", a.command.clone()),
                red: false,
            })
        })
        .collect()
}
