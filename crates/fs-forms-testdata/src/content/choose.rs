//! The choice of the content by the AS2 type.
//!
//! Pattern: Factory Method (`for_type` picks the `Content` strategy of the standard from the AS2 type).

use fs_forms_bind::Basic;
use fs_interfaces::forms::Art;

use super::types::Content;
use super::{as2, jscalendar};

/// The content of an AS2 type: JSCalendar for `Event`, plain AS2 for the rest.
#[must_use]
pub fn for_type(data_type: &str) -> Box<dyn Content> {
    match data_type {
        "Event" => Box::new(jscalendar::JsEvent),
        _ => Box::new(as2::Plain),
    }
}

/// The basic type of a field that fills a channel of `art`.
pub(crate) fn basic_of(art: Art) -> Basic {
    match art {
        Art::Flag => Basic::Bool,
        Art::Number => Basic::Number,
        Art::Time => Basic::DateTime,
        Art::Reference => Basic::Reference,
        _ => Basic::Text,
    }
}
