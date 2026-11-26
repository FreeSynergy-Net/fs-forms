//! The host of the picture tests: `--sample-data` is answered the way the starter answers it (package 4b): the data is derived from
//! the description (its layout and its section `[sample]`) with the real catalog adapter, and served by `SampleModule`.
//! One module per source of the description; a description with several sources is a code, not a guess (card: the host of
//! these tests reads only descriptions of one source).
//!
//! Pattern: Strategy (the port `Sources`, here with the sample module behind it) over the Adapter of `records_of`.

use std::path::Path;
use std::sync::Arc;

use fs_components_kanban::Answer;
use fs_forms_run::{
    CatalogBlockForms, Clock, Coded, Described, FormData, Records, SideItem, Sources, UndoState,
    Want,
};
use fs_forms_testdata::SampleSpec;

use super::{columns, records_scoped};

/// The sources of the picture tests: nothing real is connected, the sample is derived from the description.
#[derive(Clone, Copy, Debug, Default)]
pub struct Derived;

fn failed(code: &str, why: impl std::fmt::Debug) -> Coded {
    Coded::new(code).with("why", format!("{why:?}"))
}

impl Sources for Derived {
    fn connect(&self, range: &str, _: Option<&Path>, _: &Path) -> Result<Arc<dyn Records>, Coded> {
        Err(Coded::new("host-notice-source-unknown").with("contract", fs_forms_run::name_of(range)))
    }

    fn connect_sample(
        &self,
        wants: &[Want],
        described: &Described<'_>,
        _: Option<&Path>,
        _: &Path,
    ) -> Result<Arc<dyn Records>, Coded> {
        let layout = described
            .layout()
            .ok_or_else(|| Coded::new("sample-no-description"))?;
        let mut specs =
            SampleSpec::from_description(layout, &CatalogBlockForms, described.sample())
                .map_err(|e| failed("sample-spec", e))?;
        match (specs.pop(), specs.is_empty()) {
            (Some(one), true) => {
                let scope = wants.first().map_or("", |w| w.scope.as_str());
                Ok(Arc::new(WithSide::of(records_scoped(one, scope))))
            }
            (None, _) => Err(Coded::new("sample-no-source")),
            (Some(_), false) => Err(Coded::new("sample-many-sources")),
        }
    }
}

/// The records of the module and a side list. A module offers no side list (the list of a real window is the module's own form); the
/// host of these tests shows the values of the channel `column` there, so the place of the side list in the window can be drawn.
struct WithSide {
    inner: Arc<dyn Records>,
    side: Vec<SideItem>,
}

impl WithSide {
    fn of(inner: Arc<dyn Records>) -> Self {
        let side = columns(&*inner)
            .into_iter()
            .map(|key| SideItem {
                label: key.clone(),
                key,
            })
            .collect();
        Self { inner, side }
    }
}

impl Records for WithSide {
    fn side(&self) -> Vec<SideItem> {
        self.side.clone()
    }
    fn forms(&self) -> Arc<Vec<FormData>> {
        self.inner.forms()
    }
    fn command_of(&self, form: &str, channel: &str) -> Option<String> {
        self.inner.command_of(form, channel)
    }
    fn can_channel(&self, form: &str, channel: &str, record: Option<&str>) -> bool {
        self.inner.can_channel(form, channel, record)
    }
    fn invoke(
        &self,
        form: &str,
        channel: &str,
        record: Option<&str>,
        values: &[(String, String)],
    ) -> Answer {
        self.inner.invoke(form, channel, record, values)
    }
    fn history(&self) -> UndoState {
        self.inner.history()
    }
    fn step_back(&self, redo: bool) -> Answer {
        self.inner.step_back(redo)
    }
    fn changed(&self) -> u64 {
        self.inner.changed()
    }
    fn reread(&self) -> Result<(), Coded> {
        self.inner.reread()
    }
    fn module_words(&self) -> Option<Arc<dyn fs_forms_run::ModuleWords>> {
        self.inner.module_words()
    }
    fn is_sample(&self) -> bool {
        self.inner.is_sample()
    }
    fn clock(&self) -> Arc<dyn Clock> {
        self.inner.clock()
    }
    fn stale(&self) -> Option<String> {
        self.inner.stale()
    }
    fn source_problems(&self) -> Vec<(u32, Coded)> {
        self.inner.source_problems()
    }
    fn unavailable(&self) -> Option<String> {
        self.inner.unavailable()
    }
}
