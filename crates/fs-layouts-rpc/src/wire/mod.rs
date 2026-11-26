//! The wire: the model of `fs-forms-layout` to the structs of the contract `layouts` and back. Total functions, no I/O.
//! Reading returns a layout code (`format-invalid{found}`) for a word or a value that is in no closed list, never a replacement.
//!
//! Pattern: Adapter (both directions); one module per family of structs.

#[cfg(test)]
mod action_at_tests;
pub(crate) mod answers;
pub(crate) mod conflict;
pub(crate) mod doc_in;
pub(crate) mod doc_out;
pub(crate) mod filter;
pub(crate) mod ids;
pub(crate) mod overlay;
pub(crate) mod resolved;
#[cfg(test)]
mod source_tests;
pub(crate) mod text;
pub(crate) mod tree_guard;
#[cfg(test)]
mod tree_guard_tests;
pub(crate) mod tree_in;
pub(crate) mod tree_out;
#[cfg(test)]
mod tree_tests;
pub(crate) mod words;
