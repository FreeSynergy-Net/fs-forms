//! The command line of the host: the words the starter hands over (`--forms-socket`, `--socket`, `--frontend`, `--extends`,
//! `--fresh` or `--resumed`); a person never types it.
//!
//! Pattern: Command: the words become one `Command`, or a message for exit code 2.

use std::path::PathBuf;

/// The help text.
pub const HELP: &str = "fs-forms-run (--description PATH | PATH | --frontend ID --extends APPSTREAM-ID [--forms-socket PATH]) [--socket PATH] [--fresh | --resumed]\n\
    [--check] [--picture FILE.png [--size WIDTHxHEIGHT] [--screen PAGE]]\n\
The forms host: opens the window that the layout of a program describes. The starter runs it; start the program, not this.\n\
  --description P    a blueprint folder (scaffold.toml, `[frontend] layout`), a folder with forms/*.toml, or one description file;\n\
                     the host reads it and draws the window; the same as a bare PATH. Nothing of the program is built into the host\n\
  --frontend ID      the key of the frontend (a layout of the program; with a folder of several files: which one)\n\
  --extends ID       the AppStream id of the program the layout is for\n\
  --forms-socket P   the socket of `fs-forms serve` (default: $XDG_RUNTIME_DIR/freesynergy/forms.sock)\n\
  --socket PATH      the socket of the program (handed on to the data sources)\n\
  --check            only connect and fetch the layout, then exit (0 fits, 3 contract differs or no layouts, 1 failed)\n\
  --sample-data      fill the board and side list with built-in sample records (pictures, development; never a starter)\n\
  --picture FILE     draw the window to a picture, without opening it\n\
  --version, -V      print the version and exit\n\
  --help, -h         print this help and exit\n\
Exit codes: 0 done, 1 failed, 2 wrong words, 3 the layouts or their contract are not there (the starter shows the standard frontend)";

/// What the starter asked for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Config {
    /// A blueprint folder, a folder of descriptions or one description file to read instead of asking the server.
    pub description: Option<PathBuf>,
    /// The key of the frontend.
    pub frontend: String,
    /// The `AppStream` id of the program.
    pub extends: String,
    /// The socket of `fs-forms serve`, if named.
    pub forms_socket: Option<PathBuf>,
    /// The socket of the program, if named.
    pub socket: Option<PathBuf>,
    /// The session is a restart that must find its work again (`--resumed`).
    pub resumed: bool,
    /// Draw to this picture instead of opening a window.
    pub picture: Option<(PathBuf, (u32, u32))>,
    /// Fill the records blocks with the built-in sample data (pictures, development).
    pub sample_data: bool,
    /// The page to show first.
    pub page: Option<String>,
}

/// What to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// Print the version.
    Version,
    /// Print the help.
    Help,
    /// Connect, fetch the layout, exit.
    Check(Config),
    /// Open the window.
    Window(Config),
}

/// Read the words after the program name.
///
/// # Errors
/// A message for the person: an unknown option, a missing or wrong value, a missing `--frontend` or `--extends`.
pub fn parse(words: &[String]) -> Result<Command, String> {
    let mut c = Config::default();
    let (mut check, mut size, mut picture) = (false, (1000, 760), None);
    let mut it = words.iter();
    while let Some(word) = it.next() {
        let mut value = |name: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{name} needs a value"))
        };
        match word.as_str() {
            "--version" | "-V" => return Ok(Command::Version),
            "--help" | "-h" => return Ok(Command::Help),
            "--check" => check = true,
            "--sample-data" => c.sample_data = true,
            "--fresh" => c.resumed = false,
            "--resumed" => c.resumed = true,
            "--frontend" => c.frontend = value("--frontend")?,
            "--extends" => c.extends = value("--extends")?,
            "--forms-socket" => c.forms_socket = Some(PathBuf::from(value("--forms-socket")?)),
            "--socket" => c.socket = Some(PathBuf::from(value("--socket")?)),
            "--picture" => picture = Some(PathBuf::from(value("--picture")?)),
            "--size" => size = parse_size(&value("--size")?)?,
            "--screen" => c.page = Some(value("--screen")?),
            "--description" => c.description = Some(PathBuf::from(value("--description")?)),
            other if !other.starts_with('-') && c.description.is_none() => {
                c.description = Some(PathBuf::from(other));
            }
            other => return Err(format!("unknown option {other}")),
        }
    }
    if c.description.is_none() && (c.frontend.is_empty() || c.extends.is_empty()) {
        return Err(
            "--description PATH, or --frontend and --extends (the starter hands them over), is needed"
                .to_owned(),
        );
    }
    c.picture = picture.map(|p| (p, size));
    Ok(if check {
        Command::Check(c)
    } else {
        Command::Window(c)
    })
}

fn parse_size(text: &str) -> Result<(u32, u32), String> {
    let bad = || format!("--size wants WIDTHxHEIGHT, not {text}");
    let (w, h) = text.split_once('x').ok_or_else(bad)?;
    let (w, h): (u32, u32) = (w.parse().map_err(|_| bad())?, h.parse().map_err(|_| bad())?);
    if (1..=8192).contains(&w) && (1..=8192).contains(&h) {
        Ok((w, h))
    } else {
        Err(bad())
    }
}
