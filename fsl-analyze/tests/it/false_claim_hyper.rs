// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1051 seed QSL-923 (R-38 product closure, two copies): over the secure
//! vault from `(h, l) = (0, 0)`, the claim
//! `forall a forall b { (not holds(l@a = 1)) and eventually holds(l@a = 1) }`
//! is false, because `step(0)` forever keeps `l = 0`: `not P` is true and
//! `eventually P` is false. The reference refutes it on that lasso
//! (`qsl_923_reference_refutes_the_never_set_output_lasso`). Here the analysis
//! does not settle it `holds`, and a product-closure certificate that
//! partitions the real closed product into one component claimed to miss the
//! acceptance set is rejected by the check entry.

use filament_exact::Cancel;
use fsl_eval::hyper::{BodySense, HyperInputs, HyperProduct};
use fsl_eval::simulation::model::ModelSystem;
use fsl_eval::simulation::{Limits, TransitionSystem};
use fsl_foundation::IdentityLimits;
use fsl_replay::{
    check_product_closure, product_state_key, CertificateRule, Component, ComponentWitness,
    HyperOutcome, ModelCheckLimits, ProductClosure, ProductClosureCertificate, ProductClosureCheck,
    ProductClosureClause, ProductClosureRequest,
};

use ix_trace_rs::trace;

use crate::counter::UNLIMITED;
use crate::hyper_world::{instance, Vault, World};

const CLAIM: &str = "hyper Seed using inf over behaviours (v: V::Vault) \
     forall trace a of V forall trace b of V \
     { (not holds(v.l @ a = 1)) and eventually holds(v.l @ a = 1) }";

fn world() -> World {
    World::new(Vault::Secure, false, &[(0, 0)], CLAIM)
}

/// Seed QSL-923, analysis half: the engine finds the lasso and refutes.
#[trace("FR-1051")]
#[test]
fn seed_qsl_923_analysis_does_not_prove_the_never_set_output_claim() {
    let world = world();
    let outcome = world
        .run(ModelCheckLimits::default(), &Cancel::new())
        .expect("the clause is checked");
    assert!(
        matches!(outcome, HyperOutcome::Violated { .. }),
        "the claim is violated by the step(0) lasso: {outcome:?}"
    );
}

/// Seed QSL-923, checker half: the closed product of the clause, in one
/// component with the witness "no state lies in acceptance set 0", is not
/// accepted. The lasso's states lie in that set.
#[trace("FR-1051")]
#[test]
fn seed_qsl_923_a_certificate_that_the_lasso_misses_the_acceptance_set_is_rejected() {
    let world = world();
    let subject = world.subject();
    let system = ModelSystem::new(subject.simulation.clone()).expect("the subject admits");
    let seams = world.seams();
    let cancel = Cancel::new();
    let instance = instance();
    let mut product = HyperProduct::new(HyperInputs {
        systems: vec![&system; world.clause.quantifiers().len()],
        instance: &instance,
        body: world.clause.body(),
        profile: world.clause.profile(),
        sense: BodySense::Negated,
        letters: &seams,
        matcher: None,
        max_automaton_states: u64::MAX,
        meter: UNLIMITED,
        cancel: &cancel,
        copy_swap: None,
        key_generators: Vec::new(),
        constraint: None,
    })
    .expect("the negated body translates");
    product.start().expect("the initial states read");
    let explored = fsl_analyze::model_check::retain(
        &product,
        Limits {
            max_states: usize::MAX,
            max_transitions: usize::MAX,
        },
        IdentityLimits::default(),
        None,
        || false,
    )
    .expect("the keys encode");
    let mut members = explored
        .graph
        .states()
        .iter()
        .map(|member| product_state_key(&product.key(&member.state)).expect("a product key"))
        .collect::<Vec<_>>();
    members.sort();
    let forged = ProductClosureCertificate {
        form: fsl_semantics::check::HyperForm::Universal,
        instance: instance
            .iter()
            .map(|binding| (binding.parameter.clone(), binding.instance.key.clone()))
            .collect(),
        reduction: None,
        closure: ProductClosure::Product(members.clone()),
        components: Some(vec![Component {
            states: members,
            witness: ComponentWitness::MissingAcceptance { set: 0 },
        }]),
    };
    let subjects = [subject];
    let request = ProductClosureRequest {
        package: &world.package,
        clause: ProductClosureClause::Hyper(&world.clause),
        subjects: &subjects,
        meter: UNLIMITED,
    };
    let checked = check_product_closure(&request, &forged, &ModelCheckLimits::default(), &cancel)
        .expect("the check entry decides the certificate");
    let ProductClosureCheck::Rejected(rejection) = checked else {
        panic!("the forged certificate was not rejected: {checked:?}");
    };
    assert_eq!(
        rejection.rule,
        CertificateRule::WitnessFails,
        "{rejection:?}"
    );
}
