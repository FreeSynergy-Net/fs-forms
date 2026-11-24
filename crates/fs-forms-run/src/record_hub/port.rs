//! What the hub needs of a module: the read side for the cache, and one OWN connection per view (the hold) that watches a record and
//! writes. The real owner implements these over `fs_rpc::hold_with` + `FormSource.watch`; the double in [`super::fake`] in memory.
//!
//! Pattern: Port (hexagonal) – the hub knows no socket; Observer – a [`ChangeSink`] is told of every change by whomever.

use std::sync::Arc;

use crate::records::Records;
use crate::view::Coded;

/// Pattern: Observer – told by the module about EVERY change of the watched record, whoever made it (this view, a neighbour window of
/// the same process, another device); `rev` is the module's revision after the change. There is no other way a view learns of a change.
pub trait ChangeSink: Send + Sync {
    /// The record changed; its revision is now `rev`.
    fn changed(&self, rev: u64);
}

/// Pattern: Port – one module (one range). Many views connect to it, each with its own [`Link`].
pub trait Module: Send + Sync {
    /// The records of `scope` for the read cache of the hub (one object per key; it must never be written through the cache).
    fn records(&self, scope: Option<&str>) -> Arc<dyn Records>;

    /// A NEW connection of one view (never shared, never merged with another): it hears changes through `sink` and falls with the
    /// view. Nothing is watched until [`Link::watch`].
    ///
    /// # Errors
    /// The code of the refused connection.
    fn connect(&self, sink: Arc<dyn ChangeSink>) -> Result<Arc<dyn Link>, Coded>;
}

/// Pattern: Port + RAII – the own connection of ONE view to its module (the hold). Dropping it detaches it.
pub trait Link: Send + Sync {
    /// Watch `scope` from now on (the one record this view shows); the sink hears changes of that record only.
    fn watch(&self, scope: Option<&str>);
    /// The revision of the record `scope` in the module now (what a write is checked against: `conflict` 409 when the view saw less).
    fn rev(&self, scope: Option<&str>) -> u64;
    /// The writing side of the record `scope` on THIS connection: the module takes the change as from this view, and only when the
    /// record is still at `seen` (the revision the view saw). The module decides, in ONE step with taking the change: a record that
    /// moved on is refused with `conflict` (409), like an `If-Match` that no longer fits (RFC 9110 section 13.1.1, status 412). The
    /// check of the client is only a pre-check.
    fn writer(&self, scope: Option<&str>, seen: u64) -> Arc<dyn Records>;
}
