//! Running a command (`run` gives an `ExitCode`, `run_code` the same code as a number): the exit codes of the host (ADR 0094 4: a fatal connection error ends with 3 and a message, the starter then
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
use crate::form_check::lock_misfits;
use crate::link::{LayoutLink, SocketLink};
use crate::sources::{NoSources, Sources, open};
use crate::view::View;
use crate::window::failure;
use crate::words::{session_locale, words};

/// The exit code for success, as a plain number.
const CODE_SUCCESS: i32 = 0;
/// The exit code for a failure that is not fatal for the starter.
const CODE_FAILURE: i32 = 1;
/// The exit code for a usage error (wrong words on the command line).
const CODE_USAGE: i32 = 2;
/// The contract code as a plain number (`EXIT_CONTRACT`).
const CODE_CONTRACT: i32 = EXIT_CONTRACT as i32;

/// Run the command line (the words after the program name) and return the exit code.
///
/// Same as [`run_code`], handed over as a process exit code (`ExitCode` has no accessor to read the number back).
pub fn run(args: &[String]) -> ExitCode {
    run_with(args, &NoSources)
}

/// [`run`] with the data sources the host offers (the starter gives the ones that connect the module services).
pub fn run_with(args: &[String], sources: &dyn Sources) -> ExitCode {
    ExitCode::from(u8::try_from(run_code_with(args, sources)).unwrap_or(1))
}

/// Run the command line and return the REAL exit code as a number: 0 success, 1 failure, 2 usage error, `EXIT_CONTRACT` (3)
/// for a fatal connection or contract error. For hosts that must act on the number (an update hook in the starter, ADR 0105).
#[must_use]
pub fn run_code(args: &[String]) -> i32 {
    run_code_with(args, &NoSources)
}

/// [`run_code`] with the data sources the host offers.
#[must_use]
pub fn run_code_with(args: &[String], sources: &dyn Sources) -> i32 {
    match parse(args) {
        Err(message) => {
            eprintln!("fs-forms-run: {message}\nTry `fs-forms-run --help`.");
            CODE_USAGE
        }
        Ok(Command::Version) => {
            println!("fs-forms-run {}", env!("CARGO_PKG_VERSION"));
            CODE_SUCCESS
        }
        Ok(Command::Help) => {
            println!("{HELP}");
            CODE_SUCCESS
        }
        Ok(Command::Check(c)) => match fetch(&c) {
            Ok(view) => {
                println!(
                    "fs-forms-run: the description of the frontend {} is there",
                    view.frontend
                );
                CODE_SUCCESS
            }
            Err(e) => end(&e),
        },
        Ok(Command::Window(c)) => window(&c, sources),
    }
}

fn fetch(c: &Config) -> Result<View, HostError> {
    if let Some(path) = &c.description {
        let lang = session_locale().primary().to_owned();
        return FileLink::new(path, lang).resolve(&c.extends, &c.frontend);
    }
    SocketLink::open(c.forms_socket.as_deref())?.resolve(&c.extends, &c.frontend)
}

fn end(e: &HostError) -> i32 {
    eprintln!("fs-forms-run: {e}");
    if e.fatal() {
        CODE_CONTRACT
    } else {
        CODE_FAILURE
    }
}

fn window(c: &Config, sources: &dyn Sources) -> i32 {
    let locale = session_locale();
    let texts = words(&locale);
    // a layout problem shown in the window: a picture run still ends with the code of that error (ADR 0094 4)
    let mut shown: Option<HostError> = None;
    // a block whose module does not fit its form is locked with the codes of the match: a picture run ends with 3 as well
    let mut misfit = false;
    let (view, comp, records) = match fetch(c) {
        Ok(mut view) => {
            let running = std::env::current_exe().unwrap_or_default();
            let opened = open(c, &view, &texts, &running, sources);
            // real records (or a source that is not there) make "data sources are not connected in this version" untrue
            if opened.connected || opened.notice.is_some() {
                view.notices
                    .retain(|n| n.code != "host-notice-no-providers");
            }
            view.notices.extend(opened.notice);
            misfit = lock_misfits(&mut view, opened.records.as_ref());
            let comp = compose(&view, &Blocks);
            (view, comp, opened.records)
        }
        // fatal: the starter falls back to the standard frontend and says why; the others are shown here
        Err(e) if e.fatal() => return end(&e),
        Err(e) => {
            eprintln!("fs-forms-run: {e}");
            shown = Some(e.clone());
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
                return CODE_FAILURE;
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
        // the picture is written either way; the code says that the window showed an error
        Ok(()) if c.picture.is_some() && (shown.is_some() || misfit) => CODE_CONTRACT,
        Ok(()) => CODE_SUCCESS,
        Err(e) => {
            eprintln!("fs-forms-run: {e}");
            CODE_FAILURE
        }
    }
}
