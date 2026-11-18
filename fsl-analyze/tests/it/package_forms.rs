// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940 "Package forms of the source-checked evaluators" (TC-2958): each
//! package form a temporal engine calls returns the value its source form
//! returns, for a Counter unit compiled into its FR-517 handoff and the
//! `BarePackage` of the handoff's files.

use std::collections::BTreeMap;

use filament_exact::{LimitKind, Meter, ScalarLimits, Value};
use fsl_eval::value::{CheckedPackageEvaluation, HoldsAtom, HoldsBinders, ObservationIdentity};
use fsl_package::{BarePackage, BareSupply, ReadEqualityRows, ReadIdentityRows};
use fsl_semantics::check::{FormulaNode, FormulaNodeId, TemporalClauseNode};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::object_environment::ObjectEnvironment;
use fsl_semantics::model::observation::{admit_trace_snapshot, DocumentRef, ObservationLimits};
use ix_trace_rs::trace;

use crate::analyze_cli_support::{
    counter_request_with, counter_snapshot, package_selection, request_members, Workspace, HANDOFF,
};
use crate::counter::{document, package_with_mode, CONTRACT};
use filament_semantic_value::checking::CheckMode;

const UNLIMITED: ScalarLimits = ScalarLimits {
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

/// The clauses whose `holds` conditions and guards the forms read: an
/// `on origin` clause, a guarded `on each` clause, a clause whose condition
/// reads a capture and a clause with two `holds` atoms. The handoff is
/// compiled under `CheckMode::Linked`, which refuses an unproved divisor, so
/// the undefined case is [`package_holds_equals_source_holds_where_undefined`].
const CLAUSES: &str = "temporal Bounded using t0 over (c: M::Counter) clock \"model-steps\" \
     on origin { always holds(c.value <= 3 and not (c.value = 1)) }\n\
     temporal Guarded using t0 over (c: M::Counter) clock \"model-steps\" \
     on each (q: M::Counter) when (c.value >= 2) { always holds(c.value <= 2) }\n\
     temporal Captured using t0 over (c: M::Counter) clock \"model-steps\" \
     on each (q: M::Counter) when (true) { capture n: Int[0, 3] = c.value; \
     always holds(c.value >= n) }\n\
     temporal Paired using t0 over (c: M::Counter) clock \"model-steps\" \
     on origin { always (holds(c.value <= 1) or eventually holds(c.value = 3)) }\n";

/// The handoff files of the compiled Counter unit, read as FR-850
/// "Admission" reads them.
struct Handoff {
    compiled: fsl_command::checked_handoff::CompiledHandoff,
    admitted: filament_contract_model::CheckedPackageV2,
    identities: ReadIdentityRows,
    equality: Option<ReadEqualityRows>,
}

fn handoff(
    name: &str,
) -> (
    Workspace,
    Handoff,
    filament_contract_model::NativeJoinBudget,
) {
    let ws = Workspace::new(name);
    let _ = counter_request_with(&ws, CLAUSES, request_members(None));
    let (_, bytes) = package_selection(&ws, &request_members(None));
    let directory = ws.path(HANDOFF);
    let compiled = fsl_command::checked_handoff::compile(&directory)
        .expect("the handoff compiles back to its source package");
    let model = document(false);
    let supplied = fsl_semantics::model::intake::package_input([model.as_slice()])
        .expect("the Counter domain package admits");
    let limits = fsl_package::V2ReadLimits::bounded(1 << 24);
    let admitted = fsl_package::admit_checked_package(&bytes, &supplied, limits)
        .expect("the strict reader admits the handoff package");
    let mut budget = limits.native_join_limits().budget();
    let identity_bytes =
        std::fs::read(directory.join("identities.json")).expect("the handoff holds identities");
    let identities = ReadIdentityRows::read(&identity_bytes, &admitted, &mut budget)
        .expect("the identity rows admit");
    let equality = std::fs::read(directory.join("equality.json"))
        .ok()
        .map(|bytes| ReadEqualityRows::read(&bytes, &admitted).expect("the equality rows admit"));
    (
        ws,
        Handoff {
            compiled,
            admitted,
            identities,
            equality,
        },
        budget,
    )
}

/// The current snapshot holding counter `a` at `value`, admitted over the
/// source package's selections and types.
fn objects(handoff: &Handoff, value: i64) -> ObjectEnvironment {
    let graph = handoff.compiled.checked().graph();
    let selected = DocumentRef {
        authority: "test".to_owned(),
        identity: "start".to_owned(),
    };
    let provision: BTreeMap<DocumentRef, Vec<u8>> =
        BTreeMap::from([(selected.clone(), counter_snapshot(&[("a", value)]))]);
    admit_trace_snapshot(
        graph.model_selections(),
        graph.scope().types(),
        handoff.compiled.packages(),
        ModelNormalizationLimits::default(),
        &provision,
        &selected,
        ObservationLimits::default(),
        &[],
    )
    .expect("the Counter snapshot admits")
    .0
}

fn counter(objects: &ObjectEnvironment) -> filament_exact::ObjectReference {
    objects
        .references()
        .iter()
        .find(|reference| reference.object().as_str() == "a")
        .expect("counter a")
        .clone()
}

fn clause<'g>(handoff: &'g Handoff, index: usize) -> &'g TemporalClauseNode {
    handoff
        .compiled
        .checked()
        .graph()
        .temporal_clauses()
        .get(index)
        .expect("the declared clause")
}

fn holds_atoms(clause: &TemporalClauseNode) -> Vec<HoldsAtom> {
    let formula = clause.clause().formula();
    formula
        .nodes()
        .iter()
        .enumerate()
        .filter(|(_, node)| matches!(node, FormulaNode::Holds { .. }))
        .map(|(index, _)| {
            HoldsAtom::of_source(clause, FormulaNodeId::from_arena_index(index))
                .expect("a holds atom is a formula use")
        })
        .collect()
}

/// An outcome as text: `FamilyOutcome` has no `PartialEq`, its `Debug` form
/// spells every member.
fn outcome(evaluation: &filament_eval::Evaluation) -> String {
    format!("{:?}", evaluation.outcome)
}

fn boolean(evaluation: &filament_eval::Evaluation) -> bool {
    match &evaluation.outcome {
        filament_eval::FamilyOutcome::Evaluated(filament_exact::Outcome::Completed(
            Value::Boolean(value),
        )) => *value,
        other => panic!("expected a completed Boolean, got {other:?}"),
    }
}

/// The truth of a completed Boolean, `None` for an undefined reading.
fn reading(evaluation: &filament_eval::Evaluation) -> Option<bool> {
    match &evaluation.outcome {
        filament_eval::FamilyOutcome::Evaluated(filament_exact::Outcome::Undefined(_)) => None,
        _ => Some(boolean(evaluation)),
    }
}

/// FR-940-AC-6 (the forms' part): over the handoff of the Counter unit, the
/// package form of `evaluate_holds` returns the source form's value, work and
/// location for every `holds` atom of an `on origin`, a guarded `on each`, a
/// capture-reading and a two-atom clause, at counter values that make each
/// condition true and false.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn package_holds_equals_source_holds() {
    let (_ws, handoff, mut budget) = handoff("package-forms-holds");
    let bare = BarePackage::new(&handoff.admitted)
        .with_supply(
            BareSupply {
                identities: &handoff.identities,
                equality: handoff.equality.as_ref(),
            },
            &mut budget,
        )
        .expect("the handoff's supply joins the package");
    let source = handoff.compiled.checked();
    let mut seen = Vec::new();
    for (index, captures) in [
        (0, vec![]),
        (1, vec![]),
        (2, vec![Value::Integer(2_i64.into())]),
        (3, vec![]),
    ] {
        let clause = clause(&handoff, index);
        let atoms = holds_atoms(clause);
        let expected_atoms = if index == 3 { 2 } else { 1 };
        assert_eq!(atoms.len(), expected_atoms, "clause {index}'s holds atoms");
        for value in [1, 2, 3] {
            let objects = objects(&handoff, value);
            let over = Value::Reference(counter(&objects));
            let mut readings = Vec::new();
            for atom in &atoms {
                let mut source_meter = Meter::new(UNLIMITED);
                let from_source = source
                    .evaluate_holds(atom, over.clone(), &captures, &objects, &mut source_meter)
                    .expect("the source form evaluates");
                let mut package_meter = Meter::new(UNLIMITED);
                let from_package = bare
                    .evaluate_holds(atom, over.clone(), &captures, &objects, &mut package_meter)
                    .expect("the package form evaluates");
                let at = format!("clause {index} atom {} at value {value}", atom.position);
                assert_eq!(outcome(&from_package), outcome(&from_source), "{at}");
                assert_eq!(from_package.location, from_source.location, "{at}");
                assert_eq!(
                    package_meter.consumed(LimitKind::WorkUnits),
                    source_meter.consumed(LimitKind::WorkUnits),
                    "{at}"
                );
                let read = reading(&from_package);
                if read.is_none() {
                    // An undefined reading is located at the dividing node,
                    // on both routes alike.
                    assert!(from_package.location.is_some(), "{at}");
                }
                readings.push(read);
            }
            if index == 3 && value == 1 {
                // The two atoms of one clause read different values.
                assert_eq!(readings, [Some(true), Some(false)]);
            }
            seen.extend(readings);
        }
    }
    for expected in [Some(true), Some(false)] {
        assert!(seen.contains(&expected), "{seen:?}");
    }
}

/// FR-940-AC-6 (the forms' part): the package form of `evaluate_guard`
/// returns the source form's value for the guarded `on each` clause on both
/// sides of its guard, and `evaluate_holds_traced` returns the source form's
/// evaluation, trail, receiver path and work for every atom of the guarded
/// and the two-atom clause.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn package_guard_and_traced_holds_equal_their_source_forms() {
    let (_ws, handoff, mut budget) = handoff("package-forms-guard");
    let bare = BarePackage::new(&handoff.admitted)
        .with_supply(
            BareSupply {
                identities: &handoff.identities,
                equality: handoff.equality.as_ref(),
            },
            &mut budget,
        )
        .expect("the handoff's supply joins the package");
    let source = handoff.compiled.checked();
    let guarded = clause(&handoff, 1);
    let mut guards = Vec::new();
    for value in [1, 2] {
        let objects = objects(&handoff, value);
        let over = Value::Reference(counter(&objects));
        let mut source_meter = Meter::new(UNLIMITED);
        let from_source = source
            .evaluate_guard(
                &guarded.identity(),
                over.clone(),
                &objects,
                &mut source_meter,
            )
            .expect("the source guard evaluates");
        let mut package_meter = Meter::new(UNLIMITED);
        let from_package = bare
            .evaluate_guard(&guarded.identity(), over, &objects, &mut package_meter)
            .expect("the package guard evaluates");
        assert_eq!(
            outcome(&from_package),
            outcome(&from_source),
            "value {value}"
        );
        assert_eq!(from_package.location, from_source.location, "value {value}");
        assert_eq!(
            package_meter.consumed(LimitKind::WorkUnits),
            source_meter.consumed(LimitKind::WorkUnits)
        );
        guards.push(boolean(&from_package));
    }
    assert_eq!(guards, [false, true]);

    let observation = ObservationIdentity {
        authority: "test".to_owned(),
        identity: "start".to_owned(),
    };
    // The traced form over the guarded, and the two-atom clause:
    // the trail (decisions, reads and stops by location), the receiver path
    // and the work are the source form's.
    let mut traced = Vec::new();
    for index in [1, 3] {
        let clause = clause(&handoff, index);
        for value in [1, 2, 3] {
            let objects = objects(&handoff, value);
            let over = counter(&objects);
            for atom in holds_atoms(clause) {
                let binders = HoldsBinders {
                    over: &over,
                    captures: &[],
                };
                let mut source_meter = Meter::new(UNLIMITED);
                let from_source = source
                    .evaluate_holds_traced(
                        &atom,
                        binders,
                        &objects,
                        observation.clone(),
                        &mut source_meter,
                    )
                    .expect("the source traced form evaluates");
                let mut package_meter = Meter::new(UNLIMITED);
                let from_package = bare
                    .evaluate_holds_traced(
                        &atom,
                        binders,
                        &objects,
                        observation.clone(),
                        &mut package_meter,
                    )
                    .expect("the package traced form evaluates");
                let at = format!("clause {index} atom {} at value {value}", atom.position);
                assert_eq!(
                    format!("{from_package:?}"),
                    format!("{from_source:?}"),
                    "{at}"
                );
                assert_eq!(
                    package_meter.consumed(LimitKind::WorkUnits),
                    source_meter.consumed(LimitKind::WorkUnits),
                    "{at}"
                );
                assert!(from_package.receiver_path().is_some(), "{at}");
                traced.push(reading(&from_package.evaluation));
            }
        }
    }
    for expected in [Some(true), Some(false)] {
        assert!(traced.contains(&expected), "{traced:?}");
    }
}

/// A Counter snapshot `label` at `observation` holding `objects`.
fn snapshot(label: &str, observation: &str, objects: &[(&str, i64)]) -> Vec<u8> {
    let mut document: serde_json::Value =
        serde_json::from_slice(&counter_snapshot(objects)).expect("the snapshot is JSON");
    document["identity"]["identity"] = serde_json::json!(label);
    document["observation"] = serde_json::json!(observation);
    if observation != "current" {
        document
            .as_object_mut()
            .expect("a snapshot object")
            .remove("anchor");
    }
    serde_json::to_vec(&document).expect("the snapshot serializes")
}

fn document_ref(label: &str) -> DocumentRef {
    DocumentRef {
        authority: "test".to_owned(),
        identity: label.to_owned(),
    }
}

/// FR-940-AC-6 (the forms' part): the package form of `evaluate_frame` over
/// the handoff's frame node of `M::Counter::inc` returns the source form's
/// verdict, witness and location for an invocation inside the frame and one
/// that creates an object the frame does not grant.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn package_frame_equals_source_frame() {
    use filament_exact::Identifier;
    use fsl_semantics::model::observation::{
        admit_frame_invocation, FrameFacts, OperationFacts, Provisions,
    };

    let (_ws, handoff, mut budget) = handoff("package-forms-frame");
    let bare = BarePackage::new(&handoff.admitted)
        .with_supply(
            BareSupply {
                identities: &handoff.identities,
                equality: handoff.equality.as_ref(),
            },
            &mut budget,
        )
        .expect("the handoff's supply joins the package");
    let source = handoff.compiled.checked();
    let graph = source.graph();
    let name = |text: &str| Identifier::new(text).expect("an identifier");
    let selection = graph
        .resolve_operation(&name("M"), &name("Counter"), &name("inc"))
        .expect("inc resolves");
    let (context, operation_frame) = graph.operation_frame(&selection).expect("the frame node");
    let frame = operation_frame.frame();
    let facts = FrameFacts {
        context,
        frame: frame.clone(),
        operation: OperationFacts {
            declaring: operation_frame.operation().declaring.clone(),
            declaration: operation_frame.operation().declaration.clone(),
        },
    };
    let pre = document_ref("pre");
    let mut snapshots = BTreeMap::from([(pre.clone(), snapshot("pre", "pre", &[("a", 0)]))]);
    let mut invocations = BTreeMap::new();
    let mut verdicts = Vec::new();
    for (label, post, created) in [
        ("inside", vec![("a", 1)], serde_json::json!([])),
        (
            "creates",
            vec![("a", 1), ("c", 0)],
            serde_json::json!([{"population": "ix://test/counter/counters", "key": "c"}]),
        ),
    ] {
        let post_ref = document_ref(&format!("post-{label}"));
        snapshots.insert(
            post_ref.clone(),
            snapshot(&format!("post-{label}"), "post", &post),
        );
        let invocation = document_ref(label);
        invocations.insert(
            invocation.clone(),
            serde_json::to_vec(&serde_json::json!({
                "format": "filament.state.invocation/v1",
                "identity": {"authority": "test", "identity": label},
                "model": {"identity": "test/counter"},
                "context": "ix://test/counter/Counter",
                "operation": "inc",
                "self": {"population": "ix://test/counter/counters", "key": "a"},
                "pre": {"identity": {"authority": "test", "identity": "pre"}},
                "post": {"identity": {"authority": "test", "identity": format!("post-{label}")}},
                "parameters": {},
                "result": null,
                "created": created,
                "deleted": [],
            }))
            .expect("the invocation serializes"),
        );
        let admitted = admit_frame_invocation(
            graph.model_inputs(),
            graph.scope().types(),
            &facts,
            handoff.compiled.packages(),
            ModelNormalizationLimits::default(),
            &Provisions {
                snapshots: &snapshots,
                invocations: &invocations,
            },
            &invocation,
            ObservationLimits::default(),
        )
        .unwrap_or_else(|failure| panic!("{label}: the invocation admits: {failure:?}"));
        let mut meter = Meter::new(UNLIMITED);
        let from_source = source
            .evaluate_frame(frame.clone(), &admitted, &mut meter)
            .expect("the source frame evaluates");
        // The package route's operation table is the model-check input's
        // (QSL-2404); the package's own model operations stand for it here.
        let from_package =
            fsl_eval::bare::evaluate_frame(&bare, graph.scope().operations(), &frame, &admitted)
                .expect("the package frame evaluates");
        assert_eq!(
            outcome(&from_package.evaluation),
            outcome(&from_source.evaluation),
            "{label}"
        );
        assert_eq!(
            from_package.evaluation.location, from_source.evaluation.location,
            "{label}"
        );
        assert_eq!(
            format!("{:?}", from_package.witness),
            format!("{:?}", from_source.witness),
            "{label}"
        );
        verdicts.push((
            boolean(&from_package.evaluation),
            from_package.witness.is_some(),
        ));
    }
    assert_eq!(verdicts, [(true, false), (false, true)]);
}

/// FR-940-AC-6 with FR-495-AC-3: under `CheckMode::Kernel` a function whose
/// divisor can be zero checks and is undefined there. For a `holds` condition
/// that calls it, the package form over the bytes of the Kernel-checked
/// Counter returns the source form's value, location, work and trail at the
/// values where it is true and where it is undefined.
#[trace("TC-2958", "FR-940-AC-6", "FR-495-AC-3")]
#[test]
fn package_holds_equals_source_holds_where_undefined() {
    let model = document(false);
    let body = format!(
        "{CONTRACT}function part using v(a: Int[0, 3]): Int[0, 9] pure {{ 6 mod (2 - a) }}\n\
         temporal Partial using t0 over (c: M::Counter) clock \"model-steps\" \
         on origin {{ always holds(part(c.value) = 0) }}\n"
    );
    let source = package_with_mode(&model, &body, CheckMode::Kernel);
    let domain = fsl_semantics::model::intake::package_input([model.as_slice()])
        .expect("the Counter domain package admits");
    let read = fsl_package::read_evaluation_package(
        &source,
        fsl_semantics::library::LibraryName::new("package").expect("a library name"),
        &domain,
        fsl_package::V2ReadLimits::fixture(),
        fsl_package::EmitLimits::fixture(),
    )
    .expect("the strict reader admits the Kernel package's emission");
    let mut rows_budget = fsl_package::V2ReadLimits::fixture().native_join_budget();
    let rows = read
        .emission()
        .identity_rows(read.admitted(), &mut rows_budget)
        .expect("identity rows join the package");
    let equality = read
        .emission()
        .equality_witness(read.admitted(), &mut rows_budget)
        .expect("the emission writes its equality witness");
    let mut budget = fsl_package::V2ReadLimits::fixture().native_join_budget();
    let bare = BarePackage::new(read.admitted())
        .with_supply(
            BareSupply {
                identities: &rows,
                equality: Some(&equality),
            },
            &mut budget,
        )
        .expect("the bytes view admits its supply");
    let graph = source.graph();
    let [clause] = graph.temporal_clauses() else {
        panic!("one temporal clause");
    };
    let [atom] = holds_atoms(clause).try_into().expect("one holds atom");
    let observation = ObservationIdentity {
        authority: "test".to_owned(),
        identity: "start".to_owned(),
    };
    let mut readings = Vec::new();
    for value in [1, 2, 3] {
        let objects = admitted_objects(graph, &domain, value);
        let reference = counter(&objects);
        let over = Value::Reference(reference.clone());
        let mut source_meter = Meter::new(UNLIMITED);
        let from_source = source
            .evaluate_holds(&atom, over.clone(), &[], &objects, &mut source_meter)
            .expect("the source form evaluates");
        let mut package_meter = Meter::new(UNLIMITED);
        let from_package = bare
            .evaluate_holds(&atom, over, &[], &objects, &mut package_meter)
            .expect("the package form evaluates");
        assert_eq!(
            outcome(&from_package),
            outcome(&from_source),
            "value {value}"
        );
        assert_eq!(from_package.location, from_source.location, "value {value}");
        assert_eq!(
            package_meter.consumed(LimitKind::WorkUnits),
            source_meter.consumed(LimitKind::WorkUnits),
            "value {value}"
        );
        let read = reading(&from_package);
        if read.is_none() {
            assert!(from_package.location.is_some(), "value {value}");
        }
        readings.push(read);

        let binders = HoldsBinders {
            over: &reference,
            captures: &[],
        };
        let mut source_meter = Meter::new(UNLIMITED);
        let traced_source = source
            .evaluate_holds_traced(
                &atom,
                binders,
                &objects,
                observation.clone(),
                &mut source_meter,
            )
            .expect("the source traced form evaluates");
        let mut package_meter = Meter::new(UNLIMITED);
        let traced_package = bare
            .evaluate_holds_traced(
                &atom,
                binders,
                &objects,
                observation.clone(),
                &mut package_meter,
            )
            .expect("the package traced form evaluates");
        assert_eq!(
            format!("{traced_package:?}"),
            format!("{traced_source:?}"),
            "value {value}"
        );
        assert_eq!(
            package_meter.consumed(LimitKind::WorkUnits),
            source_meter.consumed(LimitKind::WorkUnits),
            "value {value}"
        );
    }
    // `6 mod (2 - value)` is 0 at 1 and 3 and divides by zero at 2.
    assert_eq!(readings, [Some(true), None, Some(true)]);
}

/// [`objects`] over any checked graph and domain package input.
fn admitted_objects(
    graph: &fsl_semantics::check::CheckedGraph,
    packages: &fsl_semantics::model::intake::PackageInput,
    value: i64,
) -> ObjectEnvironment {
    let selected = document_ref("start");
    let provision: BTreeMap<DocumentRef, Vec<u8>> =
        BTreeMap::from([(selected.clone(), counter_snapshot(&[("a", value)]))]);
    admit_trace_snapshot(
        graph.model_selections(),
        graph.scope().types(),
        packages,
        ModelNormalizationLimits::default(),
        &provision,
        &selected,
        ObservationLimits::default(),
        &[],
    )
    .expect("the Counter snapshot admits")
    .0
}
