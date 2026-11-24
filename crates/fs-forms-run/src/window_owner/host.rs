//! The real owner of the open frontends, `Window` only (stage 6, O4a): every frontend is a window of its own, opened through the port
//! [`Surfaces`] (Blinc: `open_more`). Its records are a grip of the [`RecordHub`]: the hub is only the read cache, every window holds
//! its OWN connection to the module, watches the record itself and writes only through that connection (A+: two windows are two
//! participants, also when one person has both). It draws nothing itself; the page of a window is `window::app_with`.
//!
//! Pattern: Strategy (the [`WindowOwner`] of the contract) over three ports (Ports and Adapters): [`Descriptions`] (the description of
//! a frontend), [`Modules`] (the module of a program), [`Surfaces`] (a window).
//!
//! Abweichung: the decision part of `open` (place, `exact`, `decide`) is written once more here, as in `FakeWindows`; both follow the
//! same pure functions of `open.rs`, only the entry differs (a widget instead of a counter). Lifting the common core into one
//! engine with a backend is open.

mod ask;
mod entry;
mod ports;
mod switch;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use fs_components_app::AppWidget;
use fs_components_words::Locale;

pub use self::ask::{Asked, OpenAsk};
use self::entry::{Entry, Open, family, lock, snapshot};
pub use self::ports::{BlincSurfaces, Descriptions, Modules, Surfaces};
use self::switch::{Core, rows};
use super::open::{at_place, exact};
use super::{
    Decision, OpenFrontend, Opened, OpenedFrontend, Placement, WhenOpen, WindowId, WindowOwner,
    decide,
};
use crate::catalog::Catalog;
use crate::compose::compose;
use crate::frontend_cell::{FrontendCell, root_page};
use crate::menus::menus_of;
use crate::record_hub::{RecordGrip, RecordHub, RecordKey};
use crate::repaint::Repaint;
use crate::style::StyleCell;
use crate::view::Coded;
use crate::window::status;
use crate::words::words;

/// Pattern: Strategy (the real [`WindowOwner`], `Window` only) – see the module.
pub struct HostWindows {
    catalog: Arc<dyn Catalog + Send + Sync>,
    locale: Locale,
    descriptions: Arc<dyn Descriptions>,
    modules: Arc<dyn Modules>,
    surfaces: Arc<dyn Surfaces>,
    hub: RecordHub,
    open: Arc<Mutex<Open>>,
    core: Arc<Core>,
    serial: AtomicU64,
}

impl HostWindows {
    /// An owner that composes with `catalog`, gets descriptions and modules from the ports and opens its windows through `surfaces`;
    /// `repaint` wakes the window of a view after that view's own `watch` told it of a change.
    #[must_use]
    pub fn new(
        catalog: Arc<dyn Catalog + Send + Sync>,
        locale: Locale,
        ports: (Arc<dyn Descriptions>, Arc<dyn Modules>, Arc<dyn Surfaces>),
        repaint: Arc<dyn Repaint>,
    ) -> Self {
        let (descriptions, modules, surfaces) = ports;
        let open: Arc<Mutex<Open>> = Arc::default();
        let core = Arc::new(Core {
            catalog: Arc::clone(&catalog),
            locale: locale.clone(),
            descriptions: Arc::clone(&descriptions),
            open: Arc::clone(&open),
            repaint: Arc::clone(&repaint),
        });
        Self {
            catalog,
            locale,
            descriptions,
            modules,
            surfaces,
            hub: RecordHub::with_repaint(repaint),
            open,
            core,
            serial: AtomicU64::new(0),
        }
    }

    /// The grip of an open window (its records, cache, seen revision). `None` when not open.
    #[must_use]
    pub fn grip_of(&self, id: &WindowId) -> Option<RecordGrip> {
        lock(&self.open).get(id).map(|e| e.grip.clone())
    }

    /// How many read caches are alive (one per record, however many windows show it).
    #[must_use]
    pub fn records_alive(&self) -> usize {
        self.hub.live()
    }

    fn fresh(
        &self,
        req: &OpenFrontend,
        id: &WindowId,
        parent: Option<WindowId>,
        place: (String, String),
    ) -> Result<Entry, Coded> {
        let refused = |cause: &Coded| {
            Coded::new("host-notice-frontend-refused")
                .with("extends", req.key.extends.clone())
                .with("frontend", req.key.frontend.clone())
                .with("cause", cause.code.clone())
        };
        let view = self.descriptions.view(&req.key)?;
        let module = self
            .modules
            .module(&req.key.extends)
            .map_err(|c| refused(&c))?;
        let key = RecordKey {
            range: req.key.extends.clone(),
            scope: req.scope.clone(),
        };
        let grip = self.hub.attach(&key, &module).map_err(|c| refused(&c))?;
        let comp = compose(&view, &*self.catalog);
        let texts = words(&self.locale);
        let page = root_page(
            &texts,
            &self.locale,
            &view,
            &comp,
            grip.records(),
            Arc::new(StyleCell::default()),
        );
        let cell = FrontendCell::new(page);
        let failed = |_: fs_components_app::AppError| {
            Coded::new("host-notice-window-failed").with("id", id.as_str())
        };
        let title = if view.title.is_empty() {
            texts.plain("host-title")
        } else {
            view.title.clone()
        };
        let app = AppWidget::new(title, vec![cell.screen()]).map_err(failed)?;
        let records = grip.records();
        app.set_status(Some(status(
            &texts,
            comp.state,
            records.is_sample(),
            records.unavailable().as_deref(),
        )));
        let token = self.serial.fetch_add(1, Ordering::SeqCst);
        let menus = menus_of(
            &texts,
            &view,
            &rows(&*self.descriptions, &req.key),
            &req.key.frontend,
        );
        if !menus.is_empty() {
            app.set_menus(menus, Core::handler(&self.core, token))
                .map_err(failed)?;
        }
        Ok(Entry {
            key: req.key.clone(),
            place,
            scope: req.scope.clone(),
            parent,
            grip,
            module,
            app,
            cell,
            raised: 0,
            token,
        })
    }

    /// The window of the entry with `token` is shown; when it is gone, that entry (and its children) fall.
    fn show(&self, token: u64, app: &AppWidget) -> Result<(), Coded> {
        let open = Arc::clone(&self.open);
        self.surfaces.show(
            app.clone(),
            Box::new(move || {
                let doomed: Vec<Entry> = {
                    let mut o = lock(&open);
                    // by token, not by id: a switch moved the entry to another id, a later window may own the old one
                    let ids = match o.iter().find(|(_, e)| e.token == token) {
                        Some((at, _)) => family(&o, at),
                        None => Vec::new(),
                    };
                    ids.iter().filter_map(|d| o.remove(d)).collect()
                };
                drop(doomed); // outside the lock: the grips detach their connections
            }),
        )
    }

    fn switch_here(
        &self,
        open: &mut Open,
        at: &WindowId,
        req: &OpenFrontend,
    ) -> Result<Opened, Coded> {
        let mut e = open.remove(at).expect("at_place names an open entry");
        let key = RecordKey {
            range: e.key.extends.clone(),
            scope: req.scope.clone(),
        };
        if let Err(err) = self.hub.rebind(&e.grip, key, &e.module) {
            open.insert(at.clone(), e);
            return Err(err);
        }
        let id = WindowId::of_record(&e.place.0, &e.place.1, req.scope.as_deref());
        let from = std::mem::replace(&mut e.scope, req.scope.clone());
        for c in open.values_mut().filter(|c| c.parent.as_ref() == Some(at)) {
            c.parent = Some(id.clone());
        }
        open.insert(id.clone(), e);
        Ok(Opened::Switched { id, from })
    }
}

impl WindowOwner for HostWindows {
    fn open(&self, req: OpenFrontend) -> Result<Opened, Coded> {
        if req.mode != Placement::Window {
            return Err(Coded::new("host-notice-placement-unsupported")
                .with("frontend", req.key.frontend.clone()));
        }
        let mut open = lock(&self.open);
        let (program, elem, parent) = match &req.parent {
            Some((pid, elem)) => {
                let p = open.get(pid).ok_or_else(|| {
                    Coded::new("host-notice-window-unknown").with("id", pid.as_str())
                })?;
                (
                    p.key.extends.clone(),
                    elem.as_str().to_owned(),
                    Some(pid.clone()),
                )
            }
            None => (req.key.extends.clone(), req.key.frontend.clone(), None),
        };
        let id = WindowId::of_record(&program, &elem, req.scope.as_deref());
        let now = snapshot(&open);
        let place = (program, elem);
        if let Some(same) = exact(&now, &req, &place).cloned() {
            if let Some(e) = open.get_mut(&same) {
                e.raised += 1; // brought forward by the host (`notice_of` says so), the record stays
            }
            return Ok(Opened::Raised(same));
        }
        match (decide(&now, &req), at_place(&now, &req, &place).cloned()) {
            (Decision::Open(WhenOpen::Switch), Some(at)) => self.switch_here(&mut open, &at, &req),
            (Decision::Ask { at }, _) => {
                Err(Coded::new("host-notice-open-unanswered").with("at", at.as_str()))
            }
            _ => {
                if open.contains_key(&id) {
                    return Err(Coded::new("host-notice-window-id-taken").with("id", id.as_str()));
                }
                let entry = self.fresh(&req, &id, parent, place)?;
                let (app, token) = (entry.app.clone(), entry.token);
                open.insert(id.clone(), entry);
                drop(open);
                if let Err(e) = self.show(token, &app) {
                    drop(lock(&self.open).remove(&id)); // nothing stays that is not shown
                    return Err(e);
                }
                Ok(Opened::New(id))
            }
        }
    }

    fn close(&self, id: &WindowId) -> Result<(), Coded> {
        let gone: Vec<Entry> = {
            let mut open = lock(&self.open);
            if !open.contains_key(id) {
                return Err(Coded::new("host-notice-window-unknown").with("id", id.as_str()));
            }
            let ids = family(&open, id);
            ids.iter().filter_map(|d| open.remove(d)).collect()
        };
        for e in &gone {
            e.app.quit(); // the window goes; its `on_closed` finds nothing left
        }
        Ok(())
    }

    fn place(&self, id: &WindowId, placement: Placement) -> Result<(), Coded> {
        if !lock(&self.open).contains_key(id) {
            return Err(Coded::new("host-notice-window-unknown").with("id", id.as_str()));
        }
        if placement == Placement::Window {
            return Ok(()); // it is one already
        }
        Err(Coded::new("host-notice-placement-unsupported").with("id", id.as_str()))
    }

    fn use_frontend(&self, id: &WindowId, frontend: &str) -> Result<(), Coded> {
        Core::switch(&self.core, id, frontend)
    }

    fn open_now(&self) -> Vec<OpenedFrontend> {
        snapshot(&lock(&self.open))
    }
}
