//! The model to the canonical tree (always format 3). Order: lists in the order of the model (= their `seq`), bindings by
//! (slot in byte order, nth), scheme lists NEVER sorted (suggestion order), messages by (key, lang) in byte order.
//! Views of a person (with `owner`) are left out unless asked for: they are the person's data, not the package's.
//!
//! Pattern: Adapter (model to tree).

use fs_forms_layout::{
    ActionTarget, BindFrom, BindingDoc, Coded, Item, Layout, LayoutCode, MenuDoc, PageDoc,
};

use crate::tree::Tree;
use crate::write_place::places;
use fs_forms_layout::Words;

pub(crate) type Pairs = Vec<(&'static str, Tree)>;

pub(crate) fn elem<T>(i: &Item<T>) -> (&'static str, Tree) {
    ("elem", Tree::s(i.elem.as_ref().map_or("", |e| e.as_str())))
}
pub(crate) fn int(v: u32) -> Tree {
    Tree::Int(i64::from(v))
}
pub(crate) fn opt(p: &mut Pairs, k: &'static str, v: Option<&String>) {
    if let Some(v) = v {
        p.push((k, Tree::s(v)));
    }
}
pub(crate) fn list<T>(items: &[Item<T>], f: impl Fn(&Item<T>) -> Tree) -> Tree {
    Tree::Arr(items.iter().map(f).collect())
}
pub(crate) fn obj(p: Pairs) -> Tree {
    Tree::obj(p)
}

#[allow(clippy::many_single_char_names)] // l, d, p, q, v, i: the layout, its doc, the pairs of the table being built
/// The tree of a layout. `own_views`: include views that have an owner.
pub(crate) fn layout_tree(l: &Layout, own_views: bool) -> Result<Tree, Coded<LayoutCode>> {
    let d = l.doc();
    let mut p: Pairs = vec![
        ("schema", Tree::Int(3)),
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
    let pages = d.pages.iter().map(page).collect::<Result<Vec<_>, _>>()?;
    p.push(("page", Tree::Arr(pages)));
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
                ActionTarget::Open { elem } => q.push(("open", Tree::s(elem.as_str()))),
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
    // messages are in (key, lang) order by the invariant of `Layout`
    let msgs = d.messages.iter().map(|i| {
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

fn page(i: &Item<PageDoc>) -> Result<Tree, Coded<LayoutCode>> {
    let p = &i.v;
    Ok(obj(vec![
        elem(i),
        ("page", Tree::s(&p.page)),
        ("frame", Tree::s(&p.frame)),
        ("navigation", Tree::s(p.navigation.word())),
        ("place", places(p)?),
    ]))
}

pub(crate) fn binding(i: &Item<BindingDoc>) -> Tree {
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
