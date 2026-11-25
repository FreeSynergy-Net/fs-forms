//! Shared by the tests of S6-7 (`host_embed.rs`, `host_embed_picture.rs`): the board of `net.freesynergy.Tasks` with a frontend node that
//! places the frontend of `net.freesynergy.Settings`; two fake modules, each counting its own held connections.
//!
//! Pattern: Fake over the three ports of `HostWindows`.

#![allow(dead_code)] // each test file uses a part

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use fs_components_app::AppWidget;
use fs_forms_layout::FrontendKey;
use fs_forms_run::{
    Blocks, Coded, Descriptions, FakeModule, FileLink, FormRecords, HoldCounts, HostWindows,
    LayoutLink, Module, Modules, OpenFrontend, Placement, Records, Repaint, SampleBoard, Surfaces,
    View, WindowId, WindowOpened, WindowOwner, sample_groups,
};

pub const TASKS: &str = "net.freesynergy.Tasks";
pub const SETTINGS: &str = "net.freesynergy.Settings";

pub struct Dir(PathBuf);
impl Descriptions for Dir {
    fn view(&self, key: &FrontendKey) -> Result<View, Coded> {
        FileLink::new(&self.0, "en")
            .resolve(&key.extends, &key.frontend)
            .map_err(|_| {
                Coded::new("host-notice-frontend-missing").with("frontend", key.frontend.clone())
            })
    }
    fn list(&self, extends: &str) -> Result<Vec<String>, Coded> {
        FileLink::new(&self.0, "en")
            .available(extends)
            .map_err(|_| Coded::new("host-notice-frontend-missing"))
    }
}

/// One module for each program; each counts its own connections.
pub struct Two(HashMap<String, Arc<FakeModule>>);
impl Modules for Two {
    fn module(&self, range: &str) -> Result<Arc<dyn Module>, Coded> {
        self.0
            .get(range)
            .map(|m| Arc::clone(m) as Arc<dyn Module>)
            .ok_or_else(|| Coded::new("host-notice-module-missing").with("range", range))
    }
}

#[derive(Default)]
pub struct Shown(pub Arc<Mutex<Vec<(usize, AppWidget)>>>);
impl Surfaces for Shown {
    fn show(&self, app: AppWidget, on_closed: Box<dyn FnOnce() + Send>) -> Result<(), Coded> {
        // as Blinc: the window goes, its builder (and the widget in it) is dropped, THEN `on_closed` runs
        let once = Mutex::new(Some(on_closed));
        let (list, n) = (Arc::clone(&self.0), self.0.lock().unwrap().len() + 100);
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
        self.0.lock().unwrap().push((n, app));
        Ok(())
    }
}

pub struct Quiet;
impl Repaint for Quiet {
    fn wake(&self) {}
}

pub struct World {
    pub owner: HostWindows,
    pub tasks: Arc<HoldCounts>,
    pub settings: Arc<HoldCounts>,
    pub shown: Arc<Shown>,
    pub _dir: tempfile::TempDir,
}

/// The fixture `embed/` with the mode of the node set to `mode`.
pub fn world(mode: &str) -> World {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/embed");
    let dir = tempfile::tempdir().unwrap();
    for f in ["board-embed.toml", "settings.toml"] {
        let text = std::fs::read_to_string(src.join(f)).unwrap();
        let text = text.replace("mode = \"panel\"", &format!("mode = \"{mode}\""));
        std::fs::write(dir.path().join(f), text).unwrap();
    }
    let (tasks, settings) = (
        Arc::new(HoldCounts::default()),
        Arc::new(HoldCounts::default()),
    );
    let boards: Mutex<HashMap<Option<String>, Arc<SampleBoard>>> = Mutex::default();
    let groups: Arc<dyn Records> = Arc::new(FormRecords {
        provider: "fake".into(),
        forms: sample_groups(),
        ..FormRecords::default()
    });
    let modules = HashMap::from([
        (
            TASKS.to_owned(),
            FakeModule::new(
                Arc::clone(&tasks),
                Box::new(move |scope| {
                    let mut m = boards.lock().unwrap();
                    Arc::clone(m.entry(scope.map(str::to_owned)).or_default()) as Arc<dyn Records>
                }),
            ),
        ),
        (
            SETTINGS.to_owned(),
            FakeModule::new(
                Arc::clone(&settings),
                Box::new(move |_| Arc::clone(&groups)),
            ),
        ),
    ]);
    let shown = Arc::new(Shown::default());
    let owner = HostWindows::new(
        Arc::new(Blocks),
        fs_forms_run::session_locale(),
        (
            Arc::new(Dir(dir.path().to_owned())),
            Arc::new(Two(modules)),
            Arc::clone(&shown) as Arc<dyn Surfaces>,
        ),
        Arc::new(Quiet),
    );
    World {
        owner,
        tasks,
        settings,
        shown,
        _dir: dir,
    }
}

pub fn open_board(w: &World) -> WindowId {
    let key = FrontendKey {
        extends: TASKS.into(),
        frontend: "board-embed".into(),
    };
    match w
        .owner
        .open(OpenFrontend::new(key, Placement::Window).with_scope("urn:a"))
    {
        Ok(WindowOpened::New(id)) => id,
        other => panic!("a new window: {other:?}"),
    }
}

pub fn child_id() -> WindowId {
    WindowId::of(TASKS, "settings-embed")
}
