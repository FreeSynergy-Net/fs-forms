//! Shared helpers of the `WindowOwner` conformance tests (`window_owner.rs`, `window_owner_place.rs`).

#![allow(dead_code)] // each test file uses a part of these helpers

use fs_forms_layout::{ElemId, FrontendKey};
use fs_forms_run::{
    FakeWindows, MenuItem, MenuState, OpenFrontend, PageView, PlaceView, Placement, TestCatalog,
    View, WindowId, WindowOpened as Opened, WindowOwner,
};

pub fn key(extends: &str, frontend: &str) -> FrontendKey {
    FrontendKey {
        extends: extends.into(),
        frontend: frontend.into(),
    }
}

pub fn tasks() -> FrontendKey {
    key("net.freesynergy.Tasks", "board")
}

pub fn settings() -> FrontendKey {
    key("net.freesynergy.Settings", "settings")
}

pub fn view(title: &str, block: &str) -> View {
    View {
        wants: Vec::new(),
        feeds: Vec::new(),
        frontend: title.into(),
        title: title.into(),
        style: None,
        pages: vec![PageView {
            page: "main".into(),
            seq: 1,
            frame: "workbench".into(),
            locks: Vec::new(),
            places: vec![PlaceView {
                title: None,
                place: "form".into(),
                seq: 1,
                block: block.into(),
                locks: Vec::new(),
                slot_locks: Vec::new(),
                info: fs_forms_run::PlaceInfo::default(),
                embed: None,
            }],
        }],
        menu: vec![MenuItem {
            id: "close".into(),
            label: "Close".into(),
            state: MenuState::Coming,
        }],
        notices: Vec::new(),
        actions: Vec::new(),
        sources: Vec::new(),
    }
}

pub fn fake() -> FakeWindows {
    FakeWindows::new(TestCatalog::building(&["workbench", "field-card", "board"]))
        .knowing(tasks(), view("Tasks", "board"))
        .knowing(settings(), view("Settings", "field-card"))
}

/// The id an `open` gave, whatever it did.
pub fn id(o: Opened) -> WindowId {
    match o {
        Opened::New(id) | Opened::Raised(id) | Opened::Switched { id, .. } => id,
        _ => unreachable!("unknown Opened"),
    }
}

pub fn elem(text: &str) -> ElemId {
    ElemId::parse(text).unwrap()
}

pub fn root(owner: &FakeWindows) -> WindowId {
    id(owner
        .open(OpenFrontend::new(tasks(), Placement::Window).with_switchable(true))
        .unwrap())
}

pub fn child(owner: &FakeWindows, parent: &WindowId, at: &str, mode: Placement) -> WindowId {
    id(owner
        .open(
            OpenFrontend::new(settings(), mode)
                .with_parent(parent.clone(), elem(at))
                .with_switchable(true),
        )
        .unwrap())
}
