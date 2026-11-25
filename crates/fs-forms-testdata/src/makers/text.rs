//! Strategy for channels of art `text`; the words come from the crate `fake` (no word list of our own).
//!
//! Pattern: Strategy (`ValueMaker`).

use fake::Fake;
use fake::faker::lorem::{en, zh_cn};
use fs_interfaces::forms::{Art, Channel};
use rand::rngs::ChaCha8Rng;

use crate::value::{Flavor, MakeCx, Value, ValueMaker};

/// Words; a channel with a pool takes its values from the pool in turn, so every value of the pool comes up.
pub struct TextMaker;

impl ValueMaker for TextMaker {
    fn art(&self) -> Art {
        Art::Text
    }

    fn make(&self, cx: &mut MakeCx<'_>, _channel: &Channel) -> Option<Value> {
        if !cx.pool.is_empty() {
            let at = usize::try_from(cx.index).unwrap_or(0) % cx.pool.len();
            return Some(Value::Text(cx.pool[at].clone()));
        }
        Some(Value::Text(match cx.flavor {
            Flavor::Plain => words(cx.rng, Flavor::Plain),
            Flavor::Long => en::Paragraphs(4..7)
                .fake_with_rng::<Vec<String>, _>(cx.rng)
                .join("\n\n"),
            Flavor::NonLatin => words(cx.rng, Flavor::NonLatin),
        }))
    }
}

/// One word (for the values of a pool).
pub fn word(rng: &mut ChaCha8Rng) -> String {
    en::Word().fake_with_rng::<String, _>(rng)
}

/// Two to four words in the script of `flavor` (`Long` reads as `Plain`).
pub(super) fn words(rng: &mut ChaCha8Rng, flavor: Flavor) -> String {
    match flavor {
        Flavor::NonLatin => zh_cn::Words(2..5)
            .fake_with_rng::<Vec<String>, _>(rng)
            .concat(),
        Flavor::Plain | Flavor::Long => en::Words(2..5)
            .fake_with_rng::<Vec<String>, _>(rng)
            .join(" "),
    }
}
