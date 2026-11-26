//! The words a module brings (its `catalog/texts/`), and the one rule that puts them behind the words of the host: what `ctx.say` answers.
//!
//! Pattern: Chain of Responsibility (the host's words ask first, the module's words second, the key itself last) behind the Port
//! [`ModuleWords`]. The host never reads a module's files (the starter does, `Mf2Texts::from_packed`), it is handed the words.

use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

use fs_components_words::{Args, Locale, TextId, Words};

/// The words of a connected module in the language of the session (ADR 0057: the module's own `texts/`, ids like `person.emails`).
pub trait ModuleWords: Send + Sync + Debug {
    /// The words of the module for `locale` (a language the module has no file for answers in its own fallback language).
    fn in_locale(&self, locale: &Locale) -> Arc<dyn Words>;
}

/// The sentence of a key: the sentences of the host (by key), else the module's words, else the key.
pub(crate) struct SayChain {
    host: HashMap<String, String>,
    module: Option<Arc<dyn Words>>,
}

impl SayChain {
    pub(crate) fn new(host: HashMap<String, String>, module: Option<Arc<dyn Words>>) -> Self {
        Self { host, module }
    }

    /// The sentence of `key`; the key itself when nobody has one.
    pub(crate) fn sentence(&self, key: &str) -> String {
        if let Some(own) = self.host.get(key) {
            return own.clone();
        }
        self.module
            .as_ref()
            .map(|w| w.say(&TextId::new(key), &Args::new()))
            .filter(fs_components_words::Said::is_found)
            .map_or_else(|| key.to_owned(), |said| said.text().to_owned())
    }
}

#[cfg(test)]
mod tests;
