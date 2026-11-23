//! Which form each block draws, and the one match of module and component: the catalog of forms (`fs_interfaces::forms`, forms.txt)
//! is read here, the rules are `fs_forms_bind::match_form` (no second checker). A red result becomes a lock of the place with its
//! code (`form-channel-unmapped` ...); the host never draws a block whose module does not fit.
//!
//! Pattern: Adapter (the catalog and the records in, the locks of the view out); the table block to form is an assignment, not a decision.

use fs_forms_bind::{
    Basic, Channel, ComponentForms, Count, FormDef, FormRef, Grundtyp, Kind, Matched, match_form,
};
use fs_interfaces::forms::{Art, Count as Cnt, catalog};

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
            major: f.version.major,
            minor: f.version.minor,
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

/// What each block draws, as the COMPONENT declares it (`forms=` of the block contract, read by the catalog): the host holds no
/// table of its own.
fn components() -> Vec<ComponentForms> {
    crate::blocks::forms_of_blocks()
        .into_iter()
        .map(|(block, forms)| ComponentForms {
            block,
            forms: forms
                .into_iter()
                .map(|f| FormRef {
                    name: f.name,
                    major: f.major,
                    minor: f.minor,
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
    let Some(data) = records
        .forms()
        .iter()
        .find(|d| drawn.iter().any(|f| f.name == d.declared.form.name))
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
    records
        .forms()
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
