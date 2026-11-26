//! Rows of the menu tree, the pages with their blocks and bindings, and the views: the parts of a layout that nest.
//!
//! Pattern: Adapter (model to rows), recursive for the menu tree.

use fs_forms_layout::{
    ActionTarget, BindFrom, Coded, Item, Layout, LayoutCode, MenuDoc, MenuKind, MenuStandard, Words,
};

use crate::rows::{Cell, RowBuilder, Rows, int, opt_int, opt_text, text};
use crate::spec::Table;
use crate::tables as t;
use crate::to_rows::elem;
use crate::to_rows_tree::{attr_rows, node_rows};

pub(crate) fn menu_rows(
    items: &[Item<MenuDoc>],
    parent: Option<u32>,
    id: &Cell,
    node: &mut u32,
    rows: &mut Rows,
) {
    for (seq, m) in items.iter().enumerate() {
        let me = *node;
        *node += 1;
        let (kind, standard) = match m.v.kind {
            MenuKind::Standard(MenuStandard::Program) => ("standard", text("program")),
            MenuKind::Standard(MenuStandard::Help) => ("standard", text("help")),
            other => (other.word(), Cell::Null),
        };
        RowBuilder::new(&t::MENU)
            .set("layout", id.clone())
            .set("node", int(me))
            .set("elem", elem(m))
            .set("parent", opt_int(parent))
            .set("seq", int(u32::try_from(seq).unwrap_or(0)))
            .set("kind", text(kind))
            .set("label_key", opt_text(m.v.label_key.as_ref()))
            .set("standard", standard)
            .set("command", opt_text(m.v.command.as_ref().map(|c| &c.1)))
            .set("source", opt_int(m.v.command.as_ref().map(|c| c.0)))
            .set("on_what", m.v.on.map_or(Cell::Null, |w| text(w.word())))
            .set("shortcut", opt_text(m.v.shortcut.as_ref()))
            .set("view", opt_text(m.v.view.as_ref()))
            .push_into(rows);
        menu_rows(&m.v.children, Some(me), id, node, rows);
    }
}

pub(crate) fn pages(layout: &Layout, id: &Cell, rows: &mut Rows) -> Result<(), Coded<LayoutCode>> {
    for (pn, p) in layout.doc().pages.iter().enumerate() {
        let page = text(&p.v.page);
        RowBuilder::new(&t::PAGE)
            .set("layout", id.clone())
            .set("page", page.clone())
            .set("elem", elem(p))
            .set("seq", int(u32::try_from(pn).unwrap_or(0)))
            .set("frame", text(&p.v.frame))
            .set("navigation", text(p.v.navigation.word()))
            .push_into(rows);
        for (bn, b) in p.v.blocks.iter().enumerate() {
            let place = text(&b.v.place);
            RowBuilder::new(&t::BLOCK)
                .set("layout", id.clone())
                .set("page", page.clone())
                .set("place", place.clone())
                .set("elem", elem(b))
                .set("seq", int(u32::try_from(bn).unwrap_or(0)))
                .set("block", text(&b.v.block))
                .set("source", opt_int(b.v.source))
                .set("title_key", opt_text(b.v.title_key.as_ref()))
                .set("text_key", opt_text(b.v.text_key.as_ref()))
                .push_into(rows);
            attr_rows(&b.v.set, "block", &elem(b), id, rows);
            for bd in &b.v.bindings {
                let key = |r: RowBuilder<'static>| {
                    r.set("layout", id.clone())
                        .set("page", page.clone())
                        .set("place", place.clone())
                        .set("slot", text(&bd.v.slot))
                        .set("nth", int(bd.v.nth))
                };
                binding_rows(bd, (&t::BINDING, &t::BINDING_SCHEME), &key, rows);
            }
        }
        node_rows(&p.v.places, None, id, &page, rows)?;
    }
    Ok(())
}

/// The rows of one binding and its scheme list, in the tables `tables` (binding, scheme); `key` sets the key columns of the owner.
pub(crate) fn binding_rows(
    bd: &Item<fs_forms_layout::BindingDoc>,
    tables: (&'static Table, &'static Table),
    key: &dyn Fn(RowBuilder<'static>) -> RowBuilder<'static>,
    rows: &mut Rows,
) {
    let (from, path, role, name) = match &bd.v.from {
        BindFrom::Records => ("records", Cell::Null, Cell::Null, Cell::Null),
        BindFrom::Field { path, role } => ("field", text(path), text(role), Cell::Null),
        BindFrom::Grouping(n) => ("grouping", Cell::Null, Cell::Null, text(n)),
        BindFrom::Order(n) => ("order", Cell::Null, Cell::Null, text(n)),
        BindFrom::Command(n) => ("command", Cell::Null, Cell::Null, text(n)),
        BindFrom::Lock => ("lock", Cell::Null, Cell::Null, Cell::Null),
        BindFrom::Condition => ("condition", Cell::Null, Cell::Null, Cell::Null),
    };
    key(RowBuilder::new(tables.0))
        .set("elem", elem(bd))
        .set("bind_from", text(from))
        .set("path", path)
        .set("role", role)
        .set("name", name)
        .set("source", opt_int(bd.v.source))
        .push_into(rows);
    for (n, s) in bd.v.schemes.iter().flatten().enumerate() {
        key(RowBuilder::new(tables.1))
            .set("seq", int(u32::try_from(n).unwrap_or(0)))
            .set("elem", elem(s))
            .set("scheme", text(&s.v))
            .push_into(rows);
    }
}

pub(crate) fn views(layout: &Layout, id: &Cell, rows: &mut Rows) -> Result<(), Coded<LayoutCode>> {
    for v in &layout.doc().views {
        if v.v.filter.is_some() {
            return Err(Coded::new(LayoutCode::ExportFailed)
                .with("file", "")
                .with("kind", "filter-not-supported-yet"));
        }
        let owner = v.v.owner.as_ref().map_or(String::new(), |o| o.0.clone());
        let (view, owner_cell) = (text(&v.v.view), text(&owner));
        RowBuilder::new(&t::VIEW)
            .set("layout", id.clone())
            .set("view", view.clone())
            .set("owner", owner_cell.clone())
            .set("elem", elem(v))
            .set(
                "scope",
                text(if owner.is_empty() { "all" } else { "person" }),
            )
            .set("grouping", opt_text(v.v.grouping.as_ref()))
            .set("search_kind", opt_text(v.v.search_kind.as_ref()))
            .push_into(rows);
        for (n, o) in v.v.orders.iter().enumerate() {
            RowBuilder::new(&t::VIEW_ORDER)
                .set("layout", id.clone())
                .set("view", view.clone())
                .set("owner", owner_cell.clone())
                .set("seq", int(u32::try_from(n).unwrap_or(0)))
                .set("elem", elem(o))
                .set("by", text(o.v.by.word()))
                .set("name", text(&o.v.name))
                .set("descending", int(i64::from(o.v.descending)))
                .push_into(rows);
        }
    }
    Ok(())
}

/// The cells of an action target: (target word, command, source, workflow). `open` has no column yet (S6-4): refused, never dropped.
fn action_cells(t: &ActionTarget) -> Result<(&'static str, Cell, Cell, Cell), Coded<LayoutCode>> {
    Ok(match t {
        ActionTarget::Command { source, command } => {
            ("command", text(command), int(*source), Cell::Null)
        }
        ActionTarget::Workflow(w) => ("workflow", Cell::Null, Cell::Null, text(&w.0)),
        ActionTarget::Open { .. } => {
            return Err(Coded::new(LayoutCode::ExportFailed)
                .with("file", "")
                .with("kind", "action-open-not-supported-yet"));
        }
    })
}

/// The rows of the actions, in order (`at` is the element id of the place whose command slot the action binds, or null).
pub(crate) fn actions(
    d: &fs_forms_layout::LayoutDoc,
    id: &Cell,
    rows: &mut Rows,
) -> Result<(), Coded<LayoutCode>> {
    for (n, a) in d.actions.iter().enumerate() {
        let (target, command, source, workflow) = action_cells(&a.v.target)?;
        RowBuilder::new(&t::ACTION)
            .set("layout", id.clone())
            .set("action", int(u32::try_from(n + 1).unwrap_or(0)))
            .set("elem", elem(a))
            .set("trigger", text(a.v.trigger.word()))
            .set(
                "at",
                a.v.at.as_ref().map_or(Cell::Null, |e| text(e.as_str())),
            )
            .set("target", text(target))
            .set("command", command)
            .set("source", source)
            .set("workflow", workflow)
            .set(
                "confirm",
                a.v.confirm.map_or(Cell::Null, |c| text(c.word())),
            )
            .push_into(rows);
    }
    Ok(())
}
