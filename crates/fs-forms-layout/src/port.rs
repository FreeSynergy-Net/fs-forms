//! The storage port `LayoutStore`, split by role (Interface Segregation): the host needs only [`LayoutRead`].
//! async, `?Send`, dyn-capable; no default methods. Implementations: [`MemoryLayoutStore`](crate::MemoryLayoutStore) here,
//! the database store and the RPC client in their own crates.
//!
//! Pattern: Repository (ports by role) with Ports and Adapters.

use std::future::Future;
use std::pin::Pin;

use crate::codes::LayoutCode;
use crate::doc::LayoutDoc;
use crate::ids::{ElemRef, FrontendKey, LayoutId, OverlayId, PackageRef, Rev, Stamp, Who};
use crate::layout::Layout;
use crate::overlay::{Conflict, OpKind, Overlay, Resolution};
use crate::resolved::Resolved;
use crate::shim::{Coded, ObjectId};
use crate::types::{Health, LayoutFilter, LogEntry, Mode, OverlayDoc, Page, Report, Summary};

/// The result of one port call.
pub type Pending<'a, T> = Pin<Box<dyn Future<Output = Result<T, Coded<LayoutCode>>> + 'a>>;

/// Reading (the host needs only this).
pub trait LayoutRead {
    /// The store counter.
    fn stamp(&self) -> Pending<'_, Stamp>;
    /// Health.
    fn health(&self) -> Pending<'_, Health>;
    /// Lists layouts.
    fn list<'a>(
        &'a self,
        filter: &'a LayoutFilter,
        after: Option<&'a LayoutId>,
        size: u32,
    ) -> Pending<'a, Page<Summary>>;
    /// The stored original; absent or not visible gives `None`.
    fn original<'a>(&'a self, id: &'a LayoutId) -> Pending<'a, Option<(Layout, Summary)>>;
    /// Original plus applied overlays plus open conflicts plus locks.
    fn resolve<'a>(&'a self, key: &'a FrontendKey) -> Pending<'a, Option<Resolved>>;
    /// Overlays of a layout.
    fn overlays<'a>(&'a self, layout: &'a LayoutId) -> Pending<'a, Vec<Overlay>>;
    /// One overlay.
    fn overlay<'a>(&'a self, id: &'a OverlayId) -> Pending<'a, Option<Overlay>>;
    /// Conflicts, optionally of one layout and only the open ones.
    fn conflicts<'a>(
        &'a self,
        layout: Option<&'a LayoutId>,
        open_only: bool,
    ) -> Pending<'a, Vec<Conflict>>;
    /// The log, after a cursor.
    fn log<'a>(&'a self, after: Option<&'a ObjectId>, size: u32) -> Pending<'a, Vec<LogEntry>>;
}

/// The person's own descriptions. One transaction each; compare-and-set on `rev`.
pub trait LayoutWrite {
    /// Creates a layout of the person.
    fn create<'a>(&'a self, doc: LayoutDoc, who: &'a Who) -> Pending<'a, Report>;
    /// Imports a layout.
    fn import<'a>(&'a self, doc: LayoutDoc, mode: Mode, who: &'a Who) -> Pending<'a, Report>;
    /// Replaces a layout; a package original answers `package-owned`.
    fn save<'a>(
        &'a self,
        id: &'a LayoutId,
        expected: Rev,
        doc: LayoutDoc,
        who: &'a Who,
    ) -> Pending<'a, Report>;
    /// Tombstone: elements gone, row stays, key free, id never reused.
    fn delete<'a>(&'a self, id: &'a LayoutId, expected: Rev, who: &'a Who) -> Pending<'a, ()>;
}

/// Package originals. Install of an installed identity is an update: original replaced, overlays re-applied, one transaction.
pub trait PackageLayouts {
    /// Installs or updates.
    fn install<'a>(
        &'a self,
        doc: LayoutDoc,
        package: &'a PackageRef,
        mode: Mode,
        who: &'a Who,
    ) -> Pending<'a, Report>;
    /// Tombstones the originals of a package; overlays become `orphaned`.
    fn uninstall<'a>(&'a self, package: &'a str, who: &'a Who) -> Pending<'a, Vec<Report>>;
}

/// Overlays over package originals.
pub trait OverlayWrite {
    /// Creates a person overlay.
    fn create_overlay<'a>(
        &'a self,
        layout: &'a LayoutId,
        lang: Option<&'a str>,
        title_key: Option<&'a str>,
        who: &'a Who,
    ) -> Pending<'a, OverlayId>;
    /// Replaces the op list; `expect` is filled by the store from the original.
    fn replace_ops<'a>(
        &'a self,
        id: &'a OverlayId,
        expected: Rev,
        ops: Vec<OpKind>,
        who: &'a Who,
    ) -> Pending<'a, Report>;
    /// Switches an overlay on or off.
    fn set_state<'a>(
        &'a self,
        id: &'a OverlayId,
        expected: Rev,
        on: bool,
        who: &'a Who,
    ) -> Pending<'a, Rev>;
    /// Deletes an overlay.
    fn delete_overlay<'a>(
        &'a self,
        id: &'a OverlayId,
        expected: Rev,
        who: &'a Who,
    ) -> Pending<'a, ()>;
    /// Imports an overlay with the same conflict check.
    fn import_overlay<'a>(
        &'a self,
        doc: OverlayDoc,
        mode: Mode,
        who: &'a Who,
    ) -> Pending<'a, Report>;
    /// Resolves one conflict.
    fn resolve_conflict<'a>(
        &'a self,
        id: &'a OverlayId,
        seq: u32,
        base_version: &'a str,
        how: Resolution,
        who: &'a Who,
    ) -> Pending<'a, Rev>;
}

/// Instance admin; the person service answers `unsupported{what=admin}`. A reason is mandatory for lock and unlock.
pub trait LayoutAdmin {
    /// Locks an element (or the whole layout).
    fn lock<'a>(
        &'a self,
        layout: &'a LayoutId,
        elem: Option<&'a ElemRef>,
        reason: &'a str,
        who: &'a Who,
    ) -> Pending<'a, bool>;
    /// Unlocks.
    fn unlock<'a>(
        &'a self,
        layout: &'a LayoutId,
        elem: Option<&'a ElemRef>,
        reason: &'a str,
        who: &'a Who,
    ) -> Pending<'a, bool>;
    /// Creates an instance overlay.
    fn create_instance_overlay<'a>(
        &'a self,
        layout: &'a LayoutId,
        lang: Option<&'a str>,
        title_key: Option<&'a str>,
        who: &'a Who,
    ) -> Pending<'a, OverlayId>;
}

/// The whole port of the person service (supertrait only, no methods of its own).
pub trait LayoutStore: LayoutRead + LayoutWrite + PackageLayouts + OverlayWrite {}
impl<T: LayoutRead + LayoutWrite + PackageLayouts + OverlayWrite> LayoutStore for T {}
