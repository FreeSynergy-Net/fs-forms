//! The parts of a screen with their sentences made before drawing (`Part`), and the notice of a locked part. A locked part is one
//! `partial-notice` listing every code with its sentence; no widget is painted by hand.
//!
//! Pattern: Interpreter (the nodes of the composition in, the parts out); the sentences are made here so the surfaces only draw.

use std::fmt::Write as _;

use fs_components_mf2::Mf2Texts;
use fs_components_widgets::states::{self, PartialNotice};

use crate::compose::Node;
use crate::panel::{Embedded, Panels};
use crate::place_info::PlaceInfo;
use crate::view::Coded;

/// A part, with its sentences already made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Part {
    /// A block that is built; the host has a drawing for the state blocks only.
    Block {
        place: String,
        elem: String,
        block: String,
        title: Option<String>,
        info: PlaceInfo,
    },
    /// Not drawn: the name of the part and one line per code.
    Locked { name: String, lines: Vec<String> },
    /// An empty page with its sentence.
    Empty(String),
    /// A frontend of another (or the same) program, opened as a panel in this place: its own page, drawn here.
    Panel {
        place: String,
        head: String,
        child: Embedded,
    },
}

/// The sentence of a code, with the code and its parameters behind it; a code without a sentence is shown as itself.
pub(crate) fn line(texts: &Mf2Texts, c: &Coded) -> String {
    let said = texts.plain(&c.code);
    let mut out = if said.is_empty() || said == c.code {
        c.code.clone()
    } else {
        format!("{said} ({})", c.code)
    };
    for (k, v) in &c.params {
        let _ = write!(out, " {k}={v}");
    }
    out
}

pub(crate) fn parts(texts: &Mf2Texts, nodes: &[Node]) -> Vec<Part> {
    parts_with(texts, nodes, &Panels::new())
}

/// The parts of `nodes`; a frontend node is the panel the owner opened for its place (`panels`), a locked part when there is none (this
/// window has no owner, or the child was refused). A node in a window of its own draws nothing here.
pub(crate) fn parts_with(texts: &Mf2Texts, nodes: &[Node], panels: &Panels) -> Vec<Part> {
    nodes
        .iter()
        .filter_map(|n| {
            Some(match n {
                Node::Frontend { embed, .. } if !embed.panel => return None,
                Node::Frontend { place, embed } => match panels.get(place) {
                    Some(Ok((head, child))) => Part::Panel {
                        place: place.clone(),
                        head: head.clone(),
                        child: child.clone(),
                    },
                    other => {
                        let why = match other {
                            Some(Err(c)) => c.clone(),
                            _ => Coded::new("host-notice-frontend-not-opened")
                                .with("frontend", embed.key.frontend.clone()),
                        };
                        Part::Locked {
                            name: place.clone(),
                            lines: vec![line(texts, &why)],
                        }
                    }
                },
                Node::Block {
                    place,
                    elem,
                    block,
                    title,
                    info,
                    ..
                } => Part::Block {
                    place: place.clone(),
                    elem: elem.clone(),
                    block: block.clone(),
                    title: title.clone(),
                    info: info.clone(),
                },
                Node::Locked { place, codes } => Part::Locked {
                    name: place.clone().unwrap_or_default(),
                    lines: codes.iter().map(|c| line(texts, c)).collect(),
                },
                Node::Empty { .. } => Part::Empty(texts.plain("host-page-empty")),
            })
        })
        .collect()
}

/// The lines of a notice, the name of the part in front of the first one, and how many parts the notice counts (one per line).
pub(crate) fn notice_items(name: &str, lines: &[String]) -> (Vec<String>, usize) {
    let mut missing = lines.to_vec();
    if let (false, Some(first)) = (name.is_empty(), missing.first_mut()) {
        *first = format!("{name}: {first}");
    }
    let total = missing.len();
    (missing, total)
}

/// The notice of a locked part: the component `partial-notice` with the lines and the count (its lead says "N of M").
pub(crate) fn notice(labels: &states::Labels, name: &str, lines: &[String]) -> PartialNotice {
    let (missing, total) = notice_items(name, lines);
    PartialNotice::new(labels, &missing, total)
}

#[cfg(test)]
mod tests {
    use super::{notice, notice_items};
    use fs_components_widgets::states::Labels;
    use fs_components_words::Localized as _;

    #[test]
    fn the_notice_of_a_locked_part_says_how_many_of_how_many_it_could_not_show() {
        let labels = Labels::for_locale(&fs_components_words::Locale::english());
        let lines = vec!["a".to_owned(), "b".to_owned(), "c".to_owned()];
        let lead = notice(&labels, "side", &lines).lead().to_owned();
        assert_eq!(lead, labels.partial(3, 3));
        assert_ne!(
            lead,
            labels.partial(3, 99),
            "the total is the number of lines"
        );
        assert_ne!(
            lead,
            labels.partial(1, 3),
            "the failed ones are all of them"
        );
        assert_eq!(
            notice(&labels, "", &lines[..1]).lead(),
            labels.partial(1, 1)
        );
    }

    #[test]
    fn a_notice_counts_one_per_line_and_names_the_part_in_the_first() {
        let lines = vec!["a".to_owned(), "b".to_owned()];
        assert_eq!(
            notice_items("side", &lines),
            (vec!["side: a".to_owned(), "b".to_owned()], 2)
        );
        assert_eq!(notice_items("", &lines), (lines.clone(), 2));
        assert_eq!(notice_items("x", &[]), (Vec::new(), 0));
    }

    use super::{Part, line, parts};
    use crate::compose::Node;
    use crate::view::Coded;
    use crate::words::words;
    use fs_components_words::Locale;

    #[test]
    fn a_code_without_a_sentence_is_shown_as_its_code() {
        let t = words(&Locale::english());
        let unknown = Coded::new("bind-error-locked").with("part", "page:board");
        assert_eq!(line(&t, &unknown), "bind-error-locked part=page:board");
        let known = Coded::new("host-error-no-pages");
        let l = line(&t, &known);
        assert!(
            l.contains("host-error-no-pages") && l.contains("The layout has no pages"),
            "{l}"
        );
    }

    #[test]
    fn every_code_of_a_locked_part_is_a_line() {
        let t = words(&Locale::english());
        let node = Node::Locked {
            place: Some("board".into()),
            codes: vec![Coded::new("a-code"), Coded::new("b-code")],
        };
        match &parts(&t, &[node])[0] {
            Part::Locked { name, lines } => assert_eq!((name.as_str(), lines.len()), ("board", 2)),
            other => panic!("{other:?}"),
        }
    }
}
