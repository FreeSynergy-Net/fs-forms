//! What one open window of the real owner holds, and the small reads over the table of the open windows.
//!
//! Pattern: Value Object (the entry: dropping it drops the grip, i.e. signs the connection off, and the widget).

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use fs_components_app::AppWidget;
use fs_forms_layout::FrontendKey;

use super::{OpenedFrontend, Placement, WindowId};
use crate::frontend_cell::FrontendCell;
use crate::record_hub::{Module, RecordGrip};
use std::sync::Arc;

/// What one open window holds; dropping the entry drops the grip (= its connection, signing off) and the widget.
pub(super) struct Entry {
    pub(super) key: FrontendKey,
    pub(super) place: (String, String),
    pub(super) scope: Option<String>,
    pub(super) parent: Option<WindowId>,
    pub(super) grip: RecordGrip,
    pub(super) module: Arc<dyn Module>,
    /// The window; `None` for a panel (it has no window, its page is drawn inside the page of its parent).
    pub(super) app: Option<AppWidget>,
    /// The page of the frontend that is open now (what `use_frontend` exchanges).
    pub(super) cell: Arc<FrontendCell>,
    /// The panels drawn inside it: (place, head with the program the panel comes from), in tree order.
    pub(super) panels: Vec<(String, String)>,
    /// The frontend nodes that are NOT drawn: (place, code of the reason), in place order.
    pub(super) missing: Vec<(String, String)>,
    pub(super) raised: usize,
    /// Names THIS window for good (a switch changes the id, never the token): the `on_closed` of the surface finds the entry by it, so
    /// a closed window never removes the entry of a later window that got the same id.
    pub(super) token: u64,
}

pub(super) type Open = HashMap<WindowId, Entry>;

pub(super) fn lock(open: &Mutex<Open>) -> MutexGuard<'_, Open> {
    open.lock().unwrap_or_else(PoisonError::into_inner)
}

pub(super) fn snapshot(open: &Open) -> Vec<OpenedFrontend> {
    let mut now: Vec<OpenedFrontend> = open
        .iter()
        .map(|(id, e)| OpenedFrontend {
            id: id.clone(),
            key: e.key.clone(),
            mode: if e.app.is_some() {
                Placement::Window
            } else {
                Placement::InFrame
            },
            parent: e.parent.clone(),
            scope: e.scope.clone(),
        })
        .collect();
    now.sort_by(|a, b| a.id.cmp(&b.id));
    now
}

/// `id` and every window whose parent chain reaches it.
pub(super) fn family(open: &Open, id: &WindowId) -> Vec<WindowId> {
    let mut doomed = vec![id.clone()];
    loop {
        let more: Vec<WindowId> = open
            .iter()
            .filter(|(k, e)| {
                !doomed.contains(k) && e.parent.as_ref().is_some_and(|p| doomed.contains(p))
            })
            .map(|(k, _)| k.clone())
            .collect();
        if more.is_empty() {
            return doomed;
        }
        doomed.extend(more);
    }
}

/// The id of a child follows the id of its parent (`<parent id>/<elem>`): when the parent is renamed from `from` to `to` (a record
/// switch), every entry below it is renamed the same way, with its `parent` and the place it names.
pub(super) fn rename_below(open: &mut Open, from: &WindowId, to: &WindowId) {
    let prefix = format!("{}/", from.as_str());
    let map = |id: &WindowId| {
        if id == from {
            to.clone()
        } else if let Some(rest) = id.as_str().strip_prefix(&prefix) {
            WindowId(format!("{}/{rest}", to.as_str()))
        } else {
            id.clone()
        }
    };
    let below: Vec<WindowId> = open
        .keys()
        .filter(|k| k.as_str().starts_with(&prefix))
        .cloned()
        .collect();
    for old in below {
        if let Some(mut e) = open.remove(&old) {
            e.parent = e.parent.as_ref().map(map);
            e.place.0 = map(&WindowId(e.place.0.clone())).0;
            open.insert(map(&old), e);
        }
    }
}
