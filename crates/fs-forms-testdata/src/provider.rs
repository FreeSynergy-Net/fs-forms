//! The provider text of the module: how a host recognises test data.
//!
//! Pattern: Null Object / Test Double marker – `hello` of the contract says `provider = "fake"` for example data (comment of the
//! contract, decision C3 25.11.: the wire stays text, a capnp enum would read old fake services as real).

/// What `SampleModule::provider` says. C3 brings `fs_interfaces::module::SAMPLE_PROVIDER` and `ProviderKind::Sample`; until
/// then this constant is the only place that knows the text, and it is replaced by theirs (card in `BERICHT-fixture-3.md`).
pub const SAMPLE_PROVIDER: &str = "fake";

/// Whether a provider text of `hello` means example data (what the host shows as "Testdaten"). Until C3's `ProviderKind::of`.
#[must_use]
pub fn is_sample_provider(provider: &str) -> bool {
    provider == SAMPLE_PROVIDER
}
