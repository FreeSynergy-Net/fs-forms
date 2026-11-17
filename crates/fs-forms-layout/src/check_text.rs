//! Checks of single texts: contract ranges, scheme lists, messages (key, language, MF2 braces, length).
//!
//! Pattern: none: plain functions that append problems.

use std::collections::HashSet;

use crate::codes::LayoutCode;
use crate::doc::{Item, MessageDoc};
use crate::ids::ElemId;
use crate::limits::Limits;
use crate::shim::Coded;

type Problems = Vec<Coded<LayoutCode>>;

/// `name >=a.b <c.d` with at least one bound; a lower bound must be below the upper one.
pub(crate) fn check_range(text: &str, out: &mut Problems) {
    if !range_ok(text) {
        out.push(Coded::new(LayoutCode::RangeInvalid).with("text", text));
    }
}

fn version(t: &str) -> Option<(u32, u32)> {
    let (a, b) = t.split_once('.')?;
    Some((a.parse().ok()?, b.parse().ok()?))
}

fn range_ok(text: &str) -> bool {
    let mut it = text.split_whitespace();
    let name_ok = it.next().is_some_and(|n| {
        !n.is_empty()
            && n.chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
    });
    let (mut low, mut high) = (None, None);
    for t in it {
        if let Some(v) = t.strip_prefix(">=").and_then(version) {
            if low.replace(v).is_some() {
                return false;
            }
        } else if let Some(v) = t.strip_prefix('<').and_then(version) {
            if high.replace(v).is_some() {
                return false;
            }
        } else {
            return false;
        }
    }
    name_ok && (low.is_some() || high.is_some()) && low.zip(high).is_none_or(|(l, h)| l < h)
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
