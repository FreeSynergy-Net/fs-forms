//! The model to the canonical tree (always format 2). Order: lists in the order of the model (= their `seq`), bindings by
//! (slot in byte order, nth), scheme lists NEVER sorted (suggestion order), messages by (key, lang) in byte order.
//! Views of a person (with `owner`) are left out unless asked for: they are the person's data, not the package's.
//!
//! Pattern: Adapter (model to tree).

use fs_forms_layout::{
    ActionTarget, BindFrom, BindingDoc, BlockDoc, Coded, Item, Layout, LayoutCode, MenuDoc, PageDoc,
};

use crate::tree::Tree;
use crate::words::Words;

type Pairs = Vec<(&'static str, Tree)>;

fn elem<T>(i: &Item<T>) -> (&'static str, Tree) {
    ("elem", Tree::s(i.elem.as_ref().map_or("", |e| e.as_str())))
}
fn int(v: u32) -> Tree {
    Tree::Int(i64::from(v))
}
fn opt(p: &mut Pairs, k: &'static str, v: Option<&String>) {
    if let Some(v) = v {
        p.push((k, Tree::s(v)));
    }
}
fn list<T>(items: &[Item<T>], f: impl Fn(&Item<T>) -> Tree) -> Tree {
    Tree::Arr(items.iter().map(f).collect())
}
fn obj(p: Pairs) -> Tree {
    Tree::obj(p)
}

/// The tree of a layout. `own_views`: include views that have an owner.
pub(crate) fn layout_tree(l: &Layout, own_views: bool) -> Result<Tree, Coded<LayoutCode>> {
    let d = l.doc();
    let mut p: Pairs = vec![
        ("schema", Tree::Int(2)),
        ("kind", Tree::s("frontend")),
        ("id", Tree::s(&d.key.frontend)),
        ("extends", Tree::s(&d.key.extends)),
        ("exec", Tree::s(&d.exec)),
        ("title", Tree::s(&d.title_key)),
    ];
    opt(&mut p, "style", d.style.as_ref());
    p.push((
        "requires",
        list(&d.requires, |i| {
            obj(vec![elem(i), ("contract", Tree::s(&i.v.range))])
        }),
    ));
    let sources = d.sources.iter().enumerate().map(|(n, i)| {
        let mut q = vec![
            elem(i),
            ("n", int(u32::try_from(n + 1).unwrap_or(u32::MAX))),
            ("contract", Tree::s(&i.v.range)),
            ("type", Tree::s(&i.v.data_type)),
        ];
        if let Some(s) = i.v.scope {
            q.push(("scope", Tree::s(s.word())));
        }
        obj(q)
    });
    p.push(("source", Tree::Arr(sources.collect())));
    p.push(("menu", list(&d.menu, menu)));
    p.push(("page", list(&d.pages, page)));
    p.push((
        "action",
        list(&d.actions, |i| {
            let mut q = vec![elem(i), ("trigger", Tree::s(i.v.trigger.word()))];
            match &i.v.target {
                ActionTarget::Command { source, command } => {
                    q.push(("source", int(*source)));
                    q.push(("command", Tree::s(command)));
                }
                ActionTarget::Workflow(w) => q.push(("workflow", Tree::s(&w.0))),
            }
            if let Some(c) = i.v.confirm {
                q.push(("confirm", Tree::s(c.word())));
            }
            obj(q)
        }),
    ));
    let mut views = Vec::new();
    for v in d.views.iter().filter(|v| own_views || v.v.owner.is_none()) {
        if v.v.filter.is_some() {
            return Err(Coded::new(LayoutCode::ExportFailed)
                .with("file", "")
                .with("kind", "filter-not-supported-yet"));
        }
        let mut q = vec![elem(v), ("view", Tree::s(&v.v.view))];
        opt(&mut q, "owner", v.v.owner.as_ref().map(|o| &o.0));
        opt(&mut q, "grouping", v.v.grouping.as_ref());
        opt(&mut q, "search", v.v.search_kind.as_ref());
        q.push((
            "order",
            list(&v.v.orders, |o| {
                obj(vec![
                    elem(o),
                    ("by", Tree::s(o.v.by.word())),
                    ("name", Tree::s(&o.v.name)),
                    ("descending", Tree::Bool(o.v.descending)),
                ])
            }),
        ));
        views.push(obj(q));
    }
    p.push(("view", Tree::Arr(views)));
    let mut msgs: Vec<&Item<_>> = d.messages.iter().collect();
    msgs.sort_by(|a, b| {
        (a.v.key.as_bytes(), a.v.lang.as_bytes()).cmp(&(b.v.key.as_bytes(), b.v.lang.as_bytes()))
    });
    let msgs = msgs.into_iter().map(|i| {
        obj(vec![
            elem(i),
            ("key", Tree::s(&i.v.key)),
            ("lang", Tree::s(&i.v.lang)),
            ("text", Tree::s(&i.v.message)),
        ])
    });
    p.push(("message", Tree::Arr(msgs.collect())));
    Ok(obj(p))
}

fn menu(i: &Item<MenuDoc>) -> Tree {
    let m = &i.v;
    let mut q = vec![elem(i), ("kind", Tree::s(m.kind.word()))];
    opt(&mut q, "label", m.label_key.as_ref());
    if let Some((s, c)) = &m.command {
        q.push(("source", int(*s)));
        q.push(("command", Tree::s(c)));
    }
    if let Some(o) = m.on {
        q.push(("on", Tree::s(o.word())));
    }
    opt(&mut q, "key", m.shortcut.as_ref());
    opt(&mut q, "view", m.view.as_ref());
    q.push(("entries", list(&m.children, menu)));
    obj(q)
}

fn page(i: &Item<PageDoc>) -> Tree {
    let p = &i.v;
    obj(vec![
        elem(i),
        ("page", Tree::s(&p.page)),
        ("frame", Tree::s(&p.frame)),
        ("navigation", Tree::s(p.navigation.word())),
        ("block", list(&p.blocks, block)),
    ])
}

fn block(i: &Item<BlockDoc>) -> Tree {
    let b = &i.v;
    let mut q = vec![
        elem(i),
        ("place", Tree::s(&b.place)),
        ("block", Tree::s(&b.block)),
    ];
    if let Some(s) = b.source {
        q.push(("source", int(s)));
    }
    opt(&mut q, "title", b.title_key.as_ref());
    opt(&mut q, "text", b.text_key.as_ref());
    let mut binds: Vec<&Item<BindingDoc>> = b.bindings.iter().collect();
    binds.sort_by(|x, y| (x.v.slot.as_bytes(), x.v.nth).cmp(&(y.v.slot.as_bytes(), y.v.nth)));
    q.push(("bind", Tree::Arr(binds.into_iter().map(binding).collect())));
    obj(q)
}

fn binding(i: &Item<BindingDoc>) -> Tree {
    let b = &i.v;
    let mut q = vec![elem(i), ("slot", Tree::s(&b.slot)), ("nth", int(b.nth))];
    match &b.from {
        BindFrom::Records => {}
        BindFrom::Field { path, role } => {
            q.push(("field", Tree::s(path)));
            q.push(("role", Tree::s(role)));
        }
        BindFrom::Grouping(g) => q.push(("grouping", Tree::s(g))),
        BindFrom::Order(o) => q.push(("order", Tree::s(o))),
        BindFrom::Command(c) => q.push(("command", Tree::s(c))),
        BindFrom::Lock => q.push(("lock", Tree::Bool(true))),
        BindFrom::Condition => q.push(("condition", Tree::Bool(true))),
    }
    if let Some(s) = &b.schemes {
        q.push((
            "schemes",
            Tree::Arr(
                s.iter()
                    .map(|x| obj(vec![elem(x), ("scheme", Tree::s(&x.v))]))
                    .collect(),
            ),
        ));
    }
    obj(q)
}
