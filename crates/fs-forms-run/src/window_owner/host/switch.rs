//! The change of the frontend of an open window (`WindowOwner::use_frontend`) and what a click on the menu `View` does: the same
//! function, found by the token of the window. The window, its id, its record, its records and its held connection stay; the page in
//! the [`FrontendCell`] and the menus are exchanged, the key of the entry is the new one.
//!
//! Pattern: Bridge (the surface is the abstraction, the frontend its implementation) over the same ports as the owner.
//! Abweichung: the port lists the frontends of ONE program, so a name of another program is answered `frontend-unknown{available}`
//! (not `frontend-other-program`, which only the double can tell); the state of the person (selection of a board) is in the mounts of the
//! old page and does not move to the new one (a rule for open input does not exist in the host yet).

use std::sync::{Arc, Mutex};

use fs_components_app::Status;
use fs_components_commands::FrontendRow;
use fs_components_words::Locale;
use fs_forms_layout::FrontendKey;

use super::entry::{Open, lock};
use super::{Descriptions, WindowId};
use crate::catalog::Catalog;
use crate::compose::compose;
use crate::frontend_cell::root_page;
use crate::menus::{line_prefix, menus_of};
use crate::parts::line;
use crate::repaint::Repaint;
use crate::status::set_status_line;
use crate::style::StyleCell;
use crate::view::Coded;
use crate::words::words;

/// What the change needs from the owner; the owner holds it in an `Arc`, a menu handler only a `Weak` of it (no cycle through the entry).
pub(super) struct Core {
    pub(super) catalog: Arc<dyn Catalog + Send + Sync>,
    pub(super) locale: Locale,
    pub(super) descriptions: Arc<dyn Descriptions>,
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
    /// Changes the frontend of the window `id`; any refusal leaves it as it was.
    pub(super) fn switch(self_arc: &Arc<Self>, id: &WindowId, frontend: &str) -> Result<(), Coded> {
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
            return Ok(());
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
        let view = self_arc.descriptions.view(&key).map_err(|c| refused(&c))?;
        let texts = words(&self_arc.locale);
        let comp = compose(&view, &*self_arc.catalog);
        let page = root_page(
            &texts,
            &self_arc.locale,
            &view,
            &comp,
            e.grip.records(),
            Arc::new(StyleCell::default()),
        );
        let menus = menus_of(&texts, &view, &rows, frontend);
        e.app
            .set_menus(menus, Self::handler(self_arc, e.token))
            .map_err(|_| refused(&Coded::new("menu-duplicate")))?;
        let records = e.grip.records();
        set_status_line(&e.app, &texts, &comp, &*records);
        e.cell.set(page);
        if let Some(e) = open.get_mut(id) {
            e.key = key;
        }
        drop(open);
        self_arc.repaint.wake();
        Ok(())
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
                    .map(|(id, e)| (id.clone(), e.app.clone()))
            };
            if let Some((id, app)) = found
                && let Err(refused) = Self::switch(&core, &id, frontend)
            {
                let sentence = line(&words(&core.locale), &refused);
                app.set_status(Some(Status::Problem(sentence)));
            }
        }
    }
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
