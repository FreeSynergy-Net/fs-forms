//! The G7 adapter: form + module declaration + components -> the block, or a visible reason (ENTSCHEID-form §3).
//!
//! One pure function; `--check`, the run time and the suite use this one and no second checker. Rules (codes without
//! `-error-`, the names of the plan): `form-unknown` (red), `form-version-mismatch` (red), `form-channel-unmapped` (red),
//! `form-basic-mismatch` (red), `form-no-component` (yellow: the fallback of ADR 0098 E5).
//!
//! Pattern: Specification (each rule one small function) + total result type; no panic, no I/O.

use crate::coded::Coded;
use crate::forms::{ComponentForms, FormDef, Grundtyp, ModuleForm};

/// The result of a match.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Matched {
    /// Green: these blocks draw the form in a fitting version (the frontend names one; the host never picks, H-K2).
    Blocks(Vec<String>),
    /// Yellow: the form is fine, but no component draws it; the fallback of ADR 0098 E5 shows it.
    Fallback(Coded),
    /// Red: every problem, never only the first.
    Refused(Vec<Coded>),
}

impl Matched {
    /// Red?
    #[must_use]
    pub fn is_red(&self) -> bool {
        matches!(self, Self::Refused(_))
    }
}

/// A deliberate flaw for the mutants (feature `testing`): one rule off, or one rule too strict.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormFlaw {
    /// `form-unknown` is not reported.
    UnknownOff,
    /// `form-version-mismatch` is not reported (every component counts as fitting).
    VersionOff,
    /// `form-channel-unmapped` is not reported.
    UnmappedOff,
    /// `form-basic-mismatch` is not reported.
    BasicOff,
    /// `form-no-component` is not reported (a missing component counts as green).
    NoComponentOff,
    /// Only the first type of a channel's set counts (the set is ignored).
    SetFirstOnly,
    /// An optional channel without a field is reported as unmapped (too strict).
    OptionalRequired,
}

/// Puts a module's form and the components together. `block` = the block the frontend named (`None` = any block).
#[must_use]
pub fn match_form(
    forms: &[FormDef],
    module: &ModuleForm,
    components: &[ComponentForms],
    block: Option<&str>,
) -> Matched {
    run(forms, module, components, block, None)
}

/// [`match_form`] with one flaw, for the mutants of the suite.
#[cfg(feature = "testing")]
#[must_use]
pub fn match_form_flawed(
    forms: &[FormDef],
    module: &ModuleForm,
    components: &[ComponentForms],
    block: Option<&str>,
    flaw: FormFlaw,
) -> Matched {
    run(forms, module, components, block, Some(flaw))
}

fn run(
    forms: &[FormDef],
    module: &ModuleForm,
    components: &[ComponentForms],
    block: Option<&str>,
    flaw: Option<FormFlaw>,
) -> Matched {
    let on = |f: FormFlaw| flaw != Some(f);
    let mut red = Vec::new();
    // A form is known when the catalog has its name in the module's main version and a minor at least as new.
    let def = forms.iter().find(|d| {
        d.name == module.form.name && d.major == module.form.major && d.minor >= module.form.minor
    });
    if def.is_none() && on(FormFlaw::UnknownOff) {
        red.push(
            Coded::new("form-unknown")
                .with("form", module.form.name.clone())
                .with(
                    "version",
                    format!("{}.{}", module.form.major, module.form.minor),
                ),
        );
    }
    if let Some(def) = def {
        red.extend(channels(def, module, flaw));
    }
    let fitting: Vec<&ComponentForms> = components
        .iter()
        .filter(|c| block.is_none_or(|b| c.block == b))
        .filter(|c| c.forms.iter().any(|f| f.name == module.form.name))
        .collect();
    let blocks: Vec<String> = fitting
        .iter()
        .filter(|c| !on(FormFlaw::VersionOff) || c.forms.iter().any(|f| fits(f, module)))
        .map(|c| c.block.clone())
        .collect();
    if blocks.is_empty() && !fitting.is_empty() {
        for c in &fitting {
            red.extend(
                c.forms
                    .iter()
                    .filter(|f| f.name == module.form.name)
                    .map(|f| {
                        Coded::new("form-version-mismatch")
                            .with("form", module.form.name.clone())
                            .with("block", c.block.clone())
                            .with("component", format!("{}.{}", f.major, f.minor))
                            .with(
                                "module",
                                format!("{}.{}", module.form.major, module.form.minor),
                            )
                    }),
            );
        }
    }
    if !red.is_empty() {
        Matched::Refused(red)
    } else if blocks.is_empty() && on(FormFlaw::NoComponentOff) {
        Matched::Fallback(Coded::new("form-no-component").with("form", module.form.name.clone()))
    } else {
        Matched::Blocks(blocks)
    }
}

/// Main version equal and the component's minor not older than the module's: it knows every channel the module maps.
fn fits(c: &crate::forms::FormRef, module: &ModuleForm) -> bool {
    c.major == module.form.major && c.minor >= module.form.minor
}

/// `form-channel-unmapped` and `form-basic-mismatch` for every channel of the form.
fn channels(def: &FormDef, module: &ModuleForm, flaw: Option<FormFlaw>) -> Vec<Coded> {
    let mut out = Vec::new();
    for ch in &def.channels {
        let mapped: &[String] = module
            .map
            .iter()
            .find(|(c, _)| *c == ch.name)
            .map_or(&[], |(_, f)| f.as_slice());
        let need = ch.required || flaw == Some(FormFlaw::OptionalRequired);
        if mapped.is_empty() {
            if need && flaw != Some(FormFlaw::UnmappedOff) {
                out.push(
                    Coded::new("form-channel-unmapped")
                        .with("form", def.name.clone())
                        .with("channel", ch.name.clone()),
                );
            }
            continue;
        }
        // `records` and `command` name a type or a command of the module, not a field with a basic type.
        let types: &[Grundtyp] = match flaw {
            Some(FormFlaw::SetFirstOnly) => &ch.grundtypen[..ch.grundtypen.len().min(1)],
            _ => &ch.grundtypen,
        };
        if types
            .iter()
            .all(|g| matches!(g, Grundtyp::Records | Grundtyp::Command))
        {
            continue;
        }
        if flaw == Some(FormFlaw::BasicOff) {
            continue;
        }
        for field in mapped {
            // A field the module does not list has no basic: it cannot be proven to fit, so it does not.
            let basic = module
                .fields
                .iter()
                .find(|(n, _)| n == field)
                .map(|(_, b)| *b);
            if !basic.is_some_and(|b| types.iter().any(|g| g.takes(b))) {
                out.push(
                    Coded::new("form-basic-mismatch")
                        .with("form", def.name.clone())
                        .with("channel", ch.name.clone())
                        .with("field", field.clone())
                        .with(
                            "grundtyp",
                            ch.grundtypen
                                .iter()
                                .map(|g| g.name())
                                .collect::<Vec<_>>()
                                .join("|"),
                        ),
                );
            }
        }
    }
    out
}
