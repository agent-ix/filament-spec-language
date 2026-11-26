// SPDX-License-Identifier: AGPL-3.0-or-later
//! Empty liveness products and component partitions are valid only when the
//! claim already holds at the initial state.

use filament_exact::Cancel;
use fsl_replay::{
    CertificateLocus, CertificateMember, CertificateRule, ClosureCertificate, Component,
    ComponentCertificate, ComponentWitness, ModelCheckLimits, ModelCheckOutcome, ProductStateRef,
    ProofCertificate, TemporalCertificateCheck, check_components,
};
use ix_trace_rs::trace;

use crate::liveness_support::Fixture;

/// `eventually holds(c.value <= 1)` from `c = 1`: the claim holds at the
/// initial state, so the negated formula's automaton has no successor there.
fn one_object() -> Fixture {
    crate::liveness_support::fixture(
        crate::counter::document(false),
        crate::counter::CONTRACT,
        "",
        "eventually holds(c.value <= 1)",
        &[("c", 1)],
    )
}

fn checked(
    fixture: &Fixture,
    atom: fn(i64) -> Option<bool>,
    certificate: &ComponentCertificate,
) -> TemporalCertificateCheck {
    let letters = fixture.letters(atom);
    check_components(
        &fixture.certificate_request(&letters, crate::counter::UNLIMITED),
        certificate,
        &ModelCheckLimits::default(),
        &Cancel::new(),
    )
    .expect("the component certificate is checked")
}

fn holds_initially(value: i64) -> Option<bool> {
    Some(value <= 1)
}

fn false_initially(value: i64) -> Option<bool> {
    Some(value <= 0)
}

fn certificate(states: Vec<ProductStateRef>, components: Vec<Component>) -> ComponentCertificate {
    ComponentCertificate {
        closure: ClosureCertificate { states },
        components,
    }
}

fn trivial(states: Vec<ProductStateRef>) -> Component {
    Component {
        states,
        witness: ComponentWitness::Trivial,
    }
}

/// The initial product state the checker requires when the atom is false.
fn required_initial_state(fixture: &Fixture) -> ProductStateRef {
    let result = checked(
        fixture,
        false_initially,
        &certificate(Vec::new(), Vec::new()),
    );
    let TemporalCertificateCheck::Rejected(rejection) = result else {
        panic!("the empty certificate omits a required initial product state: {result:?}");
    };
    assert_eq!(rejection.rule, CertificateRule::InitialMissing);
    let CertificateLocus::ProductState(state) = rejection.at else {
        panic!("the locus is a product state: {:?}", rejection.at);
    };
    state
}

/// `eventually p` holds when its sole initial state satisfies `p`: the
/// negated liveness product and its component partition are empty.
#[trace("TC-521")]
#[test]
fn empty_components_prove_an_eventuality_true_at_the_initial_state() {
    let one_object = one_object();
    let outcome = one_object
        .decide(holds_initially)
        .expect("the liveness run completes");
    let ModelCheckOutcome::Holds {
        certificate: ProofCertificate::Component(produced),
        ..
    } = outcome
    else {
        panic!("the initial state satisfies the eventuality: {outcome:?}");
    };
    assert_eq!(produced, certificate(Vec::new(), Vec::new()));
    assert_eq!(
        checked(&one_object, holds_initially, &produced),
        TemporalCertificateCheck::Accepted
    );
}

/// An empty certificate cannot prove an eventuality false at the initial
/// state: the checker requires its reached product state.
#[trace("TC-526")]
#[test]
fn empty_components_reject_an_eventuality_false_at_the_initial_state() {
    required_initial_state(&one_object());
}

/// A closure may list an admitted state the run never reaches; no component
/// still partitions the empty reached set.
#[trace("TC-526")]
#[test]
fn an_unreached_closure_state_without_components_is_accepted() {
    let one_object = one_object();
    let unreached = required_initial_state(&one_object);
    assert_eq!(
        checked(
            &one_object,
            holds_initially,
            &certificate(vec![unreached], Vec::new())
        ),
        TemporalCertificateCheck::Accepted
    );
}

/// No state is reached, so any component fails to partition the reached
/// states: at its listed state, or at the `components` member when it lists
/// none.
#[trace("TC-526")]
#[test]
fn a_component_over_no_reached_state_is_not_a_partition() {
    let one_object = one_object();
    let unreached = required_initial_state(&one_object);
    let rejected = |at| {
        TemporalCertificateCheck::Rejected(fsl_replay::CertificateRejection {
            rule: CertificateRule::NotPartition,
            at,
        })
    };
    for closure in [Vec::new(), vec![unreached.clone()]] {
        assert_eq!(
            checked(
                &one_object,
                holds_initially,
                &certificate(closure.clone(), vec![trivial(vec![unreached.clone()])])
            ),
            rejected(CertificateLocus::ProductState(unreached.clone()))
        );
        assert_eq!(
            checked(
                &one_object,
                holds_initially,
                &certificate(closure, vec![trivial(Vec::new())])
            ),
            rejected(CertificateLocus::CertificateMember(
                CertificateMember::Components
            ))
        );
    }
}
