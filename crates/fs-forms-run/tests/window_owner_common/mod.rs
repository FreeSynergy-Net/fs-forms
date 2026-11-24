//! Shared helpers of the `WindowOwner` conformance tests (`window_owner.rs`, `window_owner_place.rs`).

#![allow(dead_code)] // each test file uses a part of these helpers

use fs_forms_layout::{ElemId, FrontendKey};
use fs_forms_run::{
    FakeWindows, MenuItem, MenuState, OpenFrontend, PageView, PlaceView, Placement, TestCatalog,
    View, WindowId, WindowOwner,
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

pub fn elem(text: &str) -> ElemId {
    ElemId::parse(text).unwrap()
}

pub fn root(owner: &FakeWindows) -> WindowId {
    owner
        .open(OpenFrontend {
            key: tasks(),
            mode: Placement::Window,
            parent: None,
            switchable: true,
            scope: None,
        })
        .unwrap()
}

pub fn child(owner: &FakeWindows, parent: &WindowId, at: &str, mode: Placement) -> WindowId {
    owner
        .open(OpenFrontend {
            key: settings(),
            mode,
            parent: Some((parent.clone(), elem(at))),
            switchable: true,
            scope: None,
        })
        .unwrap()
}
