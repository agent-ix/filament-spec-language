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
use crate::counter::document;

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
/// `on origin` clause, a guarded `on each` clause and a clause whose
/// condition reads a capture.
const CLAUSES: &str = "temporal Bounded using t0 over (c: M::Counter) clock \"model-steps\" \
     on origin { always holds(c.value <= 3 and not (c.value = 1)) }\n\
     temporal Guarded using t0 over (c: M::Counter) clock \"model-steps\" \
     on each (q: M::Counter) when (c.value >= 2) { always holds(c.value <= 2) }\n\
     temporal Captured using t0 over (c: M::Counter) clock \"model-steps\" \
     on each (q: M::Counter) when (true) { capture n: Int[0, 3] = c.value; \
     always holds(c.value >= n) }\n";

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

/// FR-940-AC-6 (the forms' part): over the handoff of the Counter unit, the
/// package form of `evaluate_holds` returns the source form's value and work
/// for every `holds` atom of an `on origin`, a guarded `on each` and a
/// capture-reading clause, at counter values that make each condition both
/// true and false.
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
    ] {
        let clause = clause(&handoff, index);
        let atoms = holds_atoms(clause);
        assert!(!atoms.is_empty(), "clause {index} has a holds atom");
        for value in [1, 2, 3] {
            let objects = objects(&handoff, value);
            let over = Value::Reference(counter(&objects));
            for atom in &atoms {
                let mut source_meter = Meter::new(UNLIMITED);
                let from_source = source
                    .evaluate_holds(atom, over.clone(), &captures, &objects, &mut source_meter)
                    .expect("the source form evaluates");
                let mut package_meter = Meter::new(UNLIMITED);
                let from_package = bare
                    .evaluate_holds(atom, over.clone(), &captures, &objects, &mut package_meter)
                    .expect("the package form evaluates");
                assert_eq!(
                    outcome(&from_package),
                    outcome(&from_source),
                    "clause {index} at value {value}"
                );
                assert_eq!(
                    from_package.location, from_source.location,
                    "clause {index} at value {value}"
                );
                assert_eq!(
                    package_meter.consumed(LimitKind::WorkUnits),
                    source_meter.consumed(LimitKind::WorkUnits),
                    "clause {index} at value {value}"
                );
                seen.push(boolean(&from_package));
            }
        }
    }
    assert!(seen.contains(&true) && seen.contains(&false), "{seen:?}");
}

/// FR-940-AC-6 (the forms' part): the package form of `evaluate_guard`
/// returns the source form's value for the guarded `on each` clause on both
/// sides of its guard, and `evaluate_holds_traced` returns the source form's
/// evaluation and receiver path.
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

    let objects = objects(&handoff, 3);
    let over = counter(&objects);
    let observation = ObservationIdentity {
        authority: "test".to_owned(),
        identity: "start".to_owned(),
    };
    for atom in holds_atoms(guarded) {
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
        assert_eq!(
            outcome(&from_package.evaluation),
            outcome(&from_source.evaluation)
        );
        assert!(!boolean(&from_package.evaluation));
        assert_eq!(from_package.receiver_path(), from_source.receiver_path());
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
