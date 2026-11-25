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
    LayoutLink, Module, Modules, OpenFrontend, Placement, Records, Repaint, Surfaces, View,
    WindowId, WindowOpened, WindowOwner,
};

// The data of the groups module standing in for the settings data: made from a wish by the generator (package 5b).
#[path = "../sample_source/mod.rs"]
mod sample_source;
pub use sample_source::groups;

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
        let text = std::fs::read_to_string(src.join(f))
            .unwrap()
            .replace("\r\n", "\n"); // a Windows checkout has CRLF
        let text = text.replace("mode = \"panel\"", &format!("mode = \"{mode}\""));
        std::fs::write(dir.path().join(f), &text).unwrap();
        if f == "board-embed.toml" {
            for (id, text) in variants(&text) {
                std::fs::write(dir.path().join(format!("{id}.toml")), text).unwrap();
            }
        }
    }
    let (tasks, settings) = (
        Arc::new(HoldCounts::default()),
        Arc::new(HoldCounts::default()),
    );
    let boards: Mutex<HashMap<Option<String>, Arc<dyn Records>>> = Mutex::default();
    let groups: Arc<dyn Records> = Arc::new(FormRecords {
        provider: "fake".into(),
        forms: groups(),
        ..FormRecords::default()
    });
    let modules = HashMap::from([
        (
            TASKS.to_owned(),
            FakeModule::new(
                Arc::clone(&tasks),
                Box::new(move |scope| {
                    let mut m = boards.lock().unwrap();
                    Arc::clone(
                        m.entry(scope.map(str::to_owned))
                            .or_insert_with(sample_source::board),
                    )
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

const NODE: &str = "frontend = { extends = \"net.freesynergy.Settings\", frontend = \"settings\" }";

/// A board like `board-embed` (mode already set) with another id and another target of its node `settings-embed`.
fn board(base: &str, id: &str, target: Option<&str>) -> String {
    let text = base.replace("id = \"board-embed\"", &format!("id = \"{id}\""));
    #[allow(clippy::single_match_else)]
    // the two arms are two different texts, not a pattern and its rest
    match target {
        Some(t) => text.replace(
            NODE,
            &format!("frontend = {{ extends = \"{TASKS}\", frontend = \"{t}\" }}"),
        ),
        None => {
            // the node removed: a board with nothing inside it
            let from = text
                .find("[[page.place]]\nelem = \"settings-embed\"")
                .unwrap();
            let to = from
                + text[from..].find("shown = \"start\"\n").unwrap()
                + "shown = \"start\"\n".len();
            format!("{}{}", &text[..from], &text[to..])
        }
    }
}

/// The variants of the board the tests of the second round need: `board-plain` (no node), `board-loop` (A to A), `board-a` and
/// `board-b` (A to B to A), `chain-1` .. `chain-5` (five levels), `board-scope-from`, `board-on-open`.
fn variants(base: &str) -> Vec<(String, String)> {
    let mut all = vec![
        ("board-plain".to_owned(), board(base, "board-plain", None)),
        (
            "board-loop".to_owned(),
            board(base, "board-loop", Some("board-loop")),
        ),
        (
            "board-a".to_owned(),
            board(base, "board-a", Some("board-b")),
        ),
        (
            "board-b".to_owned(),
            board(base, "board-b", Some("board-a")),
        ),
        (
            "board-scope-from".to_owned(),
            board(base, "board-scope-from", Some("board-plain")).replace(
                "shown = \"start\"\n",
                "shown = \"start\"\nscope-from = \"block:board/main\"\n",
            ),
        ),
        (
            "board-on-open".to_owned(),
            board(base, "board-on-open", Some("board-plain"))
                .replace("shown = \"start\"", "shown = \"on-open\""),
        ),
    ];
    for n in 1..=5 {
        let next = format!("chain-{}", n + 1);
        let target = (n < 5).then_some(next.as_str());
        all.push((
            format!("chain-{n}"),
            board(base, &format!("chain-{n}"), target),
        ));
    }
    all
}

pub fn open_frontend(
    w: &World,
    frontend: &str,
    scope: Option<&str>,
) -> Result<WindowOpened, Coded> {
    let key = FrontendKey {
        extends: TASKS.into(),
        frontend: frontend.into(),
    };
    let mut req =
        OpenFrontend::new(key, Placement::Window).with_when_open(fs_forms_run::WhenOpen::Window);
    if let Some(s) = scope {
        req = req.with_scope(s);
    }
    w.owner.open(req)
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

pub fn child_id(parent: &WindowId) -> WindowId {
    WindowId::of(parent.as_str(), "settings-embed")
}
