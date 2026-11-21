// SPDX-License-Identifier: AGPL-3.0-or-later
//! State, terminal and claim entries over the same admitted counter states.

use super::*;
use filament_eval::FamilyOutcome;
use filament_exact::{ChargePoint, Meter, Outcome as ValueOutcome};
use fsl_eval::value::{
    evaluate_state_predicate, evaluate_terminal_predicate, CheckedPackageEvaluation, QualifiedName,
};
use fsl_semantics::check::TerminalDeclaration;
use fsl_semantics::model::observation::{AdmittedObservations, SelfKind};

#[trace("FR-406", "FR-107")]
#[test]
fn clause_terminal_and_claim_entries_read_the_actual_current_state() {
    let mut world = world(
        vec![increment()],
        "\
        terminal Done using v on M when { self.value = 1 };\n\
        possible One using v over (c: M::Counter) { c.value = 1 }\n\
        invariant OneNow using v on M::Counter at current { self.value = 1 }\n",
    );
    for value in [0, 1] {
        let snapshot = world.start(&format!("value-{value}"), &[Obj("c1", value, 0, None)]);
        let system = system(&world, vec![snapshot], &["c1"], UNLIMITED);
        let initial = system.initial();
        let [initial] = initial.as_slice() else {
            panic!("one admitted initial state");
        };
        let current = world
            .model()
            .observation(
                initial.state(),
                DocumentRef {
                    authority: "test".to_owned(),
                    identity: format!("current-{value}"),
                },
            )
            .expect("the actual checked model admits the current observation");
        let reference = world.reference("c1");
        let TerminalDeclaration::When(terminal) = world
            .package
            .graph()
            .terminal("M")
            .expect("the selected model has its terminal declaration")
            .declaration()
        else {
            panic!("the authored terminal predicate is retained");
        };
        let (_, target) = world
            .package
            .graph()
            .state_graph_claim("One")
            .expect("the authored claim checks")
            .node_predicates();
        let predicate = target
            .predicate_body()
            .expect("the target has a checked body");
        let mut terminal_meter = Meter::new(UNLIMITED);
        let terminal = evaluate_terminal_predicate(
            &world.package,
            terminal,
            &current.environment,
            Some(reference.clone()),
            &mut terminal_meter,
        )
        .expect("the real terminal entry runs");
        let mut claim_meter = Meter::new(UNLIMITED);
        let claim = evaluate_state_predicate(
            &world.package,
            predicate,
            &current.environment,
            Some(reference.clone()),
            &mut claim_meter,
        )
        .expect("the real state-predicate entry runs");
        let observations = AdmittedObservations {
            usage: Default::default(),
            clause: world
                .package
                .graph()
                .state_clause("OneNow")
                .unwrap()
                .identity(),
            current: Some(current),
            pre: None,
            post: None,
            self_object: reference,
            self_population: None,
            invocation: None,
            self_kind: SelfKind::Object,
            receiver_pre: None,
            receiver_post: None,
            parameters: Vec::new(),
            result: None,
            created: Vec::new(),
            deleted: Vec::new(),
        };
        let mut clause_meter = Meter::new(UNLIMITED);
        let clause = world
            .package
            .evaluate_clause(
                &QualifiedName::unqualified("OneNow").unwrap(),
                &observations,
                &mut clause_meter,
            )
            .expect("the actual state-clause entry runs");
        assert_eq!(
            clause.reads().len(),
            1,
            "the single field read has one trail entry"
        );
        let path = clause.reads()[0]
            .path
            .as_ref()
            .expect("the field has its actual object path");
        assert_eq!(path.observation.authority, "test");
        assert_eq!(path.observation.identity, format!("current-{value}"));
        assert_eq!(
            path.subject,
            fsl_eval::value::ValuePathSubject::Object(world.reference("c1"))
        );
        assert_eq!(
            path.steps,
            [fsl_eval::value::ValuePathStep::Member("value".to_owned())]
        );
        for result in [terminal, claim, clause.evaluation] {
            let FamilyOutcome::Evaluated(ValueOutcome::Completed(Value::Boolean(actual))) =
                result.outcome
            else {
                panic!("the predicate must complete: {:?}", result.outcome);
            };
            assert_eq!(actual, value == 1, "current counter value {value}");
        }
        for meter in [&terminal_meter, &claim_meter, &clause_meter] {
            assert!(!meter.charge_log_truncated());
            assert!(
                meter
                    .admitted_charges()
                    .iter()
                    .any(|c| c.point == ChargePoint::ModelNavigate),
                "the actual field read must be metered"
            );
        }
        // Route parity is distinct from independent-evaluator comparison.
        assert_eq!(
            terminal_meter.admitted_charges(),
            claim_meter.admitted_charges()
        );
        assert_eq!(
            terminal_meter.admitted_charges(),
            clause_meter.admitted_charges()
        );
    }
}

#[trace("FR-406")]
#[test]
fn rewards_run_the_checked_arithmetic_and_charge_the_kernel_operations() {
    let world = world(
        vec![increment()],
        "reward cost on M::Counter::increment = 2 + 3;",
    );
    let [reward] = world.package.graph().probabilistic_model().rewards() else {
        panic!("one actual checked reward");
    };
    let mut meter = Meter::new(UNLIMITED);
    let evaluation =
        fsl_eval::value::evaluate_reward(&world.package, reward, Vec::new(), &mut meter)
            .expect("the real checked reward runs");
    let FamilyOutcome::Evaluated(ValueOutcome::Completed(Value::Integer(actual))) =
        evaluation.outcome
    else {
        panic!("the reward must complete: {:?}", evaluation.outcome);
    };
    assert_eq!(actual, filament_exact::Integer::from(5_i64));
    let mut kernel_meter = Meter::new(UNLIMITED);
    let two = filament_exact::Integer::from(2_i64);
    let three = filament_exact::Integer::from(3_i64);
    let expected = filament_exact::evaluate_integer_arithmetic(
        filament_exact::IntegerArithmetic::Add(&two, &three),
        None,
        &mut kernel_meter,
    );
    let ValueOutcome::Completed(expected) = expected else {
        panic!("the direct kernel add completes");
    };
    assert_eq!(actual, expected);
    assert!(!meter.charge_log_truncated());
    assert!(!kernel_meter.admitted_charges().is_empty());
    assert_eq!(meter.admitted_charges(), kernel_meter.admitted_charges());
}
