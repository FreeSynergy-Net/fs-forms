//! The change of the frontend of an open window (`WindowOwner::use_frontend`) and what a click on the menu `View` does: the same
//! function, found by the token of the window. The window, its id, its record, its records and its held connection stay; the page in
//! the [`FrontendCell`] and the menus are exchanged, the key of the entry is the new one.
//!
//! Pattern: Bridge (the surface is the abstraction, the frontend its implementation) over the same ports as the owner.
//! Abweichung: the port lists the frontends of ONE program, so a name of another program is answered `frontend-unknown{available}`
//! (not `frontend-other-program`, which only the double can tell); the state of the person (selection of a board) is in the mounts of the
//! old page and does not move to the new one (a rule for open input does not exist in the host yet).

use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

use fs_components_app::{AppError, AppWidget, Status};
use fs_components_commands::FrontendRow;
use fs_components_words::Locale;
use fs_forms_layout::FrontendKey;

use super::build::connected;
use super::entry::{Entry, Open, family, lock};
use super::{Descriptions, Modules, OpenFrontend, WindowId};
use crate::catalog::Catalog;
use crate::compose::compose;
use crate::frontend_cell::root_page;
use crate::history::{Attached, Edit, edit_menu_for};
use crate::menus::{line_prefix, menus_of};
use crate::parts::line;
use crate::record_hub::RecordHub;
use crate::records::Records;
use crate::repaint::Repaint;
use crate::status::set_status_line;
use crate::style::StyleCell;
use crate::view::{Coded, View};
use crate::words::words;

/// What the change needs from the owner; the owner holds it in an `Arc`, a menu handler only a `Weak` of it (no cycle through the entry).
pub(super) struct Core {
    pub(super) catalog: Arc<dyn Catalog + Send + Sync>,
    pub(super) locale: Locale,
    pub(super) descriptions: Arc<dyn Descriptions>,
    pub(super) modules: Arc<dyn Modules>,
    pub(super) hub: RecordHub,
    pub(super) serial: AtomicU64,
    pub(super) open: Arc<Mutex<Open>>,
    pub(super) repaint: Arc<dyn Repaint>,
}

/// The rows of the menu `View` of the program `extends`: its frontends, as the port lists them (no table here). A source that cannot
/// list gives only the open one, so the window opens and the menu has no choice.
pub(super) fn rows(descriptions: &dyn Descriptions, key: &FrontendKey) -> Vec<FrontendRow> {
    descriptions
        .list(&key.extends)
        .unwrap_or_else(|_| vec![key.frontend.clone()])
        .into_iter()
        .map(FrontendRow::new)
        .collect()
}

impl Core {
    /// Changes the frontend of the window `id`; any refusal leaves it as it was. The panels of the old frontend fall (their grips sign
    /// off at their modules), the frontend nodes of the new one are built the same way as when it is opened (`inside_of`); the windows
    /// its nodes ask for are returned for the owner to open.
    pub(super) fn switch(
        self_arc: &Arc<Self>,
        id: &WindowId,
        frontend: &str,
    ) -> Result<Vec<OpenFrontend>, Coded> {
        let mut open = lock(&self_arc.open);
        let at = |c: &str| Coded::new(c).with("id", id.as_str());
        let e = open
            .get(id)
            .ok_or_else(|| at("host-notice-window-unknown"))?;
        let key = FrontendKey {
            extends: e.key.extends.clone(),
            frontend: frontend.to_owned(),
        };
        let listed = self_arc.descriptions.list(&e.key.extends).map_err(|c| {
            Coded::new("host-notice-frontend-list-failed")
                .with("extends", e.key.extends.clone())
                .with("cause", c.code)
        })?;
        let rows: Vec<FrontendRow> = listed.into_iter().map(FrontendRow::new).collect();
        if !rows.iter().any(|r| r.id == frontend) {
            let names: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
            return Err(Coded::new("host-notice-frontend-unknown")
                .with("id", frontend)
                .with("available", format!("[{}]", names.join(", "))));
        }
        if key == e.key {
            return Ok(Vec::new());
        }
        let elsewhere = open
            .iter()
            .find(|(k, o)| *k != id && o.parent == e.parent && o.key == key && o.scope == e.scope);
        if let Some((other, _)) = elsewhere {
            return Err(at("host-notice-frontend-open-elsewhere").with("other", other.as_str()));
        }
        let refused = |cause: &Coded| {
            Coded::new("host-notice-frontend-refused")
                .with("extends", key.extends.clone())
                .with("frontend", frontend)
                .with("cause", cause.code.clone())
        };
        let view = connected(self_arc.descriptions.view(&key).map_err(|c| refused(&c))?);
        let texts = words(&self_arc.locale);
        let comp = compose(&view, &*self_arc.catalog);
        let mut above = ancestors(&open, id);
        above.push(key.clone());
        let inside = self_arc.inside_of(id, &comp, &above);
        let page = root_page(
            &texts,
            &self_arc.locale,
            &view,
            &comp,
            e.grip.records(),
            Arc::new(StyleCell::default()),
            &inside.panels,
        );
        if let Some(app) = &e.app {
            let records = e.grip.records();
            Self::install(
                self_arc,
                app,
                e.token,
                &view,
                rows,
                frontend,
                &records,
                &e.attached,
            )
            .map_err(|_| refused(&Coded::new("menu-duplicate")))?;
            set_status_line(app, &texts, &comp, &*records);
        }
        e.cell.set(page);
        let gone: Vec<Entry> = family(&open, id)
            .iter()
            .filter(|d| *d != id)
            .filter_map(|d| open.remove(d))
            .collect();
        let (heads, missing) = (inside.heads(), inside.missing());
        open.extend(inside.children);
        if let Some(e) = open.get_mut(id) {
            e.key = key;
            e.panels = heads;
            e.missing = missing;
        }
        drop(open);
        for app in gone.iter().filter_map(|c| c.app.as_ref()) {
            app.quit(); // a window of the old frontend goes with it
        }
        drop(gone); // outside the lock: the grips sign off at their modules
        self_arc.repaint.wake();
        Ok(inside.windows)
    }

    /// Puts the menus of the window (program, Edit, View), the one handler and the shortcuts into `app`: the lines of the history
    /// are answered with `records`, the lines of the frontends by [`Core::handler`].
    #[allow(clippy::too_many_arguments)] // one call site per way a window is made; a struct of these would only be built twice
    pub(super) fn install(
        core: &Arc<Self>,
        app: &AppWidget,
        token: u64,
        view: &View,
        rows: Vec<FrontendRow>,
        chosen: &str,
        records: &Arc<dyn Records>,
        attached: &Attached,
    ) -> Result<(), AppError> {
        let (view, chosen, locale) = (view.clone(), chosen.to_owned(), core.locale.clone());
        let for_menus = Arc::clone(records);
        let edit = Edit {
            records: Arc::clone(records),
            locale: core.locale.clone(),
            repaint: Arc::clone(&core.repaint),
            menus: Arc::new(move || {
                let edit = edit_menu_for(&*for_menus, &locale);
                menus_of(&words(&locale), &view, &rows, &chosen, edit)
            }),
            inner: Arc::new(Self::handler(core, token)),
            app: attached.cell(),
        };
        edit.install(app)
    }

    /// The handler of the menus of the window with `token`: a line `layout.frontend.<name>` changes the frontend; a refusal is said in
    /// the status line of that window.
    pub(super) fn handler(
        core: &Arc<Self>,
        token: u64,
    ) -> impl Fn(&str) + Send + Sync + 'static + use<> {
        let weak = Arc::downgrade(core);
        move |chosen: &str| {
            let (Some(core), Some(frontend)) =
                (weak.upgrade(), chosen.strip_prefix(line_prefix().as_str()))
            else {
                return;
            };
            let found = {
                let open = lock(&core.open);
                open.iter()
                    .find(|(_, e)| e.token == token)
                    .and_then(|(id, e)| Some((id.clone(), e.app.clone()?)))
            };
            let Some((id, app)) = found else { return };
            match Self::switch(&core, &id, frontend) {
                Ok(windows) => {
                    // the menu has only the shared state and cannot open a window (the owner does, `use_frontend`): until S6-7b
                    // that is said, in the data (`missing_of`) and in the status line, never dropped
                    let texts = words(&core.locale);
                    for w in windows {
                        let why = Coded::new("host-notice-window-not-opened-by-menu")
                            .with("frontend", w.key.frontend.clone());
                        let at = w
                            .parent
                            .as_ref()
                            .map_or_else(String::new, |(_, e)| e.as_str().to_owned());
                        if let Some(e) = lock(&core.open).get_mut(&id) {
                            e.missing.push((at, why.code.clone()));
                            e.missing.sort();
                        }
                        app.set_status(Some(Status::Problem(line(&texts, &why))));
                    }
                }
                Err(refused) => {
                    let sentence = line(&words(&core.locale), &refused);
                    app.set_status(Some(Status::Problem(sentence)));
                }
            }
        }
    }
}

/// The keys of the frontends above `id` (the chain of parents), root first.
fn ancestors(open: &Open, id: &WindowId) -> Vec<FrontendKey> {
    let mut keys = Vec::new();
    let mut at = open.get(id).and_then(|e| e.parent.as_ref());
    while let Some(e) = at.and_then(|p| open.get(p)) {
        keys.push(e.key.clone());
        at = e.parent.as_ref();
    }
    keys.reverse();
    keys
}

impl super::HostWindows {
    /// The rows of the menu `View` of the open window `id` and the open one among them (data only; the shell draws the menu). `None`
    /// when `id` is not open.
    #[must_use]
    pub fn frontend_rows(&self, id: &WindowId) -> Option<(Vec<FrontendRow>, String)> {
        let key = lock(&self.open).get(id)?.key.clone();
        Some((rows(&*self.descriptions, &key), key.frontend))
    }
}
