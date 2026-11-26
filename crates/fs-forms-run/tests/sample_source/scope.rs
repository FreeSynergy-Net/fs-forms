//! The records of a description file the way the host reads them: the sample of its `[sample]` section, asked with the scope of its
//! first `[[source]]` (`all` is the host word for "no scope"; any other word goes to the module unchanged).
//!
//! Pattern: Adapter (description text -> `Records`), over `records_scoped`.

use std::path::Path;
use std::sync::Arc;

use fs_forms_exchange::{Format, read_layout};
use fs_forms_run::Records;
use fs_forms_testdata::SourceSpec;

use super::records_scoped;

/// The scope the module is given for the first source of the description `text` (`""` = all).
#[must_use]
pub fn scope_of(text: &str) -> String {
    let doc = read_layout(&mut text.as_bytes(), Path::new("people.toml"), Format::Toml)
        .expect("the description reads");
    doc.sources
        .first()
        .and_then(|s| s.v.scope.as_ref())
        .map(|s| s.text().to_owned())
        .filter(|s| s != "all" && s != "selected-list")
        .unwrap_or_default()
}

/// The records of the description `text`.
#[must_use]
pub fn records_of_text(text: &str) -> Arc<dyn Records> {
    let source = SourceSpec::from_wish_text(text).expect("the wish reads");
    records_scoped(source, &scope_of(text))
}
