// SPDX-License-Identifier: AGPL-3.0-or-later
//! The evaluator's static Producer bundle used to qualify owner records.

use std::sync::OnceLock;

use filament_observation::{
    AdmittedStaticBundle, ProducerConfiguration, Revision, StaticProducerBundle,
    PRODUCER_INTERFACE_VERSION,
};
use serde_json::json;

const BUNDLE_IDENTITY: &str = "fsl-eval";
const CONFIGURATION_IDENTITY: &str = "fsl-eval/default";

static PRODUCER: OnceLock<Result<AdmittedStaticBundle, String>> = OnceLock::new();

/// The one admitted static bundle that identifies this evaluator as producer.
///
/// The bundle and configuration digests are selected by the owner's canonical
/// digest API. A refusal here indicates an internal producer-contract defect.
pub(super) fn admitted() -> Result<&'static AdmittedStaticBundle, &'static str> {
    PRODUCER.get_or_init(build).as_ref().map_err(String::as_str)
}

fn build() -> Result<AdmittedStaticBundle, String> {
    let mut configuration_selection = StaticProducerBundle::default();
    configuration_selection.extra.insert(
        "configurationIdentity".to_owned(),
        json!(CONFIGURATION_IDENTITY),
    );
    let configuration_digest = configuration_selection
        .canonical_digest_selection()
        .map_err(|error| error.to_string())?;

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
    let digest = bundle
        .canonical_digest_selection()
        .map_err(|error| error.to_string())?;
    bundle.digest = Some(digest);
    bundle.admit().map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ix_trace_rs::trace("TC-3036", "FR-944-AC-8")]
    fn static_producer_bundle_is_owner_admitted_once() {
        let first = admitted().expect("static bundle admission");
        let second = admitted().expect("cached static bundle admission");

        assert!(std::ptr::eq(first, second));
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
