//! Running a command (`run` gives an `ExitCode`, `run_code` the same code as a number): the exit codes of the host (ADR 0094 4: a fatal connection error ends with 3 and a message, the starter then
//! shows the standard frontend; a layout problem is shown in the window, never silently).
//!
//! Pattern: Facade over link, catalog, composition and window.

use std::process::ExitCode;

use fs_components_app::{WindowOptions, launch};

use crate::args::{Command, Config, HELP, parse};
use crate::blocks::Blocks;
use crate::compose::compose;
use crate::described::Described;
use crate::error::{EXIT_CONTRACT, HostError};
use crate::file::FileLink;
use crate::form_check::lock_misfits;
use crate::link::{LayoutLink, SocketLink};
use crate::sources::{NoSources, Sources, open};
use crate::view::View;
use crate::window::failure;
use crate::words::{session_locale, words};
use fs_forms_layout::Layout;

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
#[must_use = "an ignored exit code would hide a failed run"]
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
        Ok(Command::ListBlocks) => {
            for (block, why) in crate::blocks::not_built_list() {
                println!("{block}: {why}");
            }
            CODE_SUCCESS
        }
        Ok(Command::Check(c)) => match fetch(&c) {
            Ok(got) => {
                if let Some(line) = got.line {
                    println!("fs-forms-run: {line}");
                }
                println!(
                    "fs-forms-run: the description of the frontend {} is there",
                    got.view.frontend
                );
                CODE_SUCCESS
            }
            Err(e) => end(&e),
        },
        Ok(Command::Window(c)) => window(&c, sources),
    }
}

/// What a fetch brings: the view, the checked description it was made from (a description on disk; the socket gives none yet) and
/// the line `frontend <id> from <file>`.
pub(crate) struct Fetched {
    pub(crate) view: View,
    pub(crate) layout: Option<Layout>,
    pub(crate) line: Option<String>,
}

pub(crate) fn fetch(c: &Config) -> Result<Fetched, HostError> {
    if let Some(path) = &c.description {
        let lang = session_locale().primary().to_owned();
        let loaded = FileLink::new(path, lang).load(&c.extends, &c.frontend)?;
        let line = loaded.line();
        return Ok(Fetched {
            view: loaded.view,
            layout: Some(loaded.layout),
            line: Some(line),
        });
    }
    let view = SocketLink::open(c.forms_socket.as_deref())?.resolve(&c.extends, &c.frontend)?;
    Ok(Fetched {
        view,
        layout: None,
        line: None,
    })
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
    // the input is read first: a file that cannot be played ends the run before anything is built or drawn
    let plan = match c.steps.as_deref().map(crate::steps::read).transpose() {
        Ok(plan) => plan,
        Err(message) => {
            eprintln!("fs-forms-run: {message}");
            return CODE_USAGE;
        }
    };
    let locale = session_locale();
    let texts = words(&locale);
    // a layout problem shown in the window: a picture run still ends with the code of that error (ADR 0094 4)
    let mut shown: Option<HostError> = None;
    // a block whose module does not fit its form is locked with the codes of the match: a picture run ends with 3 as well
    let mut misfit = false;
    let (view, comp, records) = match fetch(c) {
        Ok(Fetched {
            mut view, layout, ..
        }) => {
            let running = std::env::current_exe().unwrap_or_default();
            let described = Described::of(layout.as_ref());
            let opened = open(c, &view, &described, &texts, &running, sources);
            // real or sample records (or a source that is not there) make "data sources are not connected in this version" untrue
            if opened.connected || opened.notice.is_some() || opened.records.is_sample() {
                view.notices
                    .retain(|n| n.code != "host-notice-no-providers");
            }
            view.notices.extend(opened.notice);
            misfit = lock_misfits(&mut view, opened.records.as_ref());
            // the lane list that cannot be used is said in the place (a slot lock), never left out silently
            crate::lanes::show_notices(&mut view, opened.records.as_ref());
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
    let done = match (&c.picture, &plan) {
        (Some((path, size)), Some(plan)) => {
            if let Err((message, code)) =
                crate::steps::play(&widget, plan, *size, path, CODE_USAGE, CODE_FAILURE)
            {
                eprintln!("fs-forms-run: {message}");
                return code;
            }
            Ok(())
        }
        (Some((path, (w, h))), None) => widget.picture(*w, *h, path),
        (None, _) => launch(WindowOptions::default(), widget),
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
