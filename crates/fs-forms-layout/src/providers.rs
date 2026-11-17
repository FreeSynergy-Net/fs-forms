//! Live check against the data providers: the scheme list of a field use. One place for the `layout-error-scheme*` problems;
//! the host plan does not check again. Stand-in for the type description of fs-types 0.0.4 until it lands.
//!
//! Pattern: Strategy: the provider is behind a trait the host or the service supplies.

use crate::codes::LayoutCode;
use crate::doc::BindFrom;
use crate::layout::Layout;
use crate::port::Pending;
use crate::schemes::schemes_rule;
use crate::shim::Coded;

/// Asks a provider about a field of a data type.
pub trait ProviderCatalogs {
    /// Does the field carry schemes (webAddress, link, image)? `Ok(None)`: the type or field is not known to the provider.
    /// `Err(unreachable)`: the provider is away.
    fn field_has_schemes<'a>(
        &'a self,
        contract: &'a str,
        data_type: &'a str,
        path: &'a str,
    ) -> Pending<'a, Option<bool>>;
}

/// Checks the layout against the providers. A provider that is away gives `provider-unchecked{capability}` once, never silence.
#[must_use]
pub fn check_providers<'a>(
    layout: &'a Layout,
    providers: &'a dyn ProviderCatalogs,
) -> Pending<'a, Vec<Coded<LayoutCode>>> {
    Box::pin(async move {
        let doc = layout.doc();
        let mut out: Vec<Coded<LayoutCode>> = Vec::new();
        for page in &doc.pages {
            for block in &page.v.blocks {
                let Some(src) = block
                    .v
                    .source
                    .and_then(|n| doc.sources.get((n as usize).wrapping_sub(1)))
                else {
                    continue;
                };
                let capability = src.v.range.split_whitespace().next().unwrap_or("");
                for b in &block.v.bindings {
                    let BindFrom::Field { path, .. } = &b.v.from else {
                        continue;
                    };
                    let elem = b.elem.as_ref().map_or("", |e| e.as_str());
                    let list: Option<Vec<String>> =
                        b.v.schemes
                            .as_ref()
                            .map(|l| l.iter().map(|x| x.v.clone()).collect());
                    match providers
                        .field_has_schemes(&src.v.range, &src.v.data_type, path)
                        .await
                    {
                        Ok(Some(carries)) => {
                            out.extend(schemes_rule(elem, path, carries, list.as_deref()))
                        }
                        Err(_) => {
                            let c = Coded::new(LayoutCode::ProviderUnchecked)
                                .with("capability", capability);
                            if !out.contains(&c) {
                                out.push(c);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(out)
    })
}
