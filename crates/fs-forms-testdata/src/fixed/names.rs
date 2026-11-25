//! Names instead of IRIs: the rule of the projection for every source, generated or fixed.
//!
//! Pattern: Projection step – a text channel whose field holds an IRI shows the display name of the thing it points at; the stored
//! object keeps the reference. The name is formed here, where the real module forms it in `page`, never in a frontend.

use fs_interfaces::forms::{Art, Form};

use super::Names;
use crate::error::{Failure, SampleCode};
use crate::set::SampleRow;

/// Whether `text` is an IRI of the kinds a stored record uses for a reference (a URN, a web address, a mail address): one word.
fn is_iri(text: &str) -> bool {
    !text.contains(char::is_whitespace)
        && ["urn:", "http://", "https://", "mailto:"]
            .iter()
            .any(|scheme| text.starts_with(scheme))
}

/// Replaces the IRI in every text channel of `rows` by its name.
///
/// # Errors
/// `name-missing` (channel and key) for an IRI that `names` does not know.
pub(crate) fn resolve_names(
    form: Option<&Form>,
    names: &Names,
    rows: &mut [SampleRow],
) -> Result<(), Failure> {
    let Some(form) = form else {
        return Ok(());
    };
    for row in rows {
        for (channel, values) in &mut row.values {
            let text = form
                .channel(channel)
                .is_some_and(|c| c.arts.contains(&Art::Text));
            if !text {
                continue;
            }
            for value in values.iter_mut().filter(|v| is_iri(v)) {
                let name = names.get(value.as_str()).ok_or_else(|| {
                    Failure::new(SampleCode::NameMissing)
                        .with_text("channel", channel.as_str())
                        .with_text("key", row.key.as_str())
                })?;
                name.clone_into(value);
            }
        }
    }
    Ok(())
}
