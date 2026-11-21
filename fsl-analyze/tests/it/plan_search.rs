// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-1978: the plan search engine (FR-755, FR-1011, FR-1012) over FSpec
//! FR-650's published counter fixture: model `M`, `Counter.value` in
//! `Int[0, 20]`, `inc` then `double`, one counter `c` with `value = 1`.
//!
//! The goal, objective and policy rules are written here as the checked
//! expressions of the fixture's unit would evaluate them; the search,
//! certificates, checker and replay are the engine's.

use std::collections::BTreeMap;

use filament_exact::{Cancel, CancelCause, Integer, Rational, ScalarLimits};
use filament_semantic_value::model_transition::ModelTransition;
use fsl_analyze::plan_search::{
    search, CertificateKind, CertificateOutcome, CertificateRule, Direction, MonitorStep,
    ObjectiveKind, PlanFault, PlanLengths, PlanLimit, PlanResult, PlanRules, PlanSearchError,
    PlanSearchLimits, PlanSearchRequest, PlanStatus, PlanStep, PlanStopCause, PlanView, RuleStop,
};
use fsl_eval::simulation::model::{ModelSimState, ModelSystem};
use fsl_eval::simulation::TransitionSystem;
use ix_trace_rs::trace;
use serde_json::json;

use crate::counter::UNLIMITED;
use crate::model_world::{self, Domain, World};

const FIXTURE: &str = "proposals/planning-v1/fixtures/counter/";

/// The fixture unit's contracts on `inc` and `double`.
const CONTRACTS: &str = "pre CanInc using v on M::Counter::inc { self.value < 20 }\n\
     post Inc using v on M::Counter::inc { self.value = pre(self.value) + 1 }\n\
     pre CanDouble using v on M::Counter::double { self.value <= 10 }\n\
     post Double using v on M::Counter::double { self.value = 2 * pre(self.value) }\n";

fn counter_world() -> World {
    counter_world_with(1, "")
}

/// The counter at `initial`, its contracts followed by `extra` clauses.
fn counter_world_with(initial: i64, extra: &str) -> World {
    let document =
        filament_specification::published_file(&format!("{FIXTURE}model.semantic-ir.json"))
            .expect("FSpec publishes the counter model")
            .to_vec();
    World::new(
        Domain {
            package: "example/plan-counter",
            object: "Counter",
            population: "counters",
            document,
        },
        &format!("{CONTRACTS}{extra}"),
        model_world::INFINITE,
        &[],
        &[("c", json!({"value": {"integer": initial.to_string()}}))],
    )
}

/// The counter's policy clauses, compiled by hand to their monitors.
#[derive(Clone, Copy)]
enum Policy {
    /// `always holds(c.value != 4)`: a safety monitor.
    NotFour,
    /// `always holds(c.value != 8)`: a safety monitor.
    NotEight,
    /// `eventually holds(c.value = 3)`: a non-safety automaton.
    SomeTime,
    /// `eventually[0, 2] holds(c.value >= 4)`: a bounded safety monitor.
    SoonFour,
}

#[derive(Clone, Copy, PartialEq)]
enum Cost {
    /// The default objective, `length`.
    Length,
    /// `sum<Cost>(s in Reach.steps : if s.operation = M::Op::double then 5 else 1)`.
    DoubleCostsFive,
    /// `maximize Reach.final.value`.
    MaximizeFinal,
    /// `sum<Cost>(s in Reach.steps : if s.operation = M::Op::double then 0 else 1)`.
    FreeDouble,
    /// `maximize sum<Cost>(s in Reach.steps : 1)`: a per-step sum that is
    /// maximized, so it settles with a closure certificate.
    MaximizeLength,
    /// An objective that is undefined for every plan.
    Undefined,
}

struct Counter<'w> {
    world: &'w World,
    names: BTreeMap<String, String>,
    goal: i64,
    /// The actual field declared by this fixture's model.
    value_field: &'static str,
    cost: Cost,
    policies: Vec<Policy>,
    /// An evaluation meter of `work_units` charged once per goal evaluation.
    work_units: Option<u64>,
    used: u64,
    /// Cancelled (requested) at the goal evaluation after one held.
    trip: Option<Cancel>,
    found: bool,
    native: Option<std::sync::Arc<[(filament_exact::EffectiveId, String)]>>,
    /// Observe an actual retained payload at the library adapter boundary.
    on_native_payload: Option<Box<dyn FnMut(&PlanStep)>>,
}

impl<'w> Counter<'w> {
    fn new(world: &'w World, system: &ModelSystem<'_>, goal: i64) -> Self {
        let names = system
            .operations()
            .into_iter()
            .map(|(_, name, text)| (text.to_owned(), name.to_owned()))
            .collect();
        Self {
            world,
            names,
            goal,
            value_field: "value",
            cost: Cost::Length,
            policies: Vec::new(),
            work_units: None,
            used: 0,
            trip: None,
            found: false,
            native: None,
            on_native_payload: None,
        }
    }

    fn value(&self, state: &ModelSimState) -> i64 {
        self.world.domain.integer(state, "c", self.value_field)
    }

    fn operation(&self, step: &ModelTransition) -> &str {
        self.names.get(step.operation()).map_or("", String::as_str)
    }
}

fn integer(value: i64) -> Rational {
    Rational::from_integer(Integer::from(value))
}

impl PlanRules for Counter<'_> {
    type StepKey = (u8, Vec<String>);

    fn native_operations(&self) -> Option<std::sync::Arc<[(filament_exact::EffectiveId, String)]>> {
        self.native.clone()
    }

    fn direction(&self) -> Direction {
        match self.cost {
            Cost::MaximizeFinal | Cost::MaximizeLength => Direction::Maximize,
            Cost::Length | Cost::DoubleCostsFive | Cost::FreeDouble | Cost::Undefined => {
                Direction::Minimize
            }
        }
    }

    fn objective_kind(&self) -> ObjectiveKind {
        match self.cost {
            Cost::MaximizeFinal | Cost::Undefined => ObjectiveKind::General,
            Cost::Length | Cost::DoubleCostsFive | Cost::FreeDouble | Cost::MaximizeLength => {
                ObjectiveKind::StepSum
            }
        }
    }

    fn merges_prefixes(&self) -> bool {
        true
    }

    /// FSpec FR-804: `inc` is declared before `double`.
    fn step_key(
        &mut self,
        step: &fsl_analyze::plan_search::PlanStep,
    ) -> Result<Self::StepKey, RuleStop> {
        if let Some(observe) = &mut self.on_native_payload {
            observe(step);
        }
        let order = match self.operation(step) {
            "inc" => 0,
            "double" => 1,
            _ => 2,
        };
        Ok((order, step.arguments().to_vec()))
    }

    fn policy_count(&self) -> usize {
        self.policies.len()
    }

    fn policy_start(
        &mut self,
        policy: usize,
        state: &ModelSimState,
    ) -> Result<MonitorStep, RuleStop> {
        self.policy_advance(policy, &[0], state)
    }

    fn policy_advance(
        &mut self,
        policy: usize,
        monitor: &[u8],
        state: &ModelSimState,
    ) -> Result<MonitorStep, RuleStop> {
        let value = self.value(state);
        let seen = monitor.first().copied().unwrap_or(0);
        Ok(match self.policies[policy] {
            Policy::NotFour if value == 4 => MonitorStep::Reject,
            Policy::NotEight if value == 8 => MonitorStep::Reject,
            Policy::NotFour | Policy::NotEight => MonitorStep::Continue(vec![0]),
            Policy::SomeTime => MonitorStep::Continue(vec![u8::from(seen == 1 || value == 3)]),
            // The monitor counts positions up to the deadline; 255 is "met".
            Policy::SoonFour if seen == 255 || value >= 4 => MonitorStep::Continue(vec![255]),
            Policy::SoonFour if seen >= 2 => MonitorStep::Reject,
            Policy::SoonFour => MonitorStep::Continue(vec![seen + 1]),
        })
    }

    fn policy_accepts(
        &mut self,
        policy: usize,
        monitor: &[u8],
        last: &ModelSimState,
    ) -> Result<bool, RuleStop> {
        let seen = monitor.first().copied().unwrap_or(0);
        Ok(match self.policies[policy] {
            Policy::NotFour | Policy::NotEight => true,
            Policy::SomeTime => seen == 1,
            // The terminal stutter repeats `last` up to the deadline.
            Policy::SoonFour => seen == 255 || self.value(last) >= 4,
        })
    }

    fn goal(&mut self, plan: &PlanView<'_>) -> Result<bool, RuleStop> {
        if let Some(bound) = self.work_units {
            if self.used >= bound {
                return Err(RuleStop::Accounting {
                    counter: "work_units",
                    value: bound,
                });
            }
            self.used += 1;
        }
        let holds = plan
            .states
            .last()
            .is_some_and(|state| self.value(state) == self.goal);
        // Cancel on the evaluation after the first plan was found.
        if self.found {
            if let Some(cancel) = &self.trip {
                cancel.cancel(CancelCause::Requested);
            }
        }
        self.found |= holds;
        Ok(holds)
    }

    fn step_cost(
        &mut self,
        plan: &PlanView<'_>,
        index: usize,
    ) -> Result<Option<Rational>, RuleStop> {
        Ok(Some(match self.cost {
            Cost::DoubleCostsFive if self.operation(&plan.steps[index]) == "double" => integer(5),
            Cost::FreeDouble if self.operation(&plan.steps[index]) == "double" => integer(0),
            _ => integer(1),
        }))
    }

    fn objective(&mut self, plan: &PlanView<'_>) -> Result<Option<Rational>, RuleStop> {
        if self.cost == Cost::Undefined {
            return Ok(None);
        }
        Ok(plan.states.last().map(|state| integer(self.value(state))))
    }

    fn begin_replay(&mut self) {
        // The replay runs under its own allowance.
        self.used = 0;
    }
}

fn run(
    rules: &mut Counter<'_>,
    system: &ModelSystem<'_>,
    lengths: (u64, u64),
    limits: PlanSearchLimits,
    fault: Option<&PlanFault>,
) -> Result<PlanResult, PlanSearchError> {
    run_with(rules, system, lengths, limits, fault, &Cancel::new())
}

fn run_with(
    rules: &mut Counter<'_>,
    system: &ModelSystem<'_>,
    lengths: (u64, u64),
    limits: PlanSearchLimits,
    fault: Option<&PlanFault>,
    cancel: &Cancel,
) -> Result<PlanResult, PlanSearchError> {
    search(
        system,
        rules,
        PlanSearchRequest {
            lengths: PlanLengths::new(lengths.0, lengths.1).expect("a length range"),
            limits,
            cancel,
            fault,
        },
    )
}

fn operations(rules: &Counter<'_>, result: &PlanResult) -> Vec<String> {
    result
        .plan
        .iter()
        .flat_map(|plan| plan.steps.iter())
        .map(|step| rules.operation(step).to_owned())
        .collect()
}

fn limited(limit: PlanLimit, value: u64) -> PlanSearchLimits {
    let mut limits = PlanSearchLimits::default();
    limits.set(limit, value.try_into().expect("a positive limit"));
    limits
}

/// FSpec FR-650-AC-1, FR-804-AC-1 and FR-755-AC-1: `length` gives `optimal`
/// `inc, double, double` at 3, the first of the two 3-step plans, with a
/// `potential` certificate the core accepts.
#[test]
#[trace("TC-2105", "FR-755-AC-1")]
#[trace("TC-2862", "FR-1012-AC-2")]
fn the_counter_plans_by_length_with_a_potential_certificate() {
    let world = counter_world();
    let system = ModelSystem::new(world.subject(&["c"], UNLIMITED).simulation).expect("admits");
    let mut rules = Counter::new(&world, &system, 8);
    let result = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Optimal);
    assert_eq!(operations(&rules, &result), ["inc", "double", "double"]);
    assert_eq!(result.objective, Some(integer(3)));
    let certificate = result.certificate.expect("a certificate");
    assert_eq!(certificate.kind, CertificateKind::Potential);
    assert_eq!(certificate.outcome, CertificateOutcome::Accepted);
    assert!(result.certified);
    let plan = result.plan.expect("a plan");
    assert_eq!(plan.states.len(), 4);
    let values: Vec<i64> = plan.states.iter().map(|state| rules.value(state)).collect();
    assert_eq!(values, [1, 2, 4, 8]);
    // FSpec FR-804-AC-4: the same request gives the same counts and plan.
    let again = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    assert_eq!(again.explored, result.explored);
    assert_eq!(operations(&rules, &again), ["inc", "double", "double"]);
}

/// FSpec FR-650-AC-2 and FR-755-AC-1: the `double`-costs-five sum gives
/// `inc, inc, inc, double` at 8.
#[test]
#[trace("TC-2105", "FR-755-AC-1")]
fn the_counter_plans_by_step_cost() {
    let world = counter_world();
    let system = ModelSystem::new(world.subject(&["c"], UNLIMITED).simulation).expect("admits");
    let mut rules = Counter::new(&world, &system, 8);
    rules.cost = Cost::DoubleCostsFive;
    let result = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Optimal);
    assert_eq!(operations(&rules, &result), ["inc", "inc", "inc", "double"]);
    assert_eq!(result.objective, Some(integer(8)));
    let certificate = result.certificate.expect("a certificate");
    assert_eq!(certificate.kind, CertificateKind::Potential);
    assert!(result.certified);
}

/// FSpec FR-650-AC-3, FR-755-AC-2: goal 21 and `domain Int[0, 2]` are each
/// `infeasible` with an accepted closure certificate.
#[test]
#[trace("TC-2105", "FR-755-AC-2")]
fn unreachable_goals_are_infeasible_by_a_closure_certificate() {
    let world = counter_world();
    let system = ModelSystem::new(world.subject(&["c"], UNLIMITED).simulation).expect("admits");
    for (goal, lengths) in [(21, (0, 5)), (8, (0, 2))] {
        let mut rules = Counter::new(&world, &system, goal);
        let result = run(
            &mut rules,
            &system,
            lengths,
            PlanSearchLimits::default(),
            None,
        )
        .unwrap();
        assert_eq!(result.status, PlanStatus::Infeasible);
        assert!(result.plan.is_none());
        let certificate = result.certificate.expect("a certificate");
        assert_eq!(certificate.kind, CertificateKind::Closure);
        assert_eq!(certificate.outcome, CertificateOutcome::Accepted);
        assert!(result.certified);
    }
}

/// FSpec FR-650-AC-4, FR-803-AC-1/2/3/5 and FR-755-AC-3: a safety policy
/// prunes, a non-safety policy is decided on each completed plan, a bounded
/// one prunes at its deadline, and a policy every goal plan breaks is
/// `infeasible`.
#[test]
#[trace("TC-2105", "FR-755-AC-3")]
#[trace("TC-2862", "FR-1012-AC-1")]
fn plans_are_held_to_their_policies() {
    let world = counter_world();
    let system = ModelSystem::new(world.subject(&["c"], UNLIMITED).simulation).expect("admits");
    let plan = |policy: Policy| {
        let mut rules = Counter::new(&world, &system, 8);
        rules.policies = vec![policy];
        let result = run(
            &mut rules,
            &system,
            (0, 5),
            PlanSearchLimits::default(),
            None,
        )
        .unwrap();
        let values: Vec<i64> = result
            .plan
            .iter()
            .flat_map(|plan| plan.states.iter())
            .map(|state| rules.value(state))
            .collect();
        (
            result.status,
            operations(&rules, &result),
            values,
            result.certified,
        )
    };
    let (status, _, values, certified) = plan(Policy::NotFour);
    assert_eq!(status, PlanStatus::Optimal);
    assert!(certified);
    assert!(!values.contains(&4), "{values:?}");
    let (status, steps, values, _) = plan(Policy::SomeTime);
    assert_eq!(status, PlanStatus::Optimal);
    assert_eq!(steps, ["inc", "inc", "inc", "double"]);
    assert!(values.contains(&3));
    let (status, steps, _, _) = plan(Policy::SoonFour);
    assert_eq!(status, PlanStatus::Optimal);
    assert_eq!(steps, ["inc", "double", "double"]);
    let (status, _, _, certified) = plan(Policy::NotEight);
    assert_eq!(status, PlanStatus::Infeasible);
    assert!(certified);
}

/// FSpec FR-801-AC-8 and FR-1012-AC-6: the canonical exploration pins the
/// counts of a stopped search, with no plan.
#[test]
#[trace("TC-2862", "FR-1012-AC-6")]
#[trace("TC-2861", "FR-1011-AC-1")]
fn the_canonical_exploration_pins_stopped_counts() {
    let world = counter_world();
    let system = ModelSystem::new(world.subject(&["c"], UNLIMITED).simulation).expect("admits");
    for (limit, value, states, work) in [
        (PlanLimit::MaxStates, 2, 2, 6),
        (PlanLimit::MaxWork, 1, 1, 1),
    ] {
        let mut rules = Counter::new(&world, &system, 8);
        let result = run(&mut rules, &system, (0, 5), limited(limit, value), None).unwrap();
        assert_eq!(result.status, PlanStatus::Incomplete);
        assert_eq!(
            (result.explored.states, result.explored.work),
            (states, work)
        );
        assert!(result.plan.is_none());
        assert_eq!(
            result.cause,
            Some(PlanStopCause::LimitReached {
                limit: limit.member(),
                value,
                setting: limit.setting().to_owned(),
            })
        );
    }
}

/// FSpec FR-801-AC-2 and FR-1011-AC-2: a denied evaluation-meter charge
/// names its `accounting` setting, not a `plan_search` one.
#[test]
#[trace("TC-2861", "FR-1011-AC-2")]
fn an_exhausted_evaluation_meter_names_its_accounting_setting() {
    let world = counter_world();
    let system = ModelSystem::new(world.subject(&["c"], UNLIMITED).simulation).expect("admits");
    let mut rules = Counter::new(&world, &system, 8);
    rules.work_units = Some(1);
    let result = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Incomplete);
    let Some(PlanStopCause::LimitReached { setting, .. }) = result.cause else {
        panic!("a limit stop: {:?}", result.cause);
    };
    assert_eq!(setting, "accounting.work_units");
}

/// FSpec FR-650-AC-5/AC-7 and FR-755-AC-2: the certificate tamper seam makes
/// the core reject `closure` at entry 0; a rejected potential demotes the
/// plan to `feasible`, and a rejected closure for `infeasible` leaves the
/// run `incomplete` with no plan.
#[test]
#[trace("TC-2105", "FR-755-AC-2")]
fn a_tampered_certificate_is_rejected_by_the_core() {
    let world = counter_world();
    let system = ModelSystem::new(world.subject(&["c"], UNLIMITED).simulation).expect("admits");
    let rejected = CertificateOutcome::Rejected {
        rule: CertificateRule::Closure,
        entry: 0,
    };
    let mut rules = Counter::new(&world, &system, 8);
    let fault = PlanFault::DropSuccessor {
        entry: 0,
        successor: 1,
    };
    let result = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        Some(&fault),
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Feasible);
    assert_eq!(operations(&rules, &result), ["inc", "double", "double"]);
    assert_eq!(result.certificate.expect("a certificate").outcome, rejected);
    assert!(!result.certified);

    let mut rules = Counter::new(&world, &system, 21);
    let fault = PlanFault::DropSuccessor {
        entry: 0,
        successor: 0,
    };
    let result = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        Some(&fault),
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Incomplete);
    assert!(result.plan.is_none());
    assert_eq!(
        result.cause,
        Some(PlanStopCause::CertificateRejected {
            rule: CertificateRule::Closure,
            entry: 0,
        })
    );
}

/// FSpec FR-650-AC-6 and FR-755-AC-4: a substituted plan whose fifth
/// `double` is disabled is a method failure naming step 4.
#[test]
#[trace("TC-2105", "FR-755-AC-4")]
fn a_substituted_plan_that_fails_replay_is_a_method_failure() {
    let world = counter_world();
    let system = ModelSystem::new(world.subject(&["c"], UNLIMITED).simulation).expect("admits");
    let mut rules = Counter::new(&world, &system, 8);
    let fault = PlanFault::ReportPlan(vec!["double".to_owned(); 5]);
    let error = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        Some(&fault),
    )
    .expect_err("replay fails");
    assert_eq!(
        error,
        PlanSearchError::ReplayFailed {
            step: 4,
            operation: "double".to_owned(),
        }
    );
}

/// FSpec FR-800-AC-3 and FR-802-AC-6: a goal true at the initial state is
/// the empty plan; `maximize Reach.final.value` is general and settles with
/// a `closure` certificate.
#[test]
#[trace("TC-2862", "FR-1012-AC-2")]
fn the_empty_plan_and_a_general_objective() {
    let world = counter_world();
    let system = ModelSystem::new(world.subject(&["c"], UNLIMITED).simulation).expect("admits");
    let mut rules = Counter::new(&world, &system, 1);
    let result = run(
        &mut rules,
        &system,
        (0, 3),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Optimal);
    assert!(result
        .plan
        .as_ref()
        .is_some_and(|plan| plan.steps.is_empty()));
    assert_eq!(result.objective, Some(integer(0)));

    let mut rules = Counter::new(&world, &system, 8);
    rules.cost = Cost::MaximizeFinal;
    let result = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Optimal);
    assert_eq!(result.objective, Some(integer(8)));
    let certificate = result.certificate.expect("a certificate");
    assert_eq!(certificate.kind, CertificateKind::Closure);
    assert_eq!(certificate.outcome, CertificateOutcome::Accepted);
}

fn world_system(world: &World) -> ModelSystem<'_> {
    ModelSystem::new(world.subject(&["c"], UNLIMITED).simulation).expect("admits")
}

/// FSpec FR-650 "admissible": an invariant that excludes the otherwise-best
/// plan changes the answer, and an invariant is evaluated at the states at
/// the maximum plan length too.
#[test]
#[trace("TC-2105", "FR-755-AC-1")]
fn a_violated_invariant_makes_a_state_inadmissible() {
    let world = counter_world_with(
        1,
        "invariant NotFour using v on M::Counter at current { self.value != 4 }\n",
    );
    let system = world_system(&world);
    let mut rules = Counter::new(&world, &system, 8);
    let result = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    // `inc, double, double` and `inc, inc, inc, double` pass through 4.
    assert_eq!(result.status, PlanStatus::Optimal);
    assert_eq!(
        operations(&rules, &result),
        ["inc", "inc", "double", "inc", "inc"]
    );
    assert_eq!(result.objective, Some(integer(5)));
    assert!(result.certified);

    // The same invariant at the maximum length: no plan of at most 3 steps.
    let world = counter_world_with(
        1,
        "invariant NotEight using v on M::Counter at current { self.value != 8 }\n",
    );
    let system = world_system(&world);
    let mut rules = Counter::new(&world, &system, 8);
    let result = run(
        &mut rules,
        &system,
        (0, 3),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Infeasible);
    assert!(result.certified);
}

/// FSpec FR-650: replay rejects a plan through a state that breaks an
/// invariant, which the substitution seam can build.
#[test]
#[trace("TC-2105", "FR-755-AC-4")]
fn replay_rejects_a_plan_through_a_violated_invariant() {
    let world = counter_world();
    let system = world_system(&world);
    let mut rules = Counter::new(&world, &system, 8);
    // Without the invariant the plan is a good plan; with it, it is not.
    let plain = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    assert_eq!(plain.status, PlanStatus::Optimal);
    let world = counter_world_with(
        1,
        "invariant NotFour using v on M::Counter at current { self.value != 4 }\n",
    );
    let system = world_system(&world);
    let mut rules = Counter::new(&world, &system, 8);
    let fault = PlanFault::ReportPlan(vec![
        "inc".to_owned(),
        "double".to_owned(),
        "double".to_owned(),
    ]);
    let error = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        Some(&fault),
    )
    .expect_err("replay rejects");
    assert!(
        matches!(error, PlanSearchError::ReplayRejected(_)),
        "{error:?}"
    );
}

/// FSpec FR-801-AC-2/AC-3: a stop by the evaluation meter or a cancellation
/// is `incomplete` carrying the best plan found, never a method failure.
#[test]
#[trace("TC-1951", "FR-801-AC-3")]
fn a_stopped_search_still_reports_its_best_plan() {
    let world = counter_world();
    let system = world_system(&world);
    let mut rules = Counter::new(&world, &system, 2);
    rules.work_units = Some(2);
    let result = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Incomplete);
    assert_eq!(operations(&rules, &result), ["inc"]);
    assert!(matches!(
        result.cause,
        Some(PlanStopCause::LimitReached { .. })
    ));

    let cancel = Cancel::new();
    let mut rules = Counter::new(&world, &system, 2);
    rules.trip = Some(cancel.clone());
    let result = run_with(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        None,
        &cancel,
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Incomplete);
    assert_eq!(operations(&rules, &result), ["inc"]);
    assert_eq!(
        result.cause,
        Some(PlanStopCause::Cancelled(CancelCause::Requested))
    );
}

/// FSpec FR-802: when every goal plan has an undefined objective, the space
/// holds no admissible plan and the closure certificate is accepted.
#[test]
#[trace("TC-2105", "FR-755-AC-2")]
fn an_undefined_objective_everywhere_is_infeasible() {
    let world = counter_world();
    let system = world_system(&world);
    let mut rules = Counter::new(&world, &system, 8);
    rules.cost = Cost::Undefined;
    let result = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Infeasible);
    assert!(result.certified);
    assert!(result.explored.undefined_objectives > 0);
}

/// FSpec FR-650's potential `goal` rule is `pi <= 0` at goal nodes only in
/// one direction: a zero-cost step into the goal from a non-goal node is
/// still `optimal`.
#[test]
#[trace("TC-2105", "FR-755-AC-1")]
fn a_zero_cost_step_into_the_goal_is_still_optimal() {
    let world = counter_world();
    let system = world_system(&world);
    let mut rules = Counter::new(&world, &system, 8);
    rules.cost = Cost::FreeDouble;
    let result = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Optimal);
    assert_eq!(operations(&rules, &result), ["double", "double", "double"]);
    assert_eq!(result.objective, Some(integer(0)));
    let certificate = result.certificate.expect("a certificate");
    assert_eq!(certificate.kind, CertificateKind::Potential);
    assert_eq!(certificate.outcome, CertificateOutcome::Accepted);
}

/// FSpec FR-650: a per-step sum settled by a closure certificate has its
/// improvement rule checked against the checker's own cost sum.
#[test]
#[trace("TC-2105", "FR-755-AC-1")]
fn a_maximized_step_sum_settles_by_closure() {
    let world = counter_world();
    let system = world_system(&world);
    let mut rules = Counter::new(&world, &system, 8);
    rules.cost = Cost::MaximizeLength;
    let result = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Optimal);
    assert_eq!(result.objective, Some(integer(5)));
    let certificate = result.certificate.expect("a certificate");
    assert_eq!(certificate.kind, CertificateKind::Closure);
    assert_eq!(certificate.outcome, CertificateOutcome::Accepted);
}

/// FSpec FR-650: a forged entry state, and a parent record that points at
/// the entry itself, are each rejected at that entry (and terminate).
#[test]
#[trace("TC-2105", "FR-755-AC-2")]
fn a_forged_state_and_a_parent_cycle_are_rejected() {
    let world = counter_world();
    let system = world_system(&world);
    for fault in [
        PlanFault::ForgeEntryState {
            entry: 1,
            source: 0,
        },
        PlanFault::ParentCycle { entry: 1 },
    ] {
        let mut rules = Counter::new(&world, &system, 8);
        let result = run(
            &mut rules,
            &system,
            (0, 5),
            PlanSearchLimits::default(),
            Some(&fault),
        )
        .unwrap();
        assert_eq!(result.status, PlanStatus::Feasible, "{fault:?}");
        assert_eq!(
            result.certificate.expect("a certificate").outcome,
            CertificateOutcome::Rejected {
                rule: CertificateRule::Closure,
                entry: 1,
            },
            "{fault:?}"
        );
    }
}

/// FSpec FR-650: replay recomputes the objective, so a substituted plan
/// that is admissible but worse than the certified value is rejected.
#[test]
#[trace("TC-2105", "FR-755-AC-4")]
fn a_substituted_worse_plan_is_rejected() {
    let world = counter_world();
    let system = world_system(&world);
    let mut rules = Counter::new(&world, &system, 8);
    let fault = PlanFault::ReportPlan(["inc", "inc", "inc", "double"].map(str::to_owned).to_vec());
    let error = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        Some(&fault),
    )
    .expect_err("replay rejects");
    assert!(
        matches!(error, PlanSearchError::ReplayRejected(_)),
        "{error:?}"
    );
}

/// FSpec FR-801 "Charge order" step 4: a disabled step is charged one
/// `max_work` and yields no successor. From `value = 11`, `double` is
/// disabled; the run charges the initial admission and goal, `inc` applied,
/// its node admitted and its goal and objective, then the disabled `double`.
#[test]
#[trace("TC-1951", "FR-801-AC-8")]
fn a_disabled_step_is_charged() {
    let world = counter_world_with(11, "");
    let system = world_system(&world);
    let mut rules = Counter::new(&world, &system, 12);
    let result = run(
        &mut rules,
        &system,
        (0, 1),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Optimal);
    assert_eq!(
        (result.explored.states, result.explored.work),
        (2, 5),
        "{:?}",
        result.explored
    );
    // The fifth unit is the disabled `double`: one fewer stops there.
    let mut rules = Counter::new(&world, &system, 12);
    let result = run(
        &mut rules,
        &system,
        (0, 1),
        limited(PlanLimit::MaxWork, 4),
        None,
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Incomplete);
    assert_eq!(
        (result.explored.states, result.explored.work),
        (2, 4),
        "{:?}",
        result.explored
    );
    assert_eq!(operations(&rules, &result), ["inc"]);
}

/// A meter on which the cube invariant below is undetermined from
/// `value = 4` up: its product needs more than 5 integer bits there, while
/// the counter's own operations (results of at most 20) stay within 5.
const SMALL_INTEGERS: ScalarLimits = ScalarLimits {
    integer_bits: 5,
    ..UNLIMITED
};

/// The cube invariant that the `SMALL_INTEGERS` meter cannot decide from
/// `value = 4` up (an evaluation stopped by the meter: FR-120 "Invariants").
const CUBE: &str = "invariant Cube using v on M::Counter at current { self.value * self.value * self.value >= 0 }\n";

fn undecided_system(world: &World) -> ModelSystem<'_> {
    ModelSystem::new(world.subject(&["c"], SMALL_INTEGERS).simulation).expect("admits")
}

/// An undetermined invariant is not a violation: when the only goal path
/// crosses such a state the result is `incomplete` naming the invariant,
/// never `infeasible`.
#[test]
#[trace("TC-2105", "FR-755-AC-2")]
#[trace("TC-2862", "FR-1012-AC-7")]
fn an_undetermined_invariant_on_the_only_path_is_incomplete() {
    let world = counter_world_with(1, CUBE);
    let system = undecided_system(&world);
    let mut rules = Counter::new(&world, &system, 8);
    let result = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Incomplete);
    assert!(!result.certified);
    assert!(result.plan.is_none());
    assert!(matches!(
        &result.cause,
        Some(PlanStopCause::InvariantUndetermined { invariant, .. }) if invariant == "Cube"
    ));
}

/// A found plan plus an undecided prune elsewhere is `incomplete` carrying
/// the plan, never `optimal`; a replay of a plan through the undecided state
/// is not admissible evidence.
#[test]
#[trace("TC-2105", "FR-755-AC-2")]
#[trace("TC-2862", "FR-1012-AC-7")]
fn an_undecided_prune_caps_a_found_plan_at_incomplete() {
    let world = counter_world_with(1, CUBE);
    let system = undecided_system(&world);
    let mut rules = Counter::new(&world, &system, 2);
    let result = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Incomplete);
    assert!(!result.certified);
    assert!(result.certificate.is_none());
    assert_eq!(operations(&rules, &result), ["inc"]);
    assert_eq!(result.objective, Some(integer(1)));
    assert!(matches!(
        &result.cause,
        Some(PlanStopCause::InvariantUndetermined { invariant, .. }) if invariant == "Cube"
    ));

    // `inc, double` crosses the undecided state 4.
    let fault = PlanFault::ReportPlan(["inc", "double"].map(str::to_owned).to_vec());
    let mut rules = Counter::new(&world, &system, 2);
    let result = run(
        &mut rules,
        &system,
        (0, 5),
        PlanSearchLimits::default(),
        Some(&fault),
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Incomplete);
    assert!(result.plan.is_none());
}

/// A source plan with one non-safety policy whose truth needs continuation.
fn source_policy_world(start: &[(&str, serde_json::Value)]) -> World {
    World::new(
        model_world::counter(),
        "type Lengths = Int[0, 0];\n\
         synthesis Reach using t grammar M::Counter domain Lengths satisfies { true };\n",
        model_world::INFINITE,
        &[model_world::Clause {
            activation: "on origin",
            formula: "eventually holds(c.value = 3)",
        }],
        start,
    )
}

/// Policy prefixes, separate policies and terminal reads all use the
/// request meter; a successful exact-bound first read leaves no second budget.
#[test]
#[trace("FR-1011-AC-2", "FSpec-FR-801-AC-2")]
fn source_policy_reads_share_the_request_work_counter() {
    use filament_exact::{LimitKind, Meter};
    use fsl_analyze::plan_source::SourcePlan;
    use fsl_eval::simulation::model::{read_clause, ClauseReading};
    use fsl_eval::simulation::TransitionSystem;

    let world = source_policy_world(&[("c", json!({"value": {"integer": "0"}}))]);
    let system = world_system(&world);
    let state = system
        .initial()
        .into_iter()
        .next()
        .expect("one initial state");
    let node = &world.package.graph().temporal_clauses()[0];
    let mut calibration = Meter::new(UNLIMITED);
    assert_eq!(
        read_clause(
            &system,
            node,
            std::slice::from_ref(&state),
            false,
            &mut calibration
        )
        .expect("the policy reads the initial prefix"),
        ClauseReading::Undecided,
    );
    let one_read = calibration.consumed(LimitKind::WorkUnits);
    assert!(one_read > 0, "the policy read must charge work");
    let limits = ScalarLimits {
        work_units: one_read,
        ..UNLIMITED
    };
    let synthesis = &world.package.graph().syntheses()[0];
    for next in ["other-policy", "advance", "terminal"] {
        let mut rules =
            SourcePlan::new(&world.package, synthesis, &system, vec![node, node], limits)
                .expect("the source plan and policies are checked");
        let MonitorStep::Continue(monitor) = rules
            .policy_start(0, &state)
            .expect("the exact-bound first policy read completes")
        else {
            panic!("an unsettled prefix is retained");
        };
        let stopped = match next {
            "other-policy" => rules.policy_start(1, &state).map(|_| ()),
            "advance" => rules.policy_advance(0, &monitor, &state).map(|_| ()),
            "terminal" => rules.policy_accepts(0, &monitor, &state).map(|_| ()),
            _ => unreachable!("the listed read modes are exhaustive"),
        };
        assert!(
            matches!(stopped,
                Err(RuleStop::Accounting { counter: "work_units", value }) if value == one_read
            ),
            "the next policy read uses the exhausted request budget: {stopped:?}"
        );
    }
}

/// A policy quantified over two objects shares one budget across instances.
#[test]
#[trace("FR-1011-AC-2", "FSpec-FR-801-AC-2")]
fn source_policy_instances_share_the_request_work_counter() {
    use filament_exact::{LimitKind, Meter};
    use fsl_eval::simulation::model::{read_clause, ClauseReading};
    use fsl_eval::simulation::TransitionSystem;

    let both = source_policy_world(&[
        ("c", json!({"value": {"integer": "0"}})),
        ("d", json!({"value": {"integer": "0"}})),
    ]);
    let system = ModelSystem::new(both.subject(&["c", "d"], UNLIMITED).simulation)
        .expect("both objects are in the universe");
    let state = system
        .initial()
        .into_iter()
        .next()
        .expect("one initial state");
    let node = &both.package.graph().temporal_clauses()[0];
    let mut calibration = Meter::new(UNLIMITED);
    assert_eq!(
        read_clause(
            &system,
            node,
            std::slice::from_ref(&state),
            false,
            &mut calibration,
        )
        .expect("both instances read with an unlimited budget"),
        ClauseReading::Undecided
    );
    let total_work = calibration.consumed(LimitKind::WorkUnits);
    assert!(
        total_work > 1,
        "two nontrivial policy instances charge work"
    );
    let budget = total_work - 1;
    let mut meter = Meter::new(ScalarLimits {
        work_units: budget,
        ..UNLIMITED
    });
    assert_eq!(
        read_clause(
            &system,
            &both.package.graph().temporal_clauses()[0],
            std::slice::from_ref(&state),
            false,
            &mut meter,
        )
        .expect("accounting is a typed policy reading"),
        ClauseReading::Incomplete {
            counter: "work_units",
            value: budget
        }
    );
}

/// Closing an unmet eventual policy rejects the candidate; quantifying the
/// same policy over an empty population holds vacuously.
#[test]
#[trace("FSpec-FR-803")]
fn source_policy_acceptance_requires_closed_truth_and_preserves_vacuity() {
    use filament_exact::Meter;
    use fsl_analyze::plan_source::SourcePlan;
    use fsl_eval::simulation::model::{read_clause, ClauseReading};
    use fsl_eval::simulation::TransitionSystem;

    for empty in [false, true] {
        let objects = if empty {
            Vec::new()
        } else {
            vec![("c", json!({"value": {"integer": "0"}}))]
        };
        let world = source_policy_world(&objects);
        let keys = if empty { vec![] } else { vec!["c"] };
        let system = ModelSystem::new(world.subject(&keys, UNLIMITED).simulation)
            .expect("the declared population is complete");
        let state = system
            .initial()
            .into_iter()
            .next()
            .expect("one initial state");
        let node = &world.package.graph().temporal_clauses()[0];
        let mut rules = SourcePlan::new(
            &world.package,
            &world.package.graph().syntheses()[0],
            &system,
            vec![node],
            UNLIMITED,
        )
        .expect("the source plan is checked");
        let MonitorStep::Continue(monitor) = rules
            .policy_start(0, &state)
            .expect("the initial policy prefix reads")
        else {
            panic!("the initial prefix cannot refute eventual truth");
        };
        assert_eq!(
            rules
                .policy_accepts(0, &monitor, &state)
                .expect("the closed policy reads"),
            empty
        );
        assert_eq!(
            read_clause(
                &system,
                node,
                std::slice::from_ref(&state),
                true,
                &mut Meter::new(UNLIMITED),
            )
            .expect("the closed policy is readable"),
            if empty {
                ClauseReading::Holds
            } else {
                ClauseReading::Violated
            }
        );
    }
}

/// A genuine authored native model, retaining its declared operation order.
fn native_plan_world(model: &str, contracts: &str, start: &[(&str, serde_json::Value)]) -> World {
    native_plan_world_with_clauses(model, contracts, &[], start)
}

fn native_plan_world_with_clauses(
    model: &str,
    contracts: &str,
    clauses: &[model_world::Clause<'_>],
    start: &[(&str, serde_json::Value)],
) -> World {
    let source = fsl_cst::diagnostic::read_source(
        fsl_foundation::SourceIdentity::new("test", "native-plan-model"),
        "native-plan-model.fsl",
        model.as_bytes(),
        fsl_cst::Limits::default().source_bytes,
    )
    .expect("the authored native model source admits");
    let packages =
        fsl_semantics::model::intake::package_input_from_sources([("test/native-plan", &source)])
            .expect("the authored model compiles without modifying emitted JSON");
    World::new(
        Domain {
            package: "test/native-plan",
            object: "Counter",
            population: "counters",
            document: packages["test/native-plan"].clone(),
        },
        contracts,
        model_world::INFINITE,
        clauses,
        start,
    )
}

fn finite_argument_world(
    model: &str,
    grammar: &'static str,
    population: &'static str,
    body: &str,
    populations: &[(&str, &str, &[(&str, serde_json::Value)])],
) -> World {
    let source = fsl_cst::diagnostic::read_source(
        fsl_foundation::SourceIdentity::new("test", "finite-arguments-model"),
        "finite-arguments-model.fsl",
        model.as_bytes(),
        fsl_cst::Limits::default().source_bytes,
    )
    .expect("the authored finite model source admits");
    let packages =
        fsl_semantics::model::intake::package_input_from_sources([("test/native-plan", &source)])
            .expect("the authored finite model compiles");
    World::with_populations(
        Domain {
            package: "test/native-plan",
            object: grammar,
            population,
            document: packages["test/native-plan"].clone(),
        },
        body,
        model_world::INFINITE,
        populations,
    )
}

const DRAIN_MODEL: &str = r#"language "ix:native" edition "1-draft";
domain "test/native-plan";
object Server {
  drained: Boolean;
  operation drain(s: Server) modifies self.drained {
    pre Ready { s = self and not self.drained };
    post Drained { self.drained };
  }
}
population servers: Server;
"#;

/// The authored drain example exercises the actual native reference values,
/// source argument fields and independently checked potential certificate.
#[test]
#[trace("FSpec-FR-800-AC-1", "FSpec-FR-804-AC-3")]
fn finite_reference_arguments_drain_two_servers_and_survive_replay() {
    use filament_exact::Value;
    use fsl_analyze::plan_search::replay;
    use fsl_analyze::plan_source::SourcePlan;
    let servers = [
        ("n2", json!({"drained":{"boolean":false}})),
        ("n1", json!({"drained":{"boolean":false}})),
    ];
    let world = finite_argument_world(
        DRAIN_MODEL,
        "Server",
        "servers",
        "type Lengths = Int[0,2]; synthesis Reach using t grammar M::Server domain Lengths satisfies { Reach.length = 2 and forall(s in Reach.final.servers: s.drained) and forall(s in Reach.steps: present(s.arguments.s) and forall(t in Reach.steps: present(t.arguments.s) and (s.index != t.index implies value(s.arguments.s) != value(t.arguments.s)))) };",
        &[("servers", "Server", &servers)],
    );
    let system = ModelSystem::new(
        world
            .subject_populations(&[("servers", &["n1", "n2"])], UNLIMITED)
            .simulation,
    )
    .expect("the real servers admit");
    let mut rules = SourcePlan::new(
        &world.package,
        &world.package.graph().syntheses()[0],
        &system,
        vec![],
        UNLIMITED,
    )
    .expect("the finite source plan admits");
    let result = search(
        &system,
        &mut rules,
        PlanSearchRequest {
            lengths: PlanLengths::new(0, 2).unwrap(),
            limits: PlanSearchLimits::default(),
            cancel: &Cancel::new(),
            fault: None,
        },
    )
    .expect("the native plan is replayed");
    assert_eq!(result.status, PlanStatus::Optimal);
    assert!(result.certified);
    assert_eq!(result.objective, Some(integer(2)));
    let plan = result.plan.expect("two drains reach the goal");
    let mut receivers = Vec::new();
    for step in &plan.steps {
        let (receiver, arguments) = step
            .native_values()
            .expect("the actual runtime values are retained");
        assert!(matches!(arguments, [Value::Reference(argument)] if argument == receiver));
        receivers.push(receiver.object().as_str().to_owned());
    }
    assert_eq!(receivers, ["n1", "n2"]);
    assert_eq!(plan.states.len(), 3);
    rules.begin_replay();
    assert_eq!(
        fsl_analyze::plan_search::replay::replay(
            &system,
            &mut rules,
            &plan,
            PlanLengths::new(0, 2).unwrap(),
            ObjectiveKind::StepSum
        )
        .unwrap(),
        integer(2)
    );

    let mut identities = plan.clone();
    for step in &mut identities.steps {
        *step = PlanStep::new(step.invocation().clone(), step.outcome_ordinal());
    }
    rules.begin_replay();
    assert_eq!(
        replay::replay(
            &system,
            &mut rules,
            &identities,
            PlanLengths::new(0, 2).unwrap(),
            ObjectiveKind::StepSum
        )
        .unwrap(),
        integer(2),
        "replay reconstructs typed values from native invocation identities"
    );
    let forged = ModelTransition::new(
        plan.steps[0].operation().to_owned(),
        plan.steps[0].receiver().map(str::to_owned),
        plan.steps[1].arguments().to_vec(),
        quire_canonical::Limits::new(4096),
    )
    .unwrap();
    let mut inconsistent = plan.clone();
    inconsistent.steps[0] = PlanStep::new(forged, plan.steps[0].outcome_ordinal());
    rules.begin_replay();
    assert!(replay::replay(&system, &mut rules, &inconsistent, PlanLengths::new(0, 2).unwrap(), ObjectiveKind::StepSum).is_err(),
        "another server argument violates drain's receiver-equality contract and the recorded transition");
}

/// Canonical argument order is parameter-major, using the enum's admitted
/// ranks rather than the order of cases in its unordered authored spelling.
#[test]
#[trace("FSpec-FR-804-AC-3", "FSpec-FR-804-AC-4")]
fn finite_reference_and_enum_arguments_follow_typed_product_order() {
    use filament_exact::Value;
    use fsl_analyze::plan_source::SourcePlan;
    use fsl_eval::simulation::model::{
        NativeCandidateControl as Control, NativeCandidateEvent as Event,
    };
    let robots = [("b", json!({})), ("a", json!({}))];
    let world = finite_argument_world(
        "language \"ix:native\" edition \"1-draft\"; domain \"test/native-plan\"; enum Cell { c1, c0 } object Robot { operation move(r: Robot, c: Cell); } population robots: Robot;",
        "Robot",
        "robots",
        "type Lengths = Int[1,1]; synthesis Reach using t grammar M::Robot domain Lengths satisfies { forall(s in Reach.steps: present(s.arguments.r) and present(s.arguments.c) and value(s.arguments.c) = M::Cell::c0) };",
        &[("robots", "Robot", &robots)],
    );
    let system = ModelSystem::new(
        world
            .subject_populations(&[("robots", &["a", "b"])], UNLIMITED)
            .simulation,
    )
    .unwrap();
    let mut rows = Vec::new();
    system
        .visit_plan_candidates(
            &system.initial()[0],
            &native_order(&system, &["move"]),
            None,
            &mut |event| {
                if let Event::Completed {
                    receiver,
                    arguments,
                    ..
                } = event
                {
                    let [Value::Reference(target), Value::Enum(cell)] = arguments else {
                        panic!("native values retain both declared types");
                    };
                    rows.push((
                        receiver.object().as_str().to_owned(),
                        target.object().as_str().to_owned(),
                        cell.rank(),
                    ));
                }
                Control::Continue
            },
        )
        .unwrap();
    assert_eq!(
        rows,
        [
            ("a".to_owned(), "a".to_owned(), 0),
            ("a".to_owned(), "a".to_owned(), 1),
            ("a".to_owned(), "b".to_owned(), 0),
            ("a".to_owned(), "b".to_owned(), 1),
            ("b".to_owned(), "a".to_owned(), 0),
            ("b".to_owned(), "a".to_owned(), 1),
            ("b".to_owned(), "b".to_owned(), 0),
            ("b".to_owned(), "b".to_owned(), 1),
        ]
    );
    for _ in 0..2 {
        let mut rules = SourcePlan::new(
            &world.package,
            &world.package.graph().syntheses()[0],
            &system,
            vec![],
            UNLIMITED,
        )
        .unwrap();
        let result = search(
            &system,
            &mut rules,
            PlanSearchRequest {
                lengths: PlanLengths::new(1, 1).unwrap(),
                limits: PlanSearchLimits::default(),
                cancel: &Cancel::new(),
                fault: None,
            },
        )
        .unwrap();
        assert_eq!(result.status, PlanStatus::Optimal);
        assert!(result.certified);
        let step = &result.plan.as_ref().unwrap().steps[0];
        let (receiver, arguments) = step.native_values().unwrap();
        assert_eq!(receiver.object().as_str(), "a");
        assert!(
            matches!(&arguments[0], Value::Reference(reference) if reference.object().as_str() == "a")
        );
        assert!(matches!(&arguments[1], Value::Enum(member) if member.rank() == 0));
    }
}

#[test]
#[trace("FSpec-FR-800-AC-1", "FSpec-FR-804-AC-3")]
fn finite_arguments_keep_mixed_receiver_types_and_source_union_cases() {
    use fsl_analyze::plan_source::SourcePlan;
    let model = DRAIN_MODEL.replace("population servers", "object Router { active: Boolean; operation drain(s: Server) modifies self.active { pre Idle { not self.active }; post Routed { self.active }; } } population routers: Router;\npopulation servers");
    let servers = [
        ("n1", json!({"drained":{"boolean":false}})),
        ("n2", json!({"drained":{"boolean":false}})),
    ];
    let routers = [("r", json!({"active":{"boolean":false}}))];
    let world = finite_argument_world(
        &model,
        "Server",
        "servers",
        "type Lengths = Int[3,3]; synthesis Reach using t grammar M::Server domain Lengths satisfies { forall(s in Reach.final.servers: s.drained) and forall(r in Reach.final.routers: r.active) and forall(s in Reach.steps: s.receiver = s.receiver and present(s.arguments.s)) };",
        &[
            ("routers", "Router", &routers),
            ("servers", "Server", &servers),
        ],
    );
    let system = ModelSystem::new(
        world
            .subject_populations(
                &[("servers", &["n1", "n2"]), ("routers", &["r"])],
                UNLIMITED,
            )
            .simulation,
    )
    .unwrap();
    let synthesis = &world.package.graph().syntheses()[0];
    assert_eq!(synthesis.plan().unwrap().shape().receiver_cases.len(), 2);
    let mut rules = SourcePlan::new(&world.package, synthesis, &system, vec![], UNLIMITED).unwrap();
    let result = search(
        &system,
        &mut rules,
        PlanSearchRequest {
            lengths: PlanLengths::new(3, 3).unwrap(),
            limits: PlanSearchLimits::default(),
            cancel: &Cancel::new(),
            fault: None,
        },
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Optimal);
    assert!(result.certified);
    let plan = result.plan.unwrap();
    let names: Vec<_> = plan
        .steps
        .iter()
        .map(|step| rules.operation_name(step).unwrap())
        .collect();
    assert_eq!(names, ["drain", "drain", "drain"]);
    assert_ne!(
        rules.operation(&plan.steps[0]).unwrap().declaring,
        rules.operation(&plan.steps[2]).unwrap().declaring,
        "the declaring type keeps equal operation basenames distinct"
    );
    let receivers: Vec<_> = plan
        .steps
        .iter()
        .map(|step| step.native_values().unwrap().0.object_type().clone())
        .collect();
    assert_ne!(receivers[0], receivers[2]);
}

/// An empty but admitted target population has a genuinely empty invocation
/// product. Its one-step plan space is infeasible, with no missing argument.
#[test]
#[trace("FSpec-FR-800", "FSpec-FR-806-AC-5")]
fn finite_reference_arguments_over_an_empty_population_have_no_invocations() {
    use fsl_analyze::plan_source::SourcePlan;
    use fsl_eval::simulation::model::{
        NativeCandidateControl as Control, NativeCandidateEvent as Event,
    };
    let robots = [("r", json!({}))];
    let world = finite_argument_world(
        "language \"ix:native\" edition \"1-draft\"; domain \"test/native-plan\"; object Server {} object Robot { operation choose(s: Server); } population servers: Server; population robots: Robot;",
        "Robot", "robots", "bound EmptyServers on M::servers [0]\ntype Lengths = Int[1,1]; synthesis Reach using t grammar M::Robot domain Lengths satisfies { true };",
        &[("servers","Server",&[]),("robots","Robot",&robots)]);
    let system = ModelSystem::new(
        world
            .subject_populations(&[("servers", &[]), ("robots", &["r"])], UNLIMITED)
            .simulation,
    )
    .expect("the complete empty target population admits");
    let mut began = 0;
    system
        .visit_plan_candidates(
            &system.initial()[0],
            &native_order(&system, &["choose"]),
            None,
            &mut |event| {
                if let Event::Begin(_) = event {
                    began += 1;
                }
                Control::Continue
            },
        )
        .unwrap();
    assert_eq!(began, 0);
    let mut rules = SourcePlan::new(
        &world.package,
        &world.package.graph().syntheses()[0],
        &system,
        vec![],
        UNLIMITED,
    )
    .unwrap();
    let result = search(
        &system,
        &mut rules,
        PlanSearchRequest {
            lengths: PlanLengths::new(1, 1).unwrap(),
            limits: PlanSearchLimits::default(),
            cancel: &Cancel::new(),
            fault: None,
        },
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Infeasible);
    assert!(result.certified);
    assert!(result.plan.is_none());
}

/// Runtime conformance preserves most-specific type identity and uses the
/// full typed reference order instead of identity-first canonical JSON.
#[test]
#[trace("FSpec-FR-800", "FSpec-FR-804-AC-3")]
fn finite_reference_arguments_preserve_subtypes_and_canonical_identity_order() {
    use filament_exact::Value;
    use fsl_eval::simulation::model::{
        NativeCandidateControl as Control, NativeCandidateEvent as Event,
    };
    let bases = [("z", json!({}))];
    let children = [("a", json!({}))];
    let foreign = [("q", json!({}))];
    let world = finite_argument_world(
        "language \"ix:native\" edition \"1-draft\"; domain \"test/native-plan\"; object Base { operation select(target: Base); } object Child extends Base {} object Foreign {} population children: Child; population foreign: Foreign; population bases: Base;",
        "Base","bases","", &[("children","Child",&children),("foreign","Foreign",&foreign),("bases","Base",&bases)]);
    let system = ModelSystem::new(
        world
            .subject_populations(
                &[("children", &["a"]), ("foreign", &["q"]), ("bases", &["z"])],
                UNLIMITED,
            )
            .simulation,
    )
    .unwrap();
    let mut values = Vec::new();
    system
        .visit_plan_candidates(
            &system.initial()[0],
            &native_order(&system, &["select"]),
            None,
            &mut |event| {
                if let Event::Completed {
                    receiver,
                    arguments,
                    ..
                } = event
                {
                    let [Value::Reference(target)] = arguments else {
                        panic!("one admitted reference argument");
                    };
                    values.push((receiver.clone(), target.clone()));
                }
                Control::Continue
            },
        )
        .unwrap();
    assert_eq!(
        values.len(),
        4,
        "both conforming receivers and both conforming targets"
    );
    let mut expected: Vec<_> = system
        .invocation_receivers(&system.initial()[0])
        .unwrap()
        .into_iter()
        .map(|(_, reference)| reference)
        .filter(|reference| reference.object().as_str() != "q")
        .collect();
    expected.sort();
    let expected_pairs: Vec<_> = expected
        .iter()
        .flat_map(|receiver| {
            expected
                .iter()
                .map(move |target| (receiver.clone(), target.clone()))
        })
        .collect();
    assert_eq!(values, expected_pairs);
    assert_ne!(expected[0].object_type(), expected[1].object_type());
    let mut lexical = expected.clone();
    lexical.sort_by_key(|reference| {
        fsl_eval::simulation::model::reference_canonical_text(reference).unwrap()
    });
    assert_ne!(
        lexical, expected,
        "the fixture must distinguish JSON identity-first order from typed reference order"
    );
}

/// The later reference domain follows the admitted post-state, while a
/// value captured from an earlier invocation keeps its historical identity.
#[test]
#[trace("FSpec-FR-800")]
fn finite_reference_arguments_exclude_objects_deleted_by_an_earlier_step() {
    use filament_exact::Value;
    use fsl_eval::simulation::model::{
        NativeCandidateControl as Control, NativeCandidateEvent as Event,
    };
    let model = DRAIN_MODEL.replace("population servers", "object Controller { operation remove() deletes Server; operation choose(s: Server); } population controllers: Controller;\npopulation servers");
    let servers = [
        ("n1", json!({"drained":{"boolean":false}})),
        ("n2", json!({"drained":{"boolean":false}})),
    ];
    let controllers = [("c", json!({}))];
    let world = finite_argument_world(
        &model,
        "Controller",
        "controllers",
        "",
        &[
            ("servers", "Server", &servers),
            ("controllers", "Controller", &controllers),
        ],
    );
    let system = ModelSystem::new(
        world
            .subject_populations(
                &[("servers", &["n1", "n2"]), ("controllers", &["c"])],
                UNLIMITED,
            )
            .simulation,
    )
    .unwrap();
    let initial = system.initial()[0].clone();
    let mut historical = None;
    system.visit_plan_candidates(&initial,&native_order(&system,&["choose"]),None,&mut |event| {
        if let Event::Completed { arguments, .. }=event {
            if matches!(&arguments[0],Value::Reference(reference) if reference.object().as_str()=="n2") { historical=Some(arguments[0].clone()); }
        }
        Control::Continue
    }).unwrap();
    let mut removed = None;
    system
        .visit_plan_candidates(
            &initial,
            &native_order(&system, &["remove"]),
            None,
            &mut |event| {
                if let Event::Completed { state, .. } = event {
                    let servers = &state.state().populations[&world.domain.key("servers")];
                    if servers.contains_key("n1") && !servers.contains_key("n2") {
                        removed = Some(state.clone());
                    }
                }
                Control::Continue
            },
        )
        .unwrap();
    let removed = removed.expect("the native delete relation admits removing n2");
    let mut targets = Vec::new();
    system
        .visit_plan_candidates(
            &removed,
            &native_order(&system, &["choose"]),
            None,
            &mut |event| {
                if let Event::Completed { arguments, .. } = event {
                    let [Value::Reference(reference)] = arguments else {
                        panic!("one typed reference");
                    };
                    targets.push(reference.object().as_str().to_owned());
                }
                Control::Continue
            },
        )
        .unwrap();
    assert_eq!(targets, ["n1"]);
    assert!(
        matches!(historical,Some(Value::Reference(reference)) if reference.object().as_str()=="n2")
    );
}

/// The first retained outcome has seven semantic occurrences: the outcome
/// collection, step, receiver, two arguments, population and object.
#[test]
#[trace("FSpec-FR-801-AC-2", "FSpec-FR-804")]
fn finite_argument_retention_denies_the_exact_named_occurrence_bound() {
    use filament_exact::LimitKind;
    use fsl_analyze::plan_source::SourcePlan;
    use fsl_eval::simulation::model::{
        NativeCandidateControl as Control, NativeCandidateEvent as Event,
    };
    use fsl_eval::simulation::ExpansionLimit;
    let robots = [("a", json!({}))];
    let world=finite_argument_world("language \"ix:native\" edition \"1-draft\"; domain \"test/native-plan\"; enum Cell { c0, c1 } object Robot { operation move(r: Robot, c: Cell); } population robots: Robot;",
        "Robot","robots","type Lengths = Int[1,1]; synthesis Reach using t grammar M::Robot domain Lengths satisfies { true };",&[("robots","Robot",&robots)]);
    let system = ModelSystem::new(
        world
            .subject_populations(&[("robots", &["a"])], UNLIMITED)
            .simulation,
    )
    .unwrap();
    let limits = ScalarLimits {
        value_occurrences: 6,
        ..UNLIMITED
    };
    system.share_evaluation_meter(limits);
    let mut began = 0;
    let mut completed = 0;
    let stop = system
        .visit_plan_candidates(
            &system.initial()[0],
            &native_order(&system, &["move"]),
            None,
            &mut |event| {
                match event {
                    Event::Begin(_) => began += 1,
                    Event::Completed { .. } => completed += 1,
                    _ => {}
                }
                Control::Continue
            },
        )
        .expect_err("a six-occurrence budget cannot retain the seven-occurrence outcome");
    assert_eq!((began, completed), (1, 0));
    assert_eq!(
        stop.limit,
        Some(ExpansionLimit::EvaluationMeter(LimitKind::ValueOccurrences))
    );
    let mut rules = SourcePlan::new(
        &world.package,
        &world.package.graph().syntheses()[0],
        &system,
        vec![],
        limits,
    )
    .unwrap();
    let result = search(
        &system,
        &mut rules,
        PlanSearchRequest {
            lengths: PlanLengths::new(1, 1).unwrap(),
            limits: PlanSearchLimits::default(),
            cancel: &Cancel::new(),
            fault: None,
        },
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Incomplete);
    assert!(result.plan.is_none());
    assert!(
        matches!(result.cause,Some(PlanStopCause::LimitReached {ref setting,value:6,..}) if setting=="accounting.value_occurrences")
    );
}

fn finite_payload_world() -> World {
    native_plan_world(
        "language \"ix:native\" edition \"1-draft\"; domain \"test/native-plan\"; type ValueRange = Int[0,0]; enum Cell { c0 } object Counter { number: ValueRange; operation move(r: Counter, c: Cell); } population counters: Counter;",
        "", &[("c",json!({"number":{"integer":"0"}}))],
    )
}

fn finite_payload_rules<'a>(world: &'a World, system: &ModelSystem<'_>) -> Counter<'a> {
    let mut rules = Counter::new(world, system, 0);
    rules.value_field = "number";
    rules.native = Some(native_order(system, &["move"]).into());
    rules
}

fn finite_payload_observer() -> (Cancel, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
    let polls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observed = polls.clone();
    let cancel = Cancel::observing(move || {
        observed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    });
    (cancel, polls)
}

fn finite_payload_cancel_at(
    target: usize,
    cause: CancelCause,
) -> (Cancel, std::sync::Arc<std::sync::Mutex<Option<Cancel>>>) {
    let slot = std::sync::Arc::new(std::sync::Mutex::new(None::<Cancel>));
    let weak = std::sync::Arc::downgrade(&slot);
    let polls = std::sync::atomic::AtomicUsize::new(0);
    let cancel = Cancel::observing(move || {
        if polls.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1 == target {
            if let Some(slot) = weak.upgrade() {
                if let Some(cancel) = slot.lock().unwrap().as_ref() {
                    cancel.cancel(cause);
                }
            }
        }
    });
    *slot.lock().unwrap() = Some(cancel.clone());
    (cancel, slot)
}

/// Applied retention is seven result units; the separate retained invocation
/// payload is four. A ten-unit request admits Completed but denies the payload.
#[test]
#[trace("FR-1012", "FSpec-FR-801", "FSpec-FR-804")]
fn finite_payload_retention_stops_after_a_real_completed_invocation() {
    use filament_exact::LimitKind;
    use fsl_eval::simulation::model::{
        NativeCandidateControl as Control, NativeCandidateEvent as Event,
    };
    let world = finite_payload_world();
    let system = world_system(&world);
    let limits = ScalarLimits {
        result_units: 10,
        ..UNLIMITED
    };
    system.begin_plan_replay(limits);
    let mut completed = 0;
    system.visit_plan_candidates(&system.initial()[0],&native_order(&system,&["move"]),None,&mut|event| {
        if let Event::Completed { receiver,arguments,.. }=event {
            assert!(matches!(arguments,[filament_exact::Value::Reference(target),filament_exact::Value::Enum(case)] if target==receiver && case.rank()==0));
            completed+=1;
        }
        Control::Continue
    }).unwrap();
    assert_eq!(completed, 1);
    assert_eq!(
        system.evaluation_meter().consumed(LimitKind::ResultUnits),
        7
    );
    system.begin_plan_replay(limits);
    let mut rules = finite_payload_rules(&world, &system);
    let result = run(
        &mut rules,
        &system,
        (1, 1),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Incomplete);
    assert!(result.plan.is_none() && result.certificate.is_none() && !result.certified);
    assert!(
        matches!(result.cause,Some(PlanStopCause::LimitReached{ref setting,value:10,..}) if setting=="accounting.result_units")
    );
    assert_eq!(
        system.evaluation_meter().consumed(LimitKind::ResultUnits),
        7,
        "payload denial retains no invalid candidate"
    );
}

/// Native replay and the independent checker rederive the real payload and
/// preserve its shared-meter accounting and cancellation causes.
#[test]
#[trace("FR-1012", "FSpec-FR-650", "FSpec-FR-801")]
fn finite_payload_replay_and_checker_preserve_retention_stops() {
    use filament_exact::LimitKind;
    use fsl_eval::simulation::model::{
        NativeCandidateControl as Control, NativeCandidateEvent as Event,
    };
    use std::sync::{atomic::Ordering, Arc, Mutex};
    let world = finite_payload_world();
    let system = world_system(&world);
    system.begin_plan_replay(UNLIMITED);
    let (cancel, polls) = finite_payload_observer();
    system.cancel_plan_with(&cancel);
    let checkpoints = Arc::new(Mutex::new(Vec::new()));
    let observed = checkpoints.clone();
    let counted = polls.clone();
    let mut rules = finite_payload_rules(&world, &system);
    rules.on_native_payload = Some(Box::new(move |step| {
        assert!(step.native_values().is_some());
        observed
            .lock()
            .unwrap()
            .push(counted.load(Ordering::Relaxed));
    }));
    let result = run_with(
        &mut rules,
        &system,
        (1, 1),
        PlanSearchLimits::default(),
        None,
        &cancel,
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Optimal);
    assert!(result.certified);
    let plan = result.plan.unwrap();
    let checkpoints = checkpoints.lock().unwrap().clone();
    assert_eq!(
        checkpoints.len(),
        2,
        "search and independent checker each retain one payload"
    );
    assert_eq!(
        system.evaluation_meter().consumed(LimitKind::ResultUnits),
        33,
        "search, checker and replay each retain seven native plus four payload units"
    );

    // The checker gets its own actual Completed event after the search's eleven
    // units. At twenty-one units only its separate payload retention is denied.
    system.begin_plan_replay(ScalarLimits {
        result_units: 21,
        ..UNLIMITED
    });
    let mut rules = finite_payload_rules(&world, &system);
    let checked = run(
        &mut rules,
        &system,
        (1, 1),
        PlanSearchLimits::default(),
        None,
    )
    .unwrap();
    assert_eq!(checked.status, PlanStatus::Incomplete);
    assert!(!checked.certified && checked.certificate.is_none());
    assert!(
        rules.found,
        "the one-step search goal was found before independent checking stopped"
    );
    assert!(
        matches!(checked.cause,Some(PlanStopCause::LimitReached{ref setting,value:21,..}) if setting=="accounting.result_units")
    );
    assert_eq!(
        system.evaluation_meter().consumed(LimitKind::ResultUnits),
        18
    );

    system.begin_plan_replay(ScalarLimits {
        result_units: 10,
        ..UNLIMITED
    });
    let mut rules = finite_payload_rules(&world, &system);
    let replayed = fsl_analyze::plan_search::replay::replay(
        &system,
        &mut rules,
        &plan,
        PlanLengths::new(1, 1).unwrap(),
        ObjectiveKind::StepSum,
    );
    assert!(matches!(
        replayed,
        Err(PlanSearchError::RuleStopped(RuleStop::Accounting {
            counter: "result_units",
            value: 10
        }))
    ));
    assert_eq!(
        system.evaluation_meter().consumed(LimitKind::ResultUnits),
        7
    );

    system.begin_plan_replay(UNLIMITED);
    let (native_cancel, native_polls) = finite_payload_observer();
    system.cancel_plan_with(&native_cancel);
    let mut completed_at = None;
    system
        .visit_plan_candidates(
            &system.initial()[0],
            &native_order(&system, &["move"]),
            None,
            &mut |event| {
                if let Event::Completed { .. } = event {
                    completed_at = Some(native_polls.load(Ordering::Relaxed));
                }
                Control::Continue
            },
        )
        .unwrap();
    let completed_at = completed_at.unwrap();
    assert_eq!(
        native_polls.load(Ordering::Relaxed),
        completed_at,
        "the next charge after Completed belongs to plan payload retention"
    );
    for cause in [CancelCause::Requested, CancelCause::Deadline] {
        system.begin_plan_replay(UNLIMITED);
        let (cancel, _keep_slot) = finite_payload_cancel_at(completed_at + 1, cause);
        system.cancel_plan_with(&cancel);
        let mut rules = finite_payload_rules(&world, &system);
        let replayed = fsl_analyze::plan_search::replay::replay(
            &system,
            &mut rules,
            &plan,
            PlanLengths::new(1, 1).unwrap(),
            ObjectiveKind::StepSum,
        );
        assert!(
            matches!(replayed,Err(PlanSearchError::RuleStopped(RuleStop::Cancelled(found))) if found==cause)
        );
        assert_eq!(
            system.evaluation_meter().consumed(LimitKind::ResultUnits),
            7
        );

        system.begin_plan_replay(UNLIMITED);
        let (cancel, _keep_slot) = finite_payload_cancel_at(checkpoints[1], cause);
        system.cancel_plan_with(&cancel);
        let mut rules = finite_payload_rules(&world, &system);
        let checked = run_with(
            &mut rules,
            &system,
            (1, 1),
            PlanSearchLimits::default(),
            None,
            &cancel,
        )
        .unwrap();
        assert_eq!(checked.status, PlanStatus::Incomplete);
        assert!(!checked.certified && checked.certificate.is_none());
        assert!(matches!(checked.cause,Some(PlanStopCause::Cancelled(found)) if found==cause));
        assert_eq!(
            system.evaluation_meter().consumed(LimitKind::ResultUnits),
            18,
            "the checker completed native retention before its payload stop"
        );
    }
}

fn native_order(
    system: &ModelSystem<'_>,
    names: &[&str],
) -> Vec<(filament_exact::EffectiveId, String)> {
    let operations = system.operations();
    names
        .iter()
        .map(|name| {
            let (declaring, _, _) = operations
                .iter()
                .find(|(_, found, _)| found == name)
                .expect("the declared operation exists");
            (declaring.clone(), (*name).to_owned())
        })
        .collect()
}

/// The callback gates an invocation before any precondition or frame trial
/// consumes its request evaluation budget.
#[test]
#[trace("FSpec-FR-801", "FR-1011")]
fn native_invocation_gate_precedes_contract_evaluation() {
    use filament_exact::LimitKind;
    use fsl_eval::simulation::model::{
        NativeCandidateControl, NativeCandidateEvent, NativeCandidateVisit,
    };
    let world = native_plan_world(
        "language \"ix:native\" edition \"1-draft\"; domain \"test/native-plan\"; type ValueRange = Int[0,3]; object Counter { number: ValueRange; operation apply(): Boolean modifies self.number; } population counters: Counter;",
        "post Applied using v on M::Counter::apply { convert<Integer>(self.number) = pre(self.number) + 1 }\n",
        &[("c", json!({"number":{"integer":"0"}}))]);
    let system = world_system(&world);
    system.share_evaluation_meter(UNLIMITED);
    let mut began = 0;
    let visit = system
        .visit_plan_candidates(
            &system.initial()[0],
            &native_order(&system, &["apply"]),
            None,
            &mut |event| match event {
                NativeCandidateEvent::Begin(_) => {
                    began += 1;
                    NativeCandidateControl::Stop
                }
                NativeCandidateEvent::Completed { .. }
                | NativeCandidateEvent::Disabled(_)
                | NativeCandidateEvent::Undetermined { .. } => {
                    panic!("a stopped invocation evaluates no contract or frame")
                }
            },
        )
        .expect("the caller can stop at the invocation gate");
    assert_eq!(visit, NativeCandidateVisit::Stopped);
    assert_eq!(began, 1);
    assert_eq!(system.evaluation_meter().consumed(LimitKind::WorkUnits), 0);
}

/// Raw rejected frame trials spend evaluation accounting, while remaining
/// one logical invocation with the same two distinct Boolean results.
#[test]
#[trace("FSpec-FR-801", "FR-1011")]
fn rejected_native_frame_trials_do_not_become_extra_invocations() {
    use filament_exact::LimitKind;
    use fsl_eval::simulation::model::{
        NativeCandidateControl, NativeCandidateEvent, NativeCandidateVisit,
    };
    let mut charges = Vec::new();
    for upper in [1, 3] {
        let model = format!("language \"ix:native\" edition \"1-draft\"; domain \"test/native-plan\"; type ValueRange = Int[0,{upper}]; object Counter {{ number: ValueRange; operation apply(): Boolean modifies self.number; }} population counters: Counter;");
        let world = native_plan_world(
            &model,
            "post Applied using v on M::Counter::apply { self.number = 1 }\n",
            &[("c", json!({"number":{"integer":"0"}}))],
        );
        let system = world_system(&world);
        system.share_evaluation_meter(UNLIMITED);
        let (mut began, mut outcomes) = (0, Vec::new());
        let visit = system
            .visit_plan_candidates(
                &system.initial()[0],
                &native_order(&system, &["apply"]),
                None,
                &mut |event| {
                    match event {
                        NativeCandidateEvent::Begin(_) => began += 1,
                        NativeCandidateEvent::Completed { state, outcome, .. } => {
                            outcomes.push((outcome, world.domain.integer(state, "c", "number")))
                        }
                        NativeCandidateEvent::Disabled(_)
                        | NativeCandidateEvent::Undetermined { .. } => {
                            panic!("the postcondition admits exactly one state for both results")
                        }
                    }
                    NativeCandidateControl::Continue
                },
            )
            .expect("finite raw trials complete");
        assert_eq!(visit, NativeCandidateVisit::Complete);
        assert_eq!(began, 1);
        assert_eq!(outcomes, [(Some(0), 1), (Some(1), 1)]);
        charges.push(system.evaluation_meter().consumed(LimitKind::WorkUnits));
    }
    assert!(
        charges[1] > charges[0],
        "extra rejected frame trials spend only evaluation work: {charges:?}"
    );
}

/// The receiver is ordered before parameters. Typed Int order puts 2 before
/// 10 even though canonical JSON text would do the reverse.
#[test]
#[trace("FSpec-FR-804", "FR-1012")]
fn native_invocations_follow_operation_receiver_and_typed_argument_order() {
    use fsl_eval::simulation::model::{
        NativeCandidateControl, NativeCandidateEvent, NativeCandidateVisit,
    };
    let world = native_plan_world(
        "language \"ix:native\" edition \"1-draft\"; domain \"test/native-plan\"; type ValueRange = Int[0,0]; type ArgumentRange = Int[2,10]; object Counter { number: ValueRange; operation zeta(amount: ArgumentRange); operation alpha(amount: ArgumentRange); } population counters: Counter;",
        "", &[("9",json!({"number":{"integer":"0"}})),("10",json!({"number":{"integer":"0"}}))]);
    let system = ModelSystem::new(world.subject(&["9", "10"], UNLIMITED).simulation)
        .expect("both native receivers admit");
    let mut invocations = Vec::new();
    let operations = system.operations();
    let visit = system
        .visit_plan_candidates(
            &system.initial()[0],
            &native_order(&system, &["zeta", "alpha"]),
            None,
            &mut |event| {
                if let NativeCandidateEvent::Begin(transition) = event {
                    let name = operations
                        .iter()
                        .find(|(_, _, text)| *text == transition.operation())
                        .expect("declared operation")
                        .1;
                    let receiver: serde_json::Value =
                        serde_json::from_str(transition.receiver().expect("a receiver")).unwrap();
                    let argument: serde_json::Value =
                        serde_json::from_str(&transition.arguments()[0]).unwrap();
                    invocations.push((
                        name.to_owned(),
                        receiver["identity"].as_str().unwrap().to_owned(),
                        argument["value"].as_str().unwrap().parse::<i64>().unwrap(),
                    ));
                    NativeCandidateControl::Skip
                } else {
                    panic!("a skipped invocation produces no evaluation event")
                }
            },
        )
        .expect("the finite invocation product visits without evaluation");
    assert_eq!(visit, NativeCandidateVisit::Complete);
    let mut expected = Vec::new();
    for operation in ["zeta", "alpha"] {
        for receiver in ["10", "9"] {
            for argument in 2..=10 {
                expected.push((operation.to_owned(), receiver.to_owned(), argument));
            }
        }
    }
    assert_eq!(invocations, expected);
}

/// Result ordering precedes canonical state ordering and keeps every native
/// result/post-state pair, including different results sharing one post-state.
#[test]
#[trace("FSpec-FR-804", "FR-1012")]
fn native_outcomes_keep_all_results_in_result_then_state_order() {
    use fsl_eval::simulation::model::{
        NativeCandidateControl, NativeCandidateEvent, NativeCandidateVisit,
    };
    for (declaration, field) in [("", "Int[0,2]"), ("type Amount = Int[0,2];", "Amount")] {
        let model = format!("language \"ix:native\" edition \"1-draft\"; domain \"test/native-plan\"; {declaration} object Counter {{ number: {field}; operation choose(): Boolean modifies self.number; }} population counters: Counter;");
        let world = native_plan_world(
        &model,
        "post Chosen using v on M::Counter::choose { (not result and self.number >= 1) or (result and self.number <= 1) }\n",
        &[("c", json!({"number":{"integer":"0"}}))],
    );
        let types = world.package.graph().scope().types();
        let counter = types
            .object_types()
            .find(|object| object.name() == "M::Counter")
            .unwrap();
        let value = types
            .attributes(counter.key())
            .unwrap()
            .iter()
            .find(|attribute| attribute.field().name() == "number")
            .unwrap();
        let filament_exact::ValueType::Int(bounds) = value.field().value_type() else {
            panic!("native frame enumeration must use the actual admitted bounded type");
        };
        assert_eq!(
            (bounds.lower().to_string(), bounds.upper().to_string()),
            ("0".to_owned(), "2".to_owned())
        );
        let system = world_system(&world);
        let mut outcomes = Vec::new();
        let visit = system
            .visit_plan_candidates(
                &system.initial()[0],
                &native_order(&system, &["choose"]),
                None,
                &mut |event| {
                    match event {
                        NativeCandidateEvent::Begin(_) => {}
                        NativeCandidateEvent::Completed { state, outcome, .. } => {
                            outcomes.push((outcome, world.domain.integer(state, "c", "number")))
                        }
                        NativeCandidateEvent::Disabled(_)
                        | NativeCandidateEvent::Undetermined { .. } => {
                            panic!("four valid native pairs exist")
                        }
                    }
                    NativeCandidateControl::Continue
                },
            )
            .expect("the finite native relation completes");
        assert_eq!(visit, NativeCandidateVisit::Complete);
        assert_eq!(
            outcomes,
            [(Some(0), 1), (Some(1), 2), (Some(2), 0), (Some(3), 1)]
        );
    }
}

/// Input admission enforces inline bounds before native invocation, with the
/// same refusal family as an explicitly named bounded model type.
#[test]
#[trace("FR-710-AC-5", "FSpec-FR-106", "FSpec-FR-154")]
fn inline_integer_initial_values_refuse_outside_the_declared_interval() {
    use fsl_semantics::model::observation::AdmissionFailure;
    for initial in [-1, 3] {
        let mut causes = Vec::new();
        for (declaration, field) in [("", "Int[0,2]"), ("type Amount = Int[0,2];", "Amount")] {
            let model = format!("language \"ix:native\" edition \"1-draft\"; domain \"test/native-plan\"; {declaration} object Counter {{ number: {field}; operation choose(): Boolean modifies self.number; }} population counters: Counter;");
            let world = native_plan_world(
                &model,
                "",
                &[("c", json!({"number":{"integer":initial.to_string()}}))],
            );
            let failure = match ModelSystem::new(world.subject(&["c"], UNLIMITED).simulation) {
                Ok(_) => panic!("{initial} must not admit under {field}"),
                Err(failure) => failure,
            };
            let AdmissionFailure::Refused(record) = failure else {
                panic!("out-of-range input must refuse: {failure:?}");
            };
            assert_eq!(record.catalog.code(), "invalid_runtime_input");
            causes.push(record.catalog.cause().to_owned());
        }
        assert_eq!(
            causes[0], causes[1],
            "inline and named ranges share the same input-admission owner"
        );
    }
}

/// Native replay binds the exact ordinal, even when two results share one
/// post-state. The certificate also retains every ordinal's successor row.
#[test]
#[trace("FSpec-FR-650", "FSpec-FR-804", "FR-1012")]
fn native_replay_and_certificate_bind_each_multiple_outcome_ordinal() {
    use fsl_analyze::plan_search::replay;
    let world = native_plan_world(
        "language \"ix:native\" edition \"1-draft\"; domain \"test/native-plan\"; type ValueRange = Int[0,2]; object Counter { number: ValueRange; operation choose(): Boolean modifies self.number; } population counters: Counter;",
        "post Chosen using v on M::Counter::choose { (not result and self.number >= 1) or (result and self.number <= 1) }\n",
        &[("c",json!({"number":{"integer":"0"}}))]);
    let system = world_system(&world);
    let mut rules = Counter::new(&world, &system, 2);
    rules.value_field = "number";
    rules.native = Some(native_order(&system, &["choose"]).into());
    let result = run(
        &mut rules,
        &system,
        (1, 1),
        PlanSearchLimits::default(),
        None,
    )
    .expect("the finite native plan replays and checks");
    assert_eq!(result.status, PlanStatus::Optimal);
    assert!(result.certified);
    assert_eq!(
        result.certificate.as_ref().expect("a certificate").outcome,
        CertificateOutcome::Accepted
    );
    let plan = result.plan.expect("a one-step native plan");
    assert_eq!(plan.steps.len(), 1);
    assert_eq!(plan.steps[0].outcome_ordinal(), Some(1));
    assert_eq!(
        replay::replay(
            &system,
            &mut rules,
            &plan,
            PlanLengths::new(1, 1).unwrap(),
            ObjectiveKind::StepSum
        )
        .unwrap(),
        integer(1)
    );
    for ordinal in [None, Some(0), Some(4)] {
        let mut forged = plan.clone();
        forged.steps[0] = PlanStep::new(plan.steps[0].invocation().clone(), ordinal);
        assert!(
            matches!(
                replay::replay(
                    &system,
                    &mut rules,
                    &forged,
                    PlanLengths::new(1, 1).unwrap(),
                    ObjectiveKind::StepSum
                ),
                Err(PlanSearchError::ReplayFailed { step: 0, .. })
            ),
            "ordinal {ordinal:?} must not select the recorded post-state"
        );
    }
    // Ordinals 0 and 3 both reach value1. Removing just ordinal3 must
    // still break closure rather than being covered by ordinal0's state.
    let fault = PlanFault::DropSuccessor {
        entry: 0,
        successor: 3,
    };
    let tampered = run(
        &mut rules,
        &system,
        (1, 1),
        PlanSearchLimits::default(),
        Some(&fault),
    )
    .expect("a rejected certificate remains an honest feasible result");
    assert!(!tampered.certified);
    assert_eq!(
        tampered
            .certificate
            .expect("a certificate rejection")
            .outcome,
        CertificateOutcome::Rejected {
            rule: CertificateRule::Closure,
            entry: 0
        }
    );
}

/// An ordinal on a single-outcome invocation, or a receiver of another
/// admitted model member, cannot replay as the recorded native invocation.
#[test]
#[trace("FSpec-FR-650", "FSpec-FR-804", "FR-1012")]
fn native_replay_refuses_single_outcome_ordinals_and_wrong_receivers() {
    use fsl_analyze::plan_search::replay;
    let world = native_plan_world(
        "language \"ix:native\" edition \"1-draft\"; domain \"test/native-plan\"; type ValueRange = Int[0,1]; object Counter { number: ValueRange; operation settle() modifies self.number; } object Other { number: ValueRange; } population counters: Counter;",
        "post Settles using v on M::Counter::settle { self.number = 1 }\n",
        &[("c",json!({"number":{"integer":"0"}}))]);
    let system = world_system(&world);
    let mut rules = Counter::new(&world, &system, 1);
    rules.value_field = "number";
    rules.native = Some(native_order(&system, &["settle"]).into());
    let result = run(
        &mut rules,
        &system,
        (1, 1),
        PlanSearchLimits::default(),
        None,
    )
    .expect("the single-outcome plan replays");
    assert_eq!(result.status, PlanStatus::Optimal);
    let plan = result.plan.expect("a reported plan");
    assert_eq!(plan.steps[0].outcome_ordinal(), None);
    let mut ordinal = plan.clone();
    ordinal.steps[0] = PlanStep::new(plan.steps[0].invocation().clone(), Some(0));
    assert!(
        matches!(
            replay::replay(
                &system,
                &mut rules,
                &ordinal,
                PlanLengths::new(1, 1).unwrap(),
                ObjectiveKind::StepSum
            ),
            Err(PlanSearchError::ReplayFailed { step: 0, .. })
        ),
        "single-outcome invocations require an absent ordinal"
    );
    let mut receiver: serde_json::Value = serde_json::from_str(
        plan.steps[0]
            .receiver()
            .expect("an object operation has a receiver"),
    )
    .unwrap();
    receiver["object_type"]["original"]["node"] = json!("ix://test/native-plan/Other");
    receiver["universe"]["path"][2] = json!("ix://test/native-plan/Other");
    let invocation = ModelTransition::new(
        plan.steps[0].operation().to_owned(),
        Some(receiver.to_string()),
        plan.steps[0].arguments().to_vec(),
        quire_canonical::Limits::new(4096),
    )
    .expect("the forged foreign receiver remains canonical typed JSON");
    let mut foreign = plan.clone();
    foreign.steps[0] = PlanStep::new(invocation, None);
    assert!(
        matches!(
            replay::replay(
                &system,
                &mut rules,
                &foreign,
                PlanLengths::new(1, 1).unwrap(),
                ObjectiveKind::StepSum
            ),
            Err(PlanSearchError::ReplayFailed { step: 0, .. })
        ),
        "a different model member cannot reuse Counter's invocation"
    );
}

/// The implicit outcome collection uses the caller's named retention size,
/// without publishing a partial sorted invocation or imposing a hidden cap.
#[test]
#[trace("FSpec-FR-801", "FR-1011-AC-2")]
fn native_outcome_collection_names_its_retained_value_limit() {
    use filament_exact::LimitKind;
    use fsl_eval::simulation::model::{
        NativeCandidateControl, NativeCandidateEvent, NativeCandidateVisit,
    };
    use fsl_eval::simulation::ExpansionLimit;
    let world = native_plan_world(
        "language \"ix:native\" edition \"1-draft\"; domain \"test/native-plan\"; type ValueRange = Int[0,1]; object Counter { number: ValueRange; operation choose(): Boolean modifies self.number; } population counters: Counter;",
        "", &[("c",json!({"number":{"integer":"0"}}))]);
    for bound in [6, 21] {
        let system = world_system(&world);
        system.share_evaluation_meter(ScalarLimits {
            value_occurrences: bound,
            ..UNLIMITED
        });
        let mut began = 0;
        let mut outcomes = Vec::new();
        let visit = system.visit_plan_candidates(
            &system.initial()[0],
            &native_order(&system, &["choose"]),
            None,
            &mut |event| {
                match event {
                    NativeCandidateEvent::Begin(_) => began += 1,
                    NativeCandidateEvent::Completed { state, outcome, .. } => {
                        outcomes.push((outcome, world.domain.integer(state, "c", "number")));
                    }
                    NativeCandidateEvent::Disabled(_)
                    | NativeCandidateEvent::Undetermined { .. } => {
                        panic!("all four Boolean-result/post-state pairs are valid");
                    }
                }
                NativeCandidateControl::Continue
            },
        );
        assert_eq!(began, 1);
        if bound == 6 {
            let stop = visit.expect_err("the second retained outcome exceeds the named size");
            assert_eq!(stop.cause.code(), "resource_exhausted");
            assert_eq!(stop.cause.cause(), "insufficient-next-charge");
            assert_eq!(
                stop.limit,
                Some(ExpansionLimit::EvaluationMeter(LimitKind::ValueOccurrences))
            );
            assert_eq!(LimitKind::ValueOccurrences.as_str(), "value_occurrences");
            assert!(
                outcomes.is_empty(),
                "an incomplete collection publishes no partial sorted outcomes: {outcomes:?}"
            );
        } else {
            assert_eq!(
                visit.expect("the exact total size admits all outcomes"),
                NativeCandidateVisit::Complete
            );
            assert_eq!(
                outcomes,
                [(Some(0), 0), (Some(1), 1), (Some(2), 0), (Some(3), 1)]
            );
        }
    }
}

/// Intercept the real source arithmetic at the first potential addition.
/// Every other rule remains the production source adapter's implementation.
struct CancelPotentialAdd<'a, 'c> {
    source: fsl_analyze::plan_source::SourcePlan<'a>,
    system: &'a ModelSystem<'a>,
    cancel: &'c Cancel,
    cause: CancelCause,
    reached: bool,
}

impl<'a> PlanRules for CancelPotentialAdd<'a, '_> {
    type StepKey = <fsl_analyze::plan_source::SourcePlan<'a> as PlanRules>::StepKey;
    fn direction(&self) -> Direction {
        self.source.direction()
    }
    fn objective_kind(&self) -> ObjectiveKind {
        self.source.objective_kind()
    }
    fn merges_prefixes(&self) -> bool {
        self.source.merges_prefixes()
    }
    fn step_key(&mut self, step: &PlanStep) -> Result<Self::StepKey, RuleStop> {
        self.source.step_key(step)
    }
    fn native_operations(&self) -> Option<std::sync::Arc<[(filament_exact::EffectiveId, String)]>> {
        self.source.native_operations()
    }
    fn policy_count(&self) -> usize {
        self.source.policy_count()
    }
    fn policy_start(
        &mut self,
        policy: usize,
        state: &ModelSimState,
    ) -> Result<MonitorStep, RuleStop> {
        self.source.policy_start(policy, state)
    }
    fn policy_advance(
        &mut self,
        policy: usize,
        monitor: &[u8],
        state: &ModelSimState,
    ) -> Result<MonitorStep, RuleStop> {
        self.source.policy_advance(policy, monitor, state)
    }
    fn policy_accepts(
        &mut self,
        policy: usize,
        monitor: &[u8],
        last: &ModelSimState,
    ) -> Result<bool, RuleStop> {
        self.source.policy_accepts(policy, monitor, last)
    }
    fn goal(&mut self, plan: &PlanView<'_>) -> Result<bool, RuleStop> {
        self.source.goal(plan)
    }
    fn step_cost(
        &mut self,
        plan: &PlanView<'_>,
        index: usize,
    ) -> Result<Option<Rational>, RuleStop> {
        self.source.step_cost(plan, index)
    }
    fn step_cost_typed(
        &mut self,
        plan: &PlanView<'_>,
        index: usize,
    ) -> Result<Option<fsl_analyze::plan_search::StepTotal>, RuleStop> {
        self.source.step_cost_typed(plan, index)
    }
    fn extend_step_sum(
        &mut self,
        plan: &PlanView<'_>,
        total: &fsl_analyze::plan_search::StepTotal,
        cost: &fsl_analyze::plan_search::StepTotal,
    ) -> Result<Option<fsl_analyze::plan_search::StepTotal>, RuleStop> {
        self.source.extend_step_sum(plan, total, cost)
    }
    fn add_objective_values(
        &mut self,
        left: &Rational,
        right: &Rational,
    ) -> Result<Option<Rational>, RuleStop> {
        self.reached = true;
        self.system.cancel_plan_with(self.cancel);
        self.cancel.cancel(self.cause);
        self.source.add_objective_values(left, right)
    }
    fn objective_offset(&mut self, plan: &PlanView<'_>) -> Result<Option<Rational>, RuleStop> {
        self.source.objective_offset(plan)
    }
    fn step_sum_objective(
        &mut self,
        plan: &PlanView<'_>,
        total: &Rational,
    ) -> Result<Option<Rational>, RuleStop> {
        self.source.step_sum_objective(plan, total)
    }
    fn objective(&mut self, plan: &PlanView<'_>) -> Result<Option<Rational>, RuleStop> {
        self.source.objective(plan)
    }
    fn begin_replay(&mut self) {
        self.source.begin_replay();
    }
}

fn lambda_objective_world(objective: &str, types: &str) -> World {
    lambda_objective_world_for_goal(objective, types, "Reach.length >= 1")
}

fn lambda_objective_world_for_goal(objective: &str, types: &str, goal: &str) -> World {
    native_plan_world(
        "language \"ix:native\" edition \"1-draft\"; domain \"test/native-plan\"; type Amount = Int[0,3]; object Counter { number: Amount; operation inc() modifies self.number; } population counters: Counter;",
        &format!("{types}\npost Inc using v on M::Counter::inc {{ (pre(self.number) = 0 and self.number = 1) or (pre(self.number) = 1 and self.number = 2) or (pre(self.number) = 2 and self.number = 3) }}\ntype Lengths = Int[0,3]; synthesis Reach using v grammar M::Counter domain Lengths {objective} satisfies {{ {goal} }};"),
        &[("c",json!({"number":{"integer":"0"}}))],
    )
}

/// Cancellation must survive the new typed addition and offset paths as
/// requested/deadline cancellation, independently from named accounting stops.
#[test]
#[trace("FSpec-FR-801-AC-2", "FSpec-FR-802")]
fn lambda_objective_arithmetic_distinguishes_cancellation_from_accounting() {
    use fsl_analyze::plan_search::StepTotal;
    use fsl_analyze::plan_source::SourcePlan;
    let world = lambda_objective_world(
        "minimize Reach.steps.sum(s => convert<Cost>(0.10)) + 0.05",
        "type Cost = Decimal[0,10000;2,2;exact];",
    );
    let system = world_system(&world);
    let synthesis = &world.package.graph().syntheses()[0];
    let mut rules = SourcePlan::new(&world.package, synthesis, &system, vec![], UNLIMITED).unwrap();
    let result = search(
        &system,
        &mut rules,
        PlanSearchRequest {
            lengths: PlanLengths::new(2, 2).unwrap(),
            limits: PlanSearchLimits::default(),
            cancel: &Cancel::new(),
            fault: None,
        },
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Optimal);
    let plan = result.plan.unwrap();
    let first = PlanView {
        states: &plan.states[..2],
        steps: &plan.steps[..1],
    };
    let cost = rules.step_cost_typed(&first, 0).unwrap().unwrap();
    let total = rules
        .extend_step_sum(&first, &StepTotal::from_rational(integer(0)), &cost)
        .unwrap()
        .unwrap();
    let second_cost = rules.step_cost_typed(&plan.view(), 1).unwrap().unwrap();
    for cause in [CancelCause::Requested, CancelCause::Deadline] {
        for operation in ["typed-add", "potential-add", "offset", "objective"] {
            system.begin_plan_replay(UNLIMITED);
            let mut rules =
                SourcePlan::new(&world.package, synthesis, &system, vec![], UNLIMITED).unwrap();
            let cancel = Cancel::new();
            system.cancel_plan_with(&cancel);
            cancel.cancel(cause);
            let outcome = match operation {
                "typed-add" => rules
                    .extend_step_sum(&plan.view(), &total, &second_cost)
                    .map(|_| ()),
                "potential-add" => rules
                    .add_objective_values(&integer(2), &integer(7))
                    .map(|_| ()),
                "offset" => rules.objective_offset(&plan.view()).map(|_| ()),
                "objective" => rules
                    .step_sum_objective(&plan.view(), &integer(0))
                    .map(|_| ()),
                _ => unreachable!(),
            };
            assert_eq!(outcome, Err(RuleStop::Cancelled(cause)), "{operation}");
        }
    }
    let accounting_limits = ScalarLimits {
        work_units: 0,
        ..UNLIMITED
    };
    system.begin_plan_replay(accounting_limits);
    let mut rules = SourcePlan::new(
        &world.package,
        synthesis,
        &system,
        vec![],
        accounting_limits,
    )
    .unwrap();
    assert_eq!(
        rules.add_objective_values(&integer(2), &integer(7)),
        Err(RuleStop::Accounting {
            counter: "work_units",
            value: 0
        })
    );
    let world = lambda_objective_world_for_goal(
        "minimize Reach.steps.sum(s => convert<Cost>(0.10)) + 0.05",
        "type Cost = Decimal[0,10000;2,2;exact];",
        "Reach.final.number = 3",
    );
    let system = world_system(&world);
    for cause in [CancelCause::Requested, CancelCause::Deadline] {
        system.begin_plan_replay(UNLIMITED);
        let cancel = Cancel::new();
        let source = SourcePlan::new(
            &world.package,
            &world.package.graph().syntheses()[0],
            &system,
            vec![],
            UNLIMITED,
        )
        .unwrap();
        let mut rules = CancelPotentialAdd {
            source,
            system: &system,
            cancel: &cancel,
            cause,
            reached: false,
        };
        let result = search(
            &system,
            &mut rules,
            PlanSearchRequest {
                lengths: PlanLengths::new(0, 3).unwrap(),
                limits: PlanSearchLimits::default(),
                cancel: &cancel,
                fault: None,
            },
        )
        .unwrap();
        assert!(
            rules.reached,
            "the real potential addition must be reached before cancellation"
        );
        assert_eq!(result.status, PlanStatus::Incomplete);
        assert_eq!(result.cause, Some(PlanStopCause::Cancelled(cause)));
        assert!(!result.certified);
        assert!(result.certificate.is_none());
        assert_eq!(
            result.plan.unwrap().steps.len(),
            3,
            "an admissible goal was found before certificate construction was cancelled"
        );
    }
}

/// The checker recomputes a negative-cost prefix rather than trusting the
/// recorded total in a closure node without cost dominance.
#[test]
#[trace("FSpec-FR-802-AC-4", "FSpec-FR-650")]
fn lambda_objective_certificate_rejects_a_forged_accumulated_total() {
    use fsl_analyze::plan_source::SourcePlan;
    let world = lambda_objective_world("minimize Reach.steps.sum(s => -2) + 7", "");
    let system = world_system(&world);
    for forged in [false, true] {
        let mut rules = SourcePlan::new(
            &world.package,
            &world.package.graph().syntheses()[0],
            &system,
            vec![],
            UNLIMITED,
        )
        .unwrap();
        let fault = PlanFault::ForgeAccumulated {
            entry: 1,
            value: integer(999),
        };
        let result = search(
            &system,
            &mut rules,
            PlanSearchRequest {
                lengths: PlanLengths::new(0, 3).unwrap(),
                limits: PlanSearchLimits::default(),
                cancel: &Cancel::new(),
                fault: forged.then_some(&fault),
            },
        )
        .unwrap();
        if forged {
            assert!(!result.certified);
            assert_eq!(
                result.certificate.unwrap().outcome,
                CertificateOutcome::Rejected {
                    rule: CertificateRule::Closure,
                    entry: 0
                }
            );
        } else {
            assert_eq!(result.status, PlanStatus::Optimal);
            assert_eq!(result.objective, Some(integer(1)));
            assert_eq!(
                result.certificate.unwrap().outcome,
                CertificateOutcome::Accepted
            );
        }
    }
}

/// A potential with an initial value that omits or doubles the constant
/// offset cannot certify the independently recomputed source objective.
#[test]
#[trace("FSpec-FR-802-AC-1", "FSpec-FR-650")]
fn lambda_objective_certificate_rejects_a_forged_initial_offset() {
    use fsl_analyze::plan_source::SourcePlan;
    let world = lambda_objective_world("minimize Reach.steps.sum(s => 2) + 7", "");
    let system = world_system(&world);
    for (forged, rule) in [
        (-5, CertificateRule::Initial),
        (9, CertificateRule::Descent),
    ] {
        let mut rules = SourcePlan::new(
            &world.package,
            &world.package.graph().syntheses()[0],
            &system,
            vec![],
            UNLIMITED,
        )
        .unwrap();
        let fault = PlanFault::ForgeInitialValue {
            value: integer(forged),
        };
        let result = search(
            &system,
            &mut rules,
            PlanSearchRequest {
                lengths: PlanLengths::new(0, 3).unwrap(),
                limits: PlanSearchLimits::default(),
                cancel: &Cancel::new(),
                fault: Some(&fault),
            },
        )
        .unwrap();
        assert!(!result.certified);
        assert_eq!(
            result.certificate.unwrap().outcome,
            CertificateOutcome::Rejected { rule, entry: 0 }
        );
    }
}

/// A complete-plan-dependent summand remains a general objective; length
/// and final-value objectives retain their actual classification and proof.
#[test]
#[trace("FSpec-FR-802-AC-1", "FSpec-FR-802-AC-6")]
fn lambda_objective_classification_preserves_whole_plan_terms_and_default_length() {
    use fsl_analyze::plan_source::SourcePlan;
    for (objective, kind, expected, certificate, length) in [
        ("", ObjectiveKind::StepSum, 1, CertificateKind::Potential, 1),
        (
            "maximize Reach.final.number",
            ObjectiveKind::General,
            3,
            CertificateKind::Closure,
            3,
        ),
        (
            "maximize Reach.steps.sum(s => Reach.length)",
            ObjectiveKind::General,
            9,
            CertificateKind::Closure,
            3,
        ),
    ] {
        let world = lambda_objective_world(objective, "");
        let system = world_system(&world);
        let mut rules = SourcePlan::new(
            &world.package,
            &world.package.graph().syntheses()[0],
            &system,
            vec![],
            UNLIMITED,
        )
        .unwrap();
        assert_eq!(rules.objective_kind(), kind);
        let result = search(
            &system,
            &mut rules,
            PlanSearchRequest {
                lengths: PlanLengths::new(0, 3).unwrap(),
                limits: PlanSearchLimits::default(),
                cancel: &Cancel::new(),
                fault: None,
            },
        )
        .unwrap();
        assert_eq!(result.status, PlanStatus::Optimal);
        assert_eq!(result.objective, Some(integer(expected)));
        assert_eq!(result.plan.unwrap().steps.len(), length);
        let proof = result.certificate.unwrap();
        assert_eq!(proof.kind, certificate);
        assert_eq!(proof.outcome, CertificateOutcome::Accepted);
    }
}

const POLICY_DIAMOND_MODEL: &str = r#"language "ix:native" edition "1-draft";
domain "test/native-plan";
object Counter {
  number: Int[0,4];
  operation right() modifies self.number { pre Start { self.number = 0 }; post Next { self.number = 1 }; }
  operation left() modifies self.number { pre Start { self.number = 0 }; post Next { self.number = 2 }; }
  operation combine() modifies self.number { pre Branch { self.number = 1 or self.number = 2 }; post Next { self.number = 3 }; }
  operation settle() modifies self.number { pre Joined { self.number = 3 }; post Next { self.number = 4 }; }
}
population counters: Counter;
"#;

fn positive_policy_world(formula: &str) -> World {
    native_plan_world_with_clauses(POLICY_DIAMOND_MODEL,
        "type Lengths = Int[0,3]; synthesis Reach using t grammar M::Counter domain Lengths satisfies { forall(c in Reach.final.counters: c.number = 4) };\n",
        &[model_world::Clause { activation: "on origin", formula }],
        &[("c", json!({"number":{"integer":"0"}}))])
}

/// Different past witnesses survive a native diamond; equal obligations merge
/// and retain the earlier declared branch. Replay/checking reconstruct both.
#[test]
#[trace("FR-1012", "FR-1011", "FSpec-FR-803-AC-3")]
fn positive_source_policies_preserve_witnesses_and_canonical_native_order() {
    use fsl_analyze::plan_source::SourcePlan;
    for (formula, expected) in [
        ("eventually[0,3] holds(c.number = 2)", "left"),
        ("always holds(c.number >= 0)", "right"),
        ("always[0,5] holds(c.number <= 4)", "right"),
    ] {
        let world = positive_policy_world(formula);
        let types = world.package.graph().scope().types();
        let counter = types
            .object_types()
            .find(|object| object.name() == "M::Counter")
            .unwrap();
        let field = types
            .attributes(counter.key())
            .unwrap()
            .iter()
            .find(|field| field.field().name() == "number")
            .unwrap();
        let filament_exact::ValueType::Int(bounds) = field.field().value_type() else {
            panic!("actual admitted native interval");
        };
        assert_eq!(
            (bounds.lower().to_string(), bounds.upper().to_string()),
            ("0".to_owned(), "4".to_owned())
        );
        let system = world_system(&world);
        let node = &world.package.graph().temporal_clauses()[0];
        let mut retained = None;
        for _ in 0..2 {
            let mut rules = SourcePlan::new(
                &world.package,
                &world.package.graph().syntheses()[0],
                &system,
                vec![node],
                UNLIMITED,
            )
            .unwrap();
            let result = search(
                &system,
                &mut rules,
                PlanSearchRequest {
                    lengths: PlanLengths::new(0, 3).unwrap(),
                    limits: PlanSearchLimits::default(),
                    cancel: &Cancel::new(),
                    fault: None,
                },
            )
            .unwrap();
            assert!(rules.fault().is_none(), "{:?}", rules.fault());
            assert_eq!(result.status, PlanStatus::Optimal);
            assert!(result.certified);
            assert_eq!(result.objective, Some(integer(3)));
            assert_eq!(
                result.certificate.as_ref().unwrap().outcome,
                CertificateOutcome::Accepted
            );
            let plan = result.plan.as_ref().unwrap();
            let names: Vec<_> = plan
                .steps
                .iter()
                .map(|step| rules.operation_name(step).unwrap())
                .collect();
            assert_eq!(names, [expected, "combine", "settle"]);
            let numbers: Vec<_> = plan
                .states
                .iter()
                .map(|state| world.domain.integer(state, "c", "number"))
                .collect();
            assert_eq!(numbers, [0, if expected == "left" { 2 } else { 1 }, 3, 4]);
            assert_eq!(
                fsl_eval::simulation::model::read_clause(
                    &system,
                    node,
                    &plan.states,
                    true,
                    &mut filament_exact::Meter::new(UNLIMITED)
                )
                .unwrap(),
                fsl_eval::simulation::model::ClauseReading::Holds
            );
            let evidence = (numbers, result.explored, result.certificate);
            if let Some(previous) = &retained {
                assert_eq!(&evidence, previous);
            } else {
                retained = Some(evidence);
            }
        }
    }
}

/// Safety rejects before goal evaluation and gives a checked infeasibility
/// certificate; state substitution and missing successors cannot certify.
#[test]
#[trace("FR-1012", "FSpec-FR-803-AC-5")]
fn positive_source_policies_refuse_broken_safety_and_forged_certificates() {
    use fsl_analyze::plan_source::SourcePlan;
    let world = positive_policy_world("always holds(c.number != 4)");
    let system = world_system(&world);
    let mut rules = SourcePlan::new(
        &world.package,
        &world.package.graph().syntheses()[0],
        &system,
        vec![&world.package.graph().temporal_clauses()[0]],
        UNLIMITED,
    )
    .unwrap();
    let result = search(
        &system,
        &mut rules,
        PlanSearchRequest {
            lengths: PlanLengths::new(0, 3).unwrap(),
            limits: PlanSearchLimits::default(),
            cancel: &Cancel::new(),
            fault: None,
        },
    )
    .unwrap();
    assert_eq!(result.status, PlanStatus::Infeasible);
    assert!(result.plan.is_none());
    assert_eq!(
        result.certificate.unwrap().outcome,
        CertificateOutcome::Accepted
    );
    let world = positive_policy_world("always holds(c.number >= 0)");
    let system = world_system(&world);
    for fault in [
        PlanFault::DropSuccessor {
            entry: 0,
            successor: 0,
        },
        PlanFault::ForgeEntryState {
            entry: 1,
            source: 0,
        },
    ] {
        let mut rules = SourcePlan::new(
            &world.package,
            &world.package.graph().syntheses()[0],
            &system,
            vec![&world.package.graph().temporal_clauses()[0]],
            UNLIMITED,
        )
        .unwrap();
        let result = search(
            &system,
            &mut rules,
            PlanSearchRequest {
                lengths: PlanLengths::new(0, 3).unwrap(),
                limits: PlanSearchLimits::default(),
                cancel: &Cancel::new(),
                fault: Some(&fault),
            },
        )
        .unwrap();
        assert!(!result.certified, "{fault:?}");
        assert!(
            matches!(
                result.certificate.unwrap().outcome,
                CertificateOutcome::Rejected { .. }
            ),
            "{fault:?}"
        );
    }
}

/// A valid but incorrect recorded monitor cannot pass independent initial
/// reconstruction, even when its own canonical context and framing are valid.
#[test]
#[trace("FR-1012", "FSpec-FR-803")]
fn positive_source_policy_certificate_rejects_a_substituted_monitor() {
    use fsl_analyze::plan_source::SourcePlan;
    let world = positive_policy_world("eventually[0,3] holds(c.number = 2)");
    let system = world_system(&world);
    let mut rules = SourcePlan::new(
        &world.package,
        &world.package.graph().syntheses()[0],
        &system,
        vec![&world.package.graph().temporal_clauses()[0]],
        UNLIMITED,
    )
    .unwrap();
    let fault = PlanFault::ForgePolicyStates {
        entry: 0,
        source: 1,
    };
    let result = search(
        &system,
        &mut rules,
        PlanSearchRequest {
            lengths: PlanLengths::new(0, 3).unwrap(),
            limits: PlanSearchLimits::default(),
            cancel: &Cancel::new(),
            fault: Some(&fault),
        },
    )
    .unwrap();
    assert!(!result.certified);
    assert_eq!(
        result.certificate.unwrap().outcome,
        CertificateOutcome::Rejected {
            rule: CertificateRule::Initial,
            entry: 0
        }
    );
}

/// Positive source policies retain their original cumulative work/storage
/// denial and request/deadline cancellation, without another per-policy meter.
#[test]
#[trace("FR-1011-AC-2", "FR-1012")]
fn positive_source_policy_accounting_and_cancellation_keep_the_original_session() {
    use filament_exact::{CancelCause, ScalarLimits};
    use fsl_analyze::plan_source::SourcePlan;
    let world = positive_policy_world("always holds(c.number >= 0)");
    let system = world_system(&world);
    let node = &world.package.graph().temporal_clauses()[0];
    let initial = system.initial().remove(0);
    for (counter, limits) in [
        (
            "work_units",
            ScalarLimits {
                work_units: 0,
                ..UNLIMITED
            },
        ),
        (
            "result_units",
            ScalarLimits {
                result_units: 0,
                ..UNLIMITED
            },
        ),
    ] {
        let mut rules = SourcePlan::new(
            &world.package,
            &world.package.graph().syntheses()[0],
            &system,
            vec![node, node],
            limits,
        )
        .unwrap();
        assert!(
            matches!(rules.policy_start(0,&initial),Err(RuleStop::Accounting {counter:actual,value:0}) if actual == counter)
        );
    }
    for cause in [CancelCause::Requested, CancelCause::Deadline] {
        let system = world_system(&world);
        let mut rules = SourcePlan::new(
            &world.package,
            &world.package.graph().syntheses()[0],
            &system,
            vec![node],
            UNLIMITED,
        )
        .unwrap();
        let MonitorStep::Continue(bytes) = rules.policy_start(0, &initial).unwrap() else {
            panic!("safe initial state")
        };
        let cancel = Cancel::new();
        system.cancel_plan_with(&cancel);
        cancel.cancel(cause);
        assert_eq!(
            rules.policy_advance(0, &bytes, &initial),
            Err(RuleStop::Cancelled(cause))
        );
        assert_eq!(
            rules.policy_accepts(0, &bytes, &initial),
            Err(RuleStop::Cancelled(cause))
        );
    }
}

/// The budgeted model-owned instance collector preserves ordinary ordering,
/// two-instance source bindings and empty-population policy vacuity.
#[test]
#[trace("FR-1011", "FR-1012", "FSpec-FR-803")]
fn positive_source_policy_instances_preserve_order_vacuity_and_original_denials() {
    use filament_exact::{CancelCause, LimitKind, Meter, ScalarLimits};
    use fsl_analyze::plan_source::SourcePlan;
    use fsl_eval::automaton::PolicyStateError;
    for start in [
        vec![
            ("z", json!({"number":{"integer":"0"}})),
            ("a", json!({"number":{"integer":"0"}})),
        ],
        vec![],
    ] {
        let world = native_plan_world_with_clauses(
            POLICY_DIAMOND_MODEL,
            "type Lengths = Int[0,1]; synthesis Reach using t grammar M::Counter domain Lengths satisfies { true };",
            &[model_world::Clause { activation: "on origin", formula: "always holds(c.number >= 0)" }],
            &start,
        );
        let keys: Vec<_> = start.iter().map(|(key, _)| *key).collect();
        let system = ModelSystem::new(world.subject(&keys, UNLIMITED).simulation).unwrap();
        let node = &world.package.graph().temporal_clauses()[0];
        let over = &node.clause().over().1;
        let expected = system.instances(over).unwrap();
        let actual = system
            .instances_with_meter(over, &mut Meter::new(UNLIMITED))
            .unwrap();
        assert_eq!(actual, expected);
        if !start.is_empty() {
            assert_eq!(
                actual
                    .iter()
                    .map(|instance| instance.key.as_str())
                    .collect::<Vec<_>>(),
                ["z", "a"]
            );
            let error = system
                .instances_with_meter(
                    over,
                    &mut Meter::new(ScalarLimits {
                        result_units: 0,
                        ..UNLIMITED
                    }),
                )
                .unwrap_err();
            assert!(
                matches!(error,PolicyStateError::Accounting(denial) if denial.limit_kind == LimitKind::ResultUnits && denial.limit == 0)
            );
        }
        let mut rules = SourcePlan::new(
            &world.package,
            &world.package.graph().syntheses()[0],
            &system,
            vec![node],
            UNLIMITED,
        )
        .unwrap();
        let state = system.initial().remove(0);
        let MonitorStep::Continue(bytes) = rules.policy_start(0, &state).unwrap() else {
            panic!("all instances safe or vacuous")
        };
        assert!(rules.policy_accepts(0, &bytes, &state).unwrap());
        for cause in [CancelCause::Requested, CancelCause::Deadline] {
            let cancel = Cancel::new();
            cancel.cancel(cause);
            let error = system
                .instances_with_meter(over, &mut Meter::new(UNLIMITED).with_cancel(cancel))
                .unwrap_err();
            assert!(matches!(error,PolicyStateError::Cancelled(actual) if actual == cause));
        }
    }
}

/// Positive unsupported forms remain explicit; staged monitor support does
/// not silently discard captures or general liveness.
#[test]
#[trace("FR-1012")]
fn positive_source_policy_stage_boundary_keeps_required_future_forms_explicit() {
    use fsl_analyze::plan_source::{PlanSourceError, SourcePlan};
    for formula in [
        "eventually holds(c.number = 2)",
        "capture n: Int[0,4] = c.number; always holds(c.number >= n)",
        "eventually[1,3] holds(c.number = 2)",
    ] {
        let world = positive_policy_world(formula);
        let system = world_system(&world);
        let result = SourcePlan::new(
            &world.package,
            &world.package.graph().syntheses()[0],
            &system,
            vec![&world.package.graph().temporal_clauses()[0]],
            UNLIMITED,
        );
        assert!(
            matches!(result, Err(PlanSourceError::UnsupportedForm { .. })),
            "{formula}"
        );
    }
}
