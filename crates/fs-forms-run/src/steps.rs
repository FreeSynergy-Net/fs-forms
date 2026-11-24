//! `--steps FILE`: the input of a picture run (the body of W3C `WebDriver` "Perform Actions") read before anything is built, and
//! played on the window before the picture is drawn.
//!
//! Pattern: Facade over `fs_components_app::replay` (the parser) and `AppWidget::play` (the player): the host holds no
//! table of actions. Every refusal is a message and exit code 2 (a wrong word of the person), never a panic, and the
//! picture is not written after a refused input.

use std::path::Path;

use fs_components_app::replay::{Code, Plan, parse};
use fs_components_app::{AppError, AppWidget};

/// Read and translate the file; the message says the file and the cause.
///
/// # Errors
/// The file cannot be read, or its body is not valid or not (yet) played: a message for exit code 2.
pub(crate) fn read(path: &Path) -> Result<Plan, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("--steps: cannot read {}: {e}", path.display()))?;
    parse(&text).map_err(|e| format!("--steps: {}: {e}", path.display()))
}

/// Play `plan` on the window and write the picture. `Err((message, code))`: a plan or element the window cannot play is
/// exit 2 (nothing is drawn); anything else (no graphics adapter, the picture cannot be written) is exit 1, as for `--picture`.
pub(crate) fn play(
    widget: &AppWidget,
    plan: &Plan,
    (width, height): (u32, u32),
    picture: &Path,
    usage: i32,
    failure: i32,
) -> Result<(), (String, i32)> {
    widget
        .play(plan, width, height, picture)
        .map_err(|e| match &e {
            AppError::Replay(r) if r.code != Code::Driver => (format!("--steps: {e}"), usage),
            _ => (e.to_string(), failure),
        })
}

#[cfg(test)]
mod tests;
