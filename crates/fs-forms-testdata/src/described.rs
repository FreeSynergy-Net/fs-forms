//! A description in, one `SampleSpec` per `[[source]]` out; and the module made from it.
//!
//! Pattern: Builder director (the blocks of the description ask the forms through the port `BlockForms`; the section `[sample]` of the
//! description overrides the default wish of a form, else the defaults of the declaration hold) + Factory (`SampleModule::from_description`).
//! One module per source: the host replaces each `[[source]]` by the module of its number; two sources may serve the same form with
//! different types (`rows` for `Lane` and for `Task`) without their ids meeting.

use fs_forms_bind::FormRef;
use fs_forms_layout::Layout;
use fs_types::Zone;

use crate::blocks::{BlockForms, Need, needs};
use crate::builder::{SampleBuilder, resolve};
use crate::defaults;
use crate::error::{Failure, SampleCode, fail};
use crate::module::SampleModule;
use crate::set::SampleSet;
use crate::spec::{Anchor, Period, Relation, SampleSpec, SetWish};
use crate::spec_toml::{SampleToml, SourceWish, read_sample};

/// The seed when `[sample]` gives none.
pub const DEFAULT_SEED: u64 = 0x0F5_5A3D;

/// What one `[[source]]` of a description is asked to serve.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceSpec {
    /// The number of the source (counts from 1).
    pub source: u32,
    /// The wishes, one per form.
    pub spec: SampleSpec,
    /// The relations the declarations of the forms carry (until the builder reads them itself).
    pub relations: Vec<Relation>,
}

impl SourceSpec {
    /// The builder of this source: spec and relations.
    #[must_use]
    pub fn builder(&self) -> SampleBuilder {
        self.relations.iter().cloned().fold(
            SampleBuilder::new(self.spec.clone()),
            SampleBuilder::relation,
        )
    }

    /// Builds the set.
    ///
    /// # Errors
    /// The first wish that cannot be met (a code, never dropped).
    pub fn build(&self) -> Result<SampleSet, Failure> {
        self.builder().build()
    }
}

/// The form of a need as a requirement the builder resolves: `name@^major.minor.0` (the highest version of the catalog that keeps the
/// main version and has at least that minor).
fn requirement(form: &FormRef) -> String {
    format!("{}@^{}.{}.0", form.name, form.major, form.minor)
}

fn name_of(form: &str) -> &str {
    form.split_once('@').map_or(form, |(n, _)| n)
}

pub(crate) fn default_anchor() -> Result<Anchor, Failure> {
    Ok(Anchor {
        period: Period::Month(2026, 5),
        zone: Zone::iana("Europe/Berlin")
            .map_err(|_| fail(SampleCode::ZoneUnknown, "zone", "Europe/Berlin"))?,
    })
}

/// The wishes of one source: the default of each form its blocks need, a written wish of `[sample]` taking the place of the default of
/// the same form (by name), a written wish of a form nobody needs added at the end.
fn wishes(
    source: u32,
    data_type: &str,
    needed: &[&Need],
    written: &[&SourceWish],
) -> Result<Vec<SetWish>, Failure> {
    let mut out: Vec<SetWish> = Vec::new();
    for need in needed {
        let asked = requirement(&need.form);
        let form = resolve(&asked)?;
        out.push(defaults::wish(&asked, form, data_type));
    }
    let mut seen: Vec<&str> = Vec::new();
    for w in written {
        let name = name_of(&w.wish.form);
        if seen.contains(&name) {
            return Err(fail(
                SampleCode::WishTwice,
                "what",
                format!("source {source}: {name}"),
            ));
        }
        seen.push(name);
        if w.data_type.as_deref().is_some_and(|t| t != data_type) {
            return Err(fail(
                SampleCode::TypeMismatch,
                "what",
                format!(
                    "source {source} is {data_type}, the wish says {}",
                    w.data_type.as_deref().unwrap_or_default()
                ),
            ));
        }
        let mut wish = w.wish.clone();
        data_type.clone_into(&mut wish.data_type);
        match out.iter().position(|o| name_of(&o.form) == name) {
            Some(at) => out[at] = wish,
            None => out.push(wish),
        }
    }
    Ok(out)
}

pub(crate) fn relations(wishes: &[SetWish]) -> Result<Vec<Relation>, Failure> {
    let mut out: Vec<Relation> = Vec::new();
    for wish in wishes {
        for relation in defaults::relations(resolve(&wish.form)?)? {
            if !out.contains(&relation) {
                out.push(relation);
            }
        }
    }
    Ok(out)
}

impl SampleSpec {
    /// One [`SourceSpec`] for every `[[source]]` of the description: the forms the blocks need (`blocks`), the wishes from the section
    /// `[sample]` in `sample` (the text of the description file; `None` or without the section: the defaults of the declaration).
    ///
    /// # Errors
    /// `section-bad`, `zone-unknown`, `anchor-bad` for `[sample]`; `source-unknown` or `type-mismatch` or `wish-twice` for a wish that
    /// does not fit the description; `form-unknown` for a form the catalog lacks. Nothing is dropped silently.
    pub fn from_description(
        layout: &Layout,
        blocks: &dyn BlockForms,
        sample: Option<&str>,
    ) -> Result<Vec<SourceSpec>, Failure> {
        let read = sample.map(read_sample).transpose()?.flatten();
        let read = read.unwrap_or(SampleToml {
            seed: None,
            anchor: None,
            forbid: Vec::new(),
            sets: Vec::new(),
        });
        let needed = needs(layout, blocks)?;
        let sources = &layout.doc().sources;
        if let Some(w) = read
            .sets
            .iter()
            .find(|w| usize::try_from(w.source).map_or(true, |n| n == 0 || n > sources.len()))
        {
            return Err(fail(
                SampleCode::SourceUnknown,
                "source",
                w.source.to_string(),
            ));
        }
        let anchor = match read.anchor.clone() {
            Some(a) => a,
            None => default_anchor()?,
        };
        let mut out = Vec::new();
        for (source, s) in (1u32..).zip(sources) {
            let mine: Vec<&Need> = needed.iter().filter(|n| n.source == source).collect();
            let written: Vec<&SourceWish> =
                read.sets.iter().filter(|w| w.source == source).collect();
            let sets = wishes(source, &s.v.data_type, &mine, &written)?;
            out.push(SourceSpec {
                source,
                relations: relations(&sets)?,
                spec: SampleSpec {
                    seed: read.seed.unwrap_or(DEFAULT_SEED),
                    anchor: anchor.clone(),
                    sets,
                    forbid: read.forbid.clone(),
                },
            });
        }
        Ok(out)
    }
}

impl SampleModule {
    /// The module that stands in for `[[source]]` number `source` of a description (see [`SampleSpec::from_description`]).
    ///
    /// # Errors
    /// As [`SampleSpec::from_description`], `source-unknown` for a number the description does not have, and whatever the builder says.
    pub fn from_description(
        layout: &Layout,
        blocks: &dyn BlockForms,
        sample: Option<&str>,
        source: u32,
    ) -> Result<Self, Failure> {
        let spec = SampleSpec::from_description(layout, blocks, sample)?
            .into_iter()
            .find(|s| s.source == source)
            .ok_or_else(|| fail(SampleCode::SourceUnknown, "source", source.to_string()))?;
        Self::new(&spec.build()?)
    }
}
