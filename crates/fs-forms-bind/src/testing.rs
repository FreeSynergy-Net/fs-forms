//! Test doubles and fixtures (feature `testing`): `TestCatalog` until G7, fake provider facts, the tasks-board layout by hand.
//!
//! The choose table is written down here INDEPENDENTLY of any code (PLAN-grundtyp-felder §7.2, H-K2), so a wrong `choose` in
//! the real catalog would not be copied. Pattern: Test Double + Object Mother.

use crate::catalog::{BlockSpec, Catalog, SlotSpec};
use crate::coded::Coded;
use crate::model::{CommandIn, Target};
use crate::providers::{Offer, Provider, ProviderFacts, SettingsFacts, SettingsHealth};
use crate::types::{Basic, CommandDesc, FieldDesc, Limits, Role};

/// Catalog with a fixed table; `unbuilt` blocks are known but not built.
pub struct TestCatalog {
    /// Known blocks.
    pub blocks: Vec<BlockSpec>,
    /// Blocks without a factory.
    pub unbuilt: Vec<String>,
}

fn slot(name: &str, min: u32, max: Option<u32>) -> SlotSpec {
    SlotSpec {
        name: name.to_owned(),
        min,
        max,
    }
}

impl TestCatalog {
    /// Blocks of the tasks-board fixture plus `probe-field`.
    #[must_use]
    pub fn new() -> Self {
        let b = |n: &str, s: Vec<SlotSpec>| BlockSpec {
            name: n.to_owned(),
            slots: s,
        };
        let blocks = vec![
            b(
                "board",
                vec![
                    slot("cards", 1, Some(1)),
                    slot("card-title", 0, None),
                    slot("card-facts", 0, None),
                    slot("card-menu", 0, None),
                ],
            ),
            b(
                "board-detail",
                vec![slot("properties", 0, None), slot("actions", 0, None)],
            ),
            b("empty-state", vec![]),
            b("notes-list", vec![slot("rows", 0, None)]),
            b("form", vec![slot("fields", 0, None)]),
            b("probe-field", vec![]),
            b("filter-bar", vec![slot("conditions", 0, None)]),
        ];
        Self {
            blocks,
            unbuilt: Vec::new(),
        }
    }
}

impl Default for TestCatalog {
    fn default() -> Self {
        Self::new()
    }
}

impl Catalog for TestCatalog {
    fn need(&self, block: &str) -> Result<&BlockSpec, Coded> {
        self.blocks
            .iter()
            .find(|b| b.name == block)
            .ok_or_else(|| Coded::new("accepts-error-block-unknown").with("block", block))
    }

    fn choose(&self, f: &FieldDesc, role: Role) -> String {
        let name = match (f.basic, f.target.as_deref(), role) {
            (Basic::Reference, Some("Person"), _) => "person-picker",
            (Basic::Reference, _, _) => "field-reference",
            (Basic::Email, _, _) => "field-email",
            (Basic::DateTime, _, _) => "field-date-time",
            (Basic::Date, _, _) => "field-date",
            (Basic::Choice, _, _) => "field-choice",
            (Basic::Keywords, _, _) => "field-keywords",
            (Basic::Number, _, _) => "field-number",
            (Basic::Bool, _, _) => "field-bool",
            (Basic::Link, _, _) => "field-link",
            (Basic::Image, _, _) => "field-image",
            (Basic::Text | Basic::Unknown, _, _) => "field-text",
        };
        name.to_owned()
    }

    fn built(&self, block: &str) -> bool {
        !self.unbuilt.iter().any(|b| b == block)
    }
}

/// Fake provider facts by capability.
pub struct FakeFacts(pub Vec<(String, Offer)>);

impl ProviderFacts for FakeFacts {
    fn offer(&self, capability: &str) -> Offer {
        self.0
            .iter()
            .find(|(c, _)| c == capability)
            .map_or(Offer::NotInstalled, |(_, o)| o.clone())
    }
}

/// A provider of one version.
#[must_use]
pub fn provider(major: u32, minor: u32) -> Provider {
    Provider {
        version: (major, minor),
    }
}

/// Facts: `tasks` 0.2 reached.
#[must_use]
pub fn tasks_facts() -> FakeFacts {
    FakeFacts(vec![(
        "tasks".to_owned(),
        Offer::Reached(vec![provider(0, 2)]),
    )])
}

/// The standard menus as the host reads them from the shell.
#[must_use]
pub fn standard() -> crate::menu_std::StandardMenus {
    crate::menu_std::StandardMenus::from_shell()
}

/// Settings without a person style, store healthy.
#[must_use]
pub fn settings() -> SettingsFacts {
    SettingsFacts {
        style: None,
        limits: Limits::default(),
        search: None,
        health: SettingsHealth::Ok,
    }
}

/// A field description.
#[must_use]
pub fn field(path: &str, basic: Basic) -> FieldDesc {
    FieldDesc {
        path: vec![path.to_owned()],
        basic,
        target: None,
        read_only: false,
    }
}

/// A command description.
#[must_use]
pub fn cmd(name: &str, supported: bool, writes: bool, on_record: bool) -> CommandIn {
    CommandIn {
        source: 0,
        name: name.to_owned(),
        desc: CommandDesc {
            supported,
            writes,
            on_record,
            type_may: None,
        },
        target: Target::Program,
    }
}

pub use crate::fixtures::{board_bindings, tasks_resolved};
