// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-149-AC-1 and FR-1051 "No producer objects" (QSL-3625): on the package
//! route the simulation checker reads the refinement from the checked package,
//! never from a refinement the producer built.

use filament_exact::Cancel;
use fsl_analyze::model_check::{check_refinement, RefinementCheckRequest};
use fsl_eval::model_subject::ModelSubject;
use fsl_eval::simulation::model::{ModelSimulationRequest, PackageModelInput};
use fsl_package::AnalysisView;
use fsl_replay::{
    check_simulation_certificate, ModelCheckLimits, PackageRefinement, SafetyOutcome,
    SimulationCertificate, SimulationCertificateRequest, TemporalCertificateCheck,
};
use fsl_semantics::library::LibraryName;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use ix_trace_rs::trace;

use crate::ring_world::{RingWorld, Variant, UNLIMITED};

/// The ring unit whose one refinement, named `name`, stutters every concrete
/// step (the abstract state of the package-readback unit never changes).
fn world(name: &str) -> RingWorld {
    RingWorld::with(
        Variant {
            package_readback: true,
            ..Variant::default()
        },
        &format!(
            "refinement {name} using inf abstract Q concrete R {{\n\
             population Q::queues from R::rings;\n\
             object Q::Queue from R::Ring {{}}\n\
             step R::Slot::put -> stutter;\n\
             step R::Slot::take -> stutter;\n\
             }}"
        ),
    )
}

/// Run `body` over `world`'s package route: its admitted package's model
/// input and its one refinement read from the admitted rows.
fn over_package<T>(
    world: &RingWorld,
    body: impl FnOnce(&RingWorld, &PackageModelInput<'_>, &PackageRefinement<'_>) -> T,
) -> T {
    let emitted = fsl_package::emit_checked(world.package(), fsl_package::EmitLimits::fixture())
        .expect("the unit emits");
    let read = fsl_package::read_evaluation_emission(
        world.package(),
        &emitted,
        LibraryName::new(emitted.package().name()).expect("the emitted library identity"),
        world.packages(),
        fsl_package::V2ReadLimits::fixture(),
        fsl_package::EmitLimits::fixture(),
        fsl_package::SourceWitnessSupplier::new(&emitted),
    )
    .expect("the emitted package reads back");
    let admitted = read.admitted();
    let mut budget = fsl_package::V2ReadLimits::fixture().native_join_budget();
    let identities = emitted
        .identity_rows(admitted, &mut budget)
        .expect("the package identity rows read");
    let equality = emitted
        .equality_witness(admitted, &mut budget)
        .expect("the package equality rows read");
    let bare = fsl_package::BarePackage::new(admitted)
        .with_supply(
            fsl_package::BareSupply {
                identities: &identities,
                equality: Some(&equality),
            },
            &mut budget,
        )
        .expect("the emitted package identities join");
    let input = PackageModelInput::of(&bare, world.packages(), ModelNormalizationLimits::default())
        .expect("the admitted package model input");
    let node = admitted
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| node.semantic_form.as_ref() == "refinement")
        .expect("the admitted refinement node")
        .node_id
        .clone();
    let view = AnalysisView::refinement(&bare, &node).expect("the refinement view");
    let refinement = PackageRefinement::read(view, &bare, &input).expect("the refinement reads");
    body(world, &input, &refinement)
}

/// The concrete subject's request over the package route's input.
fn concrete<'a>(
    world: &'a RingWorld,
    input: &'a PackageModelInput<'a>,
) -> ModelSimulationRequest<'a> {
    ModelSimulationRequest {
        input: input.input(),
        ..world.concrete_request(UNLIMITED)
    }
}

/// The certificate of the holding safety half of `refinement` over `input`.
fn certificate(
    world: &RingWorld,
    input: &PackageModelInput<'_>,
    refinement: &PackageRefinement<'_>,
) -> SimulationCertificate {
    let checked = check_refinement(
        RefinementCheckRequest {
            refinement,
            concrete: ModelSubject {
                simulation: concrete(world, input),
                proof_bounds: Vec::new(),
            },
            abstract_package: input,
            abstract_initial: vec![world.abstract_initial()].into(),
            abstract_protocol_binding: None,
            max_depth: None,
            limits: ModelCheckLimits::default(),
            retained: None,
        },
        &Cancel::new(),
    )
    .expect("the package refinement is checked");
    let SafetyOutcome::Holds { certificate, .. } = checked.safety else {
        panic!("the package refinement's safety half holds");
    };
    *certificate
}

/// FR-149-AC-1, FR-1051 "No producer objects" (QSL-3625): a certificate of the
/// package's `RingIsQueue`, checked against a package that declares only
/// `RingIsQueueOther`, with the producer's `RingIsQueue` handed in as the
/// request's refinement, is not accepted: the package does not declare the
/// refinement the certificate names.
#[trace("TC-556", "FR-149-AC-1")]
#[test]
fn a_package_that_does_not_declare_the_certificates_refinement_does_not_accept_it() {
    let producer_world = world("RingIsQueue");
    let checked_world = world("RingIsQueueOther");
    over_package(
        &producer_world,
        |producer_world, producer_input, producer| {
            let certificate = certificate(producer_world, producer_input, producer);
            let own = check_simulation_certificate(
                &SimulationCertificateRequest {
                    refinement: producer,
                    concrete: concrete(producer_world, producer_input),
                    abstract_package: producer_input,
                    abstract_initial: vec![producer_world.abstract_initial()].into(),
                },
                &certificate,
                &ModelCheckLimits::default(),
                &Cancel::new(),
            );
            assert!(
                matches!(own, Ok(TemporalCertificateCheck::Accepted)),
                "the certificate is accepted against its own package: {own:?}"
            );
            over_package(&checked_world, |checked_world, checked_input, _| {
                let outcome = check_simulation_certificate(
                    &SimulationCertificateRequest {
                        refinement: producer,
                        concrete: concrete(checked_world, checked_input),
                        abstract_package: checked_input,
                        abstract_initial: vec![checked_world.abstract_initial()].into(),
                    },
                    &certificate,
                    &ModelCheckLimits::default(),
                    &Cancel::new(),
                );
                assert!(
                    !matches!(outcome, Ok(TemporalCertificateCheck::Accepted)),
                    "a refinement the checked package does not declare was accepted: {outcome:?}"
                );
            });
        },
    );
}
