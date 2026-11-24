// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-520-AC-4 / TC-1315 row 5: compensation attempt bounds at the engine.

use filament_semantic_value::checking::CheckMode;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use filament_exact::ScalarLimits;
use filament_semantic_value::checking::CheckingLimits;
use fsl_eval::model_subject::{ModelInstance, ModelSubject, ProtocolSubject, RoleBinding};
use fsl_eval::simulation::model::{
    ExpansionLimits, ModelSimulationRequest, ModelSystem, PopulationUniverse,
};
use fsl_eval::simulation::protocol::{
    ProtocolState, ProtocolSystem, ProtocolTransition, ResolvedMemory, Sc, StepKind,
};
use fsl_package::CheckedPackage;
use fsl_semantics::check::PackageDeclarations;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::{admit_unit, package_input};
use fsl_semantics::model::observation::{
    document_reference, DocumentProvision, DocumentRef, ObservationLimits,
};
use ix_trace_rs::trace;
use serde_json::json;

#[path = "../../../fsl-semantics/tests/support/tc1070.rs"]
#[allow(
    dead_code,
    clippy::duplicate_mod,
    reason = "shared TC-1070 fixture constructors are used by sibling integration tests"
)]
mod tc1070;

const METER: ScalarLimits = ScalarLimits {
    integer_bits: u64::MAX,
    decimal_digits: u64::MAX,
    scale_expansion: u64::MAX,
    text_input_bytes: u64::MAX,
    text_scalars: u64::MAX,
    normalized_scalars: u64::MAX,
    unit_edges: u64::MAX,
    value_occurrences: u64::MAX,
    work_units: u64::MAX,
    result_units: u64::MAX,
};

struct Fixture {
    package: CheckedPackage,
    packages: fsl_semantics::model::intake::PackageInput,
    snapshots: DocumentProvision,
    initial: DocumentRef,
}

fn fixture(bound: u64) -> Fixture {
    fixture_with_recovery_view(bound, false)
}

fn fixture_with_recovery_view(bound: u64, recovery_view: bool) -> Fixture {
    fixture_with_recovery_predicate(bound, recovery_view.then_some("recovered.bal = 0"))
}

fn fixture_with_recovery_predicate(bound: u64, recovery_predicate: Option<&str>) -> Fixture {
    fixture_for_template(bound, recovery_predicate, "Refund")
}

fn fixture_for_template(bound: u64, recovery_predicate: Option<&str>, template: &str) -> Fixture {
    let recovery_view = recovery_predicate.is_some();
    let mut domain = tc1070::document();
    let total = domain["types"]
        .as_array_mut()
        .expect("domain types")
        .iter_mut()
        .find(|ty| ty["displayName"] == "Total")
        .expect("bounded Node total");
    let max = total["constraints"]
        .as_array_mut()
        .expect("Total constraints")
        .iter_mut()
        .find(|constraint| constraint["keyword"] == "max")
        .expect("Total maximum");
    max["operands"]["value"] = "5".into();
    total["constraints"]
        .as_array_mut()
        .expect("Total constraints")
        .iter_mut()
        .find(|constraint| constraint["keyword"] == "min")
        .expect("Total minimum")["operands"]["value"] = "5".into();
    // A single attempt record and enabled poststate keep the three outcome
    // histories exhaustive without multiplying them by unrelated values.
    let tally = domain["types"]
        .as_array_mut()
        .expect("domain types")
        .iter_mut()
        .find(|ty| ty["displayName"] == "Tally")
        .expect("attempt tally");
    tally["constraints"]
        .as_array_mut()
        .expect("Tally constraints")
        .iter_mut()
        .find(|constraint| constraint["keyword"] == "max")
        .expect("Tally maximum")["operands"]["value"] = "0".into();
    let node = domain["types"]
        .as_array_mut()
        .expect("domain types")
        .iter_mut()
        .find(|ty| ty["displayName"] == "Node")
        .expect("Node operation owner");
    node["operations"][0]["returns"]["typeRef"] = "ix://filament/native/Boolean".into();
    node["operations"][0]["frame"]["modifies"] =
        serde_json::json!([tc1070::model::id("Node/total")]);
    if recovery_view {
        domain["types"]
            .as_array_mut()
            .expect("domain types")
            .push(tc1070::model::declaration(
                "Recovery",
                "record",
                json!({
                    "fields": [tc1070::model::field("Recovery", "bal", &tc1070::model::id("Count"))]
                }),
            ));
        let count = domain["types"]
            .as_array_mut()
            .expect("domain types")
            .iter_mut()
            .find(|ty| ty["displayName"] == "Count")
            .expect("Count bounded record member type");
        count["constraints"]
            .as_array_mut()
            .expect("Count constraints")
            .iter_mut()
            .find(|constraint| constraint["keyword"] == "max")
            .expect("Count maximum")["operands"]["value"] = "1".into();
    }
    let domain = serde_json::to_vec(&domain).expect("domain fixture serializes");
    let packages = package_input([domain.as_slice()]).expect("domain package admitted");
    let mut source = tc1070::SOURCE.to_owned();
    source = source
        .replace("  capture order: M::Total = view.total;\n", "")
        .replace("order >= 0", "true")
        .replace("trigger: M::Flag", "trigger: Boolean")
        .replace("partialTrigger: M::Flag", "partialTrigger: Boolean")
        .replace("trigger.ready", "trigger")
        .replace("partialTrigger.ready", "partialTrigger")
        .replace("as (forward: M::Receipt)", "as (forward: Boolean)")
        .replace("as (partialForward: M::Receipt)", "as (partialForward: Boolean)")
        .replace("as (applied: M::Receipt)", "as (applied: Boolean)")
        .replace("as (attempted: M::Flag)", "as (attempted: Boolean)")
        .replace("attempted.ready", "attempted")
        .replace(
            "as (attempted: Boolean) { attempted }",
            "as (attempted: Boolean) { true }",
        )
        .replace("as (committed: M::Flag)", "as (committed: Boolean)")
        .replace("as (closed: M::Receipt)", "as (closed: Boolean)")
        .replace("committed.ready", "committed")
        .replace("capture target: M::Total = forward.total;\n", "")
        .replace("capture partialTarget: M::Total = partialForward.total;\n", "")
        .replace("target >= 0", "true")
        .replace("partialTarget >= 0", "true")
        .replace("target >= 0 and not activated", "not activated")
        .replace("partialTarget >= 0 and not partialActivated", "not partialActivated")
        .replace("recover (recovered: M::Receipt)", "recover (recovered: Boolean)")
        .replace("recover (partialRecovered: M::Receipt)", "recover (partialRecovered: Boolean)")
        .replace("recovered.total = target and not activated", "true")
        .replace(
            "partialRecovered.total = partialTarget and not partialActivated",
            "true",
        )
        .replace(
            "on each (opened: M::Tick) when (opened.ready) {",
            "on origin {",
        )
        .replace(
            "not trigger.ready and target >= 0",
            "trigger.ready and target >= 0",
        )
        .replace(
            "not partialTrigger.ready and partialTarget >= 0",
            "partialTrigger.ready and partialTarget >= 0",
        )
        .replace("not trigger and true", "trigger and true")
        .replace("not partialTrigger and true", "partialTrigger and true")
        .replace(
            "capture activated: Boolean = trigger.ready;",
            "capture activated: Boolean = false;",
        )
        .replace(
            "capture partialActivated: Boolean = partialTrigger.ready;",
            "capture partialActivated: Boolean = false;",
        )
        .replace(
            "attempts 3 of M::Attempt",
            &format!("attempts {bound} of M::Attempt"),
        )
        .replace("    commit Main::Committed;\n", "    commit never;\n")
        .replace(
            "earlier.tally < 3 and later.tally = earlier.tally + 1\n      and true and not activated",
            "true",
        )
        .replace(
            "partialEarlier.tally < 3 and partialLater.tally = partialEarlier.tally + 1\n      and true and not partialActivated",
            "true",
        );
    if recovery_view {
        source = source
            .replace(
                "on origin {",
                "on each (opened: M::Tick) when (opened.ready) {",
            )
            .replace(
                "recover (recovered: Boolean) { true }",
                &format!(
                    "recover (recovered: M::Recovery) {{ {} }}",
                    recovery_predicate.expect("a recovery-view predicate is present")
                ),
            )
            .replace(
                "recover (partialRecovered: Boolean) {\n      true\n    };",
                "recover (partialRecovered: M::Recovery) {\n      partialRecovered.bal = 0\n    };",
            )
            .replace(
                "effect Applied of Main::Tried as (applied: Boolean) { true };",
                "effect Applied of Main::Tried as (applied: Boolean) { true };\n\
                 event Signal by Service as (signal: Boolean) { true };",
            );
    }
    let refund = source
        .find("  compensate Refund for")
        .expect("Refund template");
    let partial = source
        .find("  compensate Partial for")
        .expect("Partial template");
    let main = source[partial..]
        .find("  run sequence Main")
        .map(|offset| partial + offset)
        .expect("run tree follows compensation templates");
    match template {
        "Refund" => source.replace_range(partial..main, ""),
        "Partial" => source.replace_range(refund..partial, ""),
        _ => panic!("unknown fixture template {template}"),
    }
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("qsl-1757", "unit.native"),
        "unit.native",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("unit parses");
    assert!(
        parsed.is_admissible(),
        "{:?}\n{source}",
        parsed.diagnostics()
    );
    let unit = fsl_forms::build_unit(&parsed).expect("unit forms built");
    let models = admit_unit(
        &unit.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("domain model selected");
    let package = CheckedPackage::link(
        PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            "qsl-1757".to_owned(),
            unit,
            models,
            Vec::new(),
        )
        .expect("unit assembled")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_eval::value::S6aRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("protocol checks: {refusals:?}")),
    );

    let bytes = serde_json::to_vec(&json!({
        "format": "filament.state.snapshot/v1",
        "identity": {"authority": "test", "identity": format!("qsl-1757-{bound}")},
        "observation": "current",
        "model": {"identity": tc1070::model::PACKAGE},
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [{
            "population": tc1070::model::id("nodes"),
            "complete": true,
            "objects": [{
                "key": "node-1",
                "type": tc1070::model::id("Node"),
                "fields": {"total": {"integer": "5"}}
            }]
        }]
    }))
    .expect("snapshot serializes");
    let identity = document_reference(&bytes).expect("snapshot identity");
    let initial = DocumentRef {
        authority: "test".to_owned(),
        identity: format!("qsl-1757-{bound}"),
    };
    let snapshots = BTreeMap::from([(identity, bytes)]);
    Fixture {
        package,
        packages,
        snapshots,
        initial,
    }
}

fn system<'a>(fixture: &'a Fixture) -> ProtocolSystem<'a, Sc> {
    let population = filament_checked::key::DeclarationKey {
        package: tc1070::model::PACKAGE.to_owned(),
        node: tc1070::model::id("nodes"),
    };
    let request = ModelSimulationRequest {
        emit: fsl_package::EmitLimits::fixture(),
        input: fsl_eval::simulation::model::ModelCheckInput::source(&fixture.package),
        packages: &fixture.packages,
        normalization: ModelNormalizationLimits::default(),
        snapshots: &fixture.snapshots,
        initial: vec![fixture.initial.clone()],
        universes: vec![PopulationUniverse {
            population: population.clone(),
            keys: vec!["node-1".to_owned()],
        }],
        observation: ObservationLimits::default(),
        expansion: ExpansionLimits {
            max_candidates: 100_000,
        },
        meter: METER,
    };
    let model = ModelSystem::new(request.clone()).expect("model subject built");
    let protocol = fixture
        .package
        .graph()
        .protocol("Refunds")
        .expect("checked protocol");
    let subject = ProtocolSubject {
        model: ModelSubject {
            simulation: request,
            proof_bounds: Vec::new(),
        },
        protocol: "Refunds".to_owned(),
        over: ModelInstance {
            population,
            key: "node-1".to_owned(),
        },
        roles: vec![RoleBinding {
            role: "Service".to_owned(),
            model: tc1070::model::PACKAGE.to_owned(),
        }],
        capacities: Vec::new(),
        memory: ResolvedMemory::resolve(protocol, &[]).expect("sc memory selected"),
    };
    ProtocolSystem::new(model, &subject, Sc).expect("protocol subject built")
}

fn named_member<'a>(value: &'a serde_json::Value, name: &str) -> Option<&'a serde_json::Value> {
    match value {
        serde_json::Value::Object(object) => object
            .get(name)
            .or_else(|| {
                (object.get("name").and_then(serde_json::Value::as_str) == Some(name))
                    .then(|| object.get("value"))
                    .flatten()
            })
            .or_else(|| object.values().find_map(|value| named_member(value, name))),
        serde_json::Value::Array(values) => {
            values.iter().find_map(|value| named_member(value, name))
        }
        _ => None,
    }
}

fn scalar_text(value: &serde_json::Value) -> Option<&str> {
    match value {
        serde_json::Value::String(text) => Some(text),
        serde_json::Value::Object(object) => object
            .get("value")
            .and_then(scalar_text)
            .or_else(|| object.values().find_map(scalar_text)),
        serde_json::Value::Array(values) => values.iter().find_map(scalar_text),
        _ => None,
    }
}

fn registration_status<'a>(
    state: &'a serde_json::Value,
    registration: &serde_json::Value,
) -> Option<&'a str> {
    match state {
        serde_json::Value::Object(object) => {
            if object.get("key") == Some(registration) {
                return object
                    .get("value")
                    .and_then(|value| named_member(value, "status"))
                    .and_then(scalar_text);
            }
            object
                .values()
                .find_map(|value| registration_status(value, registration))
        }
        serde_json::Value::Array(values) => values
            .iter()
            .find_map(|value| registration_status(value, registration)),
        _ => None,
    }
}

/// A recovery path's wire steps and the cend successors reached after it.
type CendSuccessors = (
    Vec<serde_json::Value>,
    Vec<(ProtocolTransition, ProtocolState<Sc>)>,
);

/// Find the first recovery step in a bounded breadth-first walk. This visits
/// only the prefix needed by the assertion and stops after 256 unique states.
fn recovery_cend_successors(system: &ProtocolSystem<'_, Sc>) -> Option<CendSuccessors> {
    bounded_step_successors(system, StepKind::CEnd)
}

fn bounded_step_successors(
    system: &ProtocolSystem<'_, Sc>,
    kind: StepKind,
) -> Option<CendSuccessors> {
    use fsl_eval::simulation::{SubjectSystem, TransitionSystem};

    const MAX_STATES: usize = 256;
    let initial = system
        .try_initial()
        .expect("initial states settle")
        .into_iter()
        .next()?;
    let (key, _) = SubjectSystem::state_key(system, &initial).ok()?;
    let mut queue = VecDeque::from([(initial, Vec::new())]);
    let mut visited = BTreeSet::from([key]);
    while let Some((state, prefix)) = queue.pop_front() {
        let expansion = system.successors(&state).ok()?;
        let cends: Vec<_> = expansion
            .successors
            .iter()
            .filter(|(transition, _)| transition.kind() == kind)
            .cloned()
            .collect();
        if !cends.is_empty() {
            return Some((prefix, cends));
        }
        for (transition, next) in expansion.successors {
            let (key, _) = SubjectSystem::state_key(system, &next).ok()?;
            if visited.contains(&key) {
                continue;
            }
            if visited.len() == MAX_STATES {
                return None;
            }
            visited.insert(key);
            let identity = fsl_eval::simulation::protocol::transition_json(&transition)?;
            let mut path = prefix.clone();
            path.push(identity);
            queue.push_back((next, path));
        }
    }
    None
}

/// Explore the complete small fixture, failing rather than treating a prefix
/// as a universal result if its shape changes. Permanent outcome histories
/// yield 1 + 3 + ... + 3^5 prefixes; the cap allows lifecycle steps around
/// those histories while bounding retained states and canonical keys.
fn complete_fixture_graph(
    system: &ProtocolSystem<'_, Sc>,
) -> fsl_eval::simulation::protocol::Reachable<Sc> {
    use fsl_eval::simulation::protocol::{Reachable, ReachedState};
    use fsl_eval::simulation::{SubjectSystem, TransitionSystem};

    const MAX_STATES: usize = 16_384;
    const MAX_KEY_BYTES: usize = 32 * 1024 * 1024;
    let mut states: Vec<ReachedState<Sc>> = Vec::new();
    let mut known = BTreeMap::new();
    let mut key_bytes = 0;
    for state in system.try_initial().expect("initial states settle") {
        let key = system
            .canonical_state_key(&state)
            .expect("initial state key");
        key_bytes += key.as_bytes().len();
        assert!(key_bytes <= MAX_KEY_BYTES, "fixture key budget exceeded");
        known.insert(key.clone(), states.len());
        states.push(ReachedState {
            state,
            key,
            steps: Vec::new(),
        });
    }
    let mut at = 0;
    while at < states.len() {
        let expansion = system
            .successors(&states[at].state)
            .expect("fixture expands completely");
        for (transition, state) in expansion.successors {
            let key = system
                .canonical_state_key(&state)
                .expect("successor state key");
            let next = if let Some(&index) = known.get(&key) {
                index
            } else {
                assert!(states.len() < MAX_STATES, "fixture state budget exceeded");
                key_bytes += key.as_bytes().len();
                assert!(key_bytes <= MAX_KEY_BYTES, "fixture key budget exceeded");
                let index = states.len();
                known.insert(key.clone(), index);
                states.push(ReachedState {
                    state,
                    key,
                    steps: Vec::new(),
                });
                index
            };
            states[at].steps.push((transition, next));
        }
        at += 1;
    }
    Reachable { states, workers: 1 }
}

fn replay_transition_path(
    system: &ProtocolSystem<'_, Sc>,
    path: &[serde_json::Value],
) -> Option<serde_json::Value> {
    use fsl_eval::simulation::{SubjectSystem, TransitionSystem};

    let mut state = system
        .try_initial()
        .expect("initial states settle")
        .into_iter()
        .next()?;
    for expected in path {
        let expansion = system.successors(&state).ok()?;
        let (_, next) = expansion.successors.into_iter().find(|(transition, _)| {
            fsl_eval::simulation::protocol::transition_json(transition).as_ref() == Some(expected)
        })?;
        state = next;
    }
    let (bytes, _) = system.state_key(&state).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn replace_named_scalar(value: &mut serde_json::Value, name: &str, from: &str, to: &str) -> bool {
    match value {
        serde_json::Value::Object(object) => {
            if object.get("name").and_then(serde_json::Value::as_str) == Some(name) {
                if let Some(value) = object.get_mut("value") {
                    return replace_scalar(value, from, to);
                }
            }
            object
                .values_mut()
                .any(|value| replace_named_scalar(value, name, from, to))
        }
        serde_json::Value::Array(values) => values
            .iter_mut()
            .any(|value| replace_named_scalar(value, name, from, to)),
        _ => false,
    }
}

fn replace_scalar(value: &mut serde_json::Value, from: &str, to: &str) -> bool {
    match value {
        serde_json::Value::String(text) if text == from => {
            *text = to.to_owned();
            true
        }
        serde_json::Value::Object(object) => object
            .values_mut()
            .any(|value| replace_scalar(value, from, to)),
        serde_json::Value::Array(values) => values
            .iter_mut()
            .any(|value| replace_scalar(value, from, to)),
        _ => false,
    }
}

fn maximum_attempts(
    reachable: &fsl_eval::simulation::protocol::Reachable<Sc>,
    template: &str,
) -> usize {
    fn longest(
        reachable: &fsl_eval::simulation::protocol::Reachable<Sc>,
        index: usize,
        template: &str,
        memo: &mut [Option<usize>],
        active: &mut [bool],
    ) -> usize {
        if let Some(value) = memo[index] {
            return value;
        }
        assert!(
            !active[index],
            "the finite retry fixture has no state cycle"
        );
        active[index] = true;
        let maximum = reachable.states[index]
            .steps
            .iter()
            .map(|(transition, next)| {
                let increment = usize::from(
                    matches!(
                        transition.kind(),
                        StepKind::CAttempt | StepKind::CFail | StepKind::CTimeout
                    ) && transition.template() == Some(template),
                );
                increment + longest(reachable, *next, template, memo, active)
            })
            .max()
            .unwrap_or(0);
        active[index] = false;
        memo[index] = Some(maximum);
        maximum
    }

    let mut memo = vec![None; reachable.states.len()];
    let mut active = vec![false; reachable.states.len()];
    longest(reachable, 0, template, &mut memo, &mut active)
}

#[test]
#[trace("FR-520-AC-4", "TC-1315")]
fn compensation_attempt_bound_admits_n_and_refuses_attempt_n_plus_one() {
    for bound in [1_u64, 3, 5] {
        for name in ["Refund", "Partial"] {
            let fixture = fixture_for_template(bound, None, name);
            let keys = fixture
                .package
                .graph()
                .protocol_node_keys("Refunds")
                .expect("protocol node keys");
            let read = fsl_package::read_evaluation_package(
                &fixture.package,
                fsl_semantics::library::LibraryName::new("qsl-1757").expect("package name"),
                &fixture.packages,
                fsl_package::V2ReadLimits::fixture(),
                fsl_package::EmitLimits::fixture(),
            )
            .expect("strict reader admits the protocol");
            let compensation_key = keys
                .compensations
                .get(&0)
                .unwrap_or_else(|| panic!("compensation {name} has an emitted node"));
            let compensation = fsl_semantics::check::wire_id::structural_id(compensation_key);
            assert_eq!(
                fsl_package::protocol_view::attempts_maximum(read.admitted(), &compensation)
                    .expect("protocol view reads attempts maximum"),
                bound,
                "{name}: the read bound equals the authored count",
            );
            let system = system(&fixture);
            let reachable = complete_fixture_graph(&system);
            let template = format!("Refunds::{name}");
            assert_eq!(
                maximum_attempts(&reachable, &template),
                usize::try_from(bound).expect("test bound fits usize"),
                "{name}: exactly n dispatches are admitted across all outcomes; n+1 has no successor",
            );
            for kind in [StepKind::CAttempt, StepKind::CFail, StepKind::CTimeout] {
                assert!(
                    reachable.states.iter().flat_map(|state| &state.steps).any(
                        |(transition, _)| {
                            transition.kind() == kind
                                && transition.template() == Some(template.as_str())
                        }
                    ),
                    "{name}: the complete graph includes {kind:?}"
                );
            }
        }
    }
}

#[test]
#[trace("FSpec-FR-426-AC-1", "FSpec-FR-426-AC-9")]
fn finish_is_stepped() {
    let fixture = fixture(1);
    let system = system(&fixture);
    let (_, finishes) = bounded_step_successors(&system, StepKind::Finish)
        .expect("the bounded prefix reaches finish");
    use fsl_eval::simulation::TransitionSystem;
    assert!(
        finishes.iter().any(|(_, state)| system
            .successors(state)
            .expect("finished state expands")
            .successors
            .is_empty()),
        "the prefix includes an end state after finish"
    );
}

#[test]
#[trace("FSpec-FR-427-AC-6")]
fn the_compensation_template_is_stepped() {
    let fixture = fixture_with_recovery_view(1, true);
    let system = system(&fixture);
    let (_, cends) = recovery_cend_successors(&system).expect("the bounded prefix reaches cend");
    assert!(!cends.is_empty(), "an activation creates a stepped thread");
    let mut threads = BTreeMap::<_, BTreeSet<_>>::new();
    for (transition, _) in cends {
        let registration = transition.registration().expect("cend has registration");
        let thread = transition.thread().expect("cend has a compensation thread");
        threads
            .entry(registration.clone())
            .or_default()
            .insert(thread.clone());
    }
    assert!(
        threads.values().all(|threads| threads.len() == 1),
        "each activation has one compensation thread"
    );
}

#[test]
#[trace("FSpec-FR-730-AC-1", "FSpec-FR-730-AC-3", "FSpec-FR-427-AC-6")]
fn a_recovery_predicate_over_the_recovery_view_is_evaluated() {
    let fixture = fixture_with_recovery_view(1, true);
    let protocol_system = system(&fixture);
    let (mut path, cends) = recovery_cend_successors(&protocol_system)
        .expect("the fixed prefix reaches a recovery step");
    assert!(!cends.is_empty(), "the recovery predicate is evaluated");
    assert!(
        cends
            .iter()
            .all(|(transition, _)| transition.binder_record().is_some()),
        "each cend identity carries its recovery binder record"
    );
    let mut records = BTreeMap::<_, BTreeSet<String>>::new();
    for (transition, next) in &cends {
        let record = protocol_system
            .binder_record_json(transition)
            .expect("record encoding succeeds")
            .expect("recovery-view cend carries its record");
        let balance =
            scalar_text(named_member(&record, "bal").expect("the recovery record has bal"))
                .expect("bal is a bounded integer");
        use fsl_eval::simulation::SubjectSystem;
        let (bytes, _) =
            SubjectSystem::state_key(&protocol_system, next).expect("state key encodes");
        let state_key: serde_json::Value =
            serde_json::from_slice(&bytes).expect("state key is canonical JSON");
        let transition_json = fsl_eval::simulation::protocol::transition_json(transition)
            .expect("cend transition encodes");
        let registration = named_member(&transition_json, "registration")
            .expect("cend transition names its registration");
        let status = registration_status(&state_key, registration)
            .expect("registration status is in the successor state key");
        assert_eq!(
            status,
            if balance == "0" {
                "recovered"
            } else {
                "unrecovered"
            },
            "the recovery status follows bal={balance}"
        );
        records
            .entry(
                transition
                    .registration()
                    .expect("cend has registration")
                    .clone(),
            )
            .or_default()
            .insert(serde_json::to_string(&record).expect("record JSON serializes"));
    }
    assert!(!records.is_empty(), "a registered template is compensated");
    assert!(
        records.values().all(|records| records.len() == 2),
        "each cend has one successor per recovery record"
    );

    let (unrecovered, _) = cends
        .iter()
        .find(|(transition, _)| {
            protocol_system
                .binder_record_json(transition)
                .ok()
                .flatten()
                .and_then(|record| named_member(&record, "bal").cloned())
                .and_then(|balance| scalar_text(&balance).map(str::to_owned))
                .as_deref()
                == Some("1")
        })
        .expect("bal 1 gives an unrecovered successor");
    path.push(
        fsl_eval::simulation::protocol::transition_json(unrecovered)
            .expect("cend transition encodes"),
    );
    let cend = path.last().expect("the counterexample ends at cend");
    assert_eq!(cend["step"], "cend");
    assert_eq!(
        scalar_text(named_member(&cend["record"], "bal").expect("cend carries its record")),
        Some("1"),
        "the counterexample carries the unrecovered bal 1 record"
    );

    let replay_fixture = fixture_with_recovery_view(1, true);
    let replay_system = system(&replay_fixture);
    let replayed_state =
        replay_transition_path(&replay_system, &path).expect("the recorded cend path replays");
    let registration = named_member(cend, "registration")
        .cloned()
        .expect("cend names its registration");
    assert_eq!(
        registration_status(&replayed_state, &registration),
        Some("unrecovered"),
        "replay reproduces the unrecovered registration"
    );

    let mut changed = path;
    assert!(replace_named_scalar(
        &mut changed.last_mut().expect("cend is the final transition")["record"],
        "bal",
        "1",
        "0"
    ));
    let changed_state = replay_transition_path(&replay_system, &changed)
        .expect("the cend transition with bal 0 replays");
    assert_eq!(
        registration_status(&changed_state, &registration),
        Some("recovered"),
        "changing the carried record to bal 0 no longer reproduces the counterexample"
    );
}

#[test]
#[trace("FSpec-FR-730-AC-2", "FSpec-FR-427-AC-6")]
fn a_recovery_predicate_that_does_not_read_its_binder_binds_no_record() {
    let baseline_fixture = fixture(1);
    let baseline_system = system(&baseline_fixture);
    let (_, baseline_cends) = recovery_cend_successors(&baseline_system)
        .expect("the baseline prefix reaches a recovery step");
    let baseline_statuses: Vec<_> = baseline_cends
        .iter()
        .map(|(transition, state)| {
            let identity = fsl_eval::simulation::protocol::transition_json(transition)
                .expect("cend transition encodes");
            let registration =
                named_member(&identity, "registration").expect("cend names its registration");
            use fsl_eval::simulation::SubjectSystem;
            let (bytes, _) =
                SubjectSystem::state_key(&baseline_system, state).expect("state key encodes");
            let state: serde_json::Value =
                serde_json::from_slice(&bytes).expect("state key is canonical JSON");
            registration_status(&state, registration)
                .expect("baseline cend records its status")
                .to_owned()
        })
        .collect();

    let fixture = fixture_with_recovery_predicate(1, Some("true"));
    let system = system(&fixture);
    let (_, cends) =
        recovery_cend_successors(&system).expect("the no-read prefix reaches a recovery step");
    assert!(!cends.is_empty(), "the fixture reaches cend");
    assert!(
        cends
            .iter()
            .all(|(transition, _)| transition.binder_record().is_none()),
        "a no-read cend binds no recovery record"
    );
    let statuses: Vec<_> = cends
        .iter()
        .map(|(transition, state)| {
            let identity = fsl_eval::simulation::protocol::transition_json(transition)
                .expect("cend transition encodes");
            let registration =
                named_member(&identity, "registration").expect("cend names its registration");
            use fsl_eval::simulation::SubjectSystem;
            let (bytes, _) = SubjectSystem::state_key(&system, state).expect("state key encodes");
            let state: serde_json::Value =
                serde_json::from_slice(&bytes).expect("state key is canonical JSON");
            registration_status(&state, registration)
                .expect("no-read cend records its status")
                .to_owned()
        })
        .collect();
    assert_eq!(
        statuses, baseline_statuses,
        "an unread binder preserves the result"
    );
}
