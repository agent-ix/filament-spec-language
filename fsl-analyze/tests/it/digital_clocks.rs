// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-204 (TC-639, TC-642): EN-5's digital-clock product over ADR-028
//! §15.5's `Retx`, decided by the finite-horizon solver.

use filament_exact::{Cancel, Integer, Rational};
use fsl_analyze::exact_probabilistic::{
    build_product, decide_bounded_event, witness_scheduler, ActionChoice, EntryChoice,
    ExactProbLimits, FhResult, MonitorVerdict, ProbBuild, ProbProduct, ProductAction, ProductKind,
    ProductRefusal, ProductRequest, SchedulerChoice, WitnessError, WitnessScope,
};
use fsl_analyze::exact_probabilistic::{DeadlinePhase, MonitorState};
use fsl_eval::model_subject::ModelSubject;
use fsl_semantics::check::Bound;
use ix_trace_rs::trace;
use serde_json::json;

use crate::timed_world::{self, World};

fn ratio(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("nonzero")
}

/// `Retx`: `late` is `delivered`, `send(lost)` needs `x >= guard` and resets
/// `x`, the invariant `x <= 2 ms` holds until delivery.
fn retx(guard: &str, reset: bool, declarations: &str, claims: &str) -> World {
    retx_with(
        &[("send", &["late", "x"])],
        guard,
        reset,
        declarations,
        claims,
    )
}

/// The text of [`retx`]'s unit: the claim and declaration lines follow it.
fn retx_body(guard: &str, reset: bool, declarations: &str, claims: &str) -> String {
    let reset = if reset { " and self.x = 0 ms" } else { "" };
    format!(
        "{}\
         pre Send using v on M::Call::send {{ not self.late{guard} }}\n\
         post Sent using v on M::Call::send {{ self.late = not lost{reset} }}\n\
         random M::Call::send(lost) ~ {{ true: 1, false: 9 }};\n\
         time invariant Deadline when {{ not self.late }} {{ self.x <= 2 ms }}\n\
         {declarations}{claims}",
        timed_world::DENSE,
    )
}

/// [`retx`] over the operations `operations`, `send` first.
fn retx_with(
    operations: &[(&str, &[&str])],
    guard: &str,
    reset: bool,
    declarations: &str,
    claims: &str,
) -> World {
    let document = retx_document(operations);
    let body = retx_body(guard, reset, declarations, claims);
    timed_world::world(&document, PROFILES, &body)
}

fn retx_document(operations: &[(&str, &[&str])]) -> Vec<u8> {
    let mut document: serde_json::Value =
        serde_json::from_slice(&timed_world::document(&["x"], operations))
            .expect("the timed fixture is JSON");
    let operation = &mut document["types"][1]["operations"][0];
    operation["params"] = json!([{
        "identity": timed_world::ident("Call/send/lost"),
        "name": "lost",
        "typeRef": "ix://filament/native/Boolean",
        "presence": "required",
        "nullable": false,
        "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": {"source": {"sourceIdentity": timed_world::ident("spec"),
            "path": "spec.fspec", "startLine": 1, "startColumn": 1}},
    }]);
    serde_json::to_vec(&document).expect("serializes")
}

const PROFILES: &str = "profile ev = \"filament.temporal.timed/v1\";\n\
     profile inf = \"filament.temporal.infinite-trace/v1\";\n";

const GUARD: &str = " and self.x >= 1 ms";

fn deadline_claim(name: &str, scheduler: &str, bound: &str, body: &str) -> String {
    format!(
        "probabilistic {name} using ev on M over (c: M::Call) {scheduler} on origin {{ \
         probability {bound} [ {body}[0 ms, 4 ms] holds(c.late) ] }}\n"
    )
}

fn build(world: &World, claim: &str) -> Result<ProbBuild, ProductRefusal> {
    let claim = world
        .package
        .graph()
        .probabilistic_model()
        .claim(claim)
        .expect("the claim checks");
    build_product(
        ProductRequest {
            subject: ModelSubject {
                simulation: world.request(),
                proof_bounds: Vec::new(),
            },
            claim,
            limits: ExactProbLimits::default(),
        },
        &Cancel::new(),
    )
}

fn product(world: &World, claim: &str) -> ProbProduct {
    match build(world, claim) {
        Ok(ProbBuild::Built(product)) => product,
        other => panic!("{claim}: expected a product, got {other:?}"),
    }
}

fn refusal(world: &World, claim: &str) -> ProductRefusal {
    build(world, claim).expect_err("the build is refused")
}

fn initial_value(product: &ProbProduct, bound: Bound) -> Rational {
    let result = decide_bounded_event(product, &bound, &ExactProbLimits::default())
        .expect("the digital product is acyclic");
    let FhResult::Exact { values, .. } = result else {
        panic!("exact values: {result:?}");
    };
    values[product.initial[0].2.index()].clone()
}

fn every_claims() -> String {
    format!(
        "{}{}{}",
        deadline_claim("Deadline", "under every scheduler", ">= 0.99", "eventually"),
        deadline_claim("Max", "under every scheduler", "<= 0.999", "eventually"),
        deadline_claim("Always", "under every scheduler", ">= 0.99", "always"),
    )
}

fn every(guard: &str, reset: bool) -> World {
    retx(guard, reset, "", &every_claims())
}

/// FR-204-AC-1 (TC-639 step 1): over every scheduler the minimum is `99/100`
/// and the maximum `9999/10000`. This is the values half of FR-204-AC-1; the
/// proved/refuted settlement and the refutation path with its replay are not
/// asserted here.
#[trace("TC-639", "FR-204-AC-1")]
#[test]
fn deadline_extrema_over_every_scheduler() {
    let world = every(GUARD, true);
    let built = product(&world, "Deadline");
    assert_eq!(built.kind, ProductKind::Mdp);
    assert_eq!(
        initial_value(&built, Bound::AtLeast(ratio(99, 100))),
        ratio(99, 100)
    );
    assert_eq!(
        initial_value(&built, Bound::AtMost(ratio(999, 1000))),
        ratio(9999, 10000)
    );
    // The scheduler's delays are named unit delays, not an operation.
    assert!(built
        .edges
        .iter()
        .any(|edge| matches!(edge.choice, ProductAction::UnitDelay) && edge.action.is_none()));
}

/// FR-204-AC-4 (TC-639 step 4): a `send` without guard or reset is a
/// zero-delay cycle, and an `always` event has no digital reading.
#[trace("TC-639", "FR-204-AC-4")]
#[test]
#[ignore = "QSL-2156: zero-delay cycles and `always` windows settle verdicts, not yet implemented"]
fn zero_delay_cycle_and_form_shape_settle_verdicts() {
    // Without a guard or a reset `send` loops at the initial state with zero
    // delay; time must diverge, so the cycle's end component is transient.
    let cyclic = every("", false);
    let built = product(&cyclic, "Deadline");
    assert_eq!(
        exact_initial(&built, Bound::AtLeast(ratio(99, 100))),
        Some(ratio(1, 1))
    );

    // An `always` window is the complement of an `eventually` window.
    let world = every(GUARD, true);
    let always = product(&world, "Always");
    assert_eq!(
        exact_initial(&always, Bound::AtLeast(ratio(99, 100))),
        Some(ratio(0, 1))
    );
}

/// Two same-named claims in `claims`: each settles over its own product, and
/// the two claims keep distinct occurrences, so each verdict is reported
/// against its own `{node, occurrence}`.
fn assert_each_timed_form_settles_for_its_claim(claims: &str) {
    let world = retx(GUARD, true, "", claims);
    let checked = world.package.graph().probabilistic_model().claims();
    assert_eq!(checked.len(), 2);
    let mut occurrences = Vec::new();
    for claim in checked {
        let built = build_product(
            ProductRequest {
                subject: ModelSubject {
                    simulation: world.request(),
                    proof_bounds: Vec::new(),
                },
                claim,
                limits: ExactProbLimits::default(),
            },
            &Cancel::new(),
        );
        assert!(
            matches!(built, Ok(ProbBuild::Built(_))),
            "the timed form builds a product: {built:?}"
        );
        occurrences.push(claim.occurrence().cloned());
    }
    assert!(occurrences.iter().all(Option::is_some));
    assert_ne!(
        occurrences[0], occurrences[1],
        "each claim keeps its own identity"
    );
}

/// FR-498-AC-3 (TC-1288 step 3): two claims with the same name whose
/// `always` event is decided as a complement each settle over their own
/// product and keep their own identity.
#[trace("FR-498-AC-3", "TC-1288")]
#[test]
#[ignore = "QSL-2156: `always` timed windows over digital clocks not implemented yet"]
fn timed_form_shape_names_the_selected_claim_among_equal_names() {
    assert_each_timed_form_settles_for_its_claim(&format!(
        "{}{}",
        deadline_claim("Same", "under every scheduler", ">= 0.99", "always"),
        deadline_claim("Same", "under every scheduler", ">= 0.5", "always"),
    ));
}

/// FR-498-AC-3 (TC-1288 step 3): the same for two same-named elapsed-time
/// claims.
#[trace("FR-498-AC-3", "TC-1288")]
#[test]
#[ignore = "QSL-2156: elapsed-time quantile forms over digital clocks not implemented yet"]
fn timed_form_shape_names_the_selected_elapsed_claim_among_equal_names() {
    let elapsed = |threshold: &str| {
        format!(
            "probabilistic Same using ev on M over (c: M::Call) under every scheduler on origin {{ \
             quantile 0.5 of (elapsed from holds(not c.late) until holds(c.late) within 4 ms) \
             <= {threshold} ms }}\n"
        )
    };
    assert_each_timed_form_settles_for_its_claim(&format!("{}{}", elapsed("1"), elapsed("2")));
}

/// FR-204-AC-4: unbounded claims over a model with a zero-delay cycle are
/// decided with time divergence: each builds a product with no monitor.
#[trace("TC-639", "FR-204-AC-4")]
#[test]
#[ignore = "QSL-2156: time-divergent decision of zero-delay cycles not implemented yet"]
fn unbounded_digital_cycles_settle_with_monitorless_state_identity() {
    let world = retx(
        "",
        false,
        "reward sends on M::Call::send = 1;\n",
        concat!(
            "probabilistic Reach using inf on M over (c: M::Call) under every scheduler on origin ",
            "{ probability >= 0.5 [ eventually holds(c.late) ] }\n",
            "probabilistic Cost using inf on M over (c: M::Call) under every scheduler on origin ",
            "{ expected accumulate sends until holds(c.late) <= 5 }\n",
        ),
    );
    for name in ["Reach", "Cost"] {
        let built = product(&world, name);
        assert!(!built.states.is_empty(), "{name}");
        assert!(
            built.states.iter().all(|state| state.monitor.is_none()),
            "{name}"
        );
    }
}

/// FR-204-AC-2 (TC-639 step 2): a strict guard is decided over clock regions.
#[trace("TC-639", "FR-204-AC-2")]
#[test]
#[ignore = "QSL-2156: the region MDP for strict clock constraints not implemented yet"]
fn a_strict_guard_is_decided_over_regions() {
    let world = every(" and self.x > 1 ms", true);
    let built = product(&world, "Deadline");
    assert_eq!(built.kind, ProductKind::Mdp);
    assert_eq!(
        exact_initial(&built, Bound::AtLeast(ratio(99, 100))),
        Some(ratio(99, 100))
    );
    assert_eq!(
        exact_initial(&built, Bound::AtMost(ratio(999, 1000))),
        Some(ratio(999, 1000))
    );
}

fn one_workload(declarations: &str) -> World {
    retx(
        GUARD,
        true,
        &format!("{declarations}workload One on M {{ weight Call::send = 1; }}\n"),
        &format!(
            "{}{}",
            deadline_claim("Under", "under One", ">= 0.99", "eventually"),
            deadline_claim("UnderMax", "under One", "<= 0.999", "eventually"),
        ),
    )
}

/// FR-204-AC-3 (TC-639 step 3): a `discrete` delay under a workload is a DTMC
/// of exact value `159129/160000`, and an `exponential` one under the workload
/// is unsupported for exact evidence. The same `discrete` declaration over
/// every scheduler is decided (`discrete_delay_applies_over_every_scheduler`).
#[trace("TC-639", "FR-204-AC-3")]
#[test]
fn discrete_delays_under_a_workload_are_a_chain() {
    let world = one_workload("delay M::Call::send ~ discrete { 1 ms: 1, 2 ms: 1 };\n");
    let built = product(&world, "Under");
    assert_eq!(built.kind, ProductKind::Dtmc);
    assert_eq!(
        initial_value(&built, Bound::AtLeast(ratio(99, 100))),
        ratio(159_129, 160_000)
    );

    let exponential = one_workload("delay M::Call::send ~ exponential(1);\n");
    assert!(matches!(
        refusal(&exponential, "Under"),
        ProductRefusal::DelayDistribution { .. }
    ));
}

/// FR-204-AC-3 (TC-639): delay admission is scoped to the selected subject
/// model, so its own distribution is refused while another model's is ignored.
#[trace("FR-204-AC-3", "TC-639")]
#[test]
fn delay_admission_is_scoped_to_the_selected_subject_model() {
    const OTHER_PACKAGE: &str = "test/other-timed";
    let subject_delay = retx(
        GUARD,
        true,
        "delay M::Call::send ~ exponential(1);\n",
        &deadline_claim(
            "SubjectDelay",
            "under every scheduler",
            ">= 0.99",
            "eventually",
        ),
    );
    let ProductRefusal::DelayDistribution { operation } = refusal(&subject_delay, "SubjectDelay")
    else {
        panic!("the subject model's delay distribution is unsupported");
    };
    assert!(operation.contains("send"), "{operation}");

    let primary = retx_document(&[("send", &["late", "x"])]);
    let other = String::from_utf8(timed_world::document(&["x"], &[("send", &["late", "x"])]))
        .expect("fixture is UTF-8")
        .replace(timed_world::PACKAGE, OTHER_PACKAGE)
        .into_bytes();
    let body = format!(
        "model Other = \"{OTHER_PACKAGE}\";\ntime Other dense unit ms;\n{}",
        retx_body(
            GUARD,
            true,
            "delay Other::Call::send ~ exponential(1);\n",
            &deadline_claim("Deadline", "under every scheduler", ">= 0.99", "eventually",),
        )
        .replace(
            "time invariant Deadline when { not self.late } { self.x <= 2 ms }\n",
            ""
        ),
    );
    let world = timed_world::world_built_with_packages(&primary, &[&other], PROFILES, &body);
    let built = product(&world, "Deadline");
    assert_eq!(built.kind, ProductKind::Mdp);
    assert!(!built.states.is_empty());
}

/// FR-204-AC-5 (TC-642): under a workload that gives no delay distribution the
/// delay stays the scheduler's: minimum `99/100`, maximum `9999/10000`.
#[trace("TC-642", "FR-204-AC-5")]
#[test]
fn free_delays_under_a_workload_are_decided_by_minimum_and_maximum() {
    let world = one_workload("");
    let built = product(&world, "Under");
    assert_eq!(built.kind, ProductKind::Mdp);
    assert_eq!(
        initial_value(&built, Bound::AtLeast(ratio(99, 100))),
        ratio(99, 100)
    );
    assert_eq!(
        initial_value(&built, Bound::AtMost(ratio(999, 1000))),
        ratio(9999, 10000)
    );
}

/// FR-243-AC-2 (TC-698): over a timed subject, a probabilistic claim that
/// names `exact` evidence routes to EN-5 (the digital-clock route) and one that
/// names `statistical` evidence to EN-4; neither is marked a timed item, which
/// would restrict it to EN-6. The route is a function of the capability kind
/// and the evidence kind alone.
#[trace("TC-698", "FR-243-AC-2")]
#[test]
fn exact_timed_claims_route_to_en5_and_statistical_ones_to_en4() {
    use filament_exact::{Origin, Role};
    use fsl_foundation::source::provenance::OccurrenceKey;
    use fsl_route::request::{items_with_evidence, EvidenceSelection};
    use fsl_semantics::check::{RequirementMap, RequirementRecord};

    let world = retx(
        GUARD,
        true,
        "workload One on M { weight Call::send = 1; }\n",
        &format!(
            "{}probabilistic Sampled using ev on M over (c: M::Call) under One on origin \
             with alpha 0.1, beta 0.1, indifference 0.005 \
             {{ probability >= 0.99 [ eventually[0 ms, 4 ms] holds(c.late) ] }}\n",
            deadline_claim("Deadline", "under every scheduler", ">= 0.99", "eventually")
        ),
    );
    let claims = world.package.graph().probabilistic_model();
    let node = |fill: u8| crate::fixture_ids::node_key([fill]);
    let record = |name: &str, fill: u8| {
        let claim = claims.claim(name).expect("the claim checks");
        RequirementRecord::model_claim(claim.requirements(node(fill)), node(99))
    };
    let exact = record("Deadline", 1);
    let sampled = record("Sampled", 2);
    assert!(!exact.requirements().timed() && !sampled.requirements().timed());
    let records: RequirementMap = [
        (
            OccurrenceKey::new(node(1), Origin::new(Role::new("claim"), 0)),
            exact,
        ),
        (
            OccurrenceKey::new(node(2), Origin::new(Role::new("claim"), 0)),
            sampled,
        ),
    ]
    .into_iter()
    .collect();
    let registry = crate::probabilistic_request_items::registry();
    let items = items_with_evidence(&records, &registry, None, EvidenceSelection::Record);
    let routed: Vec<_> = items
        .iter()
        .map(crate::probabilistic_request_items::candidate_ids)
        .collect();
    assert_eq!(routed, [vec!["en-5".to_owned()], vec!["en-4".to_owned()]]);
}

/// The second operation of the mixed races: it modifies `retried`, has no
/// guard, and is enabled until it has run.
const TICK: &str = "pre Tick using v on M::Call::tick { not self.retried }\n\
     post Ticked using v on M::Call::tick { self.retried = true }\n";

fn with_tick(declarations: &str, claims: &str) -> World {
    retx_with(
        &[("send", &["late", "x"]), ("tick", &["retried"])],
        GUARD,
        true,
        &format!(
            "{TICK}{declarations}workload Both on M {{ weight Call::send = 1; \
             weight Call::tick = 1; }}\n"
        ),
        claims,
    )
}

fn claim_over(name: &str, scheduler: &str, bound: &str, event: &str) -> String {
    format!(
        "probabilistic {name} using ev on M over (c: M::Call) {scheduler} on origin \
         {{ probability {bound} [ {event} ] }}\n"
    )
}

/// The digital clocks and the deadline clock a product key carries, scaled
/// back to model units: `(clock values, deadline clock)`.
fn digital_clocks_of(key: &fsl_analyze::exact_probabilistic::ProbKey) -> (Vec<u64>, Option<u64>) {
    let bytes = &key.model;
    let word = |at: usize| {
        let eight: [u8; 8] = bytes[at..at + 8].try_into().expect("a word");
        u64::from_be_bytes(eight)
    };
    let prefix = b"filament.simulation.state-key/v1/digital\n".len();
    let model = usize::try_from(word(prefix)).expect("a length");
    let mut at = prefix + 8 + model;
    let scale = word(at);
    let count = usize::try_from(word(at + 8)).expect("a count");
    at += 16;
    let mut clocks = Vec::new();
    for _ in 0..count {
        clocks.push(word(at) / scale);
        at += 16;
    }
    let deadline = (bytes[at] == 1).then(|| word(at + 1) / scale);
    (clocks, deadline)
}

/// FR-204-AC-3 and AC-5 (TC-639, TC-642): `tick`'s given 3 ms lies outside
/// its window while `send` waits, so `tick` does not race (ADR-026 SD-2) and
/// the racing delays are all free: the state offers `UnitDelay` and one
/// `WorkloadResolve{send, 0}`, which resolves to `send` only and is a
/// distribution of mass one on its own; no `FreeDelays` row has an empty
/// given vector. The minimum and maximum stay the free delay's `99/100` and
/// `9999/10000`, and a deadline of 0 where the capped deadline clock makes
/// every choice's targets coincide still solves.
#[trace("TC-639", "FR-204-AC-3")]
#[trace("TC-642", "FR-204-AC-5")]
#[test]
fn mixed_given_and_free_races_keep_each_choice_a_distribution() {
    let world = with_tick(
        "delay M::Call::tick ~ discrete { 3 ms: 1 };\n",
        &format!(
            "{}{}{}",
            claim_over(
                "Now",
                "under Both",
                ">= 0.5",
                "eventually[0 ms, 0 ms] holds(c.late)"
            ),
            deadline_claim("Under", "under Both", ">= 0.99", "eventually"),
            deadline_claim("UnderMax", "under Both", "<= 0.999", "eventually"),
        ),
    );
    for name in ["Now", "Under"] {
        let built = product(&world, name);
        let mut mass: std::collections::BTreeMap<(usize, String), Rational> =
            std::collections::BTreeMap::new();
        for edge in &built.edges {
            if let ProductAction::FreeDelays(rows) = &edge.choice {
                assert!(
                    rows.iter().all(|row| !row.given.is_empty()),
                    "{name}: a FreeDelays row names a racing given delay: {rows:?}"
                );
            }
            if let ProductAction::WorkloadResolve { identity, delay } = &edge.choice {
                let total = mass
                    .entry((edge.from.index(), format!("{:?}", edge.choice)))
                    .or_insert_with(|| ratio(0, 1));
                *total = total.add(&edge.probability);
                assert_eq!(*delay, 0, "{name}: the all-free resolution is now");
                assert!(
                    identity.operation.contains("Call/send"),
                    "{name}: {identity:?}"
                );
                assert_eq!(
                    edge.action.as_ref(),
                    Some(identity),
                    "{name}: `tick` does not race, so the workload resolves to `send`"
                );
            }
        }
        // Under `Now` the deadline passes at the first unit delay, before
        // `send` is enabled, so no resolution is reached.
        assert_eq!(
            mass.is_empty(),
            name == "Now",
            "{name}: the free delay is a choice"
        );
        for (choice, total) in &mass {
            assert_eq!(*total, ratio(1, 1), "{name}: {choice:?}");
        }
    }
    let now = product(&world, "Now");
    assert_eq!(
        initial_value(&now, Bound::AtLeast(ratio(1, 2))),
        ratio(0, 1)
    );
    let built = product(&world, "Under");
    assert_eq!(built.kind, ProductKind::Mdp);
    assert_eq!(
        initial_value(&built, Bound::AtLeast(ratio(99, 100))),
        ratio(99, 100)
    );
    assert_eq!(
        initial_value(
            &product(&world, "UnderMax"),
            Bound::AtMost(ratio(999, 1000))
        ),
        ratio(9999, 10000)
    );
}

/// FR-204-AC-3 (TC-639): identities that tie at one delay run in sequence on
/// one edge, and the edge carries the sum of their rewards. FR-204 owns the
/// race; no acceptance criterion states this behaviour.
#[trace("TC-639", "FR-204")]
#[test]
fn tied_identities_sum_their_rewards_on_one_edge() {
    let world = with_tick(
        "delay M::Call::send ~ discrete { 1 ms: 1 };\n\
         delay M::Call::tick ~ discrete { 1 ms: 1 };\n\
         reward cost on M::Call::send = 1;\n\
         reward cost on M::Call::tick = 2;\n",
        &deadline_claim("Tie", "under Both", ">= 0.5", "eventually"),
    );
    let built = product(&world, "Tie");
    let start = built.initial[0].2;
    let race: Vec<_> = built
        .edges
        .iter()
        .filter(|edge| edge.from == start)
        .collect();
    assert!(!race.is_empty());
    for edge in race {
        let cost: Vec<_> = edge
            .rewards
            .iter()
            .filter(|(name, _)| name.as_str() == "cost")
            .map(|(_, value)| value.clone())
            .collect();
        assert!(
            matches!(cost.as_slice(), [filament_exact::Value::Integer(sum)] if *sum == Integer::from(3_i64)),
            "{edge:?}"
        );
    }
}

/// FR-204 "Admission" (TC-639): a `Reach` and an `ExpectedReward` claim over a
/// timed subject label the digital product. The send is forced by the
/// invariant and delivers with probability 9/10, so delivery is certain and
/// the expected number of sends is `10/9` under every scheduler.
#[trace("TC-639", "FR-204")]
#[test]
fn reach_and_expected_reward_label_the_digital_product() {
    use fsl_analyze::exact_probabilistic::{
        decide_expected_reward, decide_reachability, ExtRational, UnboundedObjective,
    };
    let world = retx(
        GUARD,
        true,
        "reward sends on M::Call::send = 1;\n",
        &format!(
            "{}{}",
            "probabilistic Reach using inf on M over (c: M::Call) under every scheduler on origin \
             { probability >= 0.5 [ eventually holds(c.late) ] }\n",
            "probabilistic Cost using inf on M over (c: M::Call) under every scheduler on origin \
             { expected accumulate sends until holds(c.late) <= 5 }\n",
        ),
    );
    for name in ["Reach", "Cost"] {
        let built = product(&world, name);
        let target: Vec<bool> = built.labels.iter().map(|labels| labels[0]).collect();
        assert!(target.iter().any(|reached| *reached), "{name}");
        let start = built.initial[0].2.index();
        for objective in [UnboundedObjective::Min, UnboundedObjective::Max] {
            let limits = ExactProbLimits::default();
            let result = if name == "Reach" {
                decide_reachability(&built, &target, objective, &limits)
            } else {
                decide_expected_reward(&built, &target, "sends", objective, &limits)
            }
            .expect("the labelled product is solved");
            let expected = if name == "Reach" {
                ratio(1, 1)
            } else {
                ratio(10, 9)
            };
            assert_eq!(
                result.values[start],
                ExtRational::Finite(expected),
                "{name}"
            );
        }
    }
}

/// FR-204 "Admission" (TC-639): `holds(A) until[0 ms, D] holds(B)` is a deadline
/// event with the extrema of the `eventually` form.
#[trace("TC-639", "FR-204")]
#[test]
fn an_until_deadline_has_the_same_extrema() {
    let world = retx(
        GUARD,
        true,
        "",
        &format!(
            "{}{}",
            deadline_claim(
                "Until",
                "under every scheduler",
                ">= 0.99",
                "holds(not c.late) until"
            ),
            deadline_claim(
                "UntilMax",
                "under every scheduler",
                "<= 0.999",
                "holds(not c.late) until"
            ),
        ),
    );
    assert_eq!(
        initial_value(&product(&world, "Until"), Bound::AtLeast(ratio(99, 100))),
        ratio(99, 100)
    );
    assert_eq!(
        initial_value(
            &product(&world, "UntilMax"),
            Bound::AtMost(ratio(999, 1000))
        ),
        ratio(9999, 10000)
    );
}

/// FR-204 "Admission" (TC-639): a state where no identity races and delay is
/// bounded is a time-lock and ends its run. The `send` is enabled at `x = 1`,
/// but its only delay, 5 ms, lies outside its window.
#[trace("TC-639", "FR-204")]
#[test]
#[ignore = "QSL-2156: time-locked digital states not read as terminal yet"]
fn a_bounded_state_where_nothing_races_is_a_time_lock() {
    let world = one_workload("delay M::Call::send ~ discrete { 5 ms: 1 };\n");
    assert!(matches!(build(&world, "Under"), Ok(ProbBuild::Built(_))));
}

/// FR-204-AC-1 (TC-639 step 1): the clocks span the AC's ranges, `x` from 0
/// to 2 and the deadline clock from 0 to 5 (one past `D = 4 ms`). This is the
/// clock-range clause of FR-204-AC-1 only.
#[trace("TC-639", "FR-204-AC-1")]
#[test]
fn clocks_span_the_declared_ranges() {
    let world = every(GUARD, true);
    let built = product(&world, "Deadline");
    let mut x = std::collections::BTreeSet::new();
    let mut deadline = std::collections::BTreeSet::new();
    for key in &built.states {
        let (clocks, elapsed) = digital_clocks_of(key);
        x.insert(clocks[0]);
        deadline.extend(elapsed);
    }
    assert_eq!(x.into_iter().collect::<Vec<_>>(), [0, 1, 2]);
    assert_eq!(deadline.into_iter().collect::<Vec<_>>(), [0, 1, 2, 3, 4, 5]);
}

/// FR-204 (TC-642): at the quiescent idle-delayed key at `D`, `IdleObserve`
/// reads the predicates at `D` with elapsed time `D`, and `IdleDelay` crosses
/// `D` to elapsed `D + 1`, which rejects.
/// The predicate never holds, so the success-at-`D` half of FR-204-AC-5 is not
/// asserted: this checks the distinct actions, post-keys and phases only.
#[trace("TC-642", "FR-204")]
#[test]
fn idle_observe_reads_at_the_deadline_and_idle_delay_crosses_it() {
    let world = retx(
        GUARD,
        true,
        "",
        &claim_over(
            "Never",
            "under every scheduler",
            ">= 0.5",
            "eventually[0 ms, 4 ms] holds(c.retried)",
        ),
    );
    let built = product(&world, "Never");
    let at_deadline = built.edges.iter().find(|edge| {
        matches!(edge.choice, ProductAction::IdleObserve)
            && digital_clocks_of(&built.states[edge.from.index()]).1 == Some(4)
    });
    let observe = at_deadline.expect("an observation at D");
    let source = observe.from;
    let delay = built
        .edges
        .iter()
        .find(|edge| edge.from == source && matches!(edge.choice, ProductAction::IdleDelay))
        .expect("a delay across D from the same key");
    let observed = &built.states[observe.to.index()];
    let crossed = &built.states[delay.to.index()];
    assert_eq!(digital_clocks_of(observed).1, Some(4));
    assert_eq!(digital_clocks_of(crossed).1, Some(5));
    let phase = |key: &fsl_analyze::exact_probabilistic::ProbKey| match key.monitor {
        Some(MonitorState::Deadline { phase, .. }) => phase,
        other => panic!("a deadline monitor: {other:?}"),
    };
    assert_eq!(phase(observed), DeadlinePhase::Pending);
    assert_eq!(phase(crossed), DeadlinePhase::Reject);
}

/// FR-202 (TC-642, FR-204): a randomized entry at a plain MDP state, where
/// the scheduler chooses between `UnitDelay` and `WorkloadResolve`, is bounded
/// by the transition limit: two alternatives pass a limit of 2 and refuse a
/// limit of 1.
#[trace("TC-642", "FR-204")]
#[test]
fn a_randomized_mdp_entry_is_bounded_by_the_transition_limit() {
    let world = one_workload("");
    let built = product(&world, "Under");
    let choosing = built
        .ids()
        .find(|state| {
            built
                .outgoing(*state)
                .iter()
                .any(|edge| matches!(edge.choice, ProductAction::WorkloadResolve { .. }))
        })
        .expect("a state offers WorkloadResolve beside UnitDelay");
    let mut policy: Vec<Option<SchedulerChoice>> =
        vec![Some(SchedulerChoice::Deterministic(ActionChoice(0))); built.states.len()];
    policy[choosing.index()] = Some(SchedulerChoice::Randomized(vec![
        (ActionChoice(0), ratio(1, 2)),
        (ActionChoice(1), ratio(1, 2)),
    ]));
    let mut accepting = vec![false; built.states.len()];
    for (state, verdict) in &built.decided {
        accepting[state.index()] = *verdict == MonitorVerdict::Accept;
    }
    let capped = |max: usize| {
        let mut limits = ExactProbLimits::default();
        limits.model.limits.max_transitions = max;
        limits
    };
    let entries = |max: usize| {
        witness_scheduler(
            &built,
            &policy,
            WitnessScope::Workload,
            built.initial[0].2,
            &accepting,
            &capped(max),
        )
    };
    let admitted = entries(2).expect("two alternatives fit a limit of two");
    let randomized = admitted
        .iter()
        .find(|(state, _)| *state == choosing)
        .expect("the choosing state has an entry");
    let EntryChoice::Randomized(alternatives) = &randomized.1 else {
        panic!("a randomized entry: {randomized:?}");
    };
    assert_eq!(alternatives.len(), 2);
    assert!(matches!(
        entries(1),
        Err(WitnessError::TooManyAlternatives { state }) if state == choosing.index()
    ));
}

/// FR-740-AC-3 (TC-2062), FSpec FR-640-AC-2 (TC-1563): a `time dense` unit
/// with `discrete` delays emits one `probabilistic`/`delay_distribution`
/// node per delayed operation, of family `discrete`.
#[trace("TC-2062", "FR-740-AC-3")]
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-2")]
#[test]
#[ignore = "QSL-2000: delay declarations emitted as v2 probabilistic nodes not implemented yet"]
fn discrete_delays_are_emitted_as_delay_distribution_nodes() {
    let world = with_tick(
        "delay M::Call::send ~ discrete { 1 ms: 90, 3 ms: 10 };\n",
        &deadline_claim("Late", "under Both", ">= 0.5", "eventually"),
    );
    let emission = fsl_package::emit_checked(&world.package, fsl_package::EmitLimits::fixture())
        .unwrap_or_else(|refusal| panic!("S4 emits the timed unit: {refusal:?}"));
    let wire: serde_json::Value =
        serde_json::from_slice(emission.package().bytes()).expect("JSON bytes");
    let delays: Vec<&serde_json::Value> = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .filter(|node| {
            node["node_tag"] == "probabilistic" && node["semantic_form"] == "delay_distribution"
        })
        .collect();
    let [delay] = delays[..] else {
        panic!("one delay_distribution node: {delays:?}");
    };
    assert!(delay["body"].to_string().contains("discrete"), "{delay}");
}

/// FSpec FR-420-AC-8 (TC-1561): under a workload that gives `tick` weight 0,
/// the unit checks, its digital product builds, and the probabilities
/// leaving every product state sum to 1: a tie with the weight-0 `tick`
/// orders it after `send` with no probability lost to the order draw.
#[trace("FSpec-TC-1561", "FSpec-FR-420-AC-8")]
#[test]
#[ignore = "QSL-1991: weight-0 operations in timed race ties not implemented yet"]
fn a_weight_zero_operation_takes_its_turn_last_in_a_tie() {
    let world = retx_with(
        &[("send", &["late", "x"]), ("tick", &["retried"])],
        GUARD,
        true,
        &format!(
            "{TICK}delay M::Call::send ~ discrete {{ 1 ms: 1 }};\n\
             delay M::Call::tick ~ discrete {{ 1 ms: 1 }};\n\
             workload Lean on M {{ weight Call::send = 1; weight Call::tick = 0; }}\n"
        ),
        &deadline_claim("Tie", "under Lean", ">= 0.5", "eventually"),
    );
    let built = product(&world, "Tie");
    let mut totals: std::collections::BTreeMap<usize, Rational> = Default::default();
    for edge in &built.edges {
        let total = totals
            .entry(edge.from.index())
            .or_insert_with(|| ratio(0, 1));
        *total = total.add(&edge.probability);
    }
    assert!(!totals.is_empty());
    for (state, total) in totals {
        assert_eq!(total, ratio(1, 1), "state {state}");
    }
}

/// The exact value the finite-horizon solver gives `product`'s initial state
/// for `bound`, or `None` when the solver gives no exact value.
fn exact_initial(product: &ProbProduct, bound: Bound) -> Option<Rational> {
    match decide_bounded_event(product, &bound, &ExactProbLimits::default()) {
        Ok(FhResult::Exact { values, .. }) => Some(values[product.initial[0].2.index()].clone()),
        _ => None,
    }
}

/// FR-204-AC-13 (TC-2172): a strict guard is decided over the region MDP.
#[trace("TC-2172", "FR-204-AC-13")]
#[test]
#[ignore = "QSL-2156: the region MDP for strict clock constraints not implemented yet"]
fn strict_clock_constraint_is_decided_over_regions() {
    let world = every(" and self.x > 1 ms", true);
    let built = product(&world, "Deadline");
    assert_eq!(built.kind, ProductKind::Mdp);
    assert_eq!(
        exact_initial(&built, Bound::AtLeast(ratio(99, 100))),
        Some(ratio(99, 100))
    );
    assert_eq!(
        exact_initial(&built, Bound::AtMost(ratio(999, 1000))),
        Some(ratio(999, 1000))
    );
}

/// FR-204-AC-14 (TC-2173): an `always` window is the complement of an
/// `eventually` window; `c.late` is false initially, so the minimum is 0.
#[trace("TC-2173", "FR-204-AC-14")]
#[test]
#[ignore = "QSL-2156: the `always` timed window over digital clocks not implemented yet"]
fn always_window_is_decided_as_a_complement() {
    let world = every(GUARD, true);
    let built = product(&world, "Always");
    assert_eq!(
        exact_initial(&built, Bound::AtLeast(ratio(99, 100))),
        Some(ratio(0, 1))
    );
}

/// FR-204-AC-15 (TC-2174): a `discrete` delay applies over every scheduler.
#[trace("TC-2174", "FR-204-AC-15")]
#[trace("TC-639", "FR-204-AC-3")]
#[test]
#[ignore = "QSL-2156: delay distributions over every scheduler not implemented yet"]
fn discrete_delay_applies_over_every_scheduler() {
    let world = retx(
        GUARD,
        true,
        "delay M::Call::send ~ discrete { 1 ms: 1, 2 ms: 1 };\n",
        &every_claims(),
    );
    assert_eq!(product(&world, "Deadline").kind, ProductKind::Mdp);
}

/// FR-204-AC-16 (TC-2175): a zero-delay cycle is decided with time
/// divergence instead of refused.
#[trace("TC-2175", "FR-204-AC-16")]
#[test]
#[ignore = "QSL-2156: time-divergent decision of zero-delay cycles not implemented yet"]
fn zero_delay_cycle_is_decided_with_time_divergence() {
    let world = every("", false);
    let built = product(&world, "Deadline");
    for bound in [
        Bound::AtLeast(ratio(99, 100)),
        Bound::AtMost(ratio(999, 1000)),
    ] {
        assert_eq!(exact_initial(&built, bound), Some(ratio(1, 1)));
    }
}

/// FR-204-AC-17 (TC-2176): a time-locked state is terminal.
#[trace("TC-2176", "FR-204-AC-17")]
#[test]
#[ignore = "QSL-2156: time-locked digital states not read as terminal yet"]
fn time_locked_state_is_terminal() {
    let world = one_workload("delay M::Call::send ~ discrete { 5 ms: 1 };\n");
    assert!(matches!(build(&world, "Under"), Ok(ProbBuild::Built(_))));
}

/// FR-186-AC-12 (TC-2169): over the timed `Retx` subject `expected elapsed`,
/// an `elapsed` measure and a time window each check.
#[trace("TC-2169", "FR-186-AC-12")]
#[test]
#[ignore = "QSL-2152: time forms of probabilistic claims not checked yet"]
fn time_forms_over_a_timed_model_check() {
    let claims = "probabilistic MeanLate using ev on M over (c: M::Call) under every scheduler on origin \
         { expected elapsed until holds(c.late) <= 3 ms }\n\
         probabilistic ElapsedLate using ev on M over (c: M::Call) under every scheduler on origin \
         { probability >= 0.5 [ elapsed from holds(not c.late) until holds(c.late) within 5 ms >= 1 ms ] }\n\
         probabilistic StepsLate using ev on M over (c: M::Call) under every scheduler on origin \
         { probability >= 0.5 [ steps from holds(not c.late) until holds(c.late) within 5 ms >= 1 ] }\n";
    let world = retx(GUARD, true, "", claims);
    for name in ["MeanLate", "ElapsedLate", "StepsLate"] {
        assert!(
            world
                .package
                .graph()
                .probabilistic_model()
                .claim(name)
                .is_some(),
            "{name} checks"
        );
    }
}
