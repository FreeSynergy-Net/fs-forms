//! What one open window of the real owner holds, and the small reads over the table of the open windows.
//!
//! Pattern: Value Object (the entry: dropping it drops the grip, i.e. signs the connection off, and the widget).

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use fs_components_app::AppWidget;
use fs_forms_layout::FrontendKey;

use super::{OpenedFrontend, Placement, WindowId};
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
    pub(super) app: AppWidget,
    pub(super) raised: usize,
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
            mode: Placement::Window,
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
