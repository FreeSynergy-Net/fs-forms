//! The words a module brings (its `catalog/texts/`), and the one rule that puts them behind the words of the host: what `ctx.say` answers.
//!
//! Pattern: Chain of Responsibility (the host's words ask first, the module's words second, the key itself last; the chain is the
//! existing `fs_components_words::Fallback`) behind the Port [`ModuleWords`]. The host never reads a module's files (the starter
//! does, `Mf2Texts::from_packed`), it is handed the words.

use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

use fs_components_words::{Args, Fallback, Locale, Said, TextId, Words};

/// Pattern: Port + Factory Method – the words of a connected module, made per language (`in_locale`); ADR 0057: the module's own
/// `texts/`, ids like `person.emails`.
pub trait ModuleWords: Send + Sync + Debug {
    /// The words of the module for `locale` (a language the module has no file for answers in its own fallback language).
    fn in_locale(&self, locale: &Locale) -> Arc<dyn Words>;
}

/// The sentences the host has, by key (a key it lacks is missing, so the next in the chain asks).
struct HostWords(HashMap<String, String>);

impl Words for HostWords {
    fn say(&self, id: &TextId, _args: &Args) -> Said {
        self.0
            .get(id.as_str())
            .map_or_else(|| Said::missing(id), |text| Said::found(id, text.clone()))
    }
}

/// The words of the module, or none (every key missing).
struct Brought(Option<Arc<dyn Words>>);

impl Words for Brought {
    fn say(&self, id: &TextId, args: &Args) -> Said {
        self.0
            .as_ref()
            .map_or_else(|| Said::missing(id), |w| w.say(id, args))
    }
}

/// The sentence of a key: the sentences of the host (by key), else the module's words, else the key.
pub(crate) struct SayChain(Fallback<HostWords, Brought>);

impl SayChain {
    pub(crate) fn new(host: HashMap<String, String>, module: Option<Arc<dyn Words>>) -> Self {
        Self(Fallback::new(HostWords(host), Brought(module)))
    }

    /// The sentence of `key`; the key itself when nobody has one (`Said::missing` shows the identifier).
    pub(crate) fn sentence(&self, key: &str) -> String {
        self.0
            .say(&TextId::new(key), &Args::new())
            .text()
            .to_owned()
    }
}

#[cfg(test)]
mod tests;
