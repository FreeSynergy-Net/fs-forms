//! Data types of the port: health, filter, summary, page, report, log entry, overlay exchange shape.
//!
//! Pattern: Value Object: plain data crossing the port.

use crate::codes::LayoutCode;
use crate::doc::Item;
use crate::ids::{ElemId, FrontendKey, LayoutId, OverlayId, PackageRef, Rev, Who};
use crate::overlay::{Conflict, OpKind};
use crate::shim::{Coded, ObjectId, Reference, Utc};

/// Health of the store. In emergency mode reads are empty and writes say `read-only`.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Health {
    /// All well.
    Ok,
    /// Emergency mode, with the reasons.
    Defaults(Vec<Coded<LayoutCode>>),
}
/// Where a layout came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// Installed from a package; read-only except install and uninstall.
    Package,
    /// Made by the person.
    Person,
    /// Imported from a file.
    Import,
}
/// Which layouts to list.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LayoutFilter {
    /// Only this program.
    pub extends: Option<String>,
    /// Only this origin.
    pub origin: Option<Origin>,
    /// Only this package.
    pub package: Option<String>,
    /// Include tombstones.
    pub with_deleted: bool,
}
/// One line of a list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Summary {
    /// Id.
    pub id: LayoutId,
    /// Key (the identity of a tombstone stays here too).
    pub key: FrontendKey,
    /// Origin.
    pub origin: Origin,
    /// Package, for package originals.
    pub package: Option<PackageRef>,
    /// Owner, for the person's own.
    pub owner: Option<Reference>,
    /// Counter.
    pub rev: Rev,
    /// Last change.
    pub updated: Utc,
    /// Set for tombstones.
    pub deleted: Option<Utc>,
    /// Overlays on it.
    pub overlays: u32,
    /// Open conflicts on it.
    pub open_conflicts: u32,
}
/// A page of a list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page<T> {
    /// Items.
    pub items: Vec<T>,
    /// Cursor for the next page.
    pub next: Option<LayoutId>,
}
/// Apply or only report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Report only; nothing is written.
    DryRun,
    /// Write.
    Apply,
}
/// What a write did; never silent. `problems` are refusals (Apply changed nothing when not empty), `warnings` went through.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// The layout touched.
    pub layout: Option<LayoutId>,
    /// Its new counter.
    pub rev: Option<Rev>,
    /// Refusals.
    pub problems: Vec<Coded<LayoutCode>>,
    /// Warnings.
    pub warnings: Vec<Coded<LayoutCode>>,
    /// Conflicts found while applying overlays.
    pub conflicts: Vec<Conflict>,
}
/// What a log row records.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogAction {
    /// A layout was created.
    Create,
    /// A layout was saved.
    Save,
    /// A layout was deleted.
    Delete,
    /// A layout was imported.
    Import,
    /// A package was installed.
    Install,
    /// A package was updated.
    Update,
    /// A package was uninstalled.
    Uninstall,
    /// An overlay was created.
    OverlayCreate,
    /// Overlay ops were replaced.
    OverlayOps,
    /// Overlay switched on or off.
    OverlayState,
    /// An overlay was deleted.
    OverlayDelete,
    /// An overlay was imported.
    OverlayImport,
    /// A conflict was resolved.
    ConflictResolve,
    /// An element was locked.
    Lock,
    /// An element was unlocked.
    Unlock,
}
/// One row of the append-only log. Time and os user are set by the service.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogEntry {
    /// Row id.
    pub id: ObjectId,
    /// When.
    pub at: Utc,
    /// Who.
    pub who: Who,
    /// The operating system user of the connection.
    pub os_user: Option<String>,
    /// What.
    pub action: LogAction,
    /// Layout concerned.
    pub layout: Option<LayoutId>,
    /// Overlay concerned.
    pub overlay: Option<OverlayId>,
    /// Element concerned.
    pub elem: Option<ElemId>,
    /// Counter before.
    pub rev_before: Option<Rev>,
    /// Counter after.
    pub rev_after: Option<Rev>,
    /// Package concerned.
    pub package: Option<PackageRef>,
    /// Count of things changed.
    pub count: Option<u32>,
    /// Reason (mandatory for lock and unlock).
    pub reason: Option<String>,
}
/// Exchange shape of an overlay: header plus ops by element.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverlayDoc {
    /// Exchange format.
    pub format: u32,
    /// Package.
    pub package: String,
    /// Program.
    pub extends: String,
    /// Frontend name.
    pub frontend: String,
    /// Package version it was made against.
    pub base_version: String,
    /// Language.
    pub lang: Option<String>,
    /// MF2 key of the title.
    pub title_key: Option<String>,
    /// Ops.
    pub ops: Vec<Item<OpKind>>,
}
