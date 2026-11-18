// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-578 (TC-1422, TC-1423): the read-only step view of a linked protocol
//! and the FR-351 record of a protocol refinement decided outside FSL.

use fsl_analyze::protocol_analysis::protocol_step_view;
use fsl_eval::simulation::model::ModelSystem;
use fsl_eval::simulation::protocol::{StepKind, Unsupported};
use fsl_foundation::MemoryModelName;
use fsl_replay::protocol_analysis::{node_text, AssessedRoot, RealizabilityCounterexample};
use fsl_replay::{
    AbstractProtocolSystem, ProtocolStepView, ProtocolSteps, RefinementWitness,
    RefinementWitnessError, StateIndex, StepViewRefusal, UnsteppedConstruct,
};
use ix_trace_rs::trace;

use crate::fill;
use crate::protocol_projection::{admitted, clause, root as assessed_root};
use crate::protocol_realizability::{refuted, worked, Fixture, CANCELLED, PICK};
use crate::weak_memory::{load, select, store, Case};

const LIMIT: usize = 4096;

fn view(fixture: &Fixture, max_states: usize) -> Result<ProtocolStepView, StepViewRefusal> {
    protocol_step_view(
        &fixture.admitted,
        &clause(&fixture.admitted),
        &fixture.subject(),
        max_states,
    )
}

/// A litmus protocol whose `left` branch makes `stores` relaxed stores of
/// `x` and whose `right` branch loads it, with both package forms.
fn litmus(stores: usize) -> (Case, Box<filament_contract_model::CheckedPackageV2>) {
    let stores: String = (1..=stores)
        .map(|at| store(&format!("S{at}"), "x", "StoreOne"))
        .collect();
    let run = format!(
        "parallel Both {{ branch left sequence L {{ {stores} }} \
         branch right sequence R {{ {} }} }} join all [left, right];",
        load("Lx", "x", 1)
    );
    let case = Case::new(
        &(crate::litmus::contracts()
            + &crate::litmus::protocol("SB", &run)
            + crate::litmus::NO_BOTH_ZERO),
    );
    let package = admitted(
        case.checked(),
        crate::litmus::PACKAGE,
        &crate::litmus::document(),
    );
    (case, package)
}

fn steps(fixture: &Fixture) -> ProtocolSteps {
    match view(fixture, LIMIT).expect("the view is built") {
        ProtocolStepView::Steps(steps) => steps,
        ProtocolStepView::Unsupported(found) => panic!("expected steps: {found:?}"),
    }
}

fn unsupported(fixture: &Fixture) -> Vec<UnsteppedConstruct> {
    match view(fixture, LIMIT).expect("the view is built") {
        ProtocolStepView::Unsupported(found) => found,
        ProtocolStepView::Steps(steps) => panic!("expected unsupported: {steps:?}"),
    }
}

/// TC-1422 step 1: each step is labelled with its kind, control node,
/// channel node and record, from the `fsl-eval` system's own successors.
#[trace("TC-1422", "FR-578-AC-1")]
#[test]
fn the_view_labels_every_step_of_the_fsl_eval_system() {
    let fixture = worked("exactly-once-premise", PICK, CANCELLED);
    let view = steps(&fixture);
    assert_eq!(view.initial, vec![StateIndex(0)]);
    let labelled = |kind: StepKind, node: &str, channel: &str| {
        view.steps.iter().any(|step| {
            step.kind == kind
                && step.node == Some(fixture.named(node))
                && step.channel == Some(fixture.named(channel))
                && step.record.is_some()
        })
    };
    assert!(labelled(StepKind::Send, "Placed", "Order"));
    assert!(labelled(StepKind::Receive, "Got", "Order"));
    assert!(labelled(StepKind::Send, "PickIt", "Pick"));
    assert!(labelled(StepKind::Receive, "Cn2", "Cancelled"));
    assert!(view
        .steps
        .iter()
        .any(|step| step.kind == StepKind::Finish && step.node == Some(fixture.named("Done"))));
    let subject = fixture.subject();
    let model = ModelSystem::new(subject.model.simulation.clone()).expect("the model admits");
    let system = AbstractProtocolSystem::new(model, &subject).expect("the system builds");
    for step in &view.steps {
        assert!(step.from.0 < view.states && step.to.0 < view.states);
        let communicates = matches!(step.kind, StepKind::Send | StepKind::Receive);
        assert_eq!(step.channel.is_some(), communicates, "{step:?}");
        assert_eq!(
            step.node.is_some(),
            step.transition.node().is_some(),
            "a step of a control names its node: {step:?}"
        );
        assert_eq!(
            step.record,
            system
                .binder_record_bytes(&step.transition)
                .expect("the record encodes"),
            "the record is the step's own binder record: {step:?}"
        );
    }

    // A `parallel`'s fork names its control and carries no record.
    let (case, package) = litmus(1);
    let forked = match protocol_step_view(&package, &clause(&package), &case.subject(&[]), LIMIT)
        .expect("the view is built")
    {
        ProtocolStepView::Steps(steps) => steps,
        ProtocolStepView::Unsupported(found) => panic!("expected steps: {found:?}"),
    };
    let fork = forked
        .steps
        .iter()
        .find(|step| step.kind == StepKind::Fork)
        .expect("a fork step");
    assert_eq!(fork.record, None);
    assert!(fork.node.is_some());
    let initial = system.initial();
    let expected: Vec<_> = system
        .successors(&initial[0])
        .expect("the initial state expands")
        .successors
        .into_iter()
        .map(|(transition, _)| transition)
        .collect();
    let taken: Vec<_> = view
        .steps
        .iter()
        .filter(|step| step.from == StateIndex(0))
        .map(|step| step.transition.clone())
        .collect();
    assert_eq!(taken, expected);
    assert_eq!(steps(&fixture), view, "the view is deterministic");
}

/// TC-1422 step 2: final states are the intended terminal states, and the
/// assumptions are the system's FR-148 assumption set.
#[trace("TC-1422", "FR-578-AC-2")]
#[test]
fn the_view_carries_final_states_and_assumptions() {
    let fixture = worked("exactly-once-premise", PICK, CANCELLED);
    let view = steps(&fixture);
    assert!(!view.final_states.is_empty());
    for state in &view.final_states {
        assert!(
            view.steps.iter().all(|step| step.from != *state),
            "a final state has no step"
        );
    }
    let subject = fixture.subject();
    let model = ModelSystem::new(subject.model.simulation.clone()).expect("the model admits");
    let system = AbstractProtocolSystem::new(model, &subject).expect("the system builds");
    assert_eq!(
        view.assumptions,
        system
            .assumption_fairness()
            .expect("the assumptions resolve")
    );
}

/// TC-1422 step 3: a protocol `fsl-eval` steps only up to a bound, or not at
/// all, is returned as its constructs with no steps.
#[trace("TC-1422", "FR-578-AC-3")]
#[test]
fn unsupported_and_dynamic_protocols_have_no_steps() {
    let each = Fixture::activated(
        "on each (t: M::Cell)",
        "exactly-once-premise",
        "",
        &format!("sequence Main {{ {PICK} }}"),
        &[0],
    );
    assert_eq!(
        unsupported(&each),
        vec![UnsteppedConstruct::ActivationOnEach(clause(&each.admitted))]
    );

    let pool = Fixture::built(
        "on origin",
        "exactly-once-premise",
        "role Pool each M::Cell from cells max 2 lifetime workflow;\n",
        "",
        &format!("sequence Main {{ {PICK} }}"),
        &[0],
    );
    assert_eq!(
        unsupported(&pool),
        vec![UnsteppedConstruct::ReplicatedRole(pool.named("Pool"))]
    );

    let text = Fixture::new(
        "exactly-once-premise",
        "sequence Main { send Said via Words as (said: Txt) { true };\n\
         receive Heard via Words of Said as (heard: Txt) { true }; }",
    );
    assert_eq!(
        unsupported(&text),
        vec![UnsteppedConstruct::Construct(
            Unsupported::UnenumerableRecord
        )]
    );
}

/// TC-1422 step 4: a protocol that reaches more states than the limit is
/// refused, never truncated.
#[trace("TC-1422", "FR-578-AC-4")]
#[test]
fn the_state_limit_refuses_the_view() {
    let fixture = worked("exactly-once-premise", PICK, CANCELLED);
    let reached = steps(&fixture).states;
    assert!(matches!(
        view(&fixture, reached - 1),
        Err(StepViewRefusal::StateLimit { max_states }) if max_states == reached - 1
    ));
    assert!(matches!(
        view(&fixture, reached),
        Ok(ProtocolStepView::Steps(_))
    ));
}

/// TC-1423 steps 1 and 2: the refinement witness is the protocol family's
/// FR-351 record, the same record realizability writes for the same node
/// and position, and it reads back from its own bytes.
#[trace("TC-1423", "FR-578-AC-5")]
#[test]
fn the_refinement_witness_is_the_protocol_family_record() {
    let fixture = worked("exactly-once-premise", PICK, "");
    let RealizabilityCounterexample::IndistinguishableHistories {
        history_a,
        history_b,
        obligation,
        separating_witness,
        ..
    } = refuted(fixture.run())
    else {
        panic!("expected indistinguishable histories");
    };
    let position = history_a.steps.len().max(history_b.steps.len()) - 1;
    let root = AssessedRoot::new(Some(assessed_root())).expect("the root is complete");
    let witness = RefinementWitness::new(&root, &obligation, position).expect("the record builds");
    assert!(witness.is(&separating_witness));
    assert_eq!(witness.deciding_node(), Some(obligation.clone()));
    assert_eq!(witness.trace_position(), u64::try_from(position).ok());

    let bytes = witness.to_bytes().expect("the record encodes");
    assert_eq!(
        RefinementWitness::from_bytes(&bytes).expect("the record reads"),
        witness
    );
    let elsewhere = RefinementWitness::new(&root, &obligation, position + 1).expect("builds");
    assert!(!elsewhere.is(&separating_witness));
    let read: serde_json::Value = serde_json::from_slice(&bytes).expect("JSON");
    assert_eq!(
        read["deciding_element"],
        serde_json::json!(node_text(&obligation).expect("the node encodes"))
    );
    assert_eq!(
        read["trace_position"],
        serde_json::json!(position.to_string())
    );
}

/// TC-1423 step 3: the reader refuses a record it would have to repair.
#[trace("TC-1423", "FR-578-AC-6")]
#[test]
fn the_refinement_witness_reader_refuses_repairs() {
    let fixture = worked("exactly-once-premise", PICK, "");
    let root = AssessedRoot::new(Some(assessed_root())).expect("the root is complete");
    let witness =
        RefinementWitness::new(&root, &fixture.named("Done"), 3).expect("the record builds");
    let bytes = witness.to_bytes().expect("the record encodes");
    let text = String::from_utf8(bytes.clone()).expect("JSON is UTF-8");
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).expect("JSON");

    let spaced = text.replacen(':', ": ", 1);
    assert!(matches!(
        RefinementWitness::from_bytes(spaced.as_bytes()),
        Err(RefinementWitnessError::NotCanonical)
    ));

    let padded = text.replace("\"trace_position\":\"3\"", "\"trace_position\":\"03\"");
    assert!(RefinementWitness::from_bytes(padded.as_bytes()).is_err());

    value["index"] = serde_json::json!(0);
    let indexed = serde_json::to_vec(&value).expect("JSON");
    assert!(matches!(
        RefinementWitness::from_bytes(&indexed),
        Err(RefinementWitnessError::Json(_))
    ));

    let mut missing: serde_json::Value = serde_json::from_slice(&bytes).expect("JSON");
    missing
        .as_object_mut()
        .expect("an object")
        .remove("trace_position");
    let missing = serde_json::to_vec(&missing).expect("JSON");
    assert!(RefinementWitness::from_bytes(&missing).is_err());

    let mut renamed: serde_json::Value = serde_json::from_slice(&bytes).expect("JSON");
    renamed["deciding_element"] = serde_json::json!("Done");
    let renamed = serde_json::to_vec(&renamed).expect("JSON");
    assert!(matches!(
        RefinementWitness::from_bytes(&renamed),
        Err(RefinementWitnessError::Shape(_))
    ));
}

/// TC-1422 step 5: a reached state with an undetermined finding, a state a
/// method bound cut, and a step whose control shares its name are refused.
#[trace("TC-1422", "FR-578-AC-7")]
#[test]
fn undetermined_cut_and_unnamed_steps_refuse_the_view() {
    // The attempt's binder constraint divides by zero at `c.v = 2`.
    let body = fill::contracts("self.v = 0")
        + &fill::protocol(
            "P",
            "",
            &format!(
                "sequence Main {{ {} }}",
                fill::attempt("X", "flag", "FlagPost", "x", "6 mod (2 - k.v) >= 0")
            ),
            "d",
        );
    let checked = fill::package_with(fill::PROTOCOL_PROFILE, &body);
    let mut documents = fill::Documents::new();
    let initial = vec![documents.start("s0", &[("c", 2)])];
    let package = admitted(&checked, fill::PACKAGE, &fill::document());
    let subject = fill::subject(&checked, &documents, initial, "P", "c");
    let undefined = protocol_step_view(&package, &clause(&package), &subject, LIMIT);
    assert!(
        matches!(undefined, Err(StepViewRefusal::Undetermined(StateIndex(0)))),
        "{undefined:?}"
    );

    // Five relaxed stores in one thread overflow the default store buffer
    // of four under `tso`.
    let (case, package) = litmus(5);
    let tso = [select("SB::Both", MemoryModelName::Tso)];
    let cut = protocol_step_view(&package, &clause(&package), &case.subject(&tso), LIMIT);
    assert!(matches!(cut, Err(StepViewRefusal::Cut(_))), "{cut:?}");

    let twin = worked(
        "exactly-once-premise",
        "send Twin via Pick as (p: PickMsg) { true };\n\
         receive Rp via Pick of Twin as (q: PickMsg) { true };",
        "send Twin via Cancelled as (c: CancelMsg) { true };\n\
         receive Rc via Cancelled of Twin as (d: CancelMsg) { true };",
    );
    let unnamed = view(&twin, LIMIT);
    assert!(
        matches!(&unnamed, Err(StepViewRefusal::UnnamedNode(path)) if path.ends_with("Twin")),
        "{unnamed:?}"
    );
}
