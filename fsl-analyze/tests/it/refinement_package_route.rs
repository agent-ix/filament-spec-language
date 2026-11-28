// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-149-AC-12 and FR-1051 "No producer objects" (QSL-3625): on the package
//! and source routes the simulation checker reads the refinement from the
//! checked package, never from a refinement the producer built, and settles a
//! claim only by a certificate of that claim's own refinement.

use filament_exact::Cancel;
use fsl_analyze::model_check::{check_refinement, RefinementCheckRequest};
use fsl_eval::model_subject::ModelSubject;
use fsl_eval::simulation::model::{ModelSimulationRequest, PackageModelInput};
use fsl_package::AnalysisView;
use fsl_replay::{
    check_simulation_certificate, ModelCheckLimits, PackageRefinement, RefinementRead,
    SafetyOutcome, SimulationCertificate, SimulationCertificateRequest, SimulationRefusal,
    TemporalCertificateCheck,
};
use fsl_semantics::library::LibraryName;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use ix_trace_rs::trace;

use crate::ring_world::{RingWorld, Variant, UNLIMITED};

/// The refinement block named `name`, mapping `put` and `take` as given.
fn refinement_text(name: &str, put: &str, take: &str) -> String {
    format!(
        "refinement {name} using inf abstract Q concrete R {{\n\
         population Q::queues from R::rings;\n\
         object Q::Queue from R::Ring {{}}\n\
         step R::Slot::put -> {put};\n\
         step R::Slot::take -> {take};\n\
         }}"
    )
}

fn world_of(text: &str) -> RingWorld {
    RingWorld::with(
        Variant {
            package_readback: true,
            ..Variant::default()
        },
        text,
    )
}

/// The ring unit whose one refinement, named `name`, stutters every concrete
/// step (the abstract state of the package-readback unit never changes).
fn world(name: &str) -> RingWorld {
    world_of(&refinement_text(name, "stutter", "stutter"))
}

/// The ring unit declaring two refinements, `RingIsQueue` and
/// `RingIsQueueTwin`, with the same body.
fn twin_world() -> RingWorld {
    world_of(&format!(
        "{}\n{}",
        refinement_text("RingIsQueue", "stutter", "stutter"),
        refinement_text("RingIsQueueTwin", "stutter", "stutter")
    ))
}

/// The ring unit whose `RingIsQueue` stutters `put` and maps `take` to the
/// abstract dequeue: the same name as the stuttering one, another body.
fn mutated_world() -> RingWorld {
    world_of(&refinement_text(
        "RingIsQueue",
        "stutter",
        "Q::Queue::deq(self.ring)",
    ))
}

/// Run `body` over `world`'s package route: its admitted package's model
/// input and its refinement `name` read from the admitted rows.
fn over_package<T>(
    world: &RingWorld,
    name: &str,
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
    let refinement = admitted
        .resolved_graph()
        .nodes
        .iter()
        .filter(|node| node.semantic_form.as_ref() == "refinement")
        .find_map(|node| {
            let view = AnalysisView::refinement(&bare, &node.node_id).ok()?;
            PackageRefinement::read(view, &bare, &input)
                .ok()
                .filter(|read| read.name() == name)
        })
        .expect("the admitted refinement reads");
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

/// The simulation certificate request over the package route's input, for the
/// claim of the refinement `claimed`.
fn request<'a>(
    world: &'a RingWorld,
    input: &'a PackageModelInput<'a>,
    claimed: &str,
) -> SimulationCertificateRequest<'a> {
    SimulationCertificateRequest {
        concrete: concrete(world, input),
        refinement: claimed.to_owned(),
        abstract_initial: vec![world.abstract_initial()].into(),
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

fn check(
    request: &SimulationCertificateRequest<'_>,
    certificate: &SimulationCertificate,
) -> Result<TemporalCertificateCheck, SimulationRefusal> {
    check_simulation_certificate(
        request,
        certificate,
        &ModelCheckLimits::default(),
        &Cancel::new(),
    )
}

/// Assert `outcome` refuses `certificate` as naming another refinement than
/// the claim's `declared`.
fn assert_other_refinement(
    outcome: Result<TemporalCertificateCheck, SimulationRefusal>,
    named: &str,
    declared: &str,
) {
    let Err(SimulationRefusal::Undeclared {
        certificate,
        declared: claimed,
    }) = outcome
    else {
        panic!("a certificate of another refinement was not refused: {outcome:?}");
    };
    assert_eq!((certificate.as_str(), claimed.as_str()), (named, declared));
}

/// FR-149-AC-12, FR-1051 "No producer objects" (QSL-3625): a certificate of
/// the package's `RingIsQueue`, checked for a claim whose refinement is
/// `RingIsQueueOther`, is refused as naming another refinement.
#[trace("TC-556", "FR-149-AC-12")]
#[test]
fn a_certificate_of_another_package_s_refinement_is_refused_on_the_package_route() {
    let producer_world = world("RingIsQueue");
    let checked_world = world("RingIsQueueOther");
    over_package(
        &producer_world,
        "RingIsQueue",
        |producer_world, producer_input, producer| {
            let certificate = certificate(producer_world, producer_input, producer);
            assert_eq!(certificate.refinement, "RingIsQueue");
            let own = check(
                &request(producer_world, producer_input, "RingIsQueue"),
                &certificate,
            );
            assert!(
                matches!(own, Ok(TemporalCertificateCheck::Accepted)),
                "the certificate is accepted against its own package: {own:?}"
            );
            over_package(
                &checked_world,
                "RingIsQueueOther",
                |checked_world, checked_input, _| {
                    assert_other_refinement(
                        check(
                            &request(checked_world, checked_input, "RingIsQueueOther"),
                            &certificate,
                        ),
                        "RingIsQueue",
                        "RingIsQueueOther",
                    );
                },
            );
        },
    );
}

/// FR-149-AC-12 (FND-001): two refinements declared in ONE package. The valid
/// certificate of `RingIsQueueTwin` settles the claim of `RingIsQueueTwin`,
/// and is refused for the claim of `RingIsQueue`.
#[trace("TC-556", "FR-149-AC-12")]
#[test]
fn a_certificate_of_the_package_s_other_refinement_does_not_settle_the_claim_on_the_package_route()
{
    let world = twin_world();
    over_package(&world, "RingIsQueueTwin", |world, input, twin| {
        let certificate = certificate(world, input, twin);
        assert_eq!(certificate.refinement, "RingIsQueueTwin");
        assert!(
            matches!(
                check(&request(world, input, "RingIsQueueTwin"), &certificate),
                Ok(TemporalCertificateCheck::Accepted)
            ),
            "the certificate settles its own refinement's claim"
        );
        assert_other_refinement(
            check(&request(world, input, "RingIsQueue"), &certificate),
            "RingIsQueueTwin",
            "RingIsQueue",
        );
    });
}

/// FR-149-AC-12 (FND-004): the package route reads the refinement's body from
/// the checked package. A certificate of the holding `RingIsQueue`, checked
/// against a package whose `RingIsQueue` has the same name and another body
/// is not accepted.
#[trace("TC-556", "FR-149-AC-12")]
#[test]
fn a_same_named_refinement_with_another_body_is_not_accepted_on_the_package_route() {
    let producer_world = world("RingIsQueue");
    let checked_world = mutated_world();
    over_package(
        &producer_world,
        "RingIsQueue",
        |producer_world, producer_input, producer| {
            let certificate = certificate(producer_world, producer_input, producer);
            over_package(
                &checked_world,
                "RingIsQueue",
                |checked_world, checked_input, _| {
                    let outcome = check(
                        &request(checked_world, checked_input, "RingIsQueue"),
                        &certificate,
                    );
                    assert!(
                        !matches!(outcome, Ok(TemporalCertificateCheck::Accepted)),
                        "a refinement body the package does not declare was accepted: {outcome:?}"
                    );
                },
            );
        },
    );
}

/// FR-149-AC-12 (FND-001): the source route of the two-refinement package.
/// The certificate of `RingIsQueueTwin` settles its own claim and is refused
/// for the claim of `RingIsQueue`.
#[trace("TC-556", "FR-149-AC-12")]
#[test]
fn a_certificate_of_the_package_s_other_refinement_does_not_settle_the_claim_on_the_source_route() {
    let world = twin_world();
    let twin = world
        .package()
        .graph()
        .refinements()
        .iter()
        .find(|row| row.name() == "RingIsQueueTwin")
        .expect("the package declares the twin");
    let checked = check_refinement(
        RefinementCheckRequest {
            refinement: twin,
            concrete: ModelSubject {
                simulation: world.concrete_request(UNLIMITED),
                proof_bounds: Vec::new(),
            },
            abstract_package: world.package(),
            abstract_initial: vec![world.abstract_initial()].into(),
            abstract_protocol_binding: None,
            max_depth: None,
            limits: ModelCheckLimits::default(),
            retained: None,
        },
        &Cancel::new(),
    )
    .expect("the twin refinement is checked");
    let SafetyOutcome::Holds { certificate, .. } = checked.safety else {
        panic!("the twin refinement's safety half holds");
    };
    let claim = |claimed: &str| SimulationCertificateRequest {
        concrete: world.concrete_request(UNLIMITED),
        refinement: claimed.to_owned(),
        abstract_initial: vec![world.abstract_initial()].into(),
    };
    assert!(matches!(
        check(&claim("RingIsQueueTwin"), &certificate),
        Ok(TemporalCertificateCheck::Accepted)
    ));
    assert_other_refinement(
        check(&claim("RingIsQueue"), &certificate),
        "RingIsQueueTwin",
        "RingIsQueue",
    );
}
