//! `fs-forms-testdata`: generic, deterministic test data made from the declaration of a form.
//!
//! A [`SampleSpec`] (wishes: how many records, which edge cases, which anchor in time) and the declaration of a form
//! (`fs_interfaces::forms`) give a [`SampleSet`] of ActivityStreams 2 objects (`fs_objects::StoredObject`) whose content follows the
//! standard of their type (JSCalendar for `Event`, JSContact for `Person`, plain AS2 otherwise). Nothing here names a program or a channel: strategies
//! work per kind of channel (`Art`), cases per kind of relation. The same spec and seed give the same set on every platform
//! (ChaCha8; the words come from the crate `fake`, pinned exactly, no word lists of our own). Failures are [`SampleCode`]s, never
//! a silently dropped wish.
//!
//! A description gives its wishes too: [`SampleSpec::from_description`] asks the blocks which forms they draw (the port [`BlockForms`]),
//! reads `[sample]` ([`read_sample`]) and else takes the defaults of the declaration; [`SampleModule::from_description`] serves them.
//!
//! A wish may name a fixed source instead of the generator (`fixed = "events.sample"`, see [`FixedSource`], [`standard_fixed`]): the data of a
//! model program as the contract gives them, with display names for what they point at (`SampleSet::names`) and their fixed "now".
//!
//! [`SampleModule`] serves a set as a `ModuleCore` (in memory; commands run from the `writes` column of the declaration).
//!
//! Written with the help of Claude (Anthropic). Interface status: draft (ADR 0043: pattern first, interface first).
//!
//! Pattern: Test Data Builder + Strategy: the builder ([`SampleBuilder`]) puts wishes, relations and strategies
//! ([`ValueMaker`], one per `Art`) together. Departure: the wishes are values (they are written, not only checked).

#![deny(missing_docs)]
#![warn(clippy::pedantic)]

mod assemble;
mod blocks;
mod builder;
mod case;
pub mod content;
mod defaults;
mod described;
mod error;
mod fixed;
mod interval;
mod makers;
mod module;
mod plan;
mod project;
mod set;
mod spec;
mod spec_toml;
mod value;
mod wish_text;

pub use blocks::{BlockForms, Need, needs};
pub use builder::SampleBuilder;
pub use case::{Case, IntervalCase};
pub use defaults::{DEFAULT_COUNT, DEFAULT_DISTINCT};
pub use described::{DEFAULT_SEED, SourceSpec};
pub use error::{Failure, SampleCode};
pub use fixed::{
    EventsSample, FixedRegistry, FixedSlice, FixedSource, Names, PersonDirectorySample,
    standard_fixed,
};
pub use fs_interfaces::module::{ProviderKind, SAMPLE_PROVIDER};
pub use makers::standard as standard_makers;
pub use module::{SampleModule, SampleRules};
pub use project::{Rec, Table, Values};
pub use set::{SampleRow, SampleSet, Slice};
pub use spec::{Anchor, Forbid, Period, Relation, SampleSpec, SetWish, Span};
pub use spec_toml::{SampleToml, SourceWish, read_sample};
pub use value::{Flavor, MakeCx, Moment, Record, Value, ValueMaker};
