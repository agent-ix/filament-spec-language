// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940 "Package forms of the source-checked evaluators" (TC-2958): each
//! package form a temporal engine calls returns the value its source form
//! returns, for a Counter unit compiled into its FR-517 handoff and the
//! `BarePackage` of the handoff's files.

mod hyper;

use std::collections::BTreeMap;

use filament_exact::{LimitKind, Meter, ScalarLimits, Value};
use fsl_eval::value::{CheckedPackageEvaluation, HoldsAtom, HoldsBinders, ObservationIdentity};
use fsl_package::{BarePackage, BareSupply, ReadEqualityRows, ReadIdentityRows};
use fsl_semantics::check::{FormulaNode, FormulaNodeId, TemporalClauseNode};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::object_environment::ObjectEnvironment;
use fsl_semantics::model::observation::{DocumentRef, ObservationLimits, admit_trace_snapshot};
use ix_trace_rs::trace;

use crate::analyze_cli_support::{
    HANDOFF, Workspace, counter_request_with, counter_snapshot, package_selection, request_members,
};
use crate::counter::{CONTRACT, document, package_with_mode};
use filament_semantic_value::checking::CheckMode;

/// FR-940-AC-32: direct source and admitted-package calls over the same
/// payload-bearing union argument retain their semantic outcomes and ordered
/// evaluation charge logs. The command-level run test separately verifies
/// the public canonical argument and requirement-item boundary.
#[trace("TC-2958", "FR-940-AC-32")]
#[test]
fn package_union_function_call_has_source_charge_logs() {
    use filament_eval::FamilyOutcome;
    use filament_exact::{Meter, Outcome, Value};
    use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
    use fsl_package::{
        BarePackage, BareSupply, ReadIdentityRows, V2ReadLimits, admit_checked_package,
    };

    const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"filament.value.complete/v1\";\n\
        union Shape { Empty, Circle(Boolean) }\n\
        function isCircle using v(s: Shape): Boolean pure {\
          case s { Empty: false; Circle(payload): payload; }\
        }\n\
        function shape using v(): Shape pure { Shape::Circle(true) }\n\
        function emptyShape using v(): Shape pure { Shape::Empty }\n";
    let ws = Workspace::new("package-union-call-charge-logs");
    ws.write("program.fsl", UNIT.as_bytes());
    let request = serde_json::json!({
        "models": [],
        "program": {"source": {"file": "program.fsl", "authority": "test",
            "identity": "union-call", "document": "UnionCall", "formal_revision": 1}}
    });
    let (_, bytes) = package_selection(&ws, &request);
    let compiled = fsl_command::checked_handoff::compile(&ws.path(HANDOFF))
        .expect("the exact producer handoff recompiles");
    let limits = V2ReadLimits::bounded(1 << 24);
    let admitted = admit_checked_package(&bytes, compiled.packages(), limits)
        .expect("the producer bytes admit");
    let mut budget = limits.native_join_limits().budget();
    let identities = ReadIdentityRows::read(
        &std::fs::read(ws.path(&format!("{HANDOFF}/identities.json")))
            .expect("the handoff identity rows"),
        &admitted,
        &mut budget,
    )
    .expect("the identity rows join");
    let bare = BarePackage::new(&admitted)
        .with_supply(
            BareSupply {
                identities: &identities,
                equality: None,
            },
            &mut budget,
        )
        .expect("the package identity supply joins");
    let source = compiled.checked();
    let name =
        |function: &str| QualifiedName::new(vec![function.to_owned()]).expect("function name");
    let objects = ObjectEnvironment::default();

    for (constructor, expected) in [("shape", true), ("emptyShape", false)] {
        let mut value_meter = Meter::new(UNLIMITED);
        let value = source
            .call(&name(constructor), Vec::new(), &objects, &mut value_meter)
            .expect("source constructor call");
        let argument = match value.outcome {
            FamilyOutcome::Evaluated(Outcome::Completed(value)) => value,
            other => panic!("constructor returns a union value: {other:?}"),
        };
        let mut source_meter = Meter::new(UNLIMITED);
        let mut package_meter = Meter::new(UNLIMITED);
        let source_result = source
            .call(
                &name("isCircle"),
                vec![argument.clone()],
                &objects,
                &mut source_meter,
            )
            .expect("source function accepts the union argument");
        let package_result = bare
            .call(
                &name("isCircle"),
                vec![argument],
                &objects,
                &mut package_meter,
            )
            .expect("admitted package function accepts the same union argument");
        assert_eq!(
            format!("{:?}", source_result.outcome),
            format!("{:?}", package_result.outcome)
        );
        assert!(matches!(
            source_result.outcome,
            FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(actual))) if actual == expected
        ));
        assert!(!source_meter.admitted_charges().is_empty());
        assert!(!source_meter.charge_log_truncated());
        assert!(!package_meter.charge_log_truncated());
        assert_eq!(
            package_meter.admitted_charges(),
            source_meter.admitted_charges(),
            "ordered semantic charge log for {constructor}"
        );
    }
}

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
     on origin { always (holds(c.value <= 1) or eventually holds(c.value = 3)) }\n\
     invariant NotAboveThree using v on M::Counter at current { self.value <= 3 }\n";

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
    handoff_of(name, CLAUSES)
}

/// [`handoff`] of the unit whose claims are `claims`.
fn handoff_of(
    name: &str,
    claims: &str,
) -> (
    Workspace,
    Handoff,
    filament_contract_model::NativeJoinBudget,
) {
    let ws = Workspace::new(name);
    let _ = counter_request_with(&ws, claims, request_members(None));
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

fn clause(handoff: &Handoff, index: usize) -> &TemporalClauseNode {
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

/// Every atom under a bounded past or future step operator keeps the source
/// evaluator's ordered expression charges, including its true and false cases.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn bounded_step_interval_atoms_preserve_expression_charges() {
    let claims: String = ["once", "historically", "eventually", "always"]
        .iter()
        .enumerate()
        .map(|(index, operator)| {
            format!("temporal Window{index} using t0 over (c: M::Counter) clock \"model-steps\" on origin {{ {operator}[1,2] holds(c.value <= 2) }}\n")
        })
        .collect();
    let (_ws, handoff, mut budget) = handoff_of("package-step-interval-charges", &claims);
    let bare = BarePackage::new(&handoff.admitted)
        .with_supply(
            BareSupply {
                identities: &handoff.identities,
                equality: handoff.equality.as_ref(),
            },
            &mut budget,
        )
        .expect("the handoff supply joins");
    let source = handoff.compiled.checked();
    assert_eq!(source.graph().temporal_clauses().len(), 4);
    for index in 0..4 {
        let atoms = holds_atoms(clause(&handoff, index));
        assert_eq!(atoms.len(), 1, "clause {index}");
        for value in 0..=3 {
            let objects = objects(&handoff, value);
            let over = Value::Reference(counter(&objects));
            for atom in &atoms {
                let mut source_meter = Meter::new(UNLIMITED);
                let from_source = source
                    .evaluate_holds(atom, over.clone(), &[], &objects, &mut source_meter)
                    .expect("the source atom evaluates");
                let mut package_meter = Meter::new(UNLIMITED);
                let from_package = bare
                    .evaluate_holds(atom, over.clone(), &[], &objects, &mut package_meter)
                    .expect("the byte-read atom evaluates");
                let at = format!("clause {index} at value {value}");
                assert_eq!(boolean(&from_source), value <= 2, "{at}");
                assert_eq!(outcome(&from_package), outcome(&from_source), "{at}");
                assert_eq!(from_package.location, from_source.location, "{at}");
                assert!(!source_meter.admitted_charges().is_empty(), "{at}");
                assert_eq!(
                    package_meter.admitted_charges(),
                    source_meter.admitted_charges(),
                    "{at}"
                );
            }
        }
    }
}

/// Reading a genuine timed interval does not authorize it for the untimed
/// model-check clause constructor: the remaining boundary is a typed profile
/// refusal, rather than an interval silently converted into a step window.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn timed_intervals_remain_typed_refusals_for_untimed_model_checking() {
    use fsl_package::{EmitLimits, V2ReadLimits};
    use fsl_semantics::check::{
        CheckedInterval, CheckedTemporalClause, PackageClauseRefusal, PackageFormulaNode,
        TemporalProfile,
    };

    let world = crate::zone_search::rpc_claim(
        3,
        crate::zone_search::TIMED,
        "temporal TimedWindow using tt over (c: M::Call) clock \"model-time\" on origin { eventually[1 ms,2 ms] holds(c.phase = 2) }\n",
    );
    let domain = fsl_semantics::model::intake::package_input([world.document.as_slice()])
        .expect("the timed domain admits");
    let emission = fsl_package::emit_checked(&world.package, EmitLimits::fixture())
        .expect("the real timed program emits");
    let limits = V2ReadLimits::fixture();
    let admitted = fsl_package::admit_checked_package(emission.package().bytes(), &domain, limits)
        .expect("the emitted timed bytes admit");
    let mut budget = limits.native_join_limits().budget();
    let identities = emission
        .identity_rows(&admitted, &mut budget)
        .expect("the emission's identity rows join");
    let bare = BarePackage::new(&admitted)
        .with_supply(
            BareSupply {
                identities: &identities,
                equality: None,
            },
            &mut budget,
        )
        .expect("the timed identity supply joins");
    let source_clause = world
        .package
        .graph()
        .temporal_clauses()
        .first()
        .expect("the source declares its timed clause");
    let occurrence = fsl_foundation::source::provenance::OccurrenceKey::new(
        source_clause.identity(),
        source_clause.claim().clone(),
    );
    let form = bare
        .temporal_clause_run_form(&occurrence)
        .expect("the interval is read from the actual package formula");
    assert!(form.formula.iter().any(|node| matches!(
        node,
        PackageFormulaNode::Unary {
            interval: Some(CheckedInterval::Timed(_)),
            ..
        }
    )));
    let types = bare.world().expect("the package world reads");
    assert!(matches!(
        CheckedTemporalClause::of_package_form(form, types.types(), u64::MAX),
        Err(PackageClauseRefusal::Profile(TemporalProfile::Timed))
    ));
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
        FrameFacts, OperationFacts, Provisions, admit_frame_invocation,
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
    let operations = package_operations(&bare, &handoff);
    assert_eq!(&operations, graph.scope().operations());
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
        // The package route reads the declared effect from its own operation
        // table, the one S3 declares for its domain packages (FR-940).
        let from_package = fsl_eval::simulation::model::ModelEvaluator::evaluate_frame(
            &bare,
            &frame,
            &admitted,
            &operations,
            &mut Meter::new(UNLIMITED),
        )
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

/// The operation table the package route's model-check input reads: the one
/// S3 declares for the domain packages bound to the package's selections.
fn package_operations(
    bare: &BarePackage<'_>,
    handoff: &Handoff,
) -> fsl_semantics::model::operation::OperationTable {
    let world = bare.world().expect("the package's declarations read");
    let selections = fsl_package::AnalysisView::model_selections(bare);
    fsl_semantics::model::state::StateModel::new(
        fsl_semantics::model::observation::ModelSelections::new(&selections, &[]),
        world.types(),
        handoff.compiled.packages(),
        ModelNormalizationLimits::default(),
    )
    .expect("the supplied domain package admits")
    .operation_table(&BTreeMap::new())
    .expect("its operations are typed")
}

/// FR-940: the model-check evaluator of either route evaluates a `holds`
/// atom and a `when` guard keyed by node with the value, location and work
/// of the source form, so a model check reaches the package forms through
/// the evaluator its input carries.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn the_model_evaluator_reaches_the_package_holds_and_guard_forms() {
    let (_ws, handoff, mut budget) = handoff("package-forms-evaluator");
    let bare = BarePackage::new(&handoff.admitted)
        .with_supply(
            BareSupply {
                identities: &handoff.identities,
                equality: handoff.equality.as_ref(),
            },
            &mut budget,
        )
        .expect("the handoff's supply joins the package");
    let source: &dyn fsl_eval::simulation::model::ModelEvaluator = handoff.compiled.checked();
    let package: &dyn fsl_eval::simulation::model::ModelEvaluator = &bare;
    let guarded = clause(&handoff, 1);
    let mut readings = Vec::new();
    for value in [1, 2, 3] {
        let objects = objects(&handoff, value);
        let over = Value::Reference(counter(&objects));
        for index in [0, 3] {
            for atom in holds_atoms(clause(&handoff, index)) {
                let (mut at_source, mut at_package) =
                    (Meter::new(UNLIMITED), Meter::new(UNLIMITED));
                let from_source = source
                    .evaluate_holds(&atom, over.clone(), &[], &objects, &mut at_source)
                    .expect("the source form evaluates");
                let from_package = package
                    .evaluate_holds(&atom, over.clone(), &[], &objects, &mut at_package)
                    .expect("the package form evaluates");
                assert_eq!(outcome(&from_package), outcome(&from_source));
                assert_eq!(from_package.location, from_source.location);
                assert_eq!(
                    at_package.consumed(LimitKind::WorkUnits),
                    at_source.consumed(LimitKind::WorkUnits)
                );
                readings.push(boolean(&from_package));
            }
        }
        let (mut at_source, mut at_package) = (Meter::new(UNLIMITED), Meter::new(UNLIMITED));
        let from_source = source
            .evaluate_guard(&guarded.identity(), over.clone(), &objects, &mut at_source)
            .expect("the source guard evaluates");
        let from_package = package
            .evaluate_guard(&guarded.identity(), over, &objects, &mut at_package)
            .expect("the package guard evaluates");
        assert_eq!(outcome(&from_package), outcome(&from_source));
        assert_eq!(boolean(&from_package), value >= 2);
    }
    assert!(
        readings.contains(&true) && readings.contains(&false),
        "{readings:?}"
    );
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

/// The observations of the state clause `name` of the handoff's source
/// package selected by `input`.
type Documents = BTreeMap<DocumentRef, Vec<u8>>;

fn admit_clause(
    handoff: &Handoff,
    name: &str,
    provision: (&Documents, &Documents),
    input: fsl_semantics::model::observation::ClauseSelectionInput,
) -> fsl_semantics::model::observation::AdmittedObservations {
    use fsl_semantics::model::observation::{
        ClauseFacts, ClauseSelection, OperationFacts, Provisions, admit_observations,
    };
    let graph = handoff.compiled.checked().graph();
    let clause = graph.state_clause(name).expect("the clause");
    let facts = ClauseFacts {
        identity: clause.identity(),
        kind: clause.kind(),
        context: clause.context(),
        operation: clause.operation().map(|operation| OperationFacts {
            declaring: operation.declaring.clone(),
            declaration: operation.declaration.clone(),
        }),
        operations: graph.scope().operations().clone(),
        named_populations: clause.named_reads(),
    };
    admit_observations(
        graph.model_selections(),
        graph.scope().types(),
        &facts,
        handoff.compiled.packages(),
        ModelNormalizationLimits::default(),
        &Provisions {
            snapshots: provision.0,
            invocations: provision.1,
        },
        &ClauseSelection {
            name: name.to_owned(),
            input,
        },
        ObservationLimits::default(),
    )
    .unwrap_or_else(|failure| panic!("{name}: the observations admit: {failure:?}"))
}

/// FR-940-AC-6 (the forms' part): the package forms of `evaluate_clause`
/// and `evaluate_clause_set`, reading each clause from the node its
/// observations were admitted for, return the source forms' evaluations,
/// trails and work for the invariant `NotAboveThree`, the precondition
/// `CanInc` and the postcondition `Inc`, which reads `pre(self.value)`, on
/// both sides of each, and the set's grouping, short-circuit and outcome.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn package_state_clauses_equal_their_source_forms() {
    use fsl_eval::value::QualifiedName;
    use fsl_semantics::model::observation::{
        AnchorKind, ClauseSelectionInput, SelectedAnchor, SelectedObject,
    };

    let (_ws, handoff, mut budget) = handoff("package-forms-clauses");
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
    let counter_a = || SelectedObject {
        population: "ix://test/counter/counters".to_owned(),
        key: "a".to_owned(),
    };
    let mut snapshots = BTreeMap::new();
    let mut invocations = BTreeMap::new();
    for value in [0_i64, 1, 2, 3] {
        snapshots.insert(
            document_ref(&format!("current-{value}")),
            snapshot(&format!("current-{value}"), "current", &[("a", value)]),
        );
        snapshots.insert(
            document_ref(&format!("pre-{value}")),
            snapshot(&format!("pre-{value}"), "pre", &[("a", value)]),
        );
        snapshots.insert(
            document_ref(&format!("post-{value}")),
            snapshot(&format!("post-{value}"), "post", &[("a", value)]),
        );
    }
    for (label, pre, post) in [("step", 0, 1), ("still", 1, 1)] {
        invocations.insert(
            document_ref(label),
            serde_json::to_vec(&serde_json::json!({
                "format": "filament.state.invocation/v1",
                "identity": {"authority": "test", "identity": label},
                "model": {"identity": "test/counter"},
                "context": "ix://test/counter/Counter",
                "operation": "inc",
                "self": {"population": "ix://test/counter/counters", "key": "a"},
                "pre": {"identity": {"authority": "test", "identity": format!("pre-{pre}")}},
                "post": {"identity": {"authority": "test", "identity": format!("post-{post}")}},
                "parameters": {},
                "result": null,
                "created": [],
                "deleted": [],
            }))
            .expect("the invocation serializes"),
        );
    }
    let provision = (&snapshots, &invocations);
    let current = |value: i64| ClauseSelectionInput::Current {
        snapshot: document_ref(&format!("current-{value}")),
        anchor: SelectedAnchor {
            kind: AnchorKind::Initialization,
            name: "start".to_owned(),
        },
        self_object: Some(counter_a()),
    };
    let pre_call = |value: i64| ClauseSelectionInput::PreCall {
        snapshot: document_ref(&format!("pre-{value}")),
        self_object: Some(counter_a()),
        parameters: BTreeMap::new(),
    };
    let invoked = |label: &str| ClauseSelectionInput::Invocation {
        invocation: document_ref(label),
    };
    let cases = [
        (
            "NotAboveThree",
            admit_clause(&handoff, "NotAboveThree", provision, current(3)),
        ),
        (
            "CanInc",
            admit_clause(&handoff, "CanInc", provision, pre_call(2)),
        ),
        (
            "CanInc",
            admit_clause(&handoff, "CanInc", provision, pre_call(3)),
        ),
        (
            "Inc",
            admit_clause(&handoff, "Inc", provision, invoked("step")),
        ),
        (
            "Inc",
            admit_clause(&handoff, "Inc", provision, invoked("still")),
        ),
    ];
    let mut readings = Vec::new();
    for (name, observations) in &cases {
        let qualified = QualifiedName::unqualified(*name).expect("a clause name");
        let mut source_meter = Meter::new(UNLIMITED);
        let from_source = source
            .evaluate_clause(&qualified, observations, &mut source_meter)
            .expect("the source clause evaluates");
        let mut package_meter = Meter::new(UNLIMITED);
        let from_package = bare
            .evaluate_clause(&qualified, observations, &mut package_meter)
            .expect("the package clause evaluates");
        assert_eq!(
            format!("{from_package:?}"),
            format!("{from_source:?}"),
            "{name}"
        );
        assert_eq!(
            package_meter.consumed(LimitKind::WorkUnits),
            source_meter.consumed(LimitKind::WorkUnits),
            "{name}"
        );
        readings.push(boolean(&from_package.evaluation));
    }
    assert_eq!(readings, [true, true, false, true, false]);

    // A set whose failing precondition skips the postcondition, and one
    // that reaches it.
    for (precondition, postcondition) in [(2, 3), (1, 3)] {
        let pre_obs = &cases[precondition].1;
        let post_obs = &cases[postcondition].1;
        let set = [
            (QualifiedName::unqualified("Inc").expect("a name"), post_obs),
            (
                QualifiedName::unqualified("CanInc").expect("a name"),
                pre_obs,
            ),
            (
                QualifiedName::unqualified("NotAboveThree").expect("a name"),
                &cases[0].1,
            ),
        ];
        let from_source = source
            .evaluate_clause_set(&set, UNLIMITED)
            .expect("the source set evaluates");
        let from_package = bare
            .evaluate_clause_set(&set, UNLIMITED)
            .expect("the package set evaluates");
        assert_eq!(
            format!("{from_package:?}"),
            format!("{from_source:?}"),
            "precondition {precondition}"
        );
        // The false precondition skips the postcondition; the true one reaches it.
        let expected = if precondition == 2 { 2 } else { 3 };
        assert_eq!(
            from_package.results.len(),
            expected,
            "precondition {precondition}"
        );
    }
}

/// FR-940 "Package forms of the source-checked evaluators" (QSL-2425): the
/// package form of each temporal clause of the Counter handoff, read from the
/// `BarePackage`'s admitted nodes by its node key with the supplied domain
/// packages' population domains, is the clause the source route's checked
/// graph holds: the same name, span, binders, clock, activation, operators,
/// formula and claim, so the model checker reads one clause on both routes.
#[trace("TC-2958", "FR-940-AC-6")]
#[trace("TC-2983", "FR-850-AC-8")]
#[test]
fn the_package_form_of_each_temporal_clause_equals_its_source_form() {
    let (_ws, handoff, mut budget) = handoff("package-forms-clause");
    let bare = BarePackage::new(&handoff.admitted)
        .with_supply(
            BareSupply {
                identities: &handoff.identities,
                equality: handoff.equality.as_ref(),
            },
            &mut budget,
        )
        .expect("the handoff's supply joins the package");
    let world = bare.world().expect("the package's declarations read");
    let selections = fsl_package::AnalysisView::model_selections(&bare);
    let model = fsl_semantics::model::state::StateModel::new(
        fsl_semantics::model::observation::ModelSelections::new(&selections, &[]),
        world.types(),
        handoff.compiled.packages(),
        ModelNormalizationLimits::default(),
    )
    .expect("the supplied domain package admits");
    let clauses = handoff.compiled.checked().graph().temporal_clauses();
    assert_eq!(clauses.len(), 4);
    for source in clauses {
        let mut form = bare
            .temporal_clause_form(&fsl_foundation::source::provenance::OccurrenceKey::new(
                source.identity(),
                source.claim().clone(),
            ))
            .unwrap_or_else(|refusal| panic!("{}: {refusal:?}", source.clause().name()));
        if let filament_exact::ValueType::Reference(object) = &form.over.1 {
            form.populations = model.overlapping_populations(object.clone());
        }
        let package = fsl_semantics::check::CheckedTemporalClause::of_package_form(
            form,
            world.types(),
            filament_semantic_value::checking::CheckingLimits::default().nodes(),
        )
        .unwrap_or_else(|refusal| panic!("{}: {refusal}", source.clause().name()));
        // FSpec FR-322 "Clause declaration regions": the name row carries the
        // whole declaration's region, so every field, span included, is the
        // source form's (FR-850-AC-8).
        assert_eq!(package.span(), source.clause().span());
        assert_eq!(
            format!("{package:?}"),
            format!("{:?}", source.clause()),
            "{}",
            source.clause().name()
        );
        assert!(package == *source.clause());
    }
}

/// The four state-graph claims of FR-940-AC-11 over the Counter unit: one of
/// each form, `Back` written without `from`.
const STATE_GRAPH_CLAIMS: &str = "\
     possible ReachThree using v over (c: M::Counter) { c.value = 3 }\n\
     always possible Back using v over (c: M::Counter) { c.value = 0 }\n\
     always possible FromOne using v over (c: M::Counter) from (c.value = 1) { c.value = 0 }\n\
     unique path Straight using v over (c: M::Counter) from (c.value = 0) to (c.value = 1);\n";

/// The predicates of `form` in order `from`, `target`.
fn graph_predicates(
    form: &fsl_semantics::check::StateGraphForm,
) -> Vec<&fsl_semantics::check::CheckedStatePredicate> {
    use fsl_semantics::check::StateGraphForm;
    match form {
        StateGraphForm::Possible { target } => vec![target],
        StateGraphForm::AlwaysPossible { from, target } => vec![from, target],
        StateGraphForm::UniquePath { from, to } => vec![from, to],
    }
}

/// FR-940-AC-11 (first half): `BarePackage::state_graph_claim_form` of each
/// claim's `claim_names` occurrence, rebuilt by
/// `CheckedStateGraphClaim::of_package_form`, equals the source route's
/// checked claim in name, span, `over`, form, capability, requirements and
/// occurrence; `Back`'s `from` is the constant predicate and a written
/// predicate holds its package node and no checked body.
#[trace("TC-3034", "FR-940-AC-11")]
#[test]
fn the_package_form_of_each_state_graph_claim_equals_its_source_claim() {
    let (_ws, handoff, mut budget) = handoff_of("package-forms-state-graph", STATE_GRAPH_CLAIMS);
    let bare = BarePackage::new(&handoff.admitted)
        .with_supply(
            BareSupply {
                identities: &handoff.identities,
                equality: handoff.equality.as_ref(),
            },
            &mut budget,
        )
        .expect("the handoff's supply joins the package");
    let world = bare.world().expect("the package's declarations read");
    let sources = handoff.compiled.checked().graph().state_graph_claims();
    assert_eq!(sources.len(), 4);
    for source in sources {
        let occurrence = source.occurrence().expect("an assembled claim").clone();
        let form = bare
            .state_graph_claim_form(&occurrence)
            .unwrap_or_else(|refusal| panic!("{}: {refusal:?}", source.name()));
        let package = fsl_semantics::check::CheckedStateGraphClaim::of_package_form(
            form,
            world.types(),
            filament_semantic_value::checking::CheckingLimits::default().nodes(),
        )
        .unwrap_or_else(|refusal| panic!("{}: {refusal}", source.name()));
        let name = source.name();
        assert_eq!(package.name(), name);
        assert_eq!(package.span(), source.span(), "{name}");
        assert_eq!(package.over(), source.over(), "{name}");
        assert_eq!(package.occurrence(), source.occurrence(), "{name}");
        assert_eq!(package.claim().kind(), source.claim().kind(), "{name}");
        assert_eq!(
            package.claim().requirements(occurrence.node().clone()),
            source.claim().requirements(occurrence.node().clone()),
            "{name}"
        );
        assert_eq!(
            std::mem::discriminant(package.form()),
            std::mem::discriminant(source.form()),
            "{name}"
        );
        for (package, source) in graph_predicates(package.form())
            .into_iter()
            .zip(graph_predicates(source.form()))
        {
            assert_eq!(
                package.is_constant_true(),
                source.is_constant_true(),
                "{name}"
            );
            assert_eq!(
                package.package().is_some(),
                source.body().is_some(),
                "{name}"
            );
            assert!(package.body().is_none(), "{name}");
        }
    }
}

/// FR-940-AC-11 (clause): at each state the Counter explores (`value` 0
/// through 3), the keyed evaluator port gives the package route's predicate
/// of each role of each state-graph claim the source route's value and location,
/// and the same symbolic reads (the footprint a partial-order reduction takes
/// of each predicate).
#[trace("TC-3034", "FR-940-AC-11")]
#[test]
fn the_port_evaluates_each_state_graph_role_alike_at_each_explored_state() {
    use fsl_eval::simulation::model::ModelEvaluator;
    use fsl_semantics::check::PredicateRole;
    let (_ws, handoff, mut budget) = handoff_of("package-forms-port-roles", STATE_GRAPH_CLAIMS);
    let bare = BarePackage::new(&handoff.admitted)
        .with_supply(
            BareSupply {
                identities: &handoff.identities,
                equality: handoff.equality.as_ref(),
            },
            &mut budget,
        )
        .expect("the handoff's supply joins the package");
    let world = bare.world().expect("the package's declarations read");
    let selections = fsl_package::AnalysisView::model_selections(&bare);
    let model = fsl_semantics::model::state::StateModel::new(
        fsl_semantics::model::observation::ModelSelections::new(&selections, &[]),
        world.types(),
        handoff.compiled.packages(),
        ModelNormalizationLimits::default(),
    )
    .expect("the supplied domain package admits");
    let source: &dyn ModelEvaluator = handoff.compiled.checked();
    let package: &dyn ModelEvaluator = &bare;
    let mut compared = 0;
    for claim in handoff.compiled.checked().graph().state_graph_claims() {
        let name = claim.name();
        let occurrence = claim.occurrence().expect("an assembled claim");
        let (from, target) = claim.node_predicates();
        let roles = [
            (PredicateRole::From, from),
            (PredicateRole::Target, Some(target)),
        ];
        for (role, predicate) in roles {
            let Some(predicate) = predicate.filter(|predicate| !predicate.is_constant_true())
            else {
                continue;
            };
            assert!(predicate.body().is_some(), "{name} {role:?}");
            assert_eq!(
                package.state_graph_reads(&model, occurrence, role),
                source.state_graph_reads(&model, occurrence, role),
                "{name} {role:?}"
            );
            for value in 0..=3 {
                let objects = objects(&handoff, value);
                let over = Some(counter(&objects));
                let (mut at_source, mut at_package) =
                    (Meter::new(UNLIMITED), Meter::new(UNLIMITED));
                let from_source = source
                    .evaluate_state_graph_predicate(
                        occurrence,
                        role,
                        &objects,
                        over.clone(),
                        Vec::new(),
                        &mut at_source,
                    )
                    .expect("the source predicate evaluates");
                let from_package = package
                    .evaluate_state_graph_predicate(
                        occurrence,
                        role,
                        &objects,
                        over,
                        Vec::new(),
                        &mut at_package,
                    )
                    .expect("the package predicate evaluates");
                let at = format!("{name} {role:?} at {value}");
                assert_eq!(outcome(&from_package), outcome(&from_source), "{at}");
                assert_eq!(from_package.location, from_source.location, "{at}");
                compared += 1;
            }
        }
    }
    // ReachThree and Back (target), FromOne and Straight (from and target).
    assert_eq!(compared, 6 * 4);
}

/// FR-940-AC-11, FR-940-AC-6: the target of a state-graph claim, read from the
/// package's bytes, evaluates at each explored state to the source route's
/// value and location and admits the source route's ordered charges, entry
/// for entry. The Boolean `false` literal reads no field and applies no
/// operation, so both routes admit no charge for it; `c.value < 0` reads a
/// field and compares, so both admit the same non-empty log.
#[trace("TC-3034", "FR-940-AC-11")]
#[test]
fn a_state_graph_predicate_evaluates_and_charges_as_the_source() {
    use fsl_eval::simulation::model::ModelEvaluator;
    use fsl_semantics::check::PredicateRole;
    let (_ws, handoff, mut budget) = handoff_of(
        "package-forms-false-literal",
        "possible NeverHolds using v over (c: M::Counter) { false }\n\
         possible Negative using v over (c: M::Counter) { c.value < 0 }\n",
    );
    let bare = BarePackage::new(&handoff.admitted)
        .with_supply(
            BareSupply {
                identities: &handoff.identities,
                equality: handoff.equality.as_ref(),
            },
            &mut budget,
        )
        .expect("the handoff's supply joins the package");
    let source: &dyn ModelEvaluator = handoff.compiled.checked();
    let package: &dyn ModelEvaluator = &bare;
    for (name, charges) in [("NeverHolds", false), ("Negative", true)] {
        let claim = handoff
            .compiled
            .checked()
            .graph()
            .state_graph_claims()
            .iter()
            .find(|claim| claim.name() == name)
            .expect("the claim is checked");
        let occurrence = claim.occurrence().expect("an assembled claim");
        for value in 0..=3 {
            let objects = objects(&handoff, value);
            let over = Some(counter(&objects));
            let (mut at_source, mut at_package) = (Meter::new(UNLIMITED), Meter::new(UNLIMITED));
            let from_source = source
                .evaluate_state_graph_predicate(
                    occurrence,
                    PredicateRole::Target,
                    &objects,
                    over.clone(),
                    Vec::new(),
                    &mut at_source,
                )
                .expect("the source predicate evaluates");
            let from_package = package
                .evaluate_state_graph_predicate(
                    occurrence,
                    PredicateRole::Target,
                    &objects,
                    over,
                    Vec::new(),
                    &mut at_package,
                )
                .expect("the package predicate evaluates");
            let at = format!("{name} at value {value}");
            assert!(!boolean(&from_source), "{at}");
            assert_eq!(outcome(&from_package), outcome(&from_source), "{at}");
            assert_eq!(from_package.location, from_source.location, "{at}");
            assert_eq!(
                !at_source.admitted_charges().is_empty(),
                charges,
                "{at}: {:?}",
                at_source.admitted_charges()
            );
            assert_eq!(
                at_package.admitted_charges(),
                at_source.admitted_charges(),
                "{at}"
            );
        }
    }
}

/// FR-940-AC-11, FR-155, FR-156: the package route's operation footprints,
/// state-constraint reads, `holds` atom reads and activation guard reads,
/// derived through the evaluator port from the package's own nodes, equal the
/// source route's over the Counter unit's `pre CanInc`, `post Inc` (a
/// `pre(..)` read), `invariant`, two-atom and guarded clauses.
#[trace("TC-3034", "FR-940-AC-11")]
#[test]
fn the_port_derives_the_package_footprints_the_source_derives() {
    use fsl_eval::simulation::model::ModelEvaluator;
    use fsl_semantics::check::footprint::{derive_atom_reads, derive_guard_reads};
    use fsl_semantics::model::operation::OperationLookup;
    let (_ws, handoff, mut budget) = handoff("package-forms-footprints");
    let bare = BarePackage::new(&handoff.admitted)
        .with_supply(
            BareSupply {
                identities: &handoff.identities,
                equality: handoff.equality.as_ref(),
            },
            &mut budget,
        )
        .expect("the handoff's supply joins the package");
    let world = bare.world().expect("the package's declarations read");
    let selections = fsl_package::AnalysisView::model_selections(&bare);
    let model = fsl_semantics::model::state::StateModel::new(
        fsl_semantics::model::observation::ModelSelections::new(&selections, &[]),
        world.types(),
        handoff.compiled.packages(),
        ModelNormalizationLimits::default(),
    )
    .expect("the supplied domain package admits");
    let source: &dyn ModelEvaluator = handoff.compiled.checked();
    let package: &dyn ModelEvaluator = &bare;
    let operations = model
        .operation_table(&BTreeMap::new())
        .expect("the operations resolve");
    let mut footprints = 0;
    for declaration in world.types().object_types() {
        let receiver = declaration.key();
        for name in ["inc", "dec", "reset"] {
            let OperationLookup::Declared {
                declaring,
                operation,
            } = operations.resolve(world.types(), receiver.clone(), name)
            else {
                continue;
            };
            let from_package =
                package.footprint(&model, receiver.clone(), declaring.clone(), operation, &[]);
            let from_source = source.footprint(&model, receiver.clone(), declaring, operation, &[]);
            assert!(from_source.is_ok(), "{name}: {from_source:?}");
            assert_eq!(from_package, from_source, "{name}");
            footprints += 1;
        }
    }
    assert!(footprints > 0, "the Counter declares an operation");

    assert_eq!(
        package
            .constraint_reads(&model, "NotAboveThree")
            .expect("the package declares the invariant"),
        source
            .constraint_reads(&model, "NotAboveThree")
            .expect("the source declares the invariant"),
    );
    assert!(package.constraint_reads(&model, "Missing").is_none());

    let graph = handoff.compiled.checked().graph();
    let mut atoms = 0;
    let mut guards = 0;
    for clause in graph.temporal_clauses() {
        let key = clause.identity();
        let holds = clause
            .clause()
            .formula()
            .nodes()
            .iter()
            .enumerate()
            .filter(|(_, node)| matches!(node, FormulaNode::Holds { .. }))
            .map(|(index, _)| FormulaNodeId::from_arena_index(index));
        for id in holds {
            let atom = HoldsAtom::of_source(clause, id).expect("a holds atom is a formula use");
            let (condition, _) = clause.condition(id).expect("a holds atom has a condition");
            let expected = derive_atom_reads(graph, &model, clause.clause().over(), condition)
                .expect("the source atom reads");
            let read = package
                .holds_reads(&model, &key, atom.position)
                .unwrap_or_else(|refusal| panic!("{}: {refusal:?}", clause.clause().name()));
            assert_eq!(read.reads, expected, "{}", clause.clause().name());
            atoms += 1;
        }
        let mut binders = vec![clause.clause().over().clone()];
        binders.extend(
            clause
                .clause()
                .trigger()
                .map(|(name, value_type)| (name.to_owned(), value_type.clone())),
        );
        if let Some((guard, _)) = clause.guard() {
            let expected =
                derive_guard_reads(graph, &model, &binders, guard).expect("the source guard reads");
            let read = package
                .activation_guard_reads(&model, &key)
                .unwrap_or_else(|refusal| panic!("{}: {refusal:?}", clause.clause().name()))
                .expect("the package clause has the guard");
            assert_eq!(read.reads, expected, "{}", clause.clause().name());
            guards += 1;
        }
    }
    assert!(atoms >= 5 && guards >= 1, "{atoms} atoms, {guards} guards");
}
