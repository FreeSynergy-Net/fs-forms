//! What the real owner needs from outside, as ports: the description of a frontend, the module of a program, a window.
//!
//! Pattern: Port (hexagonal) – the owner knows no socket, no file and no Blinc; [`BlincSurfaces`] is the one Blinc adapter.

use std::sync::Arc;

use fs_components_app::{AppWidget, WindowOptions, open_more};
use fs_forms_layout::FrontendKey;

use crate::record_hub::Module;
use crate::view::{Coded, View};

/// Pattern: Port – where the description of a frontend comes from (the layout service, a file for a picture).
pub trait Descriptions: Send + Sync {
    /// The description of `key`.
    ///
    /// # Errors
    /// `host-notice-frontend-missing{extends,frontend}` when there is none.
    fn view(&self, key: &FrontendKey) -> Result<View, Coded>;
}

/// Pattern: Port – the module of a program (the starter connects it; each window then makes its OWN connection to it).
pub trait Modules: Send + Sync {
    /// The module that serves `range` (the full `AppStream` id of the program).
    ///
    /// # Errors
    /// The code of the module that is not there or does not fit.
    fn module(&self, range: &str) -> Result<Arc<dyn Module>, Coded>;
}

/// Pattern: Port – a window of the operating system. `on_closed` runs once when the window is gone (the person closed it, or
/// `AppWidget::quit` was called); the surface drops the widget with the window.
pub trait Surfaces: Send + Sync {
    /// Shows `app` in a window of its own.
    ///
    /// # Errors
    /// `host-notice-window-failed` when the window could not be opened.
    fn show(&self, app: AppWidget, on_closed: Box<dyn FnOnce() + Send>) -> Result<(), Coded>;
}

/// Pattern: Adapter – the windows of Blinc (`open_more`; call it after `launch` started the loop, from a command or event handler).
#[derive(Clone, Debug, Default)]
pub struct BlincSurfaces {
    options: WindowOptions,
}

impl BlincSurfaces {
    /// Windows with `options` (size).
    #[must_use]
    pub fn new(options: WindowOptions) -> Self {
        Self { options }
    }
}

impl Surfaces for BlincSurfaces {
    fn show(&self, app: AppWidget, on_closed: Box<dyn FnOnce() + Send>) -> Result<(), Coded> {
        open_more(self.options.clone(), app, on_closed)
            .map_err(|e| Coded::new("host-notice-window-failed").with("cause", e.to_string()))
    }
}
