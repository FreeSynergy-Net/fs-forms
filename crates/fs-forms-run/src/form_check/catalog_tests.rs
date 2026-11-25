//! The match against the REAL form catalog (`fs-interfaces`): one table of field types, the old two-table disagreements, every form.
//!
//! Pattern: Specification tests over the real catalog (no fixture table).

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
            .find_map(|k| k.basics.first())
            .copied()
            .unwrap_or(Basic::Text);
        fields.push((format!("f-{}", c.name), b));
        map.push((c.name.clone(), vec![format!("f-{}", c.name)]));
    }
    ModuleForm {
        data_type: String::new(),
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

/// r1 MUSS 2: a block that needs `cards-in-columns@^1.0.0` draws up to the HIGHEST version of the catalog (`1.1.0`, after `1.0.0`
/// in the file): the module of the fixture (`1.1.0`) fits and the board is not locked. Mutant: first version of the name (`find`)
/// instead of the highest = `form-version-mismatch`.
#[test]
fn a_module_with_the_highest_version_of_the_form_does_not_lock_the_board() {
    let board = crate::blocks::forms_of_blocks()
        .into_iter()
        .find(|(_, forms)| forms.iter().any(|f| f.name() == "cards-in-columns"))
        .map(|(block, _)| block)
        .expect("a block draws cards-in-columns");
    let w = std::sync::Arc::new(crate::write::fixture::FakeWriter::default());
    let records = crate::write::fixture::records(&w, crate::write::generated::data());
    assert_eq!(super::problems(&board, &records), Vec::new());
    let ours = super::components();
    let drawn = ours
        .iter()
        .find(|c| c.block == board)
        .and_then(|c| c.forms.iter().find(|f| f.name == "cards-in-columns"))
        .expect("the component is listed");
    assert!(
        (drawn.major, drawn.minor) >= (1, 1),
        "{}.{}",
        drawn.major,
        drawn.minor
    );
}
