//! The status line of the window: how the host stands. The line "some parts are locked" appears ONLY when a part really lacks
//! something (not reachable, not allowed, no module or command), and then it NAMES the parts; a part that is merely empty never makes it.
//!
//! Pattern: Template Method (one place decides how the line is made; callers only name their inputs) – split off `window.rs`.

use fs_components_app::{AppWidget, Status};
use fs_components_mf2::Mf2Texts;

use crate::compose::{Composition, HostState};
use crate::records::Records;

/// Sets the status line of `app` from the composition and the records it shows (open and frontend switch alike).
pub(crate) fn set_status_line(
    app: &AppWidget,
    texts: &Mf2Texts,
    comp: &Composition,
    records: &dyn Records,
) {
    let down = records.unavailable();
    let line = status(
        texts,
        comp.state,
        &comp.lacking,
        records.is_sample(),
        down.as_deref(),
    );
    app.set_status(Some(line));
}

/// The names of the lacking parts as one sentence part (`""` = the window itself, said with its own word, never an empty text).
fn named(texts: &Mf2Texts, lacking: &[String]) -> String {
    lacking
        .iter()
        .map(|n| {
            if n.is_empty() {
                texts.plain("host-status-window")
            } else {
                n.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// How the host stands; sample data is said in the same line (a made-up record is never shown as a real one), and so is a data
/// source that is not there (`down`: always a problem). `lacking` are the parts that really lack something: `Partial` names them.
pub(crate) fn status(
    texts: &Mf2Texts,
    state: HostState,
    lacking: &[String],
    sample: bool,
    down: Option<&str>,
) -> Status {
    if let Some(why) = down {
        return Status::Problem(why.to_owned());
    }
    let (problem, mut text) = match state {
        HostState::Ready => (false, texts.plain("host-status-ready")),
        HostState::Partial if lacking.is_empty() => (false, texts.plain("host-status-partial")),
        HostState::Partial => (
            false,
            format!(
                "{}: {}; {}",
                texts.plain("host-status-locked"),
                named(texts, lacking),
                texts.plain("host-status-rest")
            ),
        ),
        HostState::Refused => (true, texts.plain("host-status-refused")),
    };
    if sample {
        text = format!("{text} · {}", texts.plain("host-status-sample"));
    }
    if problem {
        Status::Problem(text)
    } else {
        Status::Info(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::words::words;
    use fs_components_words::Locale;

    fn info(s: &Status) -> &str {
        match s {
            Status::Info(t) | Status::Problem(t) => t,
            _ => "",
        }
    }

    #[test]
    fn sample_data_is_said_in_the_status_in_both_languages_and_real_data_says_nothing() {
        for (lang, said) in [("en", "Sample data"), ("de", "Beispieldaten")] {
            let locale = Locale::from_posix(lang).expect("a locale");
            let texts = words(&locale);
            let lack = ["board".to_owned()];
            let with = status(&texts, HostState::Partial, &lack, true, None);
            assert!(info(&with).contains(said), "{lang}: {}", info(&with));
            assert!(
                info(&with).contains("board"),
                "the named part stays in the line"
            );
            let without = status(&texts, HostState::Partial, &lack, false, None);
            assert!(!info(&without).contains(said), "{lang}: {}", info(&without));
            let refused = status(&texts, HostState::Refused, &[], true, None);
            assert!(matches!(refused, Status::Problem(_)), "a problem stays one");
        }
    }

    #[test]
    fn a_data_source_that_is_not_there_is_a_problem_in_the_status_line_with_its_reason() {
        let texts = words(&Locale::from_posix("en").expect("a locale"));
        let down = status(
            &texts,
            HostState::Ready,
            &[],
            false,
            Some("no answer: socket x"),
        );
        assert!(matches!(&down, Status::Problem(t) if t == "no answer: socket x"));
    }

    #[test]
    fn a_lack_names_its_parts_in_both_languages_and_the_window_has_a_word_never_an_empty_text() {
        for lang in ["en", "de"] {
            let texts = words(&Locale::from_posix(lang).expect("a locale"));
            let lack = ["side".to_owned(), String::new()];
            let line = status(&texts, HostState::Partial, &lack, false, None);
            let said = info(&line);
            assert!(said.contains("side"), "{lang}: {said}");
            assert!(
                said.contains(&texts.plain("host-status-window"))
                    && !texts.plain("host-status-window").is_empty(),
                "{lang}: {said}"
            );
            assert!(
                !said.contains(", ,") && !said.ends_with(": "),
                "{lang}: {said}"
            );
        }
    }
}
