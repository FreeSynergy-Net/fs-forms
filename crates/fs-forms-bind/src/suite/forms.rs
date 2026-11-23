//! `FormsSuite` (feature `conformance`): the rules of the form match, each proven by one mutant (the matcher with that rule off).
//!
//! Rules: `form.unknown`, `form.version-mismatch`, `form.channel-unmapped`, `form.basic-mismatch`, `form.no-component`, and the
//! negative rule `form.optional-free` (an optional channel left out stays green). Same shape as [`super::PlanSuite`]:
//! every rule has exactly one mutant, `prove` needs 6 of 6 and a green base.
//!
//! Pattern: Specification suite + mutants.

use crate::form_fixtures::{form_ref, parse_forms_txt, tree_component, tree_module};
use crate::form_match::{FormFlaw, Matched, match_form, match_form_flawed};
use crate::forms::{ComponentForms, FormDef, ModuleForm};
use crate::types::Basic;

/// The fixture catalog: `tree@1.0` and `tree@1.1` (one optional channel more).
const CATALOG: &str = "\
tree@1.0 | Tree | planned | t\n\
#   nodes      | Art: records   | Grundtyp: records | Pflicht: ja (`!`) | Anzahl: offen\n\
#   parent     | Art: reference | Grundtyp: reference | Pflicht: ja (`!`) | Anzahl: offen\n\
#   label      | Art: text      | Grundtyp: text | Pflicht: ja (`!`) | Anzahl: offen\n\
#   edge-kind  | Art: grouping  | Grundtyp: text | Pflicht: nein | Anzahl: offen\n\
tree@1.1 | Tree | planned | t\n\
#   nodes      | Art: records   | Grundtyp: records | Pflicht: ja (`!`) | Anzahl: offen\n\
#   parent     | Art: reference | Grundtyp: reference | Pflicht: ja (`!`) | Anzahl: offen\n\
#   label      | Art: text      | Grundtyp: text | Pflicht: ja (`!`) | Anzahl: offen\n\
#   mark       | Art: flag      | Grundtyp: flag | Pflicht: nein | Anzahl: 0 oder 1\n";

/// The matcher a rule runs against (the real one, or one with a flaw).
type Matcher<'a> = &'a dyn Fn(&[FormDef], &ModuleForm, &[ComponentForms]) -> Matched;

/// One rule: `Ok` when the matcher behaves as the rule demands.
type Check = fn(Matcher<'_>, &[FormDef]) -> Result<(), String>;

/// A rule is held when the matcher says red with this code.
fn red_with(m: &Matched, code: &str) -> Result<(), String> {
    match m {
        Matched::Refused(list) if list.iter().any(|c| c.code == code) => Ok(()),
        other => Err(format!("expected red `{code}`, got {other:?}")),
    }
}

fn unknown(m: Matcher<'_>, f: &[FormDef]) -> Result<(), String> {
    let mut new_main = tree_module();
    new_main.form = form_ref("tree", 2, 0);
    red_with(&m(f, &new_main, &[tree_component()]), "form-unknown")?;
    let mut other_name = tree_module();
    other_name.form = form_ref("treee", 1, 0);
    red_with(&m(f, &other_name, &[tree_component()]), "form-unknown")
}

fn version(m: Matcher<'_>, f: &[FormDef]) -> Result<(), String> {
    let old_main = ComponentForms {
        block: "tree".into(),
        forms: vec![form_ref("tree", 0, 9)],
    };
    red_with(&m(f, &tree_module(), &[old_main]), "form-version-mismatch")?;
    let mut newer = tree_module();
    newer.form = form_ref("tree", 1, 1);
    red_with(&m(f, &newer, &[tree_component()]), "form-version-mismatch")?;
    let component_newer = ComponentForms {
        block: "tree".into(),
        forms: vec![form_ref("tree", 1, 1)],
    };
    match m(f, &tree_module(), &[component_newer]) {
        Matched::Blocks(b) if b == ["tree"] => Ok(()),
        other => Err(format!(
            "a newer minor of the component must fit, got {other:?}"
        )),
    }
}

fn unmapped(m: Matcher<'_>, f: &[FormDef]) -> Result<(), String> {
    for channel in ["nodes", "parent", "label"] {
        let mut module = tree_module();
        module.map.retain(|(c, _)| c != channel);
        red_with(&m(f, &module, &[tree_component()]), "form-channel-unmapped")
            .map_err(|e| format!("{channel}: {e}"))?;
    }
    Ok(())
}

fn basic(m: Matcher<'_>, f: &[FormDef]) -> Result<(), String> {
    let mut module = tree_module();
    module.fields.push(("count".into(), Basic::Number));
    module.map.retain(|(c, _)| c != "label");
    module.map.push(("label".into(), vec!["count".into()]));
    red_with(&m(f, &module, &[tree_component()]), "form-basic-mismatch")?;
    let mut unlisted = tree_module();
    unlisted.map.retain(|(c, _)| c != "label");
    unlisted
        .map
        .push(("label".into(), vec!["no-such-field".into()]));
    red_with(&m(f, &unlisted, &[tree_component()]), "form-basic-mismatch")
}

fn no_component(m: Matcher<'_>, f: &[FormDef]) -> Result<(), String> {
    for none in [
        Vec::new(),
        vec![ComponentForms {
            block: "board".into(),
            forms: vec![form_ref("cards-in-columns", 1, 0)],
        }],
    ] {
        match m(f, &tree_module(), &none) {
            Matched::Fallback(c) if c.code == "form-no-component" => {}
            other => {
                return Err(format!(
                    "expected yellow `form-no-component`, got {other:?}"
                ));
            }
        }
    }
    Ok(())
}

fn optional_free(m: Matcher<'_>, f: &[FormDef]) -> Result<(), String> {
    match m(f, &tree_module(), &[tree_component()]) {
        Matched::Blocks(b) if b == ["tree"] => Ok(()),
        other => Err(format!(
            "a form without its optional channel must be green, got {other:?}"
        )),
    }
}

/// The rules with their check, and the mutant each one must catch.
const RULES: [(&str, Check, FormFlaw); 6] = [
    ("form.unknown", unknown, FormFlaw::UnknownOff),
    ("form.version-mismatch", version, FormFlaw::VersionOff),
    ("form.channel-unmapped", unmapped, FormFlaw::UnmappedOff),
    ("form.basic-mismatch", basic, FormFlaw::BasicOff),
    ("form.no-component", no_component, FormFlaw::NoComponentOff),
    (
        "form.optional-free",
        optional_free,
        FormFlaw::OptionalRequired,
    ),
];

/// The suite of the form match.
pub struct FormsSuite;

impl FormsSuite {
    /// Rule names in fixed order.
    #[must_use]
    pub fn rules() -> Vec<&'static str> {
        RULES.iter().map(|r| r.0).collect()
    }

    /// The mutants with the rule each must turn red (one per rule).
    #[must_use]
    pub fn mutants() -> Vec<(FormFlaw, &'static str)> {
        RULES.iter().map(|r| (r.2, r.0)).collect()
    }

    /// Runs every rule against a matcher: the failing rules with the reason.
    #[must_use]
    pub fn run(matcher: Matcher<'_>) -> Vec<(&'static str, String)> {
        let Ok(forms) = parse_forms_txt(CATALOG) else {
            return vec![("form.fixture", "fixture catalog unreadable".into())];
        };
        RULES
            .iter()
            .filter_map(|(name, check, _)| check(matcher, &forms).err().map(|why| (*name, why)))
            .collect()
    }

    /// Green on the real matcher, and every mutant turns its own rule red (and no other flaw hides).
    #[must_use]
    pub fn prove() -> (bool, Vec<&'static str>) {
        let base_green = Self::run(&|f, m, c| match_form(f, m, c, None)).is_empty();
        let caught = Self::mutants()
            .into_iter()
            .filter(|(flaw, rule)| {
                Self::run(&|f, m, c| match_form_flawed(f, m, c, None, *flaw))
                    .iter()
                    .any(|(r, _)| r == rule)
            })
            .map(|(_, rule)| rule)
            .collect();
        (base_green, caught)
    }
}
