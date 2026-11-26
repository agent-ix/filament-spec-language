// SPDX-License-Identifier: AGPL-3.0-or-later
//! Empty liveness products and component partitions are valid only when the
//! claim already holds at the initial state.

use filament_exact::Cancel;
use fsl_replay::{
    ClosureCertificate, ComponentCertificate, ModelCheckLimits, ModelCheckOutcome,
    ProofCertificate, TemporalCertificateCheck, check_components,
};
use ix_trace_rs::trace;

/// FR-126-AC-1: `eventually p` holds when its sole initial state satisfies
/// `p`. The negated liveness product and its component partition are empty.
#[trace("TC-521", "FR-126-AC-1")]
#[test]
fn empty_components_prove_an_eventuality_true_at_the_initial_state() {
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

/// FR-339-AC-8: an empty certificate cannot prove an eventuality false at
/// the initial state; the checker requires its reached product state.
#[trace("TC-526", "FR-339-AC-8", "FR-126-AC-1")]
#[test]
fn empty_components_reject_an_eventuality_false_at_the_initial_state() {
    let one_object = crate::liveness_support::fixture(
        crate::counter::document(false),
        crate::counter::CONTRACT,
        "",
        "eventually holds(c.value <= 0)",
        &[("c", 1)],
    );
    let letters = one_object.letters(|value| Some(value <= 0));
    let empty = ComponentCertificate {
        closure: ClosureCertificate { states: Vec::new() },
        components: Vec::new(),
    };
    let checked = check_components(
        &one_object.certificate_request(&letters, crate::counter::UNLIMITED),
        &empty,
        &ModelCheckLimits::default(),
        &Cancel::new(),
    )
    .expect("the empty certificate is checked");
    let TemporalCertificateCheck::Rejected(rejection) = checked else {
        panic!("the empty certificate omits a required initial product state: {checked:?}");
    };
    assert_eq!(rejection.rule, fsl_replay::CertificateRule::InitialMissing);
    assert!(matches!(
        rejection.at,
        fsl_replay::CertificateLocus::ProductState(_)
    ));
}
