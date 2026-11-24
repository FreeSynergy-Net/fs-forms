//! The local rules of a description that need no catalog and no provider: contract ranges, the pair rules of menu entries, source
//! numbers, view owners, scheme lists, messages. ONE place for them: `check` runs them on a raw description, `apply` runs them on the
//! result of every overlay op (D9: no overlay may produce what `check` would refuse), and they are at least as strict as the table
//! rules of `forms_*`, so a checked layout can always be written.
//!
//! Pattern: Specification: one pure rule set that every caller asks.

use crate::check_text::{check_messages, check_range};
use crate::codes::LayoutCode;
use crate::doc::{ActionTarget, Item, LayoutDoc, MenuDoc, MenuKind};
use crate::ids::ElemId;
use crate::limits::Limits;
use crate::schemes::scheme_names_rule;
use crate::shim::Coded;

type Problems = Vec<Coded<LayoutCode>>;

fn elem<T>(i: &Item<T>) -> &str {
    i.elem.as_ref().map_or("", ElemId::as_str)
}

/// Runs the local rules and appends every problem.
pub(crate) fn doc_rules(doc: &LayoutDoc, limits: &Limits, out: &mut Problems) {
    for r in &doc.requires {
        check_range(&r.v.range, out);
    }
    for s in &doc.sources {
        check_range(&s.v.range, out);
    }
    let sources = u32::try_from(doc.sources.len()).unwrap_or(u32::MAX);
    menu_rules(&doc.menu, sources, out);
    for p in &doc.pages {
        for b in p.v.leaves() {
            source_ref(
                b.elem.as_ref().map_or("", ElemId::as_str),
                b.v.source,
                sources,
                out,
            );
            for bd in &b.v.bindings {
                if let Some(list) = &bd.v.schemes {
                    let names: Vec<String> = list.iter().map(|x| x.v.clone()).collect();
                    out.extend(scheme_names_rule(elem(bd), &names));
                }
            }
        }
    }
    for a in &doc.actions {
        if let ActionTarget::Command { source, .. } = &a.v.target {
            source_ref(elem(a), Some(*source), sources, out);
        }
    }
    for v in &doc.views {
        if v.v.owner.as_ref().is_some_and(|o| o.0.is_empty()) {
            out.push(Coded::new(LayoutCode::KeyInvalid).with("key", "owner"));
        }
    }
    check_messages(&doc.messages, limits, out);
}

fn source_ref(at: &str, source: Option<u32>, sources: u32, out: &mut Problems) {
    if let Some(n) = source.filter(|n| *n < 1 || *n > sources) {
        out.push(
            Coded::new(LayoutCode::SourceUnknown)
                .with("elem", at)
                .with("source", n),
        );
    }
}

fn menu_rules(items: &[Item<MenuDoc>], sources: u32, out: &mut Problems) {
    for m in items {
        let bad = |cause: &str| {
            Coded::new(LayoutCode::MenuInvalid)
                .with("elem", elem(m))
                .with("cause", cause)
        };
        let v = &m.v;
        // a command entry has its command (and only it), a view entry its view (and only it)
        match (v.kind == MenuKind::Command, v.command.is_some()) {
            (true, false) => out.push(bad("command-missing")),
            (false, true) => out.push(bad("command-unexpected")),
            _ => {}
        }
        match (v.kind == MenuKind::View, v.view.is_some()) {
            (true, false) => out.push(bad("view-missing")),
            (false, true) => out.push(bad("view-unexpected")),
            _ => {}
        }
        if v.kind == MenuKind::Separator && v.label_key.is_some() {
            out.push(bad("separator-has-label"));
        }
        if let Some((n, _)) = &v.command {
            source_ref(elem(m), Some(*n), sources, out);
        }
        menu_rules(&v.children, sources, out);
    }
}
