// SPDX-License-Identifier: AGPL-3.0-or-later
//! The evaluator's static Producer bundle used to qualify owner records.

use filament_observation::{
    ProducerConfiguration, Revision, StaticProducerBundle, PRODUCER_INTERFACE_VERSION,
};
use serde_json::json;

const BUNDLE_IDENTITY: &str = "fsl-eval";
const CONFIGURATION_IDENTITY: &str = "fsl-eval/default";

/// Build the evaluator's fixed producer selection for caller-owned admission.
///
/// The bundle and configuration digests are selected by the owner's canonical
/// digest API. A refusal here indicates an internal producer-contract defect.
pub fn producer_bundle() -> Result<StaticProducerBundle, filament_observation::Refusal> {
    let mut configuration_selection = StaticProducerBundle::default();
    configuration_selection.extra.insert(
        "configurationIdentity".to_owned(),
        json!(CONFIGURATION_IDENTITY),
    );
    let configuration_digest = configuration_selection.canonical_digest_selection()?;

    let mut bundle = StaticProducerBundle {
        bundle_identity: Some(BUNDLE_IDENTITY.to_owned()),
        bundle_revision: Some(Revision::producer(env!("CARGO_PKG_VERSION"))),
        interface_version: Some(PRODUCER_INTERFACE_VERSION.to_owned()),
        configuration: Some(ProducerConfiguration {
            configuration_identity: CONFIGURATION_IDENTITY.to_owned(),
            digest: configuration_digest,
        }),
        relationships: Some(Vec::new()),
        ..StaticProducerBundle::default()
    };
    let digest = bundle.canonical_digest_selection()?;
    bundle.digest = Some(digest);
    Ok(bundle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ix_trace_rs::trace("TC-3036", "FR-944-AC-8")]
    fn caller_admits_the_fixed_producer_selection() {
        let first = producer_bundle()
            .expect("producer selection")
            .admit()
            .expect("admission");
        let second = producer_bundle()
            .expect("producer selection")
            .admit()
            .expect("admission");
        assert_eq!(first, second);
        assert_eq!(first.bundle_identity(), BUNDLE_IDENTITY);
        assert_eq!(first.bundle_revision().value, env!("CARGO_PKG_VERSION"));
        assert_eq!(first.interface_version(), "1.2.0");
        assert_eq!(
            first.configuration().configuration_identity,
            CONFIGURATION_IDENTITY
        );
        assert!(first.relationships().is_empty());
    }
}
