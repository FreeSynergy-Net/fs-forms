//! The contract version a frontend needs, and the check against what the server reports.
//!
//! Pattern: Value object plus a pure function. The one rule of the toolkit (ADR 0045, 0064; V-4): a version is `semver::Version`
//! (three parts), compatible means the Cargo caret `^`. For 0.x that is strict: 0.2.0 needs 0.2.x, 0.3.0 is a break.

use semver::{Version, VersionReq};

use crate::error::Error;

/// What this client needs of the server's contract (`layouts`, draft 0.1.0).
pub const EXPECTED_CONTRACT: &str = "0.1.0";

/// Reads a version text: exactly `x.y.z` (`SemVer`), no pre-release, no build part. `0.1` is no version.
fn read(text: &str) -> Option<Version> {
    let v = Version::parse(text.trim()).ok()?;
    (v.pre.is_empty() && v.build.is_empty()).then_some(v)
}

/// Whether a server that reports `found` fits a client that needs `expected` (Cargo caret: `^expected`).
///
/// # Errors
/// [`Error::Contract`] when a text is not a three-part version, the server is not on the same compatible line, or older than needed.
pub fn check_contract(found: &str, expected: &str) -> Result<(), Error> {
    let bad = || Error::Contract {
        found: found.to_owned(),
        expected: expected.to_owned(),
    };
    let found_v = read(found).ok_or_else(bad)?;
    let need = read(expected).ok_or_else(bad)?;
    let req = VersionReq::parse(&format!("^{need}")).map_err(|_| bad())?;
    if req.matches(&found_v) {
        Ok(())
    } else {
        Err(bad())
    }
}

#[cfg(test)]
mod tests {
    use super::{EXPECTED_CONTRACT, check_contract};

    #[test]
    fn the_same_or_a_newer_patch_level_fits() {
        assert!(check_contract("0.1.0", EXPECTED_CONTRACT).is_ok());
        assert!(check_contract("0.1.7", EXPECTED_CONTRACT).is_ok());
        assert!(check_contract("1.2.0", "1.1.0").is_ok());
    }

    #[test]
    fn a_zero_x_minor_step_is_a_break_as_the_caret_says() {
        // the old reader let 0.2 pass for a need of 0.1
        assert!(check_contract("0.2.0", EXPECTED_CONTRACT).is_err());
        assert!(check_contract("0.2.0", "0.1.5").is_err());
    }

    #[test]
    fn another_main_level_an_older_level_or_no_three_part_version_is_refused() {
        for found in [
            "1.1.0",
            "0.0.9",
            "0.1",
            "0.1.0-rc.1",
            "surveys 0.1.0",
            "surveys",
            "",
            "0.x",
        ] {
            assert!(check_contract(found, EXPECTED_CONTRACT).is_err(), "{found}");
        }
        assert!(
            check_contract("0.1.0", "0.1").is_err(),
            "a need of two parts"
        );
        assert!(
            check_contract("1.0.0", "1.1.0").is_err(),
            "minor older than needed"
        );
    }
}
