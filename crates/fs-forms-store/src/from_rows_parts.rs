//! Rows of actions and views back to the raw description (the flat parts of a layout).
//!
//! Pattern: Adapter (rows to model).

use fs_forms_layout::{ActionDoc, ActionTarget, Item, OrderDoc, Reference, ViewDoc, Words};

use crate::from_rows::{each, item, same};
use crate::rows::Rows;
use crate::tables as t;

pub(crate) fn actions(rows: &Rows, id: &str) -> Vec<Item<ActionDoc>> {
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

pub(crate) fn views(rows: &Rows, id: &str) -> Vec<Item<ViewDoc>> {
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
