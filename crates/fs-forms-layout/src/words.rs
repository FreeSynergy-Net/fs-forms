//! The closed text words of the model enums (the words of the exchange file), one table per enum, read and written from one place.
//!
//! Pattern: Value Object with a 1:1 text form.

use crate::doc::{
    Confirm, MenuKind, MenuStandard, Navigation, OnWhat, OrderBy, Placement, Shown, Trigger,
};

/// A closed word list: every variant has exactly one word.
pub trait Words: Sized + Copy {
    /// The word of a variant.
    fn word(self) -> &'static str;
    /// The variant of a word.
    fn from_word(w: &str) -> Option<Self>;
}

macro_rules! words {
    ($t:ty { $($v:ident => $w:literal),* $(,)? }) => {
        impl Words for $t {
            fn word(self) -> &'static str { match self { $(<$t>::$v => $w),* } }
            fn from_word(w: &str) -> Option<Self> { match w { $($w => Some(<$t>::$v),)* _ => None } }
        }
    };
}
words!(OnWhat { Always => "always", Selected => "selected" });
words!(Navigation { Single => "single", ListDetail => "list-detail" });
words!(Trigger { Button => "button", Menu => "menu", Key => "key", Drop => "drop", QuickAdd => "quick-add", Submit => "submit" });
words!(Confirm { None => "none", Confirm => "confirm", Choice => "choice" });
words!(OrderBy { Field => "field", Order => "order" });
words!(Placement { Panel => "panel", Window => "window" });
words!(Shown { Start => "start", OnOpen => "on-open" });

impl Words for MenuKind {
    fn word(self) -> &'static str {
        match self {
            MenuKind::Menu => "menu",
            MenuKind::Standard(MenuStandard::Program) => "standard-program",
            MenuKind::Standard(MenuStandard::Help) => "standard-help",
            MenuKind::Command => "command",
            MenuKind::View => "view",
            MenuKind::Separator => "separator",
        }
    }
    fn from_word(w: &str) -> Option<Self> {
        Some(match w {
            "menu" => MenuKind::Menu,
            "standard-program" => MenuKind::Standard(MenuStandard::Program),
            "standard-help" => MenuKind::Standard(MenuStandard::Help),
            "command" => MenuKind::Command,
            "view" => MenuKind::View,
            "separator" => MenuKind::Separator,
            _ => return None,
        })
    }
}
