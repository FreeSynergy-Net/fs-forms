//! The root capability `Surveys` (shown as Forms): what this server offers today.
//!
//! Pattern: Adapter: the generated `surveys::Server` trait in, a value (`Root`) that only reports.

use capnp::capability::Rc;
use fs_interfaces::surveys_capnp::surveys;

/// The program name that `Control.version` and the support report carry; clients expect exactly this.
pub const PROGRAM: &str = "fs-forms";

/// The contract this server speaks: `surveys.capnp`, draft 0.1 (the file header of fs-interfaces v0.0.27; one source for server and
/// client, which tests). The number before the
/// dot is the main level, the one after it the minor level: a client accepts a server of the same main level whose minor
/// level is not lower (`fs-forms-client`).
pub const CONTRACT_VERSION: &str = "0.1.0";

type Done = Result<(), capnp::Error>;

/// The root capability. It holds the contract version it reports (tests give another one).
pub struct Root {
    contract: String,
}

impl Root {
    /// A root that reports `contract` as its contract version.
    #[must_use]
    pub fn new(contract: impl Into<String>) -> Self {
        Self {
            contract: contract.into(),
        }
    }

    /// The capability to hand to the transport.
    #[must_use]
    pub fn into_client(self) -> capnp::capability::Client {
        capnp_rpc::new_client::<surveys::Client, _>(self).client
    }
}

impl surveys::Server for Root {
    /// What this program offers: no capability ids yet (a frontend locks everything), and the contract version.
    async fn capabilities(
        self: Rc<Self>,
        _: surveys::CapabilitiesParams,
        mut results: surveys::CapabilitiesResults,
    ) -> Done {
        let mut out = results.get().init_capabilities();
        out.set_contract_version(self.contract.as_str());
        out.init_ids(0);
        Ok(())
    }

    /// The provider rule (ADR 0045): the contract version and the provider; no entries, because nothing is offered yet.
    async fn support_report(
        self: Rc<Self>,
        _: surveys::SupportReportParams,
        mut results: surveys::SupportReportResults,
    ) -> Done {
        let mut report = results.get().init_report();
        report.set_contract_version(self.contract.as_str());
        report.set_provider(PROGRAM);
        report.init_entries(0);
        Ok(())
    }
}
