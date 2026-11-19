//! `fs-forms`: one binary for the program (ADR 0063). A thin template over `fs-starter`: the starter does the words, the server gate,
//! the choice of the frontend and its opening; this file only names the parts.
//!
//! ```text
//! fs-forms                          make sure the server runs, open the frontend `simple`
//! fs-forms [--frontend ID] [--socket PATH]
//! fs-forms serve [options]          the server (Cap'n Proto, surveys.capnp)
//! fs-forms --version | --help
//! ```

mod program;

use std::process::ExitCode;

use fs_forms_client::{PROGRAM, default_socket};
use fs_interfaces::surveys_capnp::surveys;
use fs_rpc::{Expected, SingleTimes};
use fs_starter::{NoPreference, Parts, ProcessLauncher, SingleGate, StderrTell, XdgManifests};

use program::Forms;

fn main() -> ExitCode {
    let program = Forms::default();
    let own_file = std::env::current_exe().unwrap_or_else(|_| PROGRAM.into());
    let expected = Expected::new(env!("CARGO_PKG_VERSION")).with_program(PROGRAM);
    let gate = SingleGate::<surveys::Client>::new(own_file, expected, SingleTimes::default());
    let parts = Parts {
        program: &program,
        gate: &gate,
        manifests: &XdgManifests::from_env(),
        preference: &NoPreference,
        launcher: &ProcessLauncher::standard(),
        tell: &StderrTell,
        hooks: &[],
        mcp: None,
        default_socket: default_socket(),
    };
    fs_starter::main_exit(&parts)
}
