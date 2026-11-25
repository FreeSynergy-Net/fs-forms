//! The host of the picture tests as a process: `fs-forms-run` with the sources of `tests/sample_source` (`--sample-data` derives the
//! data from the description, as the starter does). Never shipped; the tests start it to draw a window in light and dark.
//!
//! Pattern: Facade (the library `run_with` with the test sources).

#[path = "../tests/sample_source/mod.rs"]
mod sample_source;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|w| w.to_string_lossy().into_owned())
        .collect();
    fs_forms_run::run_with(&args, &sample_source::Derived)
}
