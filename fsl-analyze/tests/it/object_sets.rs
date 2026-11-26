// SPDX-License-Identifier: AGPL-3.0-or-later
//! Hand-valued liveness component-certificate cases for claim object sets.

use filament_exact::Cancel;
use fsl_replay::{
    check_components, ClosureCertificate, ComponentCertificate, ModelCheckLimits,
    ModelCheckOutcome, ProofCertificate, TemporalCertificateCheck,
};
use ix_trace_rs::trace;

/// FR-126-AC-2: `eventually p` holds when its sole initial state satisfies
/// `p`. The negated liveness product and its component partition are empty.
#[trace("TC-521", "FR-126-AC-1")]
#[test]
fn zz_components() {
    let one_object = crate::liveness_support::fixture(
        crate::counter::document(false),
        crate::counter::CONTRACT,
        "",
        "eventually holds(c.value <= 1)",
        &[("c", 1)],
    );
    let outcome = one_object
        .decide(|value| Some(value <= 1))
        .expect("the liveness run completes");
    let ModelCheckOutcome::Holds {
        certificate: ProofCertificate::Component(certificate),
        ..
    } = outcome
    else {
        panic!("the initial state satisfies the eventuality: {outcome:?}");
    };
    assert_eq!(
        certificate,
        ComponentCertificate {
            closure: ClosureCertificate { states: Vec::new() },
            components: Vec::new(),
        }
    );
    let letters = one_object.letters(|value| Some(value <= 1));
    let checked = check_components(
        &one_object.certificate_request(&letters, crate::counter::UNLIMITED),
        &certificate,
        &ModelCheckLimits::default(),
        &Cancel::new(),
    )
    .expect("the component certificate is checked");
    assert_eq!(checked, TemporalCertificateCheck::Accepted);
}
