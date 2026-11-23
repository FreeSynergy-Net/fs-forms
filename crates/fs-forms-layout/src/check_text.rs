//! Checks of single texts: contract ranges, scheme lists, messages (key, language, MF2 braces, length).
//!
//! Pattern: none: plain functions that append problems.

use std::collections::HashSet;

use semver::{Comparator, Op, VersionReq};

use crate::codes::LayoutCode;
use crate::doc::{Item, MessageDoc};
use crate::ids::ElemId;
use crate::limits::Limits;
use crate::shim::Coded;

type Problems = Vec<Coded<LayoutCode>>;

/// `name >=a.b.c, <d.e` (the grammar of `semver::VersionReq`, as the host writes it): exactly one lower bound `>=` and one upper bound `<`
/// (the table holds them as integers), the lower one below the upper one.
pub(crate) fn check_range(text: &str, out: &mut Problems) {
    if !range_ok(text) {
        out.push(Coded::new(LayoutCode::RangeInvalid).with("text", text));
    }
}

fn range_ok(text: &str) -> bool {
    let text = text.trim();
    let (name, req) = text.split_once(char::is_whitespace).unwrap_or((text, ""));
    let name_ok = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c));
    let Ok(req) = VersionReq::parse(req.trim()) else {
        return false;
    };
    let bound = |op: Op| {
        let mut it = req.comparators.iter().filter(|c| c.op == op);
        let first = it.next()?;
        it.next().is_none().then_some(first)
    };
    let key = |c: &Comparator| (c.major, c.minor.unwrap_or(0), c.patch.unwrap_or(0));
    req.comparators.len() == 2
        && name_ok
        && bound(Op::GreaterEq)
            .zip(bound(Op::Less))
            .is_some_and(|(l, h)| key(l) < key(h))
}

fn key_ok(k: &str) -> bool {
    k.chars()
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && k.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// BCP 47 in the shape `aa[a]` followed by `-subtag` parts of 2 to 8 letters or digits (not the full registry).
fn lang_ok(l: &str) -> bool {
    let mut parts = l.split('-');
    let first = parts.next().unwrap_or("");
    (2..=3).contains(&first.len())
        && first.chars().all(|c| c.is_ascii_lowercase())
        && parts.all(|p| (2..=8).contains(&p.len()) && p.chars().all(|c| c.is_ascii_alphanumeric()))
}

/// Braces must be balanced (MF2 syntax proper is not checked here: not proven that fs-components-mf2 can do it without formatting).
fn braces_ok(m: &str) -> bool {
    let mut depth = 0_i32;
    for c in m.chars() {
        depth += i32::from(c == '{') - i32::from(c == '}');
        if depth < 0 {
            return false;
        }
    }
    depth == 0
}

/// Key, language, message and length checks of all messages.
pub(crate) fn check_messages(list: &[Item<MessageDoc>], limits: &Limits, out: &mut Problems) {
    let mut seen = HashSet::new();
    for m in list {
        let (key, lang) = (m.v.key.as_str(), m.v.lang.as_str());
        let elem = m.elem.as_ref().map_or("", ElemId::as_str);
        if !key_ok(key) {
            out.push(Coded::new(LayoutCode::KeyInvalid).with("key", key));
        }
        if !lang_ok(lang) {
            out.push(Coded::new(LayoutCode::LangInvalid).with("lang", lang));
        }
        if !braces_ok(&m.v.message) {
            out.push(
                Coded::new(LayoutCode::MessageInvalid)
                    .with("key", key)
                    .with("lang", lang)
                    .with("cause", "braces"),
            );
        }
        if m.v.message.len() > limits.text_bytes_max as usize {
            out.push(
                Coded::new(LayoutCode::TextTooLong)
                    .with("elem", elem)
                    .with("bytes", m.v.message.len())
                    .with("max", limits.text_bytes_max)
                    .with("by", limits.by.text_bytes.as_str()),
            );
        }
        if !seen.insert((key, lang)) {
            out.push(
                Coded::new(LayoutCode::KeyTwice)
                    .with("kind", "message")
                    .with("key", format!("{key}/{lang}")),
            );
        }
    }
}
