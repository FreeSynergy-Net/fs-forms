//! Shared by the tests of `HostWindows` (`host_windows.rs`, `host_windows_picture.rs`): doubles of its three ports (descriptions from
//! the fixtures, one fake module, windows kept in a list), the wake counter and an outlet for the question.

#![allow(dead_code)] // each test file uses a part

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use fs_components_app::AppWidget;
use fs_forms_layout::FrontendKey;
use fs_forms_run::WindowOpened as Opened;
use fs_forms_run::{
    Blocks, Coded, Descriptions, FakeModule, FileLink, HoldCounts, HostWindows, LayoutLink, Module,
    Modules, OpenFrontend, Placement, Records, Repaint, SampleBoard, Surfaces, View,
};
use fs_notification::{
    Form, NoteId, Notification, NotificationCode, Notifications, Outlet, Policy,
};

pub const PROGRAM: &str = "net.freesynergy.Tasks";

pub fn key(frontend: &str) -> FrontendKey {
    FrontendKey {
        extends: PROGRAM.into(),
        frontend: frontend.into(),
    }
}

pub struct Files;
impl Descriptions for Files {
    fn view(&self, key: &FrontendKey) -> Result<View, Coded> {
        let root = format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"));
        FileLink::new(root, "en")
            .resolve(&key.extends, &key.frontend)
            .map_err(|_| {
                Coded::new("host-notice-frontend-missing").with("frontend", key.frontend.clone())
            })
    }

    fn list(&self, extends: &str) -> Result<Vec<String>, Coded> {
        let root = format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"));
        FileLink::new(root, "en")
            .available(extends)
            .map_err(|_| Coded::new("host-notice-frontend-missing"))
    }
}

pub struct One(Arc<FakeModule>);
impl Modules for One {
    fn module(&self, _range: &str) -> Result<Arc<dyn Module>, Coded> {
        Ok(Arc::clone(&self.0) as Arc<dyn Module>)
    }
}

/// Windows that are kept in a list; `quit` of a widget runs its `on_closed`, as Blinc does when the window goes.
#[derive(Default)]
pub struct Shown {
    pub widgets: Arc<Mutex<Vec<(usize, AppWidget)>>>,
    pub broken: Mutex<bool>,
}
impl Surfaces for Shown {
    fn show(&self, app: AppWidget, on_closed: Box<dyn FnOnce() + Send>) -> Result<(), Coded> {
        if *self.broken.lock().unwrap() {
            return Err(Coded::new("host-notice-window-failed"));
        }
        // as Blinc: the window goes, its builder (and the widget in it) is dropped, THEN `on_closed` runs
        let once = Mutex::new(Some(on_closed));
        let (list, n) = (
            Arc::clone(&self.widgets),
            self.widgets.lock().unwrap().len() + 100,
        );
        app.set_quit(move || {
            let gone = {
                let mut l = list.lock().unwrap();
                l.iter().position(|(i, _)| *i == n).map(|at| l.remove(at))
            };
            drop(gone);
            if let Some(f) = once.lock().unwrap().take() {
                f();
            }
        });
        self.widgets.lock().unwrap().push((n, app));
        Ok(())
    }
}

#[derive(Default)]
pub struct Wakes(pub AtomicUsize);
impl Repaint for Wakes {
    fn wake(&self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

pub struct Toast(pub Mutex<Vec<NoteId>>);
impl Outlet for Toast {
    fn forms(&self) -> &[Form] {
        &[Form::Modal]
    }
    fn show(
        &self,
        _f: Form,
        id: &NoteId,
        _n: &Notification,
    ) -> Result<(), fs_components_catalog::coded::Coded<NotificationCode>> {
        self.0.lock().unwrap().push(id.clone());
        Ok(())
    }
    fn close(&self, _f: Form, _id: &NoteId) {}
}

pub struct World {
    pub owner: Arc<HostWindows>,
    pub counts: Arc<HoldCounts>,
    pub shown: Arc<Shown>,
    pub wakes: Arc<Wakes>,
}

pub fn world() -> World {
    let shown = Arc::new(Shown::default());
    let (owner, counts, wakes) = world_with(Arc::clone(&shown) as Arc<dyn Surfaces>);
    World {
        owner,
        counts,
        shown,
        wakes,
    }
}

/// The same world over windows of the caller (one that closes late, say).
pub fn world_with(surfaces: Arc<dyn Surfaces>) -> (Arc<HostWindows>, Arc<HoldCounts>, Arc<Wakes>) {
    let counts = Arc::new(HoldCounts::default());
    let boards: Mutex<HashMap<Option<String>, Arc<SampleBoard>>> = Mutex::default();
    let module = FakeModule::new(
        Arc::clone(&counts),
        Box::new(move |scope| {
            let mut m = boards.lock().unwrap();
            Arc::clone(m.entry(scope.map(str::to_owned)).or_default()) as Arc<dyn Records>
        }),
    );
    let wakes = Arc::new(Wakes::default());
    let owner = HostWindows::new(
        Arc::new(Blocks),
        en(),
        (Arc::new(Files), Arc::new(One(module)), surfaces),
        Arc::clone(&wakes) as Arc<dyn Repaint>,
    );
    (Arc::new(owner), counts, wakes)
}

/// Records of a scope, made on demand.
pub type RecordsOf = Box<dyn Fn(Option<&str>) -> Arc<dyn Records> + Send + Sync>;

/// A world over the given description port and records (the rest as `world`).
pub fn world_over(descriptions: Arc<dyn Descriptions>, records: RecordsOf) -> World {
    let shown = Arc::new(Shown::default());
    let counts = Arc::new(HoldCounts::default());
    let module = FakeModule::new(Arc::clone(&counts), records);
    let wakes = Arc::new(Wakes::default());
    let owner = HostWindows::new(
        Arc::new(Blocks),
        en(),
        (
            descriptions,
            Arc::new(One(module)),
            Arc::clone(&shown) as Arc<dyn Surfaces>,
        ),
        Arc::clone(&wakes) as Arc<dyn Repaint>,
    );
    World {
        owner: Arc::new(owner),
        counts,
        shown,
        wakes,
    }
}

pub fn req(frontend: &str, scope: &str) -> OpenFrontend {
    OpenFrontend::new(key(frontend), Placement::Window).with_scope(scope)
}

pub fn notes() -> (Notifications, Arc<Toast>) {
    let toast = Arc::new(Toast(Mutex::default()));
    (
        Notifications::new(
            Policy::default(),
            vec![Arc::clone(&toast) as Arc<dyn Outlet>],
        ),
        toast,
    )
}

pub type Seen = Arc<Mutex<Vec<Result<Opened, Coded>>>>;

pub type Done = Arc<dyn Fn(Result<Opened, Coded>) + Send + Sync>;

pub fn done() -> (Seen, Done) {
    let seen: Seen = Arc::default();
    let sink = Arc::clone(&seen);
    (seen, Arc::new(move |r| sink.lock().unwrap().push(r)))
}

/// The language of the test windows: English, whatever `LANG` of the machine says (the tests read sentences, not codes).
pub fn en() -> fs_components_words::Locale {
    fs_components_words::Locale::from_posix("en").expect("a locale")
}
