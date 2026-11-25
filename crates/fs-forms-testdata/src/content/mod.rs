//! Content by standard: how the values of a record are stored in the object of its type, and read back.
//!
//! Pattern: Strategy (`Content`, chosen from the AS2 type by `for_type`: a small table "standard -> content", no program is named)
//! + Adapter (channel values <-> the properties of the standard).

mod as2;
mod choose;
mod jscalendar;
mod text;
mod types;

pub(crate) use choose::basic_of;
pub use choose::for_type;
pub use text::moment_text;
pub use types::{Content, ContentCtx, FieldMap, Order, Parts};
