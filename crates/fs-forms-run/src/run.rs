//! Running a command: the exit codes of the host (ADR 0094 4: a fatal connection error ends with 3 and a message, the starter then
//! shows the standard frontend; a layout problem is shown in the window, never silently).
//!
//! Pattern: Facade over link, catalog, composition and window.

use std::process::ExitCode;

use fs_components_app::{WindowOptions, launch};

use crate::args::{Command, Config, HELP, parse};
use crate::blocks::Blocks;
use crate::compose::compose;
use crate::error::{EXIT_CONTRACT, HostError};
use crate::file::FileLink;
use crate::link::{LayoutLink, SocketLink};
use crate::source::open;
use crate::view::View;
use crate::window::failure;
use crate::words::{session_locale, words};

/// Run the command line (the words after the program name) and return the exit code.
#[must_use]
pub fn run(args: &[String]) -> ExitCode {
    match parse(args) {
        Err(message) => {
            eprintln!("fs-forms-run: {message}\nTry `fs-forms-run --help`.");
            ExitCode::from(2)
        }
        Ok(Command::Version) => {
            println!("fs-forms-run {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(Command::Help) => {
            println!("{HELP}");
            ExitCode::SUCCESS
        }
        Ok(Command::Check(c)) => match fetch(&c) {
            Ok(view) => {
                println!(
                    "fs-forms-run: the description of the frontend {} is there",
                    view.frontend
                );
                ExitCode::SUCCESS
            }
            Err(e) => end(&e),
        },
        Ok(Command::Window(c)) => window(&c),
    }
}

fn fetch(c: &Config) -> Result<View, HostError> {
    if let Some(path) = &c.description {
        let lang = session_locale().primary().to_owned();
        return FileLink::new(path, lang).resolve(&c.extends, &c.frontend);
    }
    SocketLink::open(c.forms_socket.as_deref())?.resolve(&c.extends, &c.frontend)
}

fn end(e: &HostError) -> ExitCode {
    eprintln!("fs-forms-run: {e}");
    if e.fatal() {
        ExitCode::from(EXIT_CONTRACT)
    } else {
        ExitCode::FAILURE
    }
}

fn window(c: &Config) -> ExitCode {
    let locale = session_locale();
    let texts = words(&locale);
    let (view, comp, records) = match fetch(c) {
        Ok(mut view) => {
            let running = std::env::current_exe().unwrap_or_default();
            let opened = open(c, &view, &texts, &running);
            // real records (or a source that is not there) make "data sources are not connected in this version" untrue
            if opened.connected || opened.notice.is_some() {
                view.notices
                    .retain(|n| n.code != "host-notice-no-providers");
            }
            view.notices.extend(opened.notice);
            let comp = compose(&view, &Blocks);
            (view, comp, opened.records)
        }
        // fatal: the starter falls back to the standard frontend and says why; the others are shown here
        Err(e) if e.fatal() => return end(&e),
        Err(e) => {
            eprintln!("fs-forms-run: {e}");
            let none: std::sync::Arc<dyn crate::records::Records> =
                std::sync::Arc::new(crate::records::NoRecords);
            (View::default(), failure(&e), none)
        }
    };
    let program_style = std::sync::Arc::new(crate::style::StyleCell::new(
        c.style.unwrap_or(fs_components_styles::StyleId::Admin),
    ));
    let widget =
        match crate::window::app_styled(&texts, &locale, &view, &comp, records, program_style) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("fs-forms-run: {e}");
                return ExitCode::FAILURE;
            }
        };
    if let Some(page) = &c.page {
        widget.select(page);
    }
    let done = match &c.picture {
        Some((path, (w, h))) => widget.picture(*w, *h, path),
        None => launch(WindowOptions::default(), widget),
    };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("fs-forms-run: {e}");
            ExitCode::FAILURE
        }
    }
}
