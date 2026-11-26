// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-170 "Timed subject" (TC-595): a state-graph replay reads a timed
//! subject the same way from source and from package bytes. Each evidence
//! here is written by hand over a two-state timed model.

use fsl_eval::simulation::model::DEFAULT_MAX_CLOCKS;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_replay::{
    GraphEvidence, GraphEvidenceEnvelope, GraphReplaySettlement, ModelPath, ModelTraceSubject,
    ReplayLimits, ReplayRefusal, ReplayRequestWire, ReplayResult, TimedReplay, TrapClosure,
    WitnessPath, WitnessSource,
};
use ix_trace_rs::trace;

use crate::timed_world;

// The package route reads a state-graph claim over a timed subject only when
// it has no `over` and a Boolean literal predicate.
const CLAIMS: &str = "possible Idle using v { true }\n\
     possible Never using v { false }\n";

/// [`timed_world::DENSE`] over a unit declared directly in its dimension, the
/// one form of time source the package route reads.
const DIRECT_DENSE: &str = "dimension Time;\nunit ms : Time = rational(1, 1);\n\
     time M dense unit ms;\n";

/// `send` takes `phase` from 0 to 2 once `x` passes `guard`.
fn guarded(guard: &str) -> timed_world::World {
    let body = format!(
        "{}\
         pre Send using v on M::Call::send {{ self.phase = 0 and {guard} }}\n\
         post Sent using v on M::Call::send {{ self.phase = 2 and self.x = 0 ms }}\n\
         {CLAIMS}",
        timed_world::DENSE
    );
    timed_world::world(
        &timed_world::document(&["x"], &[("send", &["phase", "x"])]),
        "",
        &body,
    )
}

/// The same steps over a clock no clause reads, the one timed subject the
/// package route reads.
fn unguarded() -> timed_world::World {
    let body = format!(
        "{}\
         pre Send using v on M::Call::send {{ self.phase = 0 }}\n\
         post Sent using v on M::Call::send {{ self.phase = 2 }}\n\
         {CLAIMS}",
        DIRECT_DENSE
    );
    timed_world::world(
        &timed_world::document(&["x"], &[("send", &["phase"])]),
        "",
        &body,
    )
}

enum Evidence {
    /// `Idle` holds in every state: the empty path witnesses it.
    IdleWitness,
    /// `Never` has an empty trap stem.
    NeverTrap,
}

/// The wire request and subject, read with `timing`.
fn request(
    world: &timed_world::World,
    evidence: &Evidence,
    timing: Option<TimedReplay>,
) -> (ReplayRequestWire, ModelTraceSubject) {
    let name = match evidence {
        Evidence::IdleWitness => "Idle",
        Evidence::NeverTrap => "Never",
    };
    let claim = world
        .package
        .graph()
        .state_graph_claim(name)
        .expect("the claim checks");
    let simulation = world.request();
    let payload = match evidence {
        Evidence::IdleWitness => GraphEvidence::Witness {
            paths: vec![(
                WitnessPath::Path(ModelPath {
                    initial: 0,
                    steps: Vec::new(),
                }),
                WitnessSource::Explored,
            )],
            over: None,
        },
        Evidence::NeverTrap => GraphEvidence::Trap {
            stem: ModelPath {
                initial: 0,
                steps: Vec::new(),
            },
            closure: TrapClosure::StateGraph,
            over: None,
        },
    };
    let occurrence: OccurrenceKey = claim.occurrence().expect("an occurrence").clone();
    let obligation = fsl_package::ObligationIdentity::new(occurrence.node().clone(), None, 0);
    let envelope = GraphEvidenceEnvelope::new(
        obligation.clone(),
        occurrence.clone(),
        occurrence.node().clone(),
        Vec::new(),
        simulation.meter,
        Vec::new(),
        fsl_foundation::BackendId::new("fsl-en-1"),
        payload,
    );
    let mut wire = crate::model_settle_support::request(
        &world.package,
        &[],
        &world.document,
        world.snapshots.iter(),
        &world.unit,
    );
    wire.replay_target = fsl_replay::RequestTargetWire::default();
    wire.obligation_identity = obligation;
    wire.backend = "fsl-en-1".to_owned();
    wire.accounting_limits = simulation.meter;
    wire.source = fsl_replay::ReplaySource::ModelGraph(Box::new(envelope));
    let subject = ModelTraceSubject {
        timed: timing,
        initial: simulation.initial.clone(),
        universes: simulation.universes.clone(),
        expansion: simulation.expansion,
        stutter_closure: false,
    };
    (wire, subject)
}

fn timing(digital: bool) -> Option<TimedReplay> {
    Some(TimedReplay {
        model: "M".to_owned(),
        exploration: Default::default(),
        max_clocks: DEFAULT_MAX_CLOCKS,
        digital,
    })
}

fn from_source(
    world: &timed_world::World,
    evidence: &Evidence,
    timing: Option<TimedReplay>,
) -> Result<ReplayResult, ReplayRefusal> {
    let (wire, subject) = request(world, evidence, timing);
    fsl_replay::replay_model_graph(wire, &subject, ReplayLimits::default())
}

/// The bytes of `identities.json` and `equality.json` for the world's package.
pub(crate) fn handoff(world: &timed_world::World) -> (Vec<u8>, Vec<u8>) {
    let emission = fsl_package::emit_checked(&world.package, fsl_package::EmitLimits::fixture())
        .expect("the package emits");
    let simulation = world.request();
    let admitted = fsl_package::admit_checked_package(
        emission.package().bytes(),
        simulation.packages,
        fsl_package::V2ReadLimits::fixture(),
    )
    .expect("the strict reader admits the bytes");
    let mut budget = fsl_package::V2ReadLimits::fixture().native_join_budget();
    let identities = emission
        .identity_rows(&admitted, &mut budget)
        .expect("identity rows")
        .to_bytes(&admitted)
        .expect("identities.json");
    let equality = emission
        .equality_witness(&admitted, &mut budget)
        .expect("equality rows")
        .to_bytes(&admitted)
        .expect("equality.json");
    (identities, equality)
}

fn from_package(
    world: &timed_world::World,
    evidence: &Evidence,
    timing: Option<TimedReplay>,
) -> Result<ReplayResult, ReplayRefusal> {
    let (mut wire, subject) = request(world, evidence, timing);
    let (identities, equality) = handoff(world);
    wire.byte_provision.clear();
    fsl_replay::replay_model_graph_from_package(
        wire,
        fsl_replay::HandoffSupply {
            dependencies: &[],
            identities: &identities,
            equality: Some(&equality),
        },
        &subject,
        ReplayLimits::default(),
    )
}

fn mismatch(result: Result<ReplayResult, ReplayRefusal>) -> &'static str {
    match result {
        Err(ReplayRefusal::ModelGraphMismatch { member }) => member,
        other => panic!("expected a model-graph mismatch, got {other:?}"),
    }
}

/// FR-170-AC-9: evidence over a timed subject replayed without its clocks,
/// or on zones without a certificate, is refused by both routes.
#[trace("TC-595", "FR-170-AC-9")]
#[test]
fn a_timed_graph_replay_needs_its_clocks_and_certificate_on_both_routes() {
    let world = unguarded();
    for (reading, member) in [(None, "timing"), (timing(false), "zone_certificate")] {
        let source = from_source(&world, &Evidence::IdleWitness, reading.clone());
        assert_eq!(mismatch(source), member, "source, {member}");
        let package = from_package(&world, &Evidence::IdleWitness, reading);
        assert_eq!(mismatch(package), member, "package, {member}");
    }
}

/// FR-170-AC-10: the integer-clock reading is replayed only over a closed
/// subject. The package route reads no digital-clock reading at all, and no
/// subject with a clock guard.
#[trace("TC-595", "FR-170-AC-10")]
#[test]
fn a_digital_replay_of_a_strict_subject_is_refused() {
    let closed = guarded("self.x >= 1 ms");
    let Ok(ReplayResult::ModelGraph(replayed)) =
        from_source(&closed, &Evidence::NeverTrap, timing(true))
    else {
        panic!("the closed subject replays on the integer grid");
    };
    assert_eq!(replayed.settlement, GraphReplaySettlement::Reproduced);

    let strict = guarded("self.x > 1 ms");
    let source = from_source(&strict, &Evidence::NeverTrap, timing(true));
    assert_eq!(mismatch(source), "digital_subject");
    let package = from_package(&unguarded(), &Evidence::NeverTrap, timing(true));
    assert_eq!(mismatch(package), "timing");
}
