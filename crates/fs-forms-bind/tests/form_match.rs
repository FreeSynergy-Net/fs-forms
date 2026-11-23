//! The G7 matcher against the draft form catalog (`tests/data/forms.txt`, the file of order SQ-1b) and its suite.
#![cfg(feature = "conformance")]
use fs_forms_bind::form_fixtures::{form_ref, parse_forms_txt};
use fs_forms_bind::suite::FormsSuite;
use fs_forms_bind::*;

const TXT: &str = include_str!("data/forms.txt");

#[test]
fn the_suite_proves_itself_nine_of_nine() {
    let (base_green, caught) = FormsSuite::prove();
    assert!(
        base_green,
        "{:?}",
        FormsSuite::run(&|f, m, c| match_form(f, m, c, None))
    );
    let mut want = FormsSuite::rules();
    want.sort_unstable();
    let mut got = caught;
    got.sort_unstable();
    assert_eq!(got, want, "every rule has its mutant red");
    assert_eq!(want.len(), 9);
}

#[test]
fn every_rule_has_exactly_one_mutant() {
    let mut flaws: Vec<String> = FormsSuite::mutants()
        .iter()
        .map(|m| format!("{:?}", m.0))
        .collect();
    flaws.sort_unstable();
    flaws.dedup();
    assert_eq!(flaws.len(), 9);
}

#[test]
fn the_draft_catalog_reads_with_all_channels() {
    let forms = parse_forms_txt(TXT).expect("reads");
    let names: Vec<&str> = forms.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "checklist",
            "cards-in-columns",
            "bars-in-time",
            "tiles",
            "tree",
            "members"
        ]
    );
    let count = |n: &str| {
        forms
            .iter()
            .find(|f| f.name == n)
            .map_or(0, |f| f.channels.len())
    };
    assert_eq!(
        [
            count("checklist"),
            count("cards-in-columns"),
            count("bars-in-time"),
            count("tiles")
        ],
        [4, 6, 10, 9]
    );
    assert_eq!([count("tree"), count("members")], [5, 4]);
    assert!(
        forms
            .iter()
            .flat_map(|f| &f.channels)
            .all(|c| !c.name.is_empty())
    );
}

/// A module that maps exactly the required channels of a form, each to a field of the right basic type.
fn minimal_module(def: &FormDef) -> ModuleForm {
    let mut map = Vec::new();
    let mut fields = Vec::new();
    for c in def.channels.iter().filter(|c| c.required) {
        let basic = match c.grundtypen[0] {
            Grundtyp::Time => Basic::DateTime,
            Grundtyp::Number => Basic::Number,
            Grundtyp::Flag => Basic::Bool,
            Grundtyp::Reference => Basic::Reference,
            _ => Basic::Text,
        };
        fields.push((format!("f-{}", c.name), basic));
        map.push((c.name.clone(), vec![format!("f-{}", c.name)]));
    }
    ModuleForm {
        form: form_ref(&def.name, def.major, def.minor),
        map,
        fields,
    }
}

#[test]
fn every_form_of_the_catalog_matches_with_its_required_channels() {
    let forms = parse_forms_txt(TXT).expect("reads");
    for def in &forms {
        let module = minimal_module(def);
        let comp = ComponentForms {
            block: "b".into(),
            forms: vec![form_ref(&def.name, def.major, def.minor)],
        };
        assert_eq!(
            match_form(&forms, &module, &[comp], Some("b")),
            Matched::Blocks(vec!["b".into()]),
            "{}",
            def.name
        );
        // Drop each required channel in turn: red, with that channel named.
        for c in def.channels.iter().filter(|c| c.required) {
            let mut cut = module.clone();
            cut.map.retain(|(n, _)| *n != c.name);
            let comp = ComponentForms {
                block: "b".into(),
                forms: vec![form_ref(&def.name, def.major, def.minor)],
            };
            let Matched::Refused(list) = match_form(&forms, &cut, &[comp], None) else {
                panic!("{} without {}", def.name, c.name)
            };
            assert!(list.iter().any(|x| x.code == "form-channel-unmapped"
                && x.param("channel") == Some(c.name.as_str())));
        }
    }
}

#[test]
fn the_frontend_names_the_block_and_a_block_without_the_form_is_not_taken() {
    let forms = parse_forms_txt(TXT).expect("reads");
    let tree = forms.iter().find(|f| f.name == "tree").expect("tree");
    let module = minimal_module(tree);
    let comps = [
        ComponentForms {
            block: "tree".into(),
            forms: vec![form_ref("tree", 1, 0)],
        },
        ComponentForms {
            block: "tree2".into(),
            forms: vec![form_ref("tree", 1, 4)],
        },
    ];
    assert_eq!(
        match_form(&forms, &module, &comps, None),
        Matched::Blocks(vec!["tree".into(), "tree2".into()])
    );
    assert_eq!(
        match_form(&forms, &module, &comps, Some("tree2")),
        Matched::Blocks(vec!["tree2".into()])
    );
    assert!(matches!(
        match_form(&forms, &module, &comps, Some("table")),
        Matched::Fallback(_)
    ));
}
