//! The strategies of the five kinds of channel that carry values.
//!
//! Pattern: Strategy (one `ValueMaker` per `Art`) + Registry (`standard` lists them).

mod flag;
mod number;
mod reference;
mod registry;
mod text;
mod time;

pub use registry::standard;
pub use text::word;
