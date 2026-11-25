//! One wish file in, one `SourceSpec` out: the file is a description part with only `[sample]` (tests keep such files as data
//! next to their code instead of writing the wish in Rust each time).
//!
//! Pattern: Factory method on `SourceSpec` (reads with `read_sample`, completes with the same defaults a description gets).

use crate::described::{DEFAULT_SEED, SourceSpec, default_anchor, relations};
use crate::error::{Failure, SampleCode, fail};
use crate::spec::SampleSpec;
use crate::spec_toml::read_sample;

impl SourceSpec {
    /// The spec of a wish file: seed (else [`DEFAULT_SEED`]), anchor (else the default one), every `[[sample.set]]` with its `type`
    /// (required: no description gives it here) and the relations the declarations of the forms carry.
    ///
    /// # Errors
    /// `section-bad` when the text has no `[sample]` or a set has no `type`; whatever `read_sample` and the resolving of a form say.
    pub fn from_wish_text(text: &str) -> Result<Self, Failure> {
        let bad = |what: &str| fail(SampleCode::SectionBad, "what", what);
        let read = read_sample(text)?.ok_or_else(|| bad("no section [sample]"))?;
        let mut sets = Vec::new();
        for s in read.sets {
            let mut wish = s.wish;
            wish.data_type = s.data_type.ok_or_else(|| bad("a set without `type`"))?;
            sets.push(wish);
        }
        Ok(Self {
            source: 1,
            relations: relations(&sets)?,
            spec: SampleSpec {
                seed: read.seed.unwrap_or(DEFAULT_SEED),
                anchor: match read.anchor {
                    Some(a) => a,
                    None => default_anchor()?,
                },
                sets,
                forbid: read.forbid,
            },
        })
    }
}
