//! Rows of the `forms_*` tables back to a checked layout. The rows are read into a raw description in the order of their sequence
//! columns and then pass through the same `check` as every other way in (the database holds only checked layouts, but nothing
//! trusts that blindly).
//!
//! Pattern: Adapter (rows to model).

use fs_forms_layout::{
    BindFrom, BindingDoc, BlockDoc, Coded, ElemId, FrontendKey, Item, Layout, LayoutCode,
    LayoutDoc, LayoutId, Limits, OpenCatalog, PageDoc, SchemePolicy, Words,
};

use crate::from_rows_head::{menu, messages, requires, sources};
use crate::from_rows_parts::{actions, views};
use crate::from_rows_tree::{Walk, attrs, places};
use crate::rows::{Row, RowReader, Rows, validate};
use crate::spec::Table;
use crate::tables as t;

pub(crate) fn each<'a>(rows: &'a Rows, table: &'a Table) -> impl Iterator<Item = RowReader<'a>> {
    rows.tables
        .get(table.name)
        .into_iter()
        .flatten()
        .map(move |r: &'a Row| RowReader::new(table, r))
}

pub(crate) fn item<T>(r: &RowReader<'_>, v: T) -> Item<T> {
    Item {
        elem: r.text("elem").and_then(|e| ElemId::parse(&e).ok()),
        line: 0,
        v,
    }
}

pub(crate) fn same(r: &RowReader<'_>, id: &str) -> bool {
    r.text("layout").as_deref() == Some(id)
}

fn limits_off() -> Limits {
    let mut l = Limits::declared();
    // reading rows: the writer already applied the instance policy, so only grammar and the floor are asked again
    l.schemes = SchemePolicy::any_valid();
    (
        l.elements_max,
        l.menu_depth_max,
        l.place_depth_max,
        l.text_bytes_max,
    ) = (u32::MAX, u32::MAX, u32::MAX, u32::MAX);
    l
}

/// The layout with this id.
///
/// # Errors
/// `unknown` when there is no such row; the problems of `check` when the rows do not form a valid layout.
pub fn from_rows(rows: &Rows, id: &LayoutId) -> Result<Layout, Vec<Coded<LayoutCode>>> {
    // every cell must fit its column (type, NULL rule, closed word list) BEFORE anything is read: a word the code does not know is a
    // code, never a substitute value
    let broken = broken_cells(rows);
    if !broken.is_empty() {
        return Err(broken);
    }
    let key = &id.0.0;
    let head = each(rows, &t::LAYOUT).find(|r| r.text("id").as_deref() == Some(key));
    let Some(head) = head else {
        return Err(vec![
            Coded::new(LayoutCode::Unknown)
                .with("what", "layout")
                .with("id", key),
        ]);
    };
    let mut src_problems = Vec::new();
    let mut doc = LayoutDoc {
        format: head.uint("format").unwrap_or(0), // as written; rows always carry their ids: a broken one is `elem-missing`, never replaced by a natural id
        key: FrontendKey {
            extends: head.text("extends").unwrap_or_default(),
            frontend: head.text("frontend").unwrap_or_default(),
        },
        exec: head.text("exec").unwrap_or_default(),
        title_key: head.text("title_key").unwrap_or_default(),
        style: head.text("style"),
        requires: requires(rows, key),
        sources: sources(rows, key, &mut src_problems),
        menu: menu(rows, key, None),
        pages: Vec::new(),
        actions: Vec::new(),
        views: Vec::new(),
        messages: Vec::new(),
        sample: None,
    };
    let mut walk = Walk::default();
    doc.pages = pages(rows, key, &mut walk);
    let nodes = each(rows, &t::NODE).filter(|r| same(r, key)).count();
    let mut problems = walk.problems;
    problems.append(&mut src_problems);
    if nodes != walk.seen {
        problems.push(
            Coded::new(LayoutCode::StoreBroken)
                .with("file", "forms.db")
                .with("cause", "rows")
                .with(
                    "detail",
                    format!(
                        "forms_node: {} of {nodes} rows are not reachable from a page",
                        nodes - walk.seen.min(nodes)
                    ),
                ),
        );
    }
    doc.actions = actions(rows, key);
    doc.views = views(rows, key);
    doc.messages = messages(rows, key);
    if !problems.is_empty() {
        return Err(problems);
    }
    doc.check(&OpenCatalog, &limits_off())
}

fn broken_cells(rows: &Rows) -> Vec<Coded<LayoutCode>> {
    validate(rows)
        .into_iter()
        .map(|d| {
            Coded::new(LayoutCode::StoreBroken)
                .with("file", "forms.db")
                .with("cause", "rows")
                .with("detail", d)
        })
        .collect()
}

fn pages(rows: &Rows, id: &str, walk: &mut Walk) -> Vec<Item<PageDoc>> {
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
            let places = places(rows, id, &name, None, 1, walk);
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
                    places,
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
    let bindings = list
        .iter()
        .map(|r| binding_from(rows, r, &t::BINDING_SCHEME, &at))
        .collect();
    item(
        b,
        BlockDoc {
            place,
            block: b.text("block").unwrap_or_default(),
            source: b.uint("source"),
            title_key: b.text("title_key"),
            text_key: b.text("text_key"),
            bindings,
            set: attrs(rows, id, "block", &b.text("elem").unwrap_or_default()),
        },
    )
}

pub(crate) fn binding_from(
    rows: &Rows,
    r: &RowReader<'_>,
    scheme_table: &Table,
    at: &dyn Fn(&RowReader<'_>) -> bool,
) -> Item<BindingDoc> {
    let name = || r.text("name").unwrap_or_default();
    let from = match r.text("bind_from").as_deref() {
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
    let mut sch: Vec<_> = each(rows, scheme_table)
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
            source: r.uint("source"),
        },
    )
}
