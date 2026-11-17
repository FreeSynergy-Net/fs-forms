//! Shared test helpers: a tiny executor (the port futures are always ready), builders for a small description, a store.
#![allow(dead_code)]

use std::cell::Cell;
use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

use fs_forms_layout::*;

struct Noop;
impl Wake for Noop {
    fn wake(self: Arc<Self>) {}
}

pub fn block_on<F: Future>(f: F) -> F::Output {
    let waker = Waker::from(Arc::new(Noop));
    let mut cx = Context::from_waker(&waker);
    let mut f = pin!(f);
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

pub fn who() -> Who {
    let r = Reference("https://example.org/people/test".to_owned());
    Who {
        actor: r.clone(),
        author: r,
    }
}
pub fn elem(s: &str) -> ElemId {
    ElemId::parse(s).expect("valid elem")
}
pub fn item<T>(e: &str, v: T) -> Item<T> {
    Item {
        elem: Some(elem(e)),
        line: 0,
        v,
    }
}
pub fn bare<T>(v: T) -> Item<T> {
    Item {
        elem: None,
        line: 7,
        v,
    }
}
pub fn pkg(version: &str) -> PackageRef {
    PackageRef {
        package: "net.example.pkg".to_owned(),
        version: version.to_owned(),
    }
}

/// One page `board`, one block `main`, one binding `title`, one message; format 2 with every elem.
pub fn doc(frontend: &str) -> LayoutDoc {
    let binding = BindingDoc {
        slot: "title".into(),
        nth: 0,
        from: BindFrom::Field {
            path: "title".into(),
            role: "display".into(),
        },
        schemes: None,
    };
    let block = BlockDoc {
        place: "main".into(),
        block: "board".into(),
        source: None,
        title_key: None,
        text_key: None,
        bindings: vec![item("binding:board/main/title/0", binding)],
    };
    let page = PageDoc {
        page: "board".into(),
        frame: "workbench".into(),
        navigation: Navigation::Single,
        blocks: vec![item("block:board/main", block)],
    };
    LayoutDoc {
        format: 2,
        key: FrontendKey {
            extends: "net.example.Tasks".into(),
            frontend: frontend.into(),
        },
        exec: "fs-forms-run".into(),
        title_key: "board-title".into(),
        style: None,
        requires: vec![],
        sources: vec![],
        menu: vec![],
        pages: vec![item("page:board", page)],
        actions: vec![],
        views: vec![],
        messages: vec![item(
            "message:board-title/de",
            MessageDoc {
                key: "board-title".into(),
                lang: "de".into(),
                message: "Aufgaben".into(),
            },
        )],
    }
}

pub fn store() -> MemoryLayoutStore {
    let t = Cell::new(1_000_i64);
    let clock = move || {
        t.set(t.get() + 10);
        Utc(t.get())
    };
    MemoryLayoutStore::new(Box::new(OpenCatalog), Limits::declared(), who().actor)
        .with_clock(Box::new(clock))
}
pub fn key(frontend: &str) -> FrontendKey {
    FrontendKey {
        extends: "net.example.Tasks".into(),
        frontend: frontend.into(),
    }
}
pub fn block_ref() -> ElemRef {
    ElemRef {
        kind: ElemKind::Block,
        elem: elem("block:board/main"),
    }
}
pub fn set_text(field: Field, to: &str) -> OpKind {
    OpKind::Set {
        target: block_ref(),
        field,
        value: Value::Text(to.into()),
        expect: Value::Text("wrong".into()),
    }
}
/// Installs `doc(frontend)` as the package version and returns the layout id.
pub fn install(s: &MemoryLayoutStore, d: LayoutDoc, version: &str) -> LayoutId {
    let r = block_on(s.install(d, &pkg(version), Mode::Apply, &who())).expect("install");
    assert!(r.problems.is_empty(), "{:?}", r.problems);
    r.layout.expect("id")
}
