//! `SampleModule`: a sample set served as a `ModuleCore`, so a host reads it like any module.
//!
//! Pattern: Fake Object (Test Double) behind the port `ModuleCore`. Departure from the usual fake: its data are not fixed but made
//! by the builder from the declaration of the form, and its commands are not written one by one: they run from the `writes`
//! column of the declaration. Everything stays in memory; no write leaves the process (condition of C1, 25.11.2026).
//!
//! The rights are those of the spec (`Forbid`); the module itself checks none (the wrapper asks the rules first).
//! The provider says `fake` (`SAMPLE_PROVIDER`): a host shows such a source as test data.

#![allow(clippy::result_large_err)] // `module::Refusal` is the one error type of `ModuleCore`

mod apply;
mod declared;
mod rules;
mod sample;
mod store;

pub use rules::SampleRules;
pub use sample::SampleModule;
