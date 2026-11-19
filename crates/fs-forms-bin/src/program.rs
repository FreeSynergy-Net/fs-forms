//! What `fs-forms` gives the starter (`fs-starter`): its names, its contract and its server. Nothing else lives in this binary.
//!
//! Pattern: Strategy (the program implements `fs_starter::Program`) + Adapter (`serve` over `fs_forms_server::serve`).

use std::ffi::OsString;

use fs_forms_client::PROGRAM;
use fs_forms_server::ServeError;
use fs_starter::{ContractReq, Exit, Failure, FrontendId, Program, ProgramName};
use fs_types::coded::{Coded, ErrorCode};

/// The `AppStream` id of the program: what a frontend manifest names in `<extends>` (`packaging/frontends/simple.metainfo.xml`).
pub const APP_ID: &str = "net.freesynergy.Forms";

/// The contract the server offers (the manifest of a frontend asks for `surveys 0.1`).
pub const CONTRACT: &str = "surveys 0.1";

/// The frontend that always comes with the program (ADR 0074 §8).
pub const STANDARD: &str = "simple";

/// The codes of this program's own part (`serve`): `forms-error-usage`, `forms-error-serve-failed`; the text of the server is the parameter `message`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormsCode {
    /// The words of `serve` were wrong.
    Usage,
    /// The server stopped with an error.
    ServeFailed,
}

impl ErrorCode for FormsCode {
    const PREFIX: &'static str = "forms-error-";
    const ALL: &'static [Self] = &[Self::Usage, Self::ServeFailed];

    fn as_str(self) -> &'static str {
        match self {
            Self::Usage => "usage",
            Self::ServeFailed => "serve-failed",
        }
    }
}

/// `fs-forms`.
pub struct Forms {
    name: ProgramName,
    contract: ContractReq,
    standard: FrontendId,
}

impl Default for Forms {
    fn default() -> Self {
        Self {
            // PROGRAM is `fs-forms`, the name the server says in `Control.version`; the starter's name is without `fs-`.
            name: ProgramName::parse(PROGRAM.trim_start_matches("fs-"))
                .expect("a valid program name"),
            contract: ContractReq::parse(CONTRACT).expect("a valid contract"),
            standard: FrontendId::parse(STANDARD).expect("a valid frontend id"),
        }
    }
}

impl Program for Forms {
    fn name(&self) -> &ProgramName {
        &self.name
    }

    #[allow(clippy::unnecessary_literal_bound)] // the signature is the trait's
    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn app_id(&self) -> &str {
        APP_ID
    }

    fn contract(&self) -> &ContractReq {
        &self.contract
    }

    fn standard_frontend(&self) -> &FrontendId {
        &self.standard
    }

    fn serve(&self, words: &[OsString]) -> Result<(), Failure> {
        fs_forms_server::serve(words.iter().cloned()).map_err(failure)
    }
}

/// The server's end as the starter's failure: exit 2 for wrong words, 1 for anything else; the code carries the server's text.
fn failure(e: ServeError) -> Failure {
    let (exit, code, message) = match e {
        ServeError::Usage(m) => (Exit::USAGE, FormsCode::Usage, m),
        ServeError::Failed(m) => (Exit::FAILED, FormsCode::ServeFailed, m),
    };
    Failure {
        exit,
        code: Coded::new(code).with_text("message", message).erase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_names_the_starter_reads_are_well_formed() {
        let forms = Forms::default();
        assert_eq!(forms.name().as_str(), "forms");
        assert_eq!(forms.app_id(), "net.freesynergy.Forms");
        assert_eq!(forms.contract().to_string(), "surveys 0.1");
        assert_eq!(forms.standard_frontend().as_str(), "simple");
        assert_eq!(forms.version(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn the_server_end_keeps_its_exit_and_its_text() {
        let usage = failure(ServeError::Usage("bad word".into()));
        assert_eq!(usage.exit, Exit::USAGE);
        assert_eq!(
            usage.code.to_string(),
            "forms-error-usage (message=bad word)"
        );
        let failed = failure(ServeError::Failed("no socket".into()));
        assert_eq!(failed.exit, Exit::FAILED);
        assert!(
            failed
                .code
                .to_string()
                .starts_with("forms-error-serve-failed")
        );
    }

    #[test]
    fn wrong_serve_words_are_a_usage_failure_not_a_start() {
        let e = Forms::default()
            .serve(&["--nonsense".into()])
            .expect_err("an unknown option is refused");
        assert_eq!(e.exit, Exit::USAGE);
    }
}
