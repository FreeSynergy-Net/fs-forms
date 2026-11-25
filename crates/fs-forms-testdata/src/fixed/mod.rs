//! Fixed, named test data: a set that looks like a picture of a model program, not like words made up by the generator.
//!
//! A description asks for it with `fixed = "<name>"` in `[[sample.set]]`; the builder asks the [`FixedRegistry`] and places the
//! answer ([`FixedSlice`]) next to the generated sets. Everything behind it (projection, `SampleModule`, host) stays the same.
//!
//! Pattern: Object Mother behind a Registry – see [`FixedSource`].

mod events;
mod events_record;
mod names;
mod place;
mod registry;
mod source;

pub use events::EventsSample;
pub(crate) use names::resolve_names;
pub(crate) use place::place;
pub use registry::{FixedRegistry, standard_fixed};
pub use source::{FixedSlice, FixedSource, Names};
