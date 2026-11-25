//! The head parts of a layout from rows: requires, sources (with the free scope token, RFC 6749 section 3.3: a token in the database
//! that breaks the grammar is a store error, never a silent "no scope"), messages and the menu tree.
//!
//! Pattern: Adapter (rows to model), one function per table.

use fs_forms_layout::{
    Coded, Item, LayoutCode, MenuDoc, MenuKind, MenuStandard, MessageDoc, RequiresDoc, SourceDoc,
    SourceScope, Words,
};

use crate::from_rows::{each, item, same};
use crate::rows::RowReader;
use crate::rows::Rows;
use crate::tables as t;
use crate::to_rows::range_text;

pub(crate) fn requires(rows: &Rows, key: &str) -> Vec<Item<RequiresDoc>> {
    let mut req: Vec<_> = each(rows, &t::REQUIRES).filter(|r| same(r, key)).collect();
    req.sort_by_key(|r| r.int("seq"));
    req.iter()
        .map(|r| {
            item(
                r,
                RequiresDoc {
                    range: range_text(r),
                },
            )
        })
        .collect()
}

/// The sources of the layout; a `scope` cell that is neither a host word nor a valid token goes into `problems`.
pub(crate) fn sources(
    rows: &Rows,
    key: &str,
    problems: &mut Vec<Coded<LayoutCode>>,
) -> Vec<Item<SourceDoc>> {
    let mut src: Vec<_> = each(rows, &t::SOURCE).filter(|r| same(r, key)).collect();
    src.sort_by_key(|r| r.int("source"));
    src.iter()
        .map(|r| item(r, source_doc(r, problems)))
        .collect()
}

fn source_doc(r: &RowReader<'_>, problems: &mut Vec<Coded<LayoutCode>>) -> SourceDoc {
    let scope = r.text("scope").and_then(|w| {
        let s = SourceScope::from_text(&w);
        if s.is_none() {
            problems.push(
                Coded::new(LayoutCode::StoreBroken)
                    .with("file", "forms.db")
                    .with("cause", "rows")
                    .with(
                        "detail",
                        format!("forms_source.scope: '{w}' is not a scope token (RFC 6749 3.3, 1..=256 bytes)"),
                    ),
            );
        }
        s
    });
    SourceDoc {
        range: range_text(r),
        data_type: r.text("data_type").unwrap_or_default(),
        scope,
    }
}

pub(crate) fn messages(rows: &Rows, key: &str) -> Vec<Item<MessageDoc>> {
    let mut msgs: Vec<_> = each(rows, &t::MESSAGE).filter(|r| same(r, key)).collect();
    msgs.sort_by_key(|r| (r.text("key"), r.text("lang")));
    msgs.iter()
        .map(|r| {
            let m = MessageDoc {
                key: r.text("key").unwrap_or_default(),
                lang: r.text("lang").unwrap_or_default(),
                message: r.text("message").unwrap_or_default(),
            };
            item(r, m)
        })
        .collect()
}

pub(crate) fn menu(rows: &Rows, id: &str, parent: Option<i64>) -> Vec<Item<MenuDoc>> {
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
