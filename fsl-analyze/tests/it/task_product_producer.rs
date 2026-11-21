// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-418 (S5 of QSL-2240): the producer side. EN-6's symbolic pass
//! explores the task automaton with the engine's own DBM and region walk and
//! writes the certificate; the checker accepts the certificate read back from
//! its JSON; and the engine's quotient equals the checker's, so a mistake the
//! two shared would still show as a disagreement here.

use std::collections::BTreeSet;

use filament_exact::Cancel;
use fsl_analyze::zone_check::{
    explore_task_symbolic, no_release_task_product_certificate, read_back_unchanged,
    task_product_certificate, SymbolicGraph, SymbolicStop, ZoneCheckLimits,
};
use fsl_eval::simulation::model::{TaskAutomaton, TaskProductSystem, TimedSystem};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_replay::{
    CertificateCheckLimits, CertificateRule, DivergentTarget, QuotientIdentity, Zone,
    ZoneCertificate, ZoneCertificateVerdict,
};
use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::task_product_certificate::{
    claim_of, no_release, over_utilised, preempting, rejected, releasable, solo, tied, verdict,
    Certificate,
};
use crate::timed_world::{document, world, World, DENSE};
use crate::zone_search::TIMED;

fn no_release_emitted(
    world: &World,
    limits: ZoneCheckLimits,
    cancel: &Cancel,
) -> Result<Option<Certificate>, fsl_analyze::zone_check::ZoneCheckRefusal> {
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("a timed source");
    let product = TaskProductSystem::new(timed, "NFp").expect("a task product");
    no_release_task_product_certificate(
        &product,
        claim_of(world, "NFp").occurrence().expect("occurrence"),
        limits,
        cancel,
    )
}

fn unreachable_release() -> World {
    world(&document(&[], &[("fire_a", &["phase"])]), TIMED, &format!(
        "{DENSE}taskset N {{ task a periodic 4 ms deadline 4 ms bcet 1 ms wcet 2 ms priority 1 lo; }}\n\
         schedulable NFp on N under fixed_priority;\n\
         pre Blocked using v on M::Call::fire_a {{ self.phase = 1 }}\n\
         post Fire using v on M::Call::fire_a {{ self.phase = pre(self.phase) }}\n\
         release N.a on M::Call::fire_a;\n"
    ))
}

fn release_after_enable() -> World {
    world(&document(&[], &[("fire_a", &["phase"]), ("enable", &["phase"])]), TIMED, &format!(
        "{DENSE}taskset N {{ task a periodic 4 ms deadline 4 ms bcet 1 ms wcet 2 ms priority 1 lo; }}\n\
         schedulable NFp on N under fixed_priority;\n\
         pre Blocked using v on M::Call::fire_a {{ self.phase = 1 }}\n\
         post Fire using v on M::Call::fire_a {{ self.phase = pre(self.phase) }}\n\
         post Enable using v on M::Call::enable {{ self.phase = 1 }}\n\
         release N.a on M::Call::fire_a;\n"
    ))
}

fn clockful_unreachable_release() -> World {
    world(&document(&["x"], &[("fire_a", &["phase"]), ("tick", &["phase", "x"])]), TIMED, &format!(
        "{DENSE}taskset N {{ task a periodic 4 ms deadline 4 ms bcet 1 ms wcet 2 ms priority 1 lo; }}\n\
         schedulable NFp on N under fixed_priority;\n\
         pre Blocked using v on M::Call::fire_a {{ self.phase = 1 }}\n\
         post Fire using v on M::Call::fire_a {{ self.phase = pre(self.phase) }}\n\
         pre Tick using v on M::Call::tick {{ (self.x >= 1 ms and self.x <= 2 ms) or self.x >= 3 ms }}\n\
         post Ticked using v on M::Call::tick {{ self.phase = pre(self.phase) and self.x = 0 ms }}\n\
         release N.a on M::Call::fire_a;\n"
    ))
}

/// The original clock-place encoding and every source guard part survive
/// transport; the independent checker accepts only the genuine source proof.
#[trace("FSpec-TC-371", "FSpec-FR-418-AC-14")]
#[test]
fn clockful_no_release_transport_preserves_context_and_rejects_tampering() {
    use fsl_analyze::zone_check::read_back_unchanged_with_source;
    let world = clockful_unreachable_release();
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("timed source");
    let product = TaskProductSystem::new(timed, "NFp").expect("task product");
    let built = no_release_task_product_certificate(
        &product,
        claim_of(&world, "NFp").occurrence().expect("occurrence"),
        ZoneCheckLimits::default(),
        &Cancel::new(),
    )
    .expect("source closes")
    .expect("no reachable release");
    assert!(!built.lu_bounds.lower.is_empty());
    assert!(built.edges.iter().any(|edge| edge.part == Some(0)));
    assert!(
        built.edges.iter().any(|edge| edge.part == Some(1)),
        "both convex source guard parts are retained"
    );
    let (wire, read) =
        read_back_unchanged_with_source(&built, json!({}), &built.obligation, product.source())
            .expect("genuine source metadata reads back unchanged");
    assert_eq!(
        wire["bounds"][0]["clock"],
        product
            .source()
            .clock_place_identity(0)
            .expect("original clock identity")
    );
    assert_eq!(read.lu_bounds, built.lu_bounds);
    assert_eq!(
        verdict(&world, "NFp", &read, CertificateCheckLimits::default()),
        Ok(ZoneCertificateVerdict::Accepted)
    );
    let decode = |wire: &Value| {
        ZoneCertificate::from_wire_with_source(wire, built.obligation.clone(), product.source())
            .expect("mutant retains wire shape")
            .0
    };
    let mut lu = wire.clone();
    for row in lu["bounds"].as_array_mut().expect("bounds") {
        row["lower"] = json!({"bound":"-100","strict":false});
        row["upper"] = json!({"bound":"-100","strict":false});
    }
    assert_eq!(
        rejected(verdict(
            &world,
            "NFp",
            &decode(&lu),
            CertificateCheckLimits::default()
        ))
        .0,
        CertificateRule::Malformed
    );
    let mut wrong_clock = wire.clone();
    wrong_clock["bounds"][0]["clock"] = json!("not-the-source-clock");
    assert!(ZoneCertificate::from_wire_with_source(
        &wrong_clock,
        built.obligation.clone(),
        product.source()
    )
    .is_err());
    let mut fragment = wire.clone();
    fragment["edges"][0]["part"] = json!(999);
    assert!(matches!(
        verdict(
            &world,
            "NFp",
            &decode(&fragment),
            CertificateCheckLimits::default()
        ),
        Ok(ZoneCertificateVerdict::Rejected(_))
    ));
    let mut data = wire.clone();
    data["edges"][0]["transition"]["operation"] = json!("M::Call::not_tick");
    assert!(matches!(
        verdict(
            &world,
            "NFp",
            &decode(&data),
            CertificateCheckLimits::default()
        ),
        Ok(ZoneCertificateVerdict::Rejected(_))
    ));
    let mut horizon = wire.clone();
    horizon["edges"][0]["horizon"] =
        json!([{"place":0,"operator":">=","bound":{"numerator":"1","denominator":"1"}}]);
    let read = decode(&horizon);
    assert_eq!(
        read.edges[0]
            .horizon
            .as_ref()
            .expect("horizon retained")
            .len(),
        1
    );
    assert!(matches!(
        verdict(&world, "NFp", &read, CertificateCheckLimits::default()),
        Ok(ZoneCertificateVerdict::Rejected(_))
    ));
    let mut delay = wire.clone();
    delay["edges"]
        .as_array_mut()
        .expect("edges")
        .push(json!({"node":0,"target":0,"transition":null,"horizon":[]}));
    let read = decode(&delay);
    assert!(read.edges.last().expect("delay").transition.is_none());
    assert_eq!(read.edges.last().expect("delay").horizon, Some(Vec::new()));
    assert_eq!(
        read.to_wire_with_source(json!({}), product.source())
            .expect("delay and horizon write unchanged"),
        delay
    );
    let mut delay_part = delay.clone();
    delay_part["edges"]
        .as_array_mut()
        .expect("edges")
        .last_mut()
        .expect("delay")["part"] = json!(0);
    assert!(ZoneCertificate::from_wire_with_source(
        &delay_part,
        built.obligation.clone(),
        product.source()
    )
    .is_err());
    assert!(matches!(
        verdict(&world, "NFp", &read, CertificateCheckLimits::default()),
        Ok(ZoneCertificateVerdict::Rejected(_))
    ));
}

/// A declared but unreachable release uses the same genuine source closure
/// as a release-free source, including the engine's admission witness.
#[trace("FSpec-TC-371", "FSpec-FR-418-AC-14")]
#[test]
fn the_no_reachable_release_producer_certificate_is_accepted_from_its_json() {
    for world in [no_release(), unreachable_release()] {
        let built = no_release_emitted(&world, ZoneCheckLimits::default(), &Cancel::new())
            .expect("source closure runs")
            .expect("no release is reachable");
        assert_eq!(
            built
                .task_product
                .as_ref()
                .expect("member")
                .time_progress
                .method,
            fsl_replay::TaskClosureMethod::ClosedNoReleaseProduct
        );
        let (_, read) = read_back_unchanged(&built, json!({}), &built.obligation)
            .expect("wire reads unchanged");
        assert_eq!(
            verdict(&world, "NFp", &read, CertificateCheckLimits::default()),
            Ok(ZoneCertificateVerdict::Accepted)
        );
    }
    for world in [releasable(), release_after_enable()] {
        assert!(
            no_release_emitted(&world, ZoneCheckLimits::default(), &Cancel::new())
                .expect("source search runs")
                .is_none()
        );
    }
}

/// Release absence alone is insufficient: the source must admit a genuine
/// time-divergent behaviour for the emitted certificate's witness.
#[trace("FSpec-TC-371", "FSpec-FR-418-AC-14")]
#[test]
fn a_no_release_source_without_admitted_behaviour_builds_no_certificate() {
    let world = world(&document(&["x"], &[]), TIMED, &format!(
        "{DENSE}taskset N {{ task a periodic 4 ms deadline 4 ms bcet 1 ms wcet 2 ms priority 1 lo; }}\n\
         schedulable NFp on N under fixed_priority;\n\
         time invariant Cap when {{ self.phase = 0 }} {{ self.x <= 1 ms }}\n"
    ));
    assert!(
        no_release_emitted(&world, ZoneCheckLimits::default(), &Cancel::new())
            .expect("source search runs")
            .is_none()
    );
}

/// Mutating the emitted member cannot turn the source closure into a
/// different proof; the independent reader rejects the changed bytes.
#[trace("FSpec-TC-371", "FSpec-FR-418-AC-14")]
#[test]
fn a_tampered_no_reachable_release_producer_certificate_is_rejected() {
    let world = unreachable_release();
    let built = no_release_emitted(&world, ZoneCheckLimits::default(), &Cancel::new())
        .expect("search")
        .expect("certificate");
    let mut wire = built.to_wire(json!({})).expect("wire");
    wire["task_product"]["queue_bounds"][0]["work_upper"]["numerator"] = json!("3");
    let (read, _) = ZoneCertificate::from_wire(&wire, built.obligation).expect("shape reads");
    assert_eq!(
        rejected(verdict(
            &world,
            "NFp",
            &read,
            CertificateCheckLimits::default()
        ))
        .0,
        CertificateRule::Malformed
    );
}

/// Source limits and cancellation remain typed; neither produces a certificate.
#[trace("FSpec-TC-371")]
#[test]
fn the_no_release_producer_preserves_source_limits_and_cancellation() {
    let world = unreachable_release();
    assert!(matches!(
        no_release_emitted(
            &world,
            ZoneCheckLimits {
                max_symbolic_states: 0,
                ..ZoneCheckLimits::default()
            },
            &Cancel::new()
        ),
        Err(fsl_analyze::zone_check::ZoneCheckRefusal::Stopped(
            fsl_analyze::zone_check::ZoneStop::LimitReached(
                fsl_analyze::zone_check::ZoneCheckLimit::MaxSymbolicStates(0)
            )
        ))
    ));
    let cancel = Cancel::new();
    cancel.cancel(filament_exact::CancelCause::Requested);
    assert!(matches!(
        no_release_emitted(&world, ZoneCheckLimits::default(), &cancel),
        Err(fsl_analyze::zone_check::ZoneCheckRefusal::Stopped(
            fsl_analyze::zone_check::ZoneStop::Cancelled
        ))
    ));
}

type Cells = Vec<Option<(String, bool)>>;

fn zone_cells(zone: &Zone) -> Cells {
    let mut cells = Vec::new();
    for i in 0..zone.dim() {
        for j in 0..zone.dim() {
            cells.push(
                zone.bound(i, j)
                    .expect("a bound")
                    .map(|(value, strict)| (value.to_string(), strict)),
            );
        }
    }
    cells
}

fn dbm_cells(zone: &fsl_analyze::zone_check::Dbm) -> Cells {
    let mut cells = Vec::new();
    for i in 0..zone.dim() {
        for j in 0..zone.dim() {
            cells.push(match zone.bound(i, j).expect("a bound") {
                fsl_analyze::zone_check::Bound::Finite { value, strict } => {
                    Some((value.to_string(), *strict))
                }
                fsl_analyze::zone_check::Bound::Infinite => None,
            });
        }
    }
    cells
}

fn engine_graph(world: &World, claim: &str) -> (SymbolicGraph, Certificate) {
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, claim).expect("checked task product");
    let automaton = TaskAutomaton::new(&product);
    let graph = explore_task_symbolic(&automaton, 100_000_000, &Cancel::new())
        .expect("the engine closes the product");
    let occurrence = claim_of(world, claim).occurrence().expect("an occurrence");
    let certificate =
        task_product_certificate(&automaton, &graph, occurrence).expect("a certificate");
    (graph, certificate)
}

/// One vertex: key octets and its zone cells.
type Vertex = (Vec<u8>, Cells);

/// The sets compared: vertices as (key octets, zone), discrete edges and
/// delay edges between such vertices.
#[derive(Debug, Eq, PartialEq)]
struct Shape {
    vertices: BTreeSet<Vertex>,
    discrete: BTreeSet<(Vertex, String, Vertex)>,
    time: BTreeSet<(Vertex, Vertex)>,
    initials: BTreeSet<Vertex>,
    subtractions: BTreeSet<(String, String, String)>,
}

fn engine_shape(graph: &SymbolicGraph) -> Shape {
    let at = |index: usize| {
        let vertex = &graph.vertices[index];
        (vertex.key.as_bytes().to_vec(), dbm_cells(&vertex.zone))
    };
    let identity = |index: usize| match &graph.identities[index] {
        fsl_analyze::zone_check::SymbolicIdentity::Source(transition) => {
            fsl_replay::task_source_transition_wire(transition)
                .expect("a spelling")
                .to_string()
        }
        fsl_analyze::zone_check::SymbolicIdentity::Template(template) => {
            template.to_wire().expect("a spelling").to_string()
        }
    };
    Shape {
        vertices: (0..graph.vertices.len()).map(at).collect(),
        discrete: graph
            .discrete
            .iter()
            .map(|edge| (at(edge.from), identity(edge.identity), at(edge.to)))
            .collect(),
        time: graph.time.iter().map(|&(a, b)| (at(a), at(b))).collect(),
        initials: graph
            .initials
            .iter()
            .flat_map(|initial| initial.vertices.iter().map(|&v| at(v)))
            .collect(),
        subtractions: graph
            .subtractions
            .iter()
            .map(|row| {
                (
                    row.template.to_wire().expect("a spelling").to_string(),
                    row.clock.clone(),
                    row.constant.to_string(),
                )
            })
            .collect(),
    }
}

fn checker_shape(world: &World, claim: &str, scale: &filament_exact::Integer) -> Shape {
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, claim).expect("checked task product");
    let automaton = TaskAutomaton::new(&product);
    let quotient = fsl_replay::explore_task_product(&automaton, scale, 100_000_000, &Cancel::new())
        .expect("the checker's quotient");
    let at = |index: usize| {
        let vertex = &quotient.vertices[index];
        (vertex.key.as_bytes().to_vec(), zone_cells(&vertex.zone))
    };
    let identity = |index: usize| match &quotient.identities[index] {
        QuotientIdentity::Source(transition) => fsl_replay::task_source_transition_wire(transition)
            .expect("a spelling")
            .to_string(),
        QuotientIdentity::Template(template) => template.to_wire().expect("a spelling").to_string(),
    };
    Shape {
        vertices: (0..quotient.vertices.len()).map(at).collect(),
        discrete: quotient
            .discrete
            .iter()
            .map(|edge| (at(edge.from), identity(edge.identity), at(edge.to)))
            .collect(),
        time: quotient.time.iter().map(|&(a, b)| (at(a), at(b))).collect(),
        initials: quotient
            .initials
            .iter()
            .flat_map(|initial| initial.vertices.iter().map(|&v| at(v)))
            .collect(),
        subtractions: quotient
            .subtractions
            .iter()
            .map(|row| {
                (
                    row.transition.to_wire().expect("a spelling").to_string(),
                    row.clock.clone(),
                    row.constant.to_string(),
                )
            })
            .collect(),
    }
}

const SETS: &[(&str, &str)] = &[
    ("solo", "SFp"),
    ("solo", "SEdf"),
    ("tied", "EEdf"),
    ("preempting", "QFp"),
    ("preempting", "QEdf"),
];

fn set(name: &str) -> World {
    match name {
        "solo" => solo(),
        "tied" => tied(),
        "preempting" => preempting(),
        other => panic!("no set {other}"),
    }
}

/// The emitted certificate is read back unchanged. The wire omits the
/// divergent target, so changing a witness to name a different quiescent
/// target must be refused even when encoding and decoding both succeed.
#[trace("TC-371")]
#[test]
fn a_witness_the_wire_decodes_differently_is_not_read_back_unchanged() {
    for (name, claim) in SETS {
        let world = set(name);
        let (_, certificate) = engine_graph(&world, claim);
        let occurrence = certificate.obligation.clone();
        assert!(
            read_back_unchanged(&certificate, json!({}), &occurrence).is_some(),
            "{claim}"
        );
        let mut changed = certificate.clone();
        changed
            .admitted
            .as_mut()
            .expect("a divergence witness")
            .target = DivergentTarget::Quiescent(certificate.nodes.len());
        let wire = changed
            .to_wire(json!({}))
            .expect("the target is not encoded");
        let (read, _) =
            ZoneCertificate::from_wire(&wire, occurrence.clone()).expect("the wire still decodes");
        assert_ne!(read.admitted, changed.admitted, "{claim}");
        assert!(
            read_back_unchanged(&changed, json!({}), &occurrence).is_none(),
            "{claim}"
        );
    }
}

/// FR-418: the engine's closed quotient (its own DBM and region walk) equals
/// the quotient the checker recomputes, vertex for vertex, edge for edge, so
/// a bug in one that the certificates of one side alone would hide shows.
#[trace("TC-371")]
#[test]
fn the_engine_quotient_equals_the_checker_quotient() {
    for (name, claim) in SETS {
        let world = set(name);
        let (graph, certificate) = engine_graph(&world, claim);
        let scale = &certificate.scale;
        assert_eq!(
            engine_shape(&graph),
            checker_shape(&world, claim, scale),
            "{claim}"
        );
    }
}

/// FR-418-AC-11, AC-13: the certificate the engine writes is accepted by the
/// checker after a round trip through its JSON, so the proof rests on the
/// emitted bytes.
#[trace("TC-371", "FR-418-AC-11")]
#[test]
fn the_engine_certificate_is_accepted_by_the_checker_from_its_json() {
    for (name, claim) in SETS {
        let world = set(name);
        let (_, certificate) = engine_graph(&world, claim);
        assert_eq!(
            verdict(
                &world,
                claim,
                &certificate,
                CertificateCheckLimits::default()
            )
            .expect("the check runs"),
            ZoneCertificateVerdict::Accepted,
            "{claim}"
        );
        let wire = certificate.to_wire(json!({})).expect("a wire");
        let (read, _) =
            ZoneCertificate::<OccurrenceKey>::from_wire(&wire, certificate.obligation.clone())
                .expect("the wire reads");
        assert_eq!(read.to_wire(json!({})).expect("a wire"), wire, "{claim}");
        assert_eq!(
            verdict(&world, claim, &read, CertificateCheckLimits::default())
                .expect("the check runs"),
            ZoneCertificateVerdict::Accepted,
            "{claim}"
        );
    }
}

fn tampered(
    world: &World,
    claim: &str,
    change: impl FnOnce(&mut Value),
) -> Result<ZoneCertificateVerdict, fsl_replay::ZoneCertificateRefusal<OccurrenceKey>> {
    let (_, certificate) = engine_graph(world, claim);
    let mut wire = certificate.to_wire(json!({})).expect("a wire");
    change(&mut wire);
    let (read, _) = ZoneCertificate::<OccurrenceKey>::from_wire(&wire, certificate.obligation)
        .expect("the tampered wire still reads");
    verdict(world, claim, &read, CertificateCheckLimits::default())
}

/// The rule `QFp`'s certificate is rejected with after `change` edits its
/// wire.
fn rule_after(change: impl FnOnce(&mut Value)) -> CertificateRule {
    rejected(tampered(&preempting(), "QFp", change)).0
}

/// An emitted certificate edited on the wire is rejected with the checker's
/// own rule: a changed member (FR-418-AC-11), a dropped edge, a retargeted
/// edge, squeezed zones, and (FR-418-AC-19) an extra node and an extra edge.
#[trace("TC-371", "FR-418-AC-19")]
#[test]
fn a_tampered_emitted_certificate_is_rejected() {
    // `max_jobs` of the first task one too many.
    assert_eq!(
        rule_after(|wire| {
            let jobs = wire["task_product"]["queue_bounds"][0]["max_jobs"]
                .as_u64()
                .expect("a count");
            wire["task_product"]["queue_bounds"][0]["max_jobs"] = json!(jobs + 1);
        }),
        CertificateRule::Malformed,
    );
    // The last edge dropped.
    assert_eq!(
        rule_after(|wire| {
            wire["edges"].as_array_mut().expect("edges").pop();
        }),
        CertificateRule::SuccessorMissing,
    );
    // Every node squeezed to a point.
    assert_eq!(
        rule_after(|wire| {
            for node in wire["nodes"].as_array_mut().expect("nodes") {
                for row in node["zone"].as_array_mut().expect("a zone") {
                    for entry in row.as_array_mut().expect("a row") {
                        if entry != "infinity" {
                            entry["bound"] = json!("0");
                            entry["strict"] = json!(false);
                        }
                    }
                }
            }
        }),
        CertificateRule::SuccessorMissing,
    );
    // A node nothing reaches.
    assert_eq!(
        rule_after(|wire| {
            let nodes = wire["nodes"].as_array_mut().expect("nodes");
            let copy = nodes[0].clone();
            nodes.push(copy);
        }),
        CertificateRule::UnreachedNode,
    );
    // An edge the recomputation does not produce.
    assert_eq!(
        rule_after(|wire| {
            let edges = wire["edges"].as_array_mut().expect("edges");
            let copy = edges[0].clone();
            edges.push(copy);
        }),
        CertificateRule::ExtraEdge,
    );
}

/// FR-418: the engine does not state a proof for an over-utilised set: a
/// deadline miss is reachable in its quotient, so no certificate is built.
#[trace("TC-371")]
#[test]
fn no_certificate_is_built_for_a_reachable_miss() {
    let world = over_utilised();
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, "OFp").expect("checked task product");
    let automaton = TaskAutomaton::new(&product);
    // The pass stops at the first reachable miss, so the whole quotient is
    // never explored and the concrete search refutes promptly.
    assert!(matches!(
        explore_task_symbolic(&automaton, 100_000_000, &Cancel::new()),
        Err(SymbolicStop::Missed)
    ));
}

/// The engine's pass obeys its work limit and the cancel (the analyze-level
/// FR-418-AC-20 test is `a_small_max_check_work_settles_limit_reached`).
#[trace("TC-371")]
#[test]
fn the_symbolic_pass_stops_at_its_work_limit_and_on_cancel() {
    let world = preempting();
    let model = world.system();
    let timed = TimedSystem::new(&model, "M").expect("dense timed model");
    let product = TaskProductSystem::new(timed, "QFp").expect("checked task product");
    let automaton = TaskAutomaton::new(&product);
    assert!(matches!(
        explore_task_symbolic(&automaton, 10, &Cancel::new()),
        Err(SymbolicStop::WorkLimit(10))
    ));
    let cancel = Cancel::new();
    cancel.cancel(filament_exact::CancelCause::Requested);
    assert!(matches!(
        explore_task_symbolic(&automaton, 100_000_000, &cancel),
        Err(SymbolicStop::Cancelled(_))
    ));
}

/// A receiver reference whose `object_type` is `null` is not an effective-id
/// object (the spine's `read_effective` refuses a null root), so the wire is
/// not read.
#[trace("TC-371")]
#[test]
fn a_null_receiver_object_type_is_not_read() {
    let world = preempting();
    let (_, certificate) = engine_graph(&world, "QFp");
    let mut wire = certificate.to_wire(json!({})).expect("a wire");
    let edge = wire["edges"]
        .as_array_mut()
        .expect("edges")
        .iter_mut()
        .find(|edge| edge["transition"]["source_transition"]["receiver"].is_object())
        .expect("an edge with a source receiver");
    edge["transition"]["source_transition"]["receiver"]["object_type"] = Value::Null;
    let refusal = ZoneCertificate::<OccurrenceKey>::from_wire(&wire, certificate.obligation)
        .expect_err("a null object type is refused");
    assert!(
        format!("{refusal:?}").contains("object_type"),
        "the refusal names the member: {refusal:?}"
    );
}
