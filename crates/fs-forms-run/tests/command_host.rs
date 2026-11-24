//! A consumer outside the crate: `Command` is `non_exhaustive`, so this host MUST carry a fallback arm with a visible message.
//! (Without the `_` arm this file would not compile. Not red against the old state: the old enum allowed a complete match.)

use fs_forms_run::{Command, parse};

fn host_line(words: &[&str]) -> String {
    let words: Vec<String> = words.iter().map(|w| (*w).to_owned()).collect();
    match parse(&words) {
        Err(message) => format!("usage: {message}"),
        Ok(Command::Version) => "version".to_owned(),
        Ok(Command::Help) => "help".to_owned(),
        Ok(Command::ListBlocks) => "list-blocks".to_owned(),
        Ok(Command::Check(_)) => "check".to_owned(),
        Ok(Command::Window(_)) => "window".to_owned(),
        Ok(_) => "this command is not supported by this host".to_owned(),
    }
}

#[test]
fn a_host_with_a_fallback_arm_names_every_known_command() {
    assert_eq!(host_line(&["--version"]), "version");
    assert_eq!(host_line(&["--help"]), "help");
    assert_eq!(host_line(&["--list-blocks"]), "list-blocks");
    assert_eq!(host_line(&["--description", "x", "--check"]), "check");
    assert_eq!(host_line(&["--description", "x"]), "window");
    assert!(host_line(&["--nope"]).starts_with("usage: "));
}
