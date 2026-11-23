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
/// A word the table names but bind does not know becomes `Unknown`, which no field has: the channel then stays red, never silently open.
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
mod catalog_tests {
    use super::{basic, forms};
    use fs_forms_bind::{
        Basic, ComponentForms, FormDef, FormRef, Grundtyp, Matched, ModuleForm, match_form,
    };
    use fs_interfaces::forms::{Art, catalog};

    fn fref(def: &FormDef) -> FormRef {
        FormRef {
            name: def.name.clone(),
            major: def.major,
            minor: def.minor,
        }
    }

    /// ONE table: what bind answers for a channel is exactly `Art::allowed_basics`, word for word (nothing dropped, nothing added,
    /// no word unknown to bind) for every channel of the real catalog.
    #[test]
    fn bind_and_the_catalog_answer_the_same_question_with_one_table() {
        let mut seen = 0;
        for (real, mine) in catalog().iter().zip(forms()) {
            for (r, m) in real.channels.iter().zip(&mine.channels) {
                assert_eq!(r.arts.len(), m.kinds.len());
                for (art, kind) in r.arts.iter().zip(&m.kinds) {
                    let words = art.allowed_basics();
                    assert_eq!(kind.basics.len(), words.len(), "{}.{}", real.name, r.name);
                    for (w, b) in words.iter().zip(&kind.basics) {
                        assert_ne!(*b, Basic::Unknown, "`{w}` is unknown to bind");
                        assert_eq!(basic(w), *b);
                    }
                    seen += 1;
                }
            }
        }
        assert!(seen > 30, "the whole catalog is read, {seen}");
        // every art of the catalog is covered by the catalog or by this list (a new art needs a word here)
        for art in [
            Art::Records,
            Art::Text,
            Art::Flag,
            Art::Command,
            Art::Time,
            Art::Reference,
            Art::Number,
        ] {
            assert!(
                art.allowed_basics()
                    .iter()
                    .all(|w| basic(w) != Basic::Unknown),
                "{}",
                art.name()
            );
        }
    }

    fn module_with(def: &FormDef, channel: &str, field: Basic) -> ModuleForm {
        let mut m = minimal_module(def);
        m.map.retain(|(c, _)| c != channel);
        m.map.push((channel.to_owned(), vec!["x".into()]));
        m.fields.push(("x".into(), field));
        m
    }

    fn matches(forms: &[FormDef], module: &ModuleForm) -> Matched {
        let comp = ComponentForms {
            block: "b".into(),
            forms: vec![module.form.clone()],
        };
        match_form(forms, module, &[comp], Some("b"))
    }

    /// The questions the two old tables answered DIFFERENTLY: a picture fills `tiles.preview` (reference); a reference fills a
    /// text channel (`label`); the catalog says so, the host must follow it.
    #[test]
    fn a_field_the_catalog_allows_is_never_refused_by_the_host() {
        let all = forms();
        let tiles = all.iter().find(|f| f.name == "tiles").expect("tiles");
        let green = Matched::Blocks(vec!["b".into()]);
        for image in [Basic::Image, Basic::Link, Basic::Reference] {
            assert_eq!(
                matches(&all, &module_with(tiles, "preview", image)),
                green,
                "{image:?}"
            );
        }
        assert_eq!(
            matches(&all, &module_with(tiles, "label", Basic::Reference)),
            green
        );
        // and what the catalog does not allow stays red
        assert!(matches!(
            matches(&all, &module_with(tiles, "preview", Basic::Number)),
            Matched::Refused(_)
        ));
        assert!(matches!(
            matches(&all, &module_with(tiles, "preview", Basic::Text)),
            Matched::Refused(_)
        ));
    }

    /// A module that maps exactly the required channels of a form, each to a field of a basic type the channel takes.
    fn minimal_module(def: &FormDef) -> ModuleForm {
        let mut map = Vec::new();
        let mut fields = Vec::new();
        for c in def.channels.iter().filter(|c| c.required) {
            let b = c
                .kinds
                .iter()
                .flat_map(|k| k.basics.first())
                .next()
                .copied()
                .unwrap_or(Basic::Text);
            fields.push((format!("f-{}", c.name), b));
            map.push((c.name.clone(), vec![format!("f-{}", c.name)]));
        }
        ModuleForm {
            form: fref(def),
            map,
            fields,
        }
    }

    #[test]
    fn every_form_of_the_catalog_matches_with_its_required_channels() {
        let all = forms();
        assert!(all.len() >= 6);
        for def in &all {
            let module = minimal_module(def);
            assert_eq!(
                matches(&all, &module),
                Matched::Blocks(vec!["b".into()]),
                "{}",
                def.name
            );
            for c in def.channels.iter().filter(|c| c.required) {
                let mut cut = module.clone();
                cut.map.retain(|(n, _)| *n != c.name);
                let comp = ComponentForms {
                    block: "b".into(),
                    forms: vec![fref(def)],
                };
                let Matched::Refused(list) = match_form(&all, &cut, &[comp], None) else {
                    panic!("{} without {}", def.name, c.name)
                };
                assert!(list.iter().any(|x| x.code == "form-channel-unmapped"
                    && x.param("channel") == Some(c.name.as_str())));
            }
        }
    }

    #[test]
    fn records_and_command_channels_carry_no_field_types() {
        for def in forms() {
            for k in def.channels.iter().flat_map(|c| &c.kinds) {
                let names_no_field = matches!(k.grundtyp, Grundtyp::Records | Grundtyp::Command);
                assert_eq!(k.basics.is_empty(), names_no_field, "{}", def.name);
            }
        }
    }
}
