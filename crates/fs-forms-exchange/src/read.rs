//! The tree to the raw description (layer 1), format 3 (older formats are refused with `format-old`). The `elem` of a node is needed (the check reports
//! `elem-missing`); attributes get theirs from their path. Keys, types, words: every problem is collected, with its line where the file can tell.
//!
//! Pattern: Adapter (tree to model) with a collecting error list.

use fs_forms_layout::{
    ActionDoc, ActionTarget, BindFrom, BindingDoc, Coded, FrontendKey, Item, LayoutCode, LayoutDoc,
    MenuDoc, MenuKind, MessageDoc, OrderDoc, PageDoc, Reference, RequiresDoc, SourceDoc, ViewDoc,
};

use crate::fields::{Ctx, Fields};
use crate::read_place::places;
use crate::tree::Tree;
use fs_forms_layout::Words;

fn word<W: Words>(f: &mut Fields<'_>, cx: &mut Ctx<'_>, key: &str) -> Option<W> {
    let w = f.str(cx, key)?;
    let got = W::from_word(&w);
    if got.is_none() {
        cx.bad("type", &w);
    }
    got
}

/// Reads every element of a list with `one`, which gets the table fields and returns the value.
pub(crate) fn items<'a, T>(
    list: &'a [Tree],
    what: &'a str,
    cx: &mut Ctx<'_>,
    mut one: impl FnMut(&mut Fields<'a>, &mut Ctx<'_>) -> T,
) -> Vec<Item<T>> {
    let mut out = Vec::new();
    for t in list {
        let Some(mut f) = Fields::new(t, what, cx) else {
            continue;
        };
        let (elem, line) = f.elem(cx);
        let v = one(&mut f, cx);
        f.done(cx);
        out.push(Item { elem, line, v });
    }
    out
}

/// The whole document.
pub(crate) fn layout_doc(root: &Tree, cx: &mut Ctx<'_>) -> Option<LayoutDoc> {
    let mut f = Fields::new(root, "schema", cx)?;
    let format = f.uint(cx, "schema").unwrap_or_else(|| {
        cx.bad("key-missing", "schema");
        0
    });
    if let Some(k) = f.str(cx, "kind")
        && k != "frontend"
    {
        cx.bad("type", "kind");
    }
    if (1..fs_forms_layout::FORMAT_KNOWN).contains(&format) {
        cx.errs.push(
            Coded::new(LayoutCode::FormatOld)
                .with("found", format)
                .with("known", fs_forms_layout::FORMAT_KNOWN),
        );
        return None; // no second reading path: old files are written again, not converted
    }
    let frontend = f.req_str(cx, "id");
    let extends = f.req_str(cx, "extends");
    let (exec, title_key, style) = (
        f.req_str(cx, "exec"),
        f.req_str(cx, "title"),
        f.str(cx, "style"),
    );
    let requires = items(f.list(cx, "requires"), "requires", cx, |f, cx| {
        RequiresDoc {
            range: f.req_str(cx, "contract"),
        }
    });
    let sources = items(f.list(cx, "source"), "source", cx, |f, cx| {
        let _ = f.uint(cx, "n"); // the number is the position; kept for readers of the file
        SourceDoc {
            range: f.req_str(cx, "contract"),
            data_type: f.req_str(cx, "type"),
            scope: word(f, cx, "scope"),
        }
    });
    let menu = menus(f.list(cx, "menu"), cx);
    let pages = items(f.list(cx, "page"), "page", cx, |f, cx| PageDoc {
        page: f.req_str(cx, "page"),
        frame: f.req_str(cx, "frame"),
        navigation: word(f, cx, "navigation").unwrap_or(fs_forms_layout::Navigation::Single),
        blocks: Vec::new(),
        places: places(f.list(cx, "place"), cx),
    });
    let actions = items(f.list(cx, "action"), "action", cx, action);
    let views = items(f.list(cx, "view"), "view", cx, view);
    let messages = items(f.list(cx, "message"), "message", cx, |f, cx| MessageDoc {
        key: f.req_str(cx, "key"),
        lang: f.req_str(cx, "lang"),
        message: f.req_str(cx, "text"),
    });
    f.done(cx);
    let key = FrontendKey { extends, frontend };
    Some(LayoutDoc {
        format,
        key,
        exec,
        title_key,
        style,
        requires,
        sources,
        menu,
        pages,
        actions,
        views,
        messages,
    })
}

fn menus(list: &[Tree], cx: &mut Ctx<'_>) -> Vec<Item<MenuDoc>> {
    // children are read in the same call: the closure cannot recurse, so a helper does it
    fn one(f: &mut Fields<'_>, cx: &mut Ctx<'_>) -> MenuDoc {
        let kind = word::<MenuKind>(f, cx, "kind").unwrap_or(MenuKind::Menu);
        let source = f.uint(cx, "source");
        let command = f.str(cx, "command");
        MenuDoc {
            kind,
            label_key: f.str(cx, "label"),
            command: source.zip(command),
            on: word(f, cx, "on"),
            shortcut: f.str(cx, "key"),
            view: f.str(cx, "view"),
            children: items(f.list(cx, "entries"), "entries", cx, one),
        }
    }
    items(list, "menu", cx, one)
}

pub(crate) fn binding(f: &mut Fields<'_>, cx: &mut Ctx<'_>) -> BindingDoc {
    let (slot, nth) = (f.req_str(cx, "slot"), f.uint(cx, "nth").unwrap_or(0));
    let (field, role) = (f.str(cx, "field"), f.str(cx, "role"));
    let (grouping, order, command) = (
        f.str(cx, "grouping"),
        f.str(cx, "order"),
        f.str(cx, "command"),
    );
    let (lock, condition) = (
        f.bool(cx, "lock").unwrap_or(false),
        f.bool(cx, "condition").unwrap_or(false),
    );
    let from = match (field, grouping, order, command) {
        (Some(path), _, _, _) => BindFrom::Field {
            path,
            role: role.unwrap_or_default(),
        },
        (_, Some(g), _, _) => BindFrom::Grouping(g),
        (_, _, Some(o), _) => BindFrom::Order(o),
        (_, _, _, Some(c)) => BindFrom::Command(c),
        _ if lock => BindFrom::Lock,
        _ if condition => BindFrom::Condition,
        _ => BindFrom::Records,
    };
    let schemes = f.has_list("schemes").then(|| scheme_items(f, cx));
    BindingDoc {
        slot,
        nth,
        from,
        schemes,
    }
}

/// Scheme entries are tables `{ elem, scheme }` (format 2) or plain names (format 1).
fn scheme_items(f: &mut Fields<'_>, cx: &mut Ctx<'_>) -> Vec<Item<String>> {
    let mut out = Vec::new();
    for t in f.list(cx, "schemes") {
        match t {
            Tree::Str(s) => out.push(Item {
                elem: None,
                line: cx.line_of(&format!("\"{s}\"")),
                v: s.clone(),
            }),
            Tree::Obj(..) => out.extend(items(std::slice::from_ref(t), "schemes", cx, |f, cx| {
                f.req_str(cx, "scheme")
            })),
            _ => cx.bad("type", "schemes"),
        }
    }
    out
}

fn action(f: &mut Fields<'_>, cx: &mut Ctx<'_>) -> ActionDoc {
    let trigger = word(f, cx, "trigger").unwrap_or(fs_forms_layout::Trigger::Button);
    let target = match (
        f.uint(cx, "source"),
        f.str(cx, "command"),
        f.str(cx, "workflow"),
    ) {
        (Some(source), Some(command), _) => ActionTarget::Command { source, command },
        (_, _, Some(w)) => ActionTarget::Workflow(Reference(w)),
        _ => {
            cx.bad("key-missing", "action");
            ActionTarget::Workflow(Reference(String::new()))
        }
    };
    ActionDoc {
        trigger,
        target,
        confirm: word(f, cx, "confirm"),
    }
}

fn view(f: &mut Fields<'_>, cx: &mut Ctx<'_>) -> ViewDoc {
    ViewDoc {
        view: f.req_str(cx, "view"),
        owner: f.str(cx, "owner").map(Reference),
        grouping: f.str(cx, "grouping"),
        search_kind: f.str(cx, "search"),
        filter: None, // filters are not part of this cut: a `filter` key is reported as key-unknown
        orders: items(f.list(cx, "order"), "order", cx, |f, cx| OrderDoc {
            by: word(f, cx, "by").unwrap_or(fs_forms_layout::OrderBy::Field),
            name: f.req_str(cx, "name"),
            descending: f.bool(cx, "descending").unwrap_or(false),
        }),
    }
}
