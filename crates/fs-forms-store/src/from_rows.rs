//! Rows of the `forms_*` tables back to a checked layout. The rows are read into a raw description in the order of their sequence
//! columns and then pass through the same `check` as every other way in (the database holds only checked layouts, but nothing
//! trusts that blindly).
//!
//! Pattern: Adapter (rows to model).

use fs_forms_layout::{
    ActionDoc, ActionTarget, BindFrom, BindingDoc, BlockDoc, Coded, ElemId, FrontendKey, Item,
    Layout, LayoutCode, LayoutDoc, LayoutId, Limits, MenuDoc, MenuKind, MenuStandard, MessageDoc,
    OpenCatalog, OrderDoc, PageDoc, Reference, RequiresDoc, SourceDoc, ViewDoc, Words,
};

use crate::rows::{Row, RowReader, Rows};
use crate::spec::Table;
use crate::tables as t;
use crate::to_rows::range_text;

fn each<'a>(rows: &'a Rows, table: &'a Table) -> impl Iterator<Item = RowReader<'a>> {
    rows.tables
        .get(table.name)
        .into_iter()
        .flatten()
        .map(move |r: &'a Row| RowReader::new(table, r))
}

fn item<T>(r: &RowReader<'_>, v: T) -> Item<T> {
    Item {
        elem: r.text("elem").and_then(|e| ElemId::parse(&e).ok()),
        line: 0,
        v,
    }
}

fn same(r: &RowReader<'_>, id: &str) -> bool {
    r.text("layout").as_deref() == Some(id)
}

fn limits_off() -> Limits {
    let mut l = Limits::declared();
    (l.elements_max, l.menu_depth_max, l.text_bytes_max) = (u32::MAX, u32::MAX, u32::MAX);
    l
}

/// The layout with this id.
///
/// # Errors
/// `unknown` when there is no such row; the problems of `check` when the rows do not form a valid layout.
pub fn from_rows(rows: &Rows, id: &LayoutId) -> Result<Layout, Vec<Coded<LayoutCode>>> {
    let key = &id.0.0;
    let head = each(rows, &t::LAYOUT).find(|r| r.text("id").as_deref() == Some(key));
    let Some(head) = head else {
        return Err(vec![
            Coded::new(LayoutCode::Unknown)
                .with("what", "layout")
                .with("id", key),
        ]);
    };
    let mut doc = LayoutDoc {
        format: head.uint("format").unwrap_or(2),
        key: FrontendKey {
            extends: head.text("extends").unwrap_or_default(),
            frontend: head.text("frontend").unwrap_or_default(),
        },
        exec: head.text("exec").unwrap_or_default(),
        title_key: head.text("title_key").unwrap_or_default(),
        style: head.text("style"),
        requires: Vec::new(),
        sources: Vec::new(),
        menu: Vec::new(),
        pages: Vec::new(),
        actions: Vec::new(),
        views: Vec::new(),
        messages: Vec::new(),
    };
    let mut req: Vec<_> = each(rows, &t::REQUIRES).filter(|r| same(r, key)).collect();
    req.sort_by_key(|r| r.int("seq"));
    doc.requires = req
        .iter()
        .map(|r| {
            item(
                r,
                RequiresDoc {
                    range: range_text(r),
                },
            )
        })
        .collect();
    let mut src: Vec<_> = each(rows, &t::SOURCE).filter(|r| same(r, key)).collect();
    src.sort_by_key(|r| r.int("source"));
    doc.sources = src
        .iter()
        .map(|r| {
            let scope = r.text("scope").and_then(|w| Words::from_word(&w));
            item(
                r,
                SourceDoc {
                    range: range_text(r),
                    data_type: r.text("data_type").unwrap_or_default(),
                    scope,
                },
            )
        })
        .collect();
    doc.menu = menu(rows, key, None);
    doc.pages = pages(rows, key);
    doc.actions = actions(rows, key);
    doc.views = views(rows, key);
    let mut msgs: Vec<_> = each(rows, &t::MESSAGE).filter(|r| same(r, key)).collect();
    msgs.sort_by_key(|r| (r.text("key"), r.text("lang")));
    doc.messages = msgs
        .iter()
        .map(|r| {
            let m = MessageDoc {
                key: r.text("key").unwrap_or_default(),
                lang: r.text("lang").unwrap_or_default(),
                message: r.text("message").unwrap_or_default(),
            };
            item(r, m)
        })
        .collect();
    doc.check(&OpenCatalog, &limits_off())
}

fn menu(rows: &Rows, id: &str, parent: Option<i64>) -> Vec<Item<MenuDoc>> {
    let mut list: Vec<_> = each(rows, &t::MENU)
        .filter(|r| same(r, id) && r.int("parent") == parent)
        .collect();
    list.sort_by_key(|r| r.int("seq"));
    list.iter()
        .map(|r| {
            let kind = match (r.text("kind").as_deref(), r.text("standard").as_deref()) {
                (Some("standard"), Some("help")) => MenuKind::Standard(MenuStandard::Help),
                (Some("standard"), _) => MenuKind::Standard(MenuStandard::Program),
                (Some(w), _) => MenuKind::from_word(w).unwrap_or(MenuKind::Menu),
                _ => MenuKind::Menu,
            };
            let command = r.text("command").zip(r.uint("source")).map(|(c, s)| (s, c));
            let node = r.int("node");
            let m = MenuDoc {
                kind,
                label_key: r.text("label_key"),
                command,
                on: r.text("on_what").and_then(|w| Words::from_word(&w)),
                shortcut: r.text("shortcut"),
                view: r.text("view"),
                children: node.map_or_else(Vec::new, |n| menu(rows, id, Some(n))),
            };
            item(r, m)
        })
        .collect()
}

fn pages(rows: &Rows, id: &str) -> Vec<Item<PageDoc>> {
    let mut list: Vec<_> = each(rows, &t::PAGE).filter(|r| same(r, id)).collect();
    list.sort_by_key(|r| r.int("seq"));
    list.iter()
        .map(|p| {
            let name = p.text("page").unwrap_or_default();
            let mut blocks: Vec<_> = each(rows, &t::BLOCK)
                .filter(|r| same(r, id) && r.text("page").as_deref() == Some(&name))
                .collect();
            blocks.sort_by_key(|r| r.int("seq"));
            let blocks = blocks.iter().map(|b| block(rows, id, &name, b)).collect();
            let nav = p
                .text("navigation")
                .and_then(|w| Words::from_word(&w))
                .unwrap_or(fs_forms_layout::Navigation::Single);
            item(
                p,
                PageDoc {
                    page: name,
                    frame: p.text("frame").unwrap_or_default(),
                    navigation: nav,
                    blocks,
                },
            )
        })
        .collect()
}

fn block(rows: &Rows, id: &str, page: &str, b: &RowReader<'_>) -> Item<BlockDoc> {
    let place = b.text("place").unwrap_or_default();
    let at = |r: &RowReader<'_>| {
        same(r, id)
            && r.text("page").as_deref() == Some(page)
            && r.text("place").as_deref() == Some(&place)
    };
    let mut list: Vec<_> = each(rows, &t::BINDING).filter(|r| at(r)).collect();
    list.sort_by_key(|r| (r.text("slot"), r.int("nth")));
    let bindings = list.iter().map(|r| binding(rows, r, &at)).collect();
    item(
        b,
        BlockDoc {
            place,
            block: b.text("block").unwrap_or_default(),
            source: b.uint("source"),
            title_key: b.text("title_key"),
            text_key: b.text("text_key"),
            bindings,
        },
    )
}

fn binding(
    rows: &Rows,
    r: &RowReader<'_>,
    at: &dyn Fn(&RowReader<'_>) -> bool,
) -> Item<BindingDoc> {
    let name = || r.text("name").unwrap_or_default();
    let from = match r.text("from").as_deref() {
        Some("field") => BindFrom::Field {
            path: r.text("path").unwrap_or_default(),
            role: r.text("role").unwrap_or_default(),
        },
        Some("grouping") => BindFrom::Grouping(name()),
        Some("order") => BindFrom::Order(name()),
        Some("command") => BindFrom::Command(name()),
        Some("lock") => BindFrom::Lock,
        Some("condition") => BindFrom::Condition,
        _ => BindFrom::Records,
    };
    let mut sch: Vec<_> = each(rows, &t::BINDING_SCHEME)
        .filter(|s| at(s) && s.text("slot") == r.text("slot") && s.int("nth") == r.int("nth"))
        .collect();
    sch.sort_by_key(|s| s.int("seq"));
    let schemes = (!sch.is_empty()).then(|| {
        sch.iter()
            .map(|s| item(s, s.text("scheme").unwrap_or_default()))
            .collect()
    });
    item(
        r,
        BindingDoc {
            slot: r.text("slot").unwrap_or_default(),
            nth: r.uint("nth").unwrap_or(0),
            from,
            schemes,
        },
    )
}

fn actions(rows: &Rows, id: &str) -> Vec<Item<ActionDoc>> {
    let mut list: Vec<_> = each(rows, &t::ACTION).filter(|r| same(r, id)).collect();
    list.sort_by_key(|r| r.int("action"));
    list.iter()
        .map(|r| {
            let target = match (r.uint("source"), r.text("command"), r.text("workflow")) {
                (Some(source), Some(command), _) => ActionTarget::Command { source, command },
                (_, _, w) => ActionTarget::Workflow(Reference(w.unwrap_or_default())),
            };
            let a = ActionDoc {
                trigger: r
                    .text("trigger")
                    .and_then(|w| Words::from_word(&w))
                    .unwrap_or(fs_forms_layout::Trigger::Button),
                target,
                confirm: r.text("confirm").and_then(|w| Words::from_word(&w)),
            };
            item(r, a)
        })
        .collect()
}

fn views(rows: &Rows, id: &str) -> Vec<Item<ViewDoc>> {
    let mut list: Vec<_> = each(rows, &t::VIEW).filter(|r| same(r, id)).collect();
    list.sort_by_key(|r| (r.text("view"), r.text("owner")));
    list.iter()
        .map(|v| {
            let (name, owner) = (v.text("view"), v.text("owner"));
            let mut ord: Vec<_> = each(rows, &t::VIEW_ORDER)
                .filter(|o| same(o, id) && o.text("view") == name && o.text("owner") == owner)
                .collect();
            ord.sort_by_key(|o| o.int("seq"));
            let orders = ord
                .iter()
                .map(|o| {
                    let d = OrderDoc {
                        by: o
                            .text("by")
                            .and_then(|w| Words::from_word(&w))
                            .unwrap_or(fs_forms_layout::OrderBy::Field),
                        name: o.text("name").unwrap_or_default(),
                        descending: o.int("descending") == Some(1),
                    };
                    item(o, d)
                })
                .collect();
            let doc = ViewDoc {
                view: name.unwrap_or_default(),
                owner: owner.filter(|o| !o.is_empty()).map(Reference),
                grouping: v.text("grouping"),
                orders,
                filter: None,
                search_kind: v.text("search_kind"),
            };
            item(v, doc)
        })
        .collect()
}
