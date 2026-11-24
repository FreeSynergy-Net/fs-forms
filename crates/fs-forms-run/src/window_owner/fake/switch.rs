//! `use_frontend` of the double: the surface keeps its id, record, records and hold; only the key and the composition change.
//!
//! Pattern: Bridge – the surface (entry) stays, the frontend it is drawn with is exchanged.

use fs_forms_layout::FrontendKey;

use super::FakeWindows;
use crate::compose::compose;
use crate::view::Coded;

use super::super::WindowId;

impl FakeWindows {
    /// The code of every refusal leaves the surface as it was.
    pub(super) fn switch_frontend(&self, id: &WindowId, frontend: &str) -> Result<(), Coded> {
        let mut open = self.lock();
        let at = |c: &str| Coded::new(c).with("id", id.as_str());
        let e = open
            .get(id)
            .ok_or_else(|| at("host-notice-window-unknown"))?;
        let key = FrontendKey {
            extends: e.key.extends.clone(),
            frontend: frontend.to_owned(),
        };
        let Some(view) = self.known.get(&key) else {
            return Err(if self.known.keys().any(|k| k.frontend == frontend) {
                Coded::new("host-notice-frontend-other-program")
                    .with("id", id.as_str())
                    .with("frontend", frontend)
            } else {
                let mut names: Vec<&str> = self
                    .known
                    .keys()
                    .filter(|k| k.extends == key.extends)
                    .map(|k| k.frontend.as_str())
                    .collect();
                names.sort_unstable();
                Coded::new("host-notice-frontend-unknown")
                    .with("id", frontend)
                    .with("available", format!("[{}]", names.join(", ")))
            });
        };
        if self.refusing.contains(&key) {
            return Err(Coded::new("host-notice-frontend-refused")
                .with("extends", key.extends.clone())
                .with("frontend", frontend));
        }
        let elsewhere = open
            .iter()
            .find(|(k, o)| *k != id && o.parent == e.parent && o.key == key && o.scope == e.scope);
        if let Some((other, _)) = elsewhere {
            return Err(at("host-notice-frontend-open-elsewhere").with("other", other.as_str()));
        }
        let composition = compose(view, &self.catalog);
        if let Some(e) = open.get_mut(id) {
            e.key = key;
            e.composition = composition;
        }
        Ok(())
    }
}
