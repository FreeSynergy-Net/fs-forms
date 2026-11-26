//! `fs-forms-layout`: the typed model of a forms description and the port that stores it (probe F1b, first cut).
//!
//! Three layers: the raw description ([`LayoutDoc`], as read from a file or the wire), the checked original
//! ([`Layout`]) and the resolved layout with overlays applied ([`Resolved`], what the host gets).
//! No file format lives here (that is `fs-forms-exchange`), no runtime, no database.
//!
//! Interface status: draft (ADR 0045). Dependencies that have not landed yet (fs-types 0.0.4, fs-components-catalog)
//! are replaced by the small stand-ins in [`shim`]; they are swapped at the lift.
//!
//! Pattern: Facade: this file only declares modules and re-exports the public API.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]

mod apply;
mod check;
mod check_actions;
mod check_frontend;
mod check_places;
mod check_sources;
mod check_text;
mod codes;
mod doc;
mod doc_words;
mod elems;
mod ids;
mod layout;
mod limits;
mod memory;
mod memory_admin;
mod memory_overlay;
mod memory_overlay_port;
mod memory_read;
mod memory_rebase;
mod memory_write;
mod overlay;
mod place;
#[cfg(test)]
mod place_tests;
mod port;
mod providers;
mod resolved;
mod rules;
mod schemes;
mod scope_token;
pub mod shim;
mod source_of;
mod types;
mod walk;
mod words;

pub use apply::apply;
pub use check::{CatalogRules, OpenCatalog, PlaceRule};
pub use check_text::{RangeBounds, range_bounds, split_range};
pub use codes::{LayoutCode, TEXTS_DE, TEXTS_EN};
pub use doc::*;
pub use ids::{
    ElemId, ElemKind, ElemRef, FrontendKey, LayoutId, OverlayId, PackageRef, Rev, Stamp, Who,
};
pub use layout::{BindingView, El, FORMAT_KNOWN, FORMAT_MIN, Layout};
pub use limits::{CeilingSet, LimitSource, LimitSources, Limits, Technical};
pub use memory::MemoryLayoutStore;
pub use overlay::{
    Cause, Conflict, ElemLock, Field, Op, OpKind, Overlay, OverlayScope, OverlayState, Position,
    Resolution, Value,
};
pub use place::{AttrDoc, EmbedDoc, FrameDoc, Leaf, LeafMut, PlaceNode};
pub use port::{
    LayoutAdmin, LayoutRead, LayoutStore, LayoutWrite, OverlayWrite, PackageLayouts, Pending,
};
pub use providers::{
    ProviderCatalogs, check_providers, split_provider_codes, unchecked_capabilities,
};
pub use resolved::{Applied, Effect, Layer, LayerInfo, Resolved};
pub use schemes::{
    DEFAULT_ALLOWED, NEVER_ALLOWED, SchemePolicy, scheme_names_rule, scheme_names_rule_with,
    schemes_rule,
};
pub use scope_token::{SCOPE_TOKEN_MAX, ScopeToken};
pub use shim::{Coded, ErrorCode, ObjectId, Reference, Utc};
pub use types::{
    Health, LayoutFilter, LogAction, LogEntry, Mode, Origin, OverlayDoc, Page, Report, Summary,
};
pub use words::Words;
