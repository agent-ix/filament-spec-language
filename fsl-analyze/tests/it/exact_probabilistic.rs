// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-196 (TC-631, TC-641): EN-5's probabilistic product, over ADR-024 §7's
//! and ADR-028 §15's models (see [`crate::prob_world`]).

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use filament_exact::{Cancel, CancelCause, Integer, Rational, Value};
use filament_semantic_value::catalog::UndefinedReason;
use fsl_analyze::exact_probabilistic::{
    backward_induction, build_product, decide_bounded_event, decide_fair_long_run_mdp,
    decide_fair_reachability, maximal_fair_end_components, Accumulator, Amount, ExactProbLimits,
    FhObjective, FhResult, FormulaState, MeasurePhase, MonitorState, MonitorVerdict, PredicateRole,
    ProbBuild, ProbEdge, ProbKey, ProbProduct, ProbStateId, ProductAction, ProductKind,
    ProductRefusal, ProductRequest, ProductStop, UnboundedObjective, UndefinedRead,
};
use fsl_eval::model_subject::ModelSubject;
use fsl_eval::simulation::Limits;
use fsl_eval::simulation::TransitionSystem;
use fsl_eval::temporal::UndefinedCause;
use fsl_replay::{ModelCheckLimit, ModelCheckLimits};
use ix_trace_rs::trace;

use crate::prob_world::{self, World};

fn ratio(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("nonzero")
}

fn dyadic(value: &fsl_analyze::exact_probabilistic::Dyadic) -> Rational {
    Rational::from_integer(Integer::from_big(value.mantissa.clone()))
        .divided_by_power_of_two(u64::from(value.exponent))
}

fn integer_of(value: &Value) -> i64 {
    match value {
        Value::Integer(integer) => integer.to_string().parse().expect("small"),
        other => panic!("not an integer: {other:?}"),
    }
}

fn boolean_of(value: &Value) -> bool {
    match value {
        Value::Boolean(flag) => *flag,
        other => panic!("not a Boolean: {other:?}"),
    }
}

fn built(world: &World, claim: &str) -> ProbProduct {
    match build_product(
        world.request(claim, ModelCheckLimits::default()),
        &Cancel::new(),
    ) {
        Ok(ProbBuild::Built(product)) => product,
        other => panic!("{claim}: expected a product, got {other:?}"),
    }
}

/// FR-254-AC-3 (TC-709): an exact request for the strict-guard timed RPC is
/// decided by the exact timed route over clock regions, never admitted as
/// untimed.
#[trace("TC-709", "FR-254-AC-3")]
#[trace("TC-639", "FR-204-AC-2")]
#[test]
fn exact_request_over_strict_guard_timed_rpc_is_decided_over_regions() {
    let document =
        crate::timed_world::document(&["x"], &[("send", &["phase"]), ("reply", &["phase"])]);
    let body = format!(
        "{}\
         pre Waiting using v on M::Call::send {{ self.phase = 0 }}\n\
         post Sent using v on M::Call::send {{ self.phase = 1 }}\n\
         pre StrictReply using v on M::Call::reply {{ self.phase = 1 and self.x > 3 ms }}\n\
         post Replied using v on M::Call::reply {{ self.phase = 2 }}\n\
         delay M::Call::send ~ discrete {{ 0 ms: 1 }};\n\
         delay M::Call::reply ~ discrete {{ 1 ms: 1 }};\n\
         workload Even on M {{ weight Call::send = 1; weight Call::reply = 1; }}\n\
         probabilistic StrictExact using ev on M over (c: M::Call) under Even on origin\n\
             with alpha 0.4, beta 0.4, indifference 0.005\n\
             {{ probability <= 0.01 [ eventually[0 ms, 4 ms] holds(c.phase = 2) ] }}\n",
        crate::timed_world::DENSE,
    );
    let world = crate::timed_world::world(
        &document,
        "profile ev = \"filament.temporal.timed/v1\";\n",
        &body,
    );
    let claim = world
        .package
        .graph()
        .probabilistic_model()
        .claim("StrictExact")
        .expect("the strict timed claim checks");
    let refusal = build_product(
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
    // FR-204-AC-2: the strict guard `x > 3 ms` is read through clock regions.
    assert!(
        matches!(refusal, Ok(ProbBuild::Built(_))),
        "the strict-guard request builds a product: {refusal:?}"
    );
}

/// FR-196-AC-13: a timed product's undefined key keeps its pre-read monitor,
/// model state, and digital clock/deadline control after the actual delay.
#[trace("FR-196-AC-13", "TC-641")]
#[test]
fn a_timed_undefined_read_keeps_its_complete_pre_read_product_key() {
    let mut document: serde_json::Value =
        serde_json::from_slice(&crate::timed_world::document(&["x"], &[("drop", &[])]))
            .expect("the timed fixture emits JSON");
    let operation = &mut document["types"][1]["operations"][0];
    operation["params"] = serde_json::json!([{
        "identity": crate::timed_world::ident("Call/drop/b"),
        "name": "b",
        "typeRef": crate::timed_world::ident("Phase"),
        "presence": "required",
        "nullable": false,
        "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": {"generated": {
            "generatorIdentity": crate::timed_world::ident("Call/drop/b"),
            "generatorVersion": "1.0.0",
            "inputIdentities": [crate::timed_world::ident("Call/drop/b")]
        }}
    }]);
    operation["frame"]["deletes"] =
        serde_json::json!([crate::timed_world::ident("Call")]);
    let body = format!(
        "{}\
         post Drop using v on M::Call::drop {{ (b = 0 and count(o in M::calls where true) = 0) or (b = 1 and count(o in M::calls where true) = 1) }}\n\
         random Call::drop(b) ~ {{ 0: 1, 1: 1 }};\n\
         delay M::Call::drop ~ discrete {{ 1 ms: 1 }};\n\
         workload Even on M {{ weight Call::drop = 1; }}\n\
         probabilistic UndefinedTimed using timed on M over (c: M::Call) under Even on origin\n\
             with alpha 0.4, beta 0.4, indifference 0.005\n\
             {{ probability >= 0.5 [ always[0 ms, 3 ms] holds(c.phase = 0) ] }}\n",
        crate::timed_world::DENSE,
    );
    let world = crate::timed_world::world_holding(
        &document.to_string().into_bytes(),
        "profile timed = \"filament.temporal.timed/v1\";\n",
        &body,
        &serde_json::json!({"x": {"integer": "0"}}),
    );
    let claim = world
        .package
        .graph()
        .probabilistic_model()
        .claim("UndefinedTimed")
        .expect("the timed undefined claim checks");
    let request = ProductRequest {
        subject: ModelSubject {
            simulation: world.request(),
            proof_bounds: Vec::new(),
        },
        claim,
        limits: ExactProbLimits::default(),
    };
    let ProbBuild::Undefined(path) = build_product(request, &Cancel::new())
        .expect("the timed product reports its undefined read")
    else {
        panic!("the timed product returns an undefined path");
    };
    let [step] = path.steps.as_slice() else {
        panic!("one delayed drop reaches the undefined read: {:?}", path.steps);
    };
    assert_eq!(path.position, 1);
    assert_eq!(path.probability, ratio(1, 2));
    assert_eq!(step.probability, ratio(1, 2));
    assert_eq!(step.target, path.key);
    assert!(matches!(path.read, UndefinedRead::Atom(_)));
    assert!(path.key.monitor.is_some(), "the bounded control is retained");
    assert!(path.key.accumulator.is_none());
    assert!(path.key.pending.is_none());
    let timed_key = path
        .key
        .certificate_state
        .as_ref()
        .expect("the product key carries its independently checkable timed state");
    assert!(fsl_replay::digital_key_clocks(&world.system(), claim, timed_key).is_some(),
        "the pre-read key carries the delayed clock valuation"
    );
}

fn refused(world: &World, claim: &str) -> ProductRefusal {
    match build_product(
        world.request(claim, ModelCheckLimits::default()),
        &Cancel::new(),
    ) {
        Err(refusal) => refusal,
        other => panic!("{claim}: expected a refusal, got {other:?}"),
    }
}

/// FR-195-AC-4: EN-4 refuses each exact-only class with its first applicable
/// cause, while a confidence-bounded quantile claim passes negotiation.
#[trace("TC-630", "FR-195-AC-4")]
#[test]
fn the_statistical_gate_returns_each_exact_only_cause() {
    use fsl_analyze::statistical::require_statistical;
    use fsl_replay::statistical::StatisticalRefusal;

    let workload = World::new(&prob_world::link(
        "probabilistic Cost using ev on M over (m: M::Msg) under Even on origin { expected accumulate cost until holds(m.delivered) <= rational(6, 5) }\n\
         probabilistic Bounded using ev on M over (m: M::Msg) under Even on origin { probability >= 0.5 [ eventually[0, 2] holds(m.delivered) ] }\n\
         probabilistic P95 using ev on M over (m: M::Msg) under Steady on origin with alpha 0.01, beta 0.01, indifference 0.01 { quantile 0.95 of (accumulate cost from holds(m.attempts = 0) until holds(m.delivered) within 2) <= 3 }\n\
         workload Steady on M { weight Msg::send_a = 1; weight Msg::send_b = 1; }",
    ));
    let every = World::new(&prob_world::process(
        "probabilistic FairTerminates using inf on M over (p: M::Proc) under every scheduler on origin { probability >= 0.99 [ eventually holds(p.done) ] }",
    ));

    assert_eq!(
        require_statistical(every.claim("FairTerminates")),
        Err(fsl_replay::statistical::StatisticalRefusal::EveryScheduler)
    );
    assert_eq!(
        require_statistical(workload.claim("Cost")),
        Err(StatisticalRefusal::ExactOnlyForm)
    );
    assert_eq!(
        require_statistical(workload.claim("Bounded")),
        Err(StatisticalRefusal::MissingConfidence)
    );
    assert_eq!(require_statistical(workload.claim("P95")), Ok(()));
}

/// FR-195-AC-8: simultaneous EN-4 gates select the specified first refusal,
/// before claim reduction or sampler admission; the confidence-bounded P95
/// passes the gate and proceeds as far as the deliberately invalid sampler.
#[trace("TC-630", "FR-195-AC-8")]
#[test]
fn statistical_gate_precedence_precedes_claim_reduction_and_sampling() {
    use fsl_analyze::statistical::{check_statistical, StatisticalRequest};
    use fsl_foundation::selection::DefinitionRef;
    use fsl_replay::statistical::{StatisticalLimits, StatisticalMethod, StatisticalRefusal};

    let workload = World::new(&prob_world::link(
        "probabilistic Cost using ev on M over (m: M::Msg) under Even on origin { expected accumulate cost until holds(m.delivered) <= rational(6, 5) }\n\
         probabilistic Bounded using ev on M over (m: M::Msg) under Even on origin { probability >= 0.5 [ eventually[0, 2] holds(m.delivered) ] }\n\
         probabilistic P95 using ev on M over (m: M::Msg) under Steady on origin with alpha 0.01, beta 0.01, indifference 0.01 { quantile 0.95 of (accumulate cost from holds(m.attempts = 0) until holds(m.delivered) within 2) <= 3 }\n\
         workload Steady on M { weight Msg::send_a = 1; weight Msg::send_b = 1; }",
    ));
    let every = World::new(&prob_world::process(
        "probabilistic FairTerminates using inf on M over (p: M::Proc) under every scheduler on origin { probability >= 0.99 [ eventually holds(p.done) ] }",
    ));
    let invalid_sampler = DefinitionRef::new("agent-ix", "not-a-sampler/v1").unwrap();

    for (world, name, expected) in [
        (&every, "FairTerminates", StatisticalRefusal::EveryScheduler),
        (&workload, "Cost", StatisticalRefusal::ExactOnlyForm),
        (&workload, "Bounded", StatisticalRefusal::MissingConfidence),
    ] {
        let refusal = check_statistical(
            StatisticalRequest {
                subject: world.subject(),
                claim: world.claim(name),
                method: StatisticalMethod::Okamoto,
                seed: 7,
                sampler: invalid_sampler.clone(),
                limits: StatisticalLimits::default(),
                max_witnesses: 1,
                symmetry: Vec::new(),
                timed_sampling: fsl_replay::statistical::TimedSampling::default(),
            },
            &Cancel::new(),
        );
        assert_eq!(refusal, Err(expected));
    }

    let passed_gate = check_statistical(
        StatisticalRequest {
            subject: workload.subject(),
            claim: workload.claim("P95"),
            method: StatisticalMethod::Okamoto,
            seed: 7,
            sampler: invalid_sampler.clone(),
            limits: StatisticalLimits::default(),
            max_witnesses: 1,
            symmetry: Vec::new(),
            timed_sampling: fsl_replay::statistical::TimedSampling::default(),
        },
        &Cancel::new(),
    );
    assert!(matches!(
        passed_gate,
        Err(StatisticalRefusal::GeneratorMismatch { supplied }) if supplied == invalid_sampler
    ));
}

/// FR-195-AC-5: EN-5 advertises only exact unbounded probabilistic evidence,
/// and FR-192 maps each EN-4 gate refusal to unsupported with its cause.
#[trace("TC-630", "FR-195-AC-5")]
#[test]
fn exact_probability_manifest_and_statistical_refusal_settlement_are_exact() {
    use fsl_analyze::exact_probabilistic::manifest;
    use fsl_analyze::statistical::manifest as statistical_manifest;
    use fsl_foundation::bound::DomainKind;
    use fsl_replay::statistical::{settle_statistical_refusal, StatisticalRefusal};
    use fsl_replay::{StatisticalRequestRefusal, TerminalValue, UnavailabilityCause};
    use fsl_route::{EvidenceKind, Mode, ProviderOrigin};

    let exact = manifest::descriptor(ProviderOrigin::Linked).unwrap();
    assert_eq!(
        exact.advertises().collect::<Vec<_>>(),
        [(
            fsl_semantics::check::Capability::ProbabilisticSatisfaction,
            Mode::Unbounded,
        )]
    );
    assert_eq!(
        exact.evidence_kinds(),
        &BTreeSet::from([EvidenceKind::Exact])
    );
    assert_eq!(
        exact.domains(),
        Some(&BTreeSet::from([
            DomainKind::Population,
            DomainKind::Integer
        ]))
    );
    assert_eq!(
        statistical_manifest::descriptor(ProviderOrigin::Linked)
            .unwrap()
            .evidence_kinds(),
        &BTreeSet::from([EvidenceKind::Statistical])
    );

    for (refusal, cause) in [
        (
            StatisticalRefusal::EveryScheduler,
            StatisticalRequestRefusal::EveryScheduler,
        ),
        (
            StatisticalRefusal::ExactOnlyForm,
            StatisticalRequestRefusal::ExactOnlyForm,
        ),
        (
            StatisticalRefusal::MissingConfidence,
            StatisticalRequestRefusal::MissingConfidence,
        ),
    ] {
        let settled = settle_statistical_refusal(&refusal).expect("request refusal settles");
        assert_eq!(
            settled.value,
            TerminalValue::Unsupported(UnavailabilityCause::StatisticalRequest(cause))
        );
        assert_eq!(settled.diagnostic, None);
    }
}

/// FR-195-AC-6 (TC-1163): exact claims have a claim-occurrence-keyed
/// probabilistic requirement with its exact evidence and full record shape.
#[trace("TC-1163")]
#[test]
fn exact_probabilistic_requirement_records_keep_occurrence_and_evidence() {
    use fsl_foundation::bound::{DomainKey, DomainKind};
    use fsl_semantics::check::Capability;
    use fsl_semantics::check::RequirementRecord;
    use fsl_semantics::family::{ClaimEvidence, ClaimExtent};
    use std::collections::BTreeMap;

    let world = World::new(&prob_world::process(
        "probabilistic FairTerminates using inf on M over (p: M::Proc) under every scheduler on origin { probability >= 0.99 [ eventually holds(p.done) ] }",
    ));
    let claim = world.claim("FairTerminates");
    let occurrence = claim.occurrence().expect("the package claim occurrence");
    let boolean = crate::fixture_ids::node_key([99]);
    let record = RequirementRecord::model_claim(
        claim.requirements(occurrence.node().clone()),
        boolean.clone(),
    );
    let requirements = record.requirements();
    assert_eq!(requirements.kind(), Capability::ProbabilisticSatisfaction);
    assert_eq!(requirements.evidence(), Some(ClaimEvidence::Exact));
    assert_eq!(
        requirements.extent(),
        &ClaimExtent::from_domains(BTreeMap::from([(
            DomainKey::Node {
                node: occurrence.node().clone(),
                path: Vec::new()
            },
            DomainKind::InfiniteTrace,
        )]))
    );
    assert_eq!(record.result_bound().node(), boolean);
    assert!(record.conversion_context().is_none());
    assert!(record.path_condition().is_empty());
}

/// The probabilities of `edges`, summed.
fn total(edges: &[ProbEdge]) -> Rational {
    edges
        .iter()
        .fold(ratio(0, 1), |sum, edge| sum.add(&edge.probability))
}

/// The one state of `product` with no incoming edge and every initial state
/// of the subject: the product has one.
fn start(product: &ProbProduct) -> ProbStateId {
    let [(_, _, state)] = &product.initial[..] else {
        panic!("one initial state: {:?}", product.initial);
    };
    *state
}

/// Each state's breadth-first depth from `from`, by its index.
fn depths(product: &ProbProduct, from: ProbStateId) -> BTreeMap<usize, usize> {
    let mut depth = BTreeMap::from([(from.index(), 0)]);
    let mut queue = VecDeque::from([from]);
    while let Some(at) = queue.pop_front() {
        let here = depth[&at.index()];
        for edge in product.outgoing(at) {
            if let Entry::Vacant(slot) = depth.entry(edge.to.index()) {
                slot.insert(here + 1);
                queue.push_back(edge.to);
            }
        }
    }
    depth
}

/// `items` in ascending order, each once: a set whose members hold exact
/// rationals, which a `BTreeSet` key may not.
fn distinct<T: Ord>(mut items: Vec<T>) -> Vec<T> {
    items.sort();
    items.dedup();
    items
}

fn is_decided(product: &ProbProduct, state: ProbStateId) -> bool {
    product.states[state.index()]
        .monitor
        .is_some_and(MonitorState::is_decided)
}

fn measure_event_terminals(product: &ProbProduct, threshold: &Rational) -> Vec<Option<Rational>> {
    product
        .states
        .iter()
        .map(|key| match (key.monitor, key.accumulator.as_ref()) {
            (
                Some(MonitorState::Measure {
                    phase:
                        MeasurePhase::Complete | MeasurePhase::Censored | MeasurePhase::Unactivated,
                    ..
                }),
                Some(Accumulator::Reward { activated, amount })
                | Some(Accumulator::Steps { activated, amount }),
            ) => {
                let holds =
                    *activated && matches!(amount, Amount::Exact(value) if value <= threshold);
                Some(if holds { ratio(1, 1) } else { ratio(0, 1) })
            }
            _ => None,
        })
        .collect()
}

fn quantile_margin_terminals(
    product: &ProbProduct,
    threshold: &Rational,
    level: &Rational,
) -> Vec<Option<Rational>> {
    product
        .states
        .iter()
        .map(|key| match (key.monitor, key.accumulator.as_ref()) {
            (
                Some(MonitorState::Measure {
                    phase:
                        MeasurePhase::Complete | MeasurePhase::Censored | MeasurePhase::Unactivated,
                    ..
                }),
                Some(Accumulator::Reward { activated, amount })
                | Some(Accumulator::Steps { activated, amount }),
            ) => {
                let contribution = if *activated {
                    let event = matches!(amount, Amount::Exact(value) if value <= threshold);
                    if event { ratio(1, 1) } else { ratio(0, 1) }.sub(level)
                } else {
                    ratio(0, 1)
                };
                Some(contribution)
            }
            _ => None,
        })
        .collect()
}

fn selected_operation(product: &ProbProduct, state: ProbStateId, choice: usize) -> String {
    let mut identities = Vec::new();
    for edge in product.outgoing(state) {
        let identity = edge.action.as_ref().expect("a model action");
        if !identities.iter().any(|seen| seen == identity) {
            identities.push(identity.clone());
        }
    }
    identities
        .get(choice)
        .expect("the policy selects an enabled action")
        .operation
        .clone()
}

const NO_FAULT: &str = "probabilistic NoFault using ev on M over (n: M::Node) under Steady \
     on origin { probability >= 0.999 [ always[0,1000] holds(n.healthy) ] }";

/// Checks FR-197-AC-3 (TC-632 step 3): exact backward induction keeps the
/// §15.1 rational while it fits, then outward dyadic rounding encloses it.
#[trace("TC-632", "FR-197-AC-3")]
#[test]
fn no_fault_uses_exact_then_dyadic_backward_induction() {
    let world = World::new(&prob_world::no_fault(NO_FAULT));
    let product = built(&world, "NoFault");
    let start = start(&product);
    let exact = decide_bounded_event(
        &product,
        &fsl_semantics::check::Bound::AtLeast(ratio(999, 1000)),
        &ExactProbLimits {
            max_rational_bits: 23_254,
            ..ExactProbLimits::default()
        },
    )
    .expect("acyclic bounded product");
    let FhResult::Exact { values, .. } = exact else {
        panic!("the published rational budget keeps NoFault exact");
    };
    let expected = ratio(9_999_999, 10_000_000)
        .pow(&Integer::from(1000_i64))
        .expect("positive integer exponent");
    assert_eq!(values[start.index()], expected);

    let bounded = decide_bounded_event(
        &product,
        &fsl_semantics::check::Bound::AtLeast(ratio(999, 1000)),
        &ExactProbLimits {
            max_rational_bits: 1024,
            ..ExactProbLimits::default()
        },
    )
    .expect("dyadic fallback");
    let FhResult::Bounds {
        lower,
        upper,
        precision_bits,
        ..
    } = bounded
    else {
        panic!("the interval clears the threshold at 64 bits");
    };
    let start_lower = dyadic(&lower[start.index()]);
    let start_upper = dyadic(&upper[start.index()]);
    assert!(start_lower <= expected && expected <= start_upper);
    assert!(start_lower > ratio(999, 1000));
    assert_eq!(precision_bits, 64);
    let maximum_width = ratio(2 * 1001, 1).divided_by_power_of_two(64);
    assert!(start_upper.sub(&start_lower) <= maximum_width);
}

/// Checks FR-196-AC-1 (TC-631 step 1): §15.1's `NoFault` product is a DTMC whose
/// undecided states hold the healthy model state at one position each, at
/// most 1,001 of them, plus one accepting and one rejecting decided state.
/// Every undecided state's outgoing probabilities sum to 1, and every path
/// reaches a decided state within 1,001 positions.
#[trace("TC-631", "FR-196-AC-1")]
#[test]
fn the_no_fault_product_is_a_layered_dtmc_with_two_decided_states() {
    let world = World::new(&prob_world::no_fault(NO_FAULT));
    let product = built(&world, "NoFault");
    assert_eq!(product.kind, ProductKind::Dtmc);
    let decided: BTreeSet<MonitorVerdict> = product
        .decided
        .iter()
        .map(|(_, verdict)| *verdict)
        .collect();
    assert_eq!(product.decided.len(), 2);
    assert_eq!(
        decided,
        BTreeSet::from([MonitorVerdict::Accept, MonitorVerdict::Reject])
    );
    let undecided: Vec<ProbStateId> = product
        .ids()
        .filter(|state| !product.outgoing(*state).is_empty())
        .collect();
    assert!(
        undecided.len() <= 1001,
        "{} undecided states",
        undecided.len()
    );
    assert_eq!(
        undecided.len() + product.decided.len(),
        product.states.len()
    );
    // One position each: the monitor state differs at every undecided state.
    let monitors: BTreeSet<_> = undecided
        .iter()
        .map(|state| product.states[state.index()].monitor)
        .collect();
    assert_eq!(monitors.len(), undecided.len());
    for state in &undecided {
        let edges = product.outgoing(*state);
        assert_eq!(edges.len(), 2, "tick draws fault or not");
        assert_eq!(total(edges), ratio(1, 1));
        let faulty = edges
            .iter()
            .find(|edge| boolean_of(&edge.draw.as_ref().expect("a draw")[0]));
        assert_eq!(
            faulty.map(|edge| edge.probability.clone()),
            Some(ratio(1, 10_000_000))
        );
    }
    for (state, _) in &product.decided {
        assert!(
            product.outgoing(*state).is_empty(),
            "a decided state is absorbing"
        );
    }
    let depth = depths(&product, start(&product));
    assert_eq!(
        depth.len(),
        product.states.len(),
        "every state is reachable"
    );
    let deepest = depth.values().copied().max().expect("states");
    assert!(
        deepest <= 1001,
        "every path is decided within {deepest} positions"
    );
    let accept = product
        .decided
        .iter()
        .find(|(_, verdict)| *verdict == MonitorVerdict::Accept)
        .expect("an accepting state");
    assert_eq!(depth[&accept.0.index()], 1000);
}

const P95: &str = "probabilistic P95 using ev on M over (s: M::Server) under Steady on origin \
     { quantile 0.95 of (accumulate duration from holds(s.phase = 1) until holds(s.phase = 2) \
     within 10) <= 5 }\n\
     probabilistic P95Two using ev on M over (s: M::Server) under Steady on origin \
     { quantile 0.95 of (accumulate duration from holds(s.phase = 1) until holds(s.phase = 2) \
     within 10) <= 2 }\n\
     probabilistic P95Every using ev on M over (s: M::Server) under every scheduler on origin \
     { quantile 0.95 of (accumulate duration from holds(s.phase = 1) until holds(s.phase = 2) \
     within 10) <= 5 }\n\
     probabilistic P95EveryTwo using ev on M over (s: M::Server) under every scheduler on origin \
     { quantile 0.95 of (accumulate duration from holds(s.phase = 1) until holds(s.phase = 2) \
     within 10) <= 2 }";

/// Checks FR-196-AC-2 (TC-631 step 2): §15.2's product for `P95` at 5 carries the
/// `duration` accumulator, which takes the values 0, 1, 2, 3, 4 and the one
/// value above the threshold; the activation flag is set from position 1.
/// The edge for `attempt` with `d = 3`, `outcome = Ok` carries `343/5000`.
#[trace("TC-631", "FR-196-AC-2")]
#[test]
fn the_p95_product_accumulates_duration_and_saturates_above_the_threshold() {
    let world = World::new(&prob_world::service(P95));
    let product = built(&world, "P95");
    assert_eq!(product.kind, ProductKind::Dtmc);
    let held = distinct(
        product
            .states
            .iter()
            .map(|key| match &key.accumulator {
                Some(Accumulator::Reward { activated, amount }) => (*activated, amount.clone()),
                other => panic!("a reward accumulator: {other:?}"),
            })
            .collect(),
    );
    let exact = |value| (true, Amount::Exact(ratio(value, 1)));
    assert_eq!(
        held,
        distinct(vec![
            (false, Amount::Exact(ratio(0, 1))),
            exact(0),
            exact(1),
            exact(2),
            exact(3),
            exact(4),
            (true, Amount::AboveBound),
        ])
    );
    // Position 0 is not activated; the step into position 1 activates.
    let first = start(&product);
    assert!(matches!(
        product.states[first.index()].accumulator,
        Some(Accumulator::Reward {
            activated: false,
            ..
        })
    ));
    let [request] = product.outgoing(first) else {
        panic!("only `request` is enabled at Idle");
    };
    assert_eq!(request.probability, ratio(1, 1));
    let busy = request.to;
    assert_eq!(
        product.states[busy.index()].monitor,
        Some(MonitorState::Measure {
            remaining: 9,
            phase: MeasurePhase::Active
        })
    );
    assert_eq!(
        product.states[busy.index()].accumulator,
        Some(Accumulator::Reward {
            activated: true,
            amount: Amount::Exact(ratio(0, 1))
        })
    );
    assert_eq!(total(product.outgoing(busy)), ratio(1, 1));
    let three_ok = product
        .outgoing(busy)
        .iter()
        .find(|edge| {
            let draw = edge.draw.as_ref().expect("a draw");
            integer_of(&draw[0]) == 3 && boolean_of(&draw[1])
        })
        .expect("attempt with d = 3 and ok");
    assert_eq!(three_ok.probability, ratio(343, 5000));
    let (name, reward) = &three_ok.rewards[0];
    assert_eq!((name.as_str(), integer_of(reward)), ("duration", 3));
    let done = &product.states[three_ok.to.index()];
    assert_eq!(
        done.monitor,
        Some(MonitorState::Measure {
            remaining: 8,
            phase: MeasurePhase::Complete
        })
    );
    assert_eq!(
        done.accumulator,
        Some(Accumulator::Reward {
            activated: true,
            amount: Amount::Exact(ratio(3, 1))
        })
    );
    assert!(product.outgoing(three_ok.to).is_empty());
    // 12 is above the threshold 5; equal to it would stay exact.
    let twelve_ok = product
        .outgoing(busy)
        .iter()
        .find(|edge| integer_of(&edge.draw.as_ref().expect("a draw")[0]) == 12)
        .expect("attempt with d = 12");
    assert_eq!(
        product.states[twelve_ok.to.index()].accumulator,
        Some(Accumulator::Reward {
            activated: true,
            amount: Amount::AboveBound
        })
    );
}

/// Checks FR-197-AC-2 (TC-632 step 2): exact §15.2 probabilities and the
/// XF-2 transformation agree under `Steady` and every scheduler.
#[trace("TC-632", "FR-197-AC-2")]
#[test]
fn p95_probabilities_match_the_exact_scheduler_transform() {
    let world = World::new(&prob_world::service(P95));
    let limits = ExactProbLimits::default();
    let quantile = ratio(19, 20);
    for (workload_name, every_name, threshold, expected) in [
        ("P95", "P95Every", ratio(5, 1), ratio(24_233, 25_000)),
        ("P95Two", "P95EveryTwo", ratio(2, 1), ratio(4_491, 5_000)),
    ] {
        let workload_product = built(&world, workload_name);
        let every_product = built(&world, every_name);
        assert_eq!(workload_product.kind, ProductKind::Dtmc);
        assert_eq!(every_product.kind, ProductKind::Mdp);

        let workload_result = backward_induction(
            &workload_product,
            &measure_event_terminals(&workload_product, &threshold),
            FhObjective::Min,
            &quantile,
            limits.max_rational_bits,
            limits.precision_bits,
            limits.max_precision_bits,
        )
        .expect("the bounded workload measure is acyclic");
        let FhResult::Exact {
            values: workload_values,
            ..
        } = workload_result
        else {
            panic!("the §15.2 workload probability remains exact");
        };
        let workload_initial = start(&workload_product);
        let probability = &workload_values[workload_initial.index()];
        assert_eq!(probability, &expected, "{workload_name}");

        let every_result = backward_induction(
            &every_product,
            &quantile_margin_terminals(&every_product, &threshold, &quantile),
            FhObjective::Min,
            &ratio(0, 1),
            limits.max_rational_bits,
            limits.precision_bits,
            limits.max_precision_bits,
        )
        .expect("the XF-2 bounded scheduler product is acyclic");
        let FhResult::Exact {
            values: every_values,
            ..
        } = every_result
        else {
            panic!("the §15.2 scheduler transform remains exact");
        };
        let every_initial = start(&every_product);
        let transformed = &every_values[every_initial.index()];
        assert_eq!(transformed, &expected.sub(&quantile), "{every_name}");

        let workload_holds = probability >= &quantile;
        let every_holds = transformed >= &ratio(0, 1);
        assert_eq!(every_holds, workload_holds, "{every_name}");
    }
}

const THRESHOLDS: &str = "probabilistic Four using ev on M over (s: M::Server) under Steady \
     on origin { quantile 0.95 of (accumulate duration from holds(s.phase = 1) \
     until holds(s.phase = 2) within 10) <= 4 }\n\
     probabilistic Three using ev on M over (s: M::Server) under Steady \
     on origin { quantile 0.95 of (accumulate duration from holds(s.phase = 1) \
     until holds(s.phase = 2) within 10) >= 3 }";

/// Checks FR-196 "Behavior": an amount equal to the claim's threshold stays exact,
/// and one above it is the one value above it, whichever way the comparison
/// goes.
#[trace("TC-631", "FR-196-AC-2")]
#[test]
fn an_amount_equal_to_the_threshold_stays_exact() {
    let world = World::new(&prob_world::service(THRESHOLDS));
    let amounts = |claim: &str| {
        distinct(
            built(&world, claim)
                .states
                .iter()
                .filter_map(|key| match &key.accumulator {
                    Some(Accumulator::Reward {
                        activated: true,
                        amount,
                    }) => Some(amount.clone()),
                    _ => None,
                })
                .collect(),
        )
    };
    let exact = |value| Amount::Exact(ratio(value, 1));
    // 1 + 3 and 3 + 1 reach 4 exactly; 3 + 3 and any 12 are above it.
    assert_eq!(
        amounts("Four"),
        distinct(vec![
            exact(0),
            exact(1),
            exact(2),
            exact(3),
            exact(4),
            Amount::AboveBound
        ])
    );
    // At threshold 3, 4 is above and 3 is not.
    assert_eq!(
        amounts("Three"),
        distinct(vec![
            exact(0),
            exact(1),
            exact(2),
            exact(3),
            Amount::AboveBound
        ])
    );
}

const DELIVER: &str = "probabilistic Deliver using ev on M over (m: M::Msg) under every scheduler \
     on origin { probability >= 0.97 [ eventually[0,2] holds(m.delivered) ] }\n\
     probabilistic DeliverMax using ev on M over (m: M::Msg) under every scheduler \
     on origin { probability <= 0.97 [ eventually[0,2] holds(m.delivered) ] }\n\
     probabilistic DeliverFair using ev on M over (m: M::Msg) \
     under every scheduler fair weak Msg::send_a \
     on origin { probability >= 0.97 [ eventually[0,2] holds(m.delivered) ] }\n\
     probabilistic DeliverEven using ev on M over (m: M::Msg) under Even \
     on origin { probability >= 0.97 [ eventually[0,2] holds(m.delivered) ] }";

/// The edges of `edges` taking the operation named `operation`, with the
/// probability of each draw by its `lost` value.
fn sends(edges: &[ProbEdge], operation: &str) -> BTreeMap<bool, Rational> {
    edges
        .iter()
        .filter(|edge| {
            edge.action
                .as_ref()
                .is_some_and(|action| action.operation.contains(operation))
        })
        .map(|edge| {
            let draw = edge.draw.as_ref().expect("a draw");
            (boolean_of(&draw[0]), edge.probability.clone())
        })
        .collect()
}

/// Checks FR-197-AC-1 (TC-632 step 1): §15.4's `Deliver` scheduler extrema,
/// selected actions and workload values match the published exact vectors.
#[trace("TC-632", "FR-197-AC-1")]
#[test]
fn deliver_backward_induction_matches_the_published_scheduler_vectors() {
    let world = World::new(&prob_world::link(DELIVER));
    let limits = ExactProbLimits::default();
    let minimum_product = built(&world, "Deliver");
    let maximum_product = built(&world, "DeliverMax");
    let workload_product = built(&world, "DeliverEven");
    let initial = start(&minimum_product);
    let lost = minimum_product
        .outgoing(initial)
        .iter()
        .find(|edge| edge.draw.as_ref().is_some_and(|draw| boolean_of(&draw[0])))
        .expect("the first send can be lost")
        .to;

    let FhResult::Exact {
        values: minimum,
        policy: minimum_policy,
    } = decide_bounded_event(
        &minimum_product,
        &fsl_semantics::check::Bound::AtLeast(ratio(97, 100)),
        &limits,
    )
    .expect("the bounded scheduler product is acyclic")
    else {
        panic!("the §15.4 minimum remains exact");
    };
    assert_eq!(minimum[initial.index()], ratio(24, 25));
    assert_eq!(minimum[lost.index()], ratio(4, 5));
    assert!(selected_operation(
        &minimum_product,
        initial,
        minimum_policy[initial.index()].expect("the initial state has actions"),
    )
    .contains("send_b"));
    assert!(selected_operation(
        &minimum_product,
        lost,
        minimum_policy[lost.index()].expect("the post-loss state has actions"),
    )
    .contains("send_b"));

    let FhResult::Exact {
        values: maximum, ..
    } = decide_bounded_event(
        &maximum_product,
        &fsl_semantics::check::Bound::AtMost(ratio(97, 100)),
        &limits,
    )
    .expect("the bounded scheduler product is acyclic")
    else {
        panic!("the §15.4 maximum remains exact");
    };
    assert_eq!(maximum[initial.index()], ratio(99, 100));

    let FhResult::Exact {
        values: workload, ..
    } = decide_bounded_event(
        &workload_product,
        &fsl_semantics::check::Bound::AtLeast(ratio(97, 100)),
        &limits,
    )
    .expect("the workload product is acyclic")
    else {
        panic!("the §15.4 workload values remain exact");
    };
    let workload_initial = start(&workload_product);
    let workload_lost = workload_product
        .outgoing(workload_initial)
        .iter()
        .find(|edge| edge.draw.as_ref().is_some_and(|draw| boolean_of(&draw[0])))
        .expect("the first workload send can be lost")
        .to;
    assert_eq!(workload[workload_initial.index()], ratio(391, 400));
    assert_eq!(workload[workload_lost.index()], ratio(17, 20));
}

/// FR-200-AC-4 (TC-635): bounded decisions are unchanged by fairness, while
/// the checked fairness set accompanies the output.
#[trace("TC-635", "FR-200-AC-4", "FR-411-AC-11")]
#[test]
fn fair_deliver_keeps_the_24_over_25_minimum_and_records_weak_fairness() {
    let world = World::new(&prob_world::link(DELIVER));
    let ordinary = built(&world, "Deliver");
    let fair = built(&world, "DeliverFair");
    let fair_claim = world
        .package
        .graph()
        .probabilistic_model()
        .claim("DeliverFair")
        .expect("fair claim checks");
    let checked_fairness = fair_claim.fairness.clone();
    let result = fsl_analyze::exact_probabilistic::decide_fair_bounded_event(
        &fair,
        &fsl_semantics::check::Bound::AtLeast(ratio(97, 100)),
        &ExactProbLimits::default(),
    )
    .expect("bounded event is unchanged under fairness");
    let fsl_analyze::exact_probabilistic::FhResult::Exact { values, .. } = result.outcome else {
        panic!("bounded result remains exact");
    };
    assert_eq!(values[start(&ordinary).index()], ratio(24, 25));
    assert_eq!(values[start(&fair).index()], ratio(24, 25));
    assert_eq!(fair.fairness, checked_fairness);
    assert_eq!(result.fairness, fair.fairness);
}

/// FR-200-AC-3 (TC-635): the fair infimum is the slow-repair limit, while a
/// k=1 exact randomized witness mixes both repairs equally and refutes .9995.
#[trace("TC-635", "FR-200-AC-3")]
#[test]
fn fair_availability_uses_long_run_limit_and_randomized_refutation() {
    const CLAIM: &str = "probabilistic AvailFair using inf on M over (s: M::Server) \
         under every scheduler fair strong Server::repair_fast on origin \
         { long_run_fraction holds(s.up) >= rational(9995, 10000) }";
    let world = World::new(&prob_world::avail2(CLAIM));
    let product = built(&world, "AvailFair");
    let components = maximal_fair_end_components(&product, &product.fairness)
        .expect("well-formed checked fairness");
    assert_eq!(components.len(), 1, "{components:?}");
    assert_eq!(components[0].states.len(), 2);
    assert_eq!(components[0].actions.len(), 3);

    let predicate = product
        .labels
        .iter()
        .map(|labels| labels[0])
        .collect::<Vec<_>>();
    let result = decide_fair_long_run_mdp(
        &product,
        &predicate,
        None,
        &fsl_semantics::check::Bound::AtLeast(ratio(9995, 10000)),
        &ExactProbLimits::default(),
    )
    .expect("exact fair long-run decision");
    assert_eq!(result.components[0].rho, ratio(1000, 1001));
    assert_eq!(
        result.values[start(&product).index()],
        fsl_analyze::exact_probabilistic::ExtRational::Finite(ratio(1000, 1001))
    );
    let witness = result.witness.as_ref().expect("the threshold is refuted");
    assert_eq!(witness.k, 1);
    assert_eq!(
        witness.values[start(&product).index()],
        fsl_analyze::exact_probabilistic::ExtRational::Finite(ratio(1400, 1401))
    );
    let down = product
        .ids()
        .find(|state| !predicate[state.index()])
        .expect("down state");
    let randomized = match witness.policy[down.index()]
        .as_ref()
        .expect("down scheduler choice")
    {
        fsl_analyze::exact_probabilistic::SchedulerChoice::Randomized(choices) => choices,
        other => panic!("expected a randomized fair witness, got {other:?}"),
    };
    assert_eq!(randomized.len(), 2);
    assert!(randomized
        .iter()
        .all(|(_, probability)| probability == &ratio(1, 2)));
    let mut names = randomized
        .iter()
        .map(|(choice, _)| {
            product
                .outgoing(down)
                .iter()
                .filter_map(|edge| edge.action.as_ref())
                .fold(Vec::<String>::new(), |mut names, action| {
                    if !names.contains(&action.operation) {
                        names.push(action.operation.clone());
                    }
                    names
                })
                .get(choice.0)
                .expect("selected action index")
                .clone()
        })
        .collect::<Vec<_>>();
    names.sort();
    assert!(names[0].contains("repair_fast"));
    assert!(names[1].contains("repair_slow"));
    assert_eq!(result.fairness, product.fairness);

    let records = fsl_analyze::analyze::analyze_items(
        &fsl_analyze::analyze::Engines::fsl().expect("FSL engines"),
        vec![fsl_analyze::analyze::exact_probabilistic_item(
            fsl_foundation::RequestIndex::new(0),
            world.request("AvailFair", ModelCheckLimits::default()),
        )],
        &Cancel::new(),
    );
    let record = &records[0];
    let terminal = record.terminal().unwrap_or_else(|| {
        panic!(
            "fair long-run result did not settle: {:?}",
            record.settlement
        )
    });
    assert_eq!(
        terminal.value(),
        &fsl_replay::TerminalValue::Refuted,
        "terminal: {terminal:?}"
    );
    let witness = record
        .probabilistic_counterexample
        .as_ref()
        .expect("fair long-run refutation has randomized witness evidence");
    let randomized = witness
        .evidence()
        .scheduler
        .iter()
        .find_map(|(_, choice)| match choice {
            fsl_replay::ProbabilisticSchedulerChoice::Randomized(distribution)
                if distribution.len() == 2
                    && distribution.iter().all(|(_, p)| p == &ratio(1, 2)) =>
            {
                Some(distribution)
            }
            _ => None,
        })
        .expect("the witness randomizes two alternatives equally");
    let names = randomized
        .iter()
        .map(|(choice, _)| {
            let bytes = choice.to_bytes().expect("scheduled choice encodes");
            String::from_utf8_lossy(&bytes).into_owned()
        })
        .collect::<Vec<_>>();
    assert!(
        names.iter().any(|name| name.contains("repair_fast")),
        "{names:?}"
    );
    assert!(
        names.iter().any(|name| name.contains("repair_slow")),
        "{names:?}"
    );
    let subsystem = witness
        .evidence()
        .subsystem
        .as_ref()
        .expect("the exact induced long-run chain is certified");
    let certificate = fsl_replay::read_long_run_probability_certificate(
        subsystem.as_bytes(),
        crate::long_run_certificate::limits(),
        &Cancel::new(),
    )
    .expect("the independent checker accepted the induced-chain document");
    assert!(certificate
        .values
        .iter()
        .all(|row| row.value == ratio(1400, 1401)));
}

/// FR-198-AC-2 under FR-200: a zero-reward wait MEC collapses while retaining
/// its positive-reward exit, yielding a finite maximum of five.
#[trace("TC-633", "FR-198-AC-2")]
#[test]
fn fair_max_expected_reward_preserves_zero_reward_mec_exits() {
    const CLAIM: &str = "probabilistic CostFair using inf on M over (p: M::Proc) \
         under every scheduler fair strong Proc::exit on origin \
         { expected accumulate cost until holds(p.done) <= 5 }";
    let world = World::new(&prob_world::zero_reward_exit(CLAIM));
    let product = built(&world, "CostFair");
    let target = product
        .labels
        .iter()
        .map(|labels| labels[0])
        .collect::<Vec<_>>();
    let result = fsl_analyze::exact_probabilistic::decide_fair_expected_reward(
        &product,
        &target,
        "cost",
        UnboundedObjective::Max,
        &ExactProbLimits::default(),
    )
    .expect("the strong-fair expected maximum is finite");
    assert_eq!(
        result.values[start(&product).index()],
        fsl_analyze::exact_probabilistic::ExtRational::Finite(ratio(5, 1))
    );
    let action = match result.policy.as_ref().expect("a fair scheduler")[start(&product).index()]
        .as_ref()
        .expect("a live choice")
    {
        fsl_analyze::exact_probabilistic::SchedulerChoice::Deterministic(
            fsl_analyze::exact_probabilistic::ActionChoice(index),
        ) => *index,
        other => panic!("expected the exit action, got {other:?}"),
    };
    let mut operations = Vec::new();
    for edge in product.outgoing(start(&product)) {
        if let Some(identity) = edge.action.as_ref() {
            if !operations.iter().any(|seen| seen == identity) {
                operations.push(identity.clone());
            }
        }
    }
    assert!(operations[action].operation.contains("exit"));
}

/// FR-200 FS-8: an unfair wait MEC collapses to a proper route into the fair
/// recurrent component instead of becoming a closed policy class.
#[trace("TC-635")]
#[test]
fn fair_long_run_routes_away_from_a_nonfair_mec() {
    const CLAIM: &str = "probabilistic FairEscape using inf on M over (p: M::Proc) \
         under every scheduler fair strong Proc::go_fair on origin \
         { long_run_fraction holds(p.fair) >= rational(9, 10) }";
    let world = World::new(&prob_world::fair_longrun_escape(CLAIM));
    let product = built(&world, "FairEscape");
    let fair_components = maximal_fair_end_components(&product, &product.fairness)
        .expect("well-formed checked fairness");
    assert_eq!(fair_components.len(), 1);
    assert_eq!(fair_components[0].states.len(), 1);
    assert!(fair_components[0].actions[0]
        .1
        .operation
        .contains("continue_fair"));
    let predicate = product
        .labels
        .iter()
        .map(|labels| labels[0])
        .collect::<Vec<_>>();
    let result = decide_fair_long_run_mdp(
        &product,
        &predicate,
        None,
        &fsl_semantics::check::Bound::AtLeast(ratio(9, 10)),
        &ExactProbLimits::default(),
    )
    .expect("fair long-run routes through the non-fair MEC");
    assert_eq!(
        result.values[start(&product).index()],
        fsl_analyze::exact_probabilistic::ExtRational::Finite(ratio(1, 1))
    );
    assert_eq!(
        result.gain_bias[0].bias,
        vec![(fair_components[0].states[0], ratio(0, 1))]
    );
}

/// Checks FR-196-AC-3 (TC-631 step 3): §15.4's `Deliver` over every scheduler is an
/// MDP with two actions at each live state, whose draws carry 9/10 and 1/10
/// (`send_a`) and 4/5 and 1/5 (`send_b`); under `Even` it is a DTMC with
/// FR-187-AC-2's probabilities.
#[trace("TC-631", "FR-196-AC-3")]
#[test]
fn deliver_is_an_mdp_over_every_scheduler_and_a_dtmc_under_a_workload() {
    let world = World::new(&prob_world::link(DELIVER));
    let mdp = built(&world, "Deliver");
    assert_eq!(mdp.kind, ProductKind::Mdp);
    let first = start(&mdp);
    let edges = mdp.outgoing(first);
    assert_eq!(edges.len(), 4);
    assert_eq!(
        sends(edges, "send_a"),
        BTreeMap::from([(false, ratio(9, 10)), (true, ratio(1, 10))])
    );
    assert_eq!(
        sends(edges, "send_b"),
        BTreeMap::from([(false, ratio(4, 5)), (true, ratio(1, 5))])
    );
    // Each action's draws sum to 1: the scheduler chooses among actions.
    for operation in ["send_a", "send_b"] {
        let draws: Vec<ProbEdge> = edges
            .iter()
            .filter(|edge| {
                edge.action
                    .as_ref()
                    .is_some_and(|action| action.operation.contains(operation))
            })
            .cloned()
            .collect();
        assert_eq!(total(&draws), ratio(1, 1), "{operation}");
    }
    // Delivering decides the event; losing leaves a live state with the same
    // two actions.
    let lost = edges
        .iter()
        .find(|edge| boolean_of(&edge.draw.as_ref().expect("a draw")[0]))
        .expect("a lost draw");
    assert_eq!(mdp.outgoing(lost.to).len(), 4);
    let delivered = edges
        .iter()
        .find(|edge| !boolean_of(&edge.draw.as_ref().expect("a draw")[0]))
        .expect("a delivered draw");
    assert!(is_decided(&mdp, delivered.to));
    assert!(mdp
        .decided
        .contains(&(delivered.to, MonitorVerdict::Accept)));

    let dtmc = built(&world, "DeliverEven");
    assert_eq!(dtmc.kind, ProductKind::Dtmc);
    let edges = dtmc.outgoing(start(&dtmc));
    assert_eq!(edges.len(), 4);
    assert_eq!(total(edges), ratio(1, 1));
    assert_eq!(
        sends(edges, "send_a"),
        BTreeMap::from([(false, ratio(9, 20)), (true, ratio(1, 20))])
    );
    assert_eq!(
        sends(edges, "send_b"),
        BTreeMap::from([(false, ratio(2, 5)), (true, ratio(1, 10))])
    );
}

const CLOSING: &str = "probabilistic Late using ev on M over (m: M::Msg) under every scheduler \
     on origin { probability >= 0.5 [ eventually[0,5] holds(m.delivered) ] }\n\
     probabilistic Safe using ev on M over (m: M::Msg) under every scheduler \
     on origin { probability >= 0.5 \
     [ always[0,5] (holds(m.delivered) implies holds(m.attempts < 3)) ] }";

/// Checks FR-196 "Behavior": a terminal model state closes a bounded event under
/// the profile's false extension (FR-126), into the decided state, with no
/// model step; an event the false extension violates is rejected there and
/// one it keeps is accepted.
#[trace("TC-631")]
#[test]
fn a_terminal_state_closes_a_bounded_event_by_false_extension() {
    let world = World::new(&prob_world::link(CLOSING));
    for (claim, verdict) in [
        ("Late", MonitorVerdict::Reject),
        ("Safe", MonitorVerdict::Accept),
    ] {
        let product = built(&world, claim);
        let closing: Vec<&ProbEdge> = product
            .edges
            .iter()
            .filter(|edge| matches!(edge.choice, ProductAction::TerminalStutter))
            .collect();
        assert!(!closing.is_empty(), "{claim}: a terminal state closes");
        for edge in &closing {
            assert_eq!(edge.probability, ratio(1, 1), "{claim}");
            assert!(edge.action.is_none() && edge.draw.is_none(), "{claim}");
            let [(name, cost)] = &edge.rewards[..] else {
                panic!("{claim}: the stutter step carries every reward");
            };
            assert_eq!((name.as_str(), integer_of(cost)), ("cost", 0));
            assert_eq!(
                product.states[edge.to.index()].monitor,
                Some(MonitorState::Formula(FormulaState::Decided(verdict))),
                "{claim}"
            );
            assert!(product.outgoing(edge.to).is_empty(), "{claim}");
        }
    }
}

const HEALTH: &str = "probabilistic Steadily using ev on M over (n: M::Node) under Steady \
     on origin { probability >= 0.99 [ always[0,2] holds(n.healthy) ] }\n\
     probabilistic Anyhow using ev on M over (n: M::Node) under every scheduler \
     on origin { probability >= 0.99 [ always[0,2] holds(n.healthy) ] }";

fn loose_health() -> World {
    World::new(&prob_world::health(
        "self.healthy = (pre(self.healthy) and not fault)",
        "true: 1, false: 9999999",
        true,
        HEALTH,
    ))
}

fn blocked_health() -> World {
    World::new(&prob_world::health(
        "not fault and self.healthy = pre(self.healthy) and self.spare = pre(self.spare)",
        "true: 1, false: 1",
        true,
        HEALTH,
    ))
}

/// Checks FR-196-AC-4 (TC-631 step 4): FR-187-AC-3's `Health` variant with two
/// post-states per draw builds, over every scheduler, an intermediate state
/// per draw with two probability-1 actions, and the monitor does not advance
/// on it; under a workload the build stops with `NotMarkov`; the variant with
/// no post-state stops with `NotMarkov` under both. Two builds of one request
/// are equal.
#[trace("TC-631", "FR-196-AC-4")]
#[test]
fn a_draw_with_several_post_states_leads_to_an_intermediate_state() {
    let world = loose_health();
    let product = built(&world, "Anyhow");
    assert_eq!(product.kind, ProductKind::Mdp);
    let first = start(&product);
    let draws = product.outgoing(first);
    let [healthy, faulty] = draws else {
        panic!("two draws of `tick`: {draws:?}");
    };
    assert_eq!(healthy.probability, ratio(9_999_999, 10_000_000));
    assert_eq!(faulty.probability, ratio(1, 10_000_000));
    let source = &product.states[first.index()];
    for (draw, post_healthy) in [(healthy, true), (faulty, false)] {
        let intermediate = &product.states[draw.to.index()];
        let pending = intermediate.pending.as_ref().expect("a pending draw");
        assert!(pending.identity.operation.contains("tick"));
        assert_eq!(boolean_of(&pending.random[0]), !post_healthy);
        // The monitor and the accumulator are the source's: nothing advanced.
        assert_eq!(intermediate.monitor, source.monitor);
        assert_eq!(intermediate.accumulator, source.accumulator);
        assert_eq!(intermediate.model, source.model);
        let resolutions = product.outgoing(draw.to);
        assert_eq!(resolutions.len(), 2, "one per `spare`");
        assert_eq!(total(resolutions), ratio(2, 1));
        for edge in resolutions {
            assert_eq!(edge.probability, ratio(1, 1));
            assert!(edge.action.is_none() && edge.draw.is_none());
            let ProductAction::PostState(target) = &edge.choice else {
                panic!("a resolution names its post-state: {:?}", edge.choice);
            };
            assert_eq!(**target, product.states[edge.to.index()]);
            let reached = &product.states[edge.to.index()];
            assert!(reached.pending.is_none());
            // It advanced once: a fault is a bad prefix, health is position 1.
            let verdict = if post_healthy {
                assert_ne!(reached.monitor, source.monitor);
                None
            } else {
                Some(MonitorVerdict::Reject)
            };
            assert_eq!(
                reached.monitor.is_some_and(MonitorState::is_decided),
                verdict.is_some()
            );
            if let Some(verdict) = verdict {
                assert!(product.decided.contains(&(edge.to, verdict)));
            }
        }
    }
    let again = built(&world, "Anyhow");
    assert_eq!(product.states, again.states);
    assert_eq!(format!("{product:?}"), format!("{again:?}"));

    let ProductRefusal::NotMarkov(loose) = refused(&world, "Steadily") else {
        panic!("a workload needs one post-state");
    };
    assert_eq!(loose.post_states, 2);
    let mut expected_posts = product
        .outgoing(healthy.to)
        .iter()
        .map(|edge| {
            fsl_eval::simulation::StateKey::from_canonical_bytes(
                product.states[edge.to.index()].model.clone(),
            )
        })
        .collect::<Vec<_>>();
    expected_posts.sort();
    assert_eq!(loose.post_state_examples, expected_posts);
    assert_eq!(loose.post_state_examples.len(), 2);
    assert_ne!(loose.post_state_examples[0], loose.post_state_examples[1]);
    assert!(loose.transition.operation.contains("tick"));
    assert_eq!(
        loose.random.iter().map(boolean_of).collect::<Vec<_>>(),
        [false]
    );

    let blocked = blocked_health();
    for claim in ["Steadily", "Anyhow"] {
        let ProductRefusal::NotMarkov(none) = refused(&blocked, claim) else {
            panic!("{claim}: a draw with no post-state is not Markov");
        };
        assert_eq!(none.post_states, 0, "{claim}");
        assert!(none.post_state_examples.is_empty(), "{claim}");
        assert_eq!(
            none.random.iter().map(boolean_of).collect::<Vec<_>>(),
            [true],
            "{claim}"
        );
    }
}

const COIN: &str = "probabilistic Dropped using ev on M over (c: M::Coin) under Even on origin \
     { probability >= 0.5 [ always[0,3] holds(c.v = 0) ] }";

/// Checks FR-196-AC-5 (TC-641 step 1): when the claim reads a value that is
/// undefined at a product state of positive probability, the build stops at
/// the first such state in breadth-first order and returns the path to it.
/// The spec's `Coin` divides by `1 - c.v`; S3 types neither `/` in a model
/// unit nor a population name in a claim's predicate, so this `Coin` is
/// dropped when `b = 0` and the claim's `c.v` then reads `absent-key`
/// undefined. The path is `drop` with `b = 0`, probability 1/2, at
/// position 1.
#[trace("TC-641", "FR-196-AC-5", "FR-196-AC-12")]
#[test]
fn an_undefined_state_stops_the_build_with_its_path() {
    let world = World::new(&prob_world::coin(COIN));
    let ProbBuild::Undefined(path) = build_product(
        world.request("Dropped", ModelCheckLimits::default()),
        &Cancel::new(),
    )
    .expect("the build ends with a refutation") else {
        panic!("an undefined path");
    };
    let [step] = &path.steps[..] else {
        panic!("one step: {:?}", path.steps);
    };
    assert!(step
        .action
        .as_ref()
        .is_some_and(|action| action.operation.contains("drop")));
    assert_eq!(
        step.draw
            .as_ref()
            .map(|draw| draw.iter().map(integer_of).collect::<Vec<_>>()),
        Some(vec![0])
    );
    assert_eq!(step.probability, ratio(1, 2));
    assert_eq!(path.probability, ratio(1, 2));
    assert_eq!(path.position, 1);
    assert_eq!(step.target, path.key);
    assert!(step.resolved.is_none());
    assert_eq!(path.binding, path.key.binding);
    assert!(path.key.pending.is_none());
    assert!(matches!(path.read, UndefinedRead::Atom(_)));
    assert!(matches!(
        path.cause,
        UndefinedCause::Family(ref record) if record.reason == UndefinedReason::AbsentKey
    ));
}

const ARMED: &str = "probabilistic Armed using ev on M over (c: M::Coin) under Even on origin \
     { probability >= 0.5 [ always[0,3] holds(c.armed = c.armed) ] }";

/// Checks FR-196-AC-5 (TC-641 step 1): the path to an undefined state lists every
/// model step from the initial state, in order, with the product of their
/// probabilities and the position, the step count.
#[trace("TC-641", "FR-196-AC-5")]
#[test]
fn the_undefined_path_lists_every_step_in_order() {
    let world = World::new(&prob_world::armed_coin(ARMED));
    let ProbBuild::Undefined(path) = build_product(
        world.request("Armed", ModelCheckLimits::default()),
        &Cancel::new(),
    )
    .expect("the build ends with a refutation") else {
        panic!("an undefined path");
    };
    let operations: Vec<bool> = path
        .steps
        .iter()
        .map(|step| {
            step.action
                .as_ref()
                .is_some_and(|action| action.operation.contains("Coin/arm"))
        })
        .collect();
    assert_eq!(
        operations,
        [true, false],
        "`arm`, then `drop`: {:?}",
        path.steps
    );
    assert_eq!(path.steps[0].probability, ratio(1, 1));
    assert_eq!(path.steps[1].probability, ratio(1, 2));
    assert_eq!(
        path.steps[1]
            .draw
            .as_ref()
            .map(|draw| draw.iter().map(integer_of).collect::<Vec<_>>()),
        Some(vec![0])
    );
    assert_eq!(path.probability, ratio(1, 2));
    assert_eq!(path.position, 2);
    assert!(path.steps.iter().all(|step| step.resolved.is_none()));
}

const GHOST: &str = "probabilistic Ghost using ev on M over (c: M::Coin) under Even on origin \
     { probability >= 0.5 [ always[0,3] holds(c.v = 0) ] }\n\
     probabilistic GhostSteps using ev on M over (c: M::Coin) under Even on origin \
     { probability >= 0.5 [ steps from holds(c.v = 0) until holds(c.v = 1) within 3 <= 3 ] }\n\
     probabilistic GhostFrom using inf on M over (c: M::Coin) under Even on origin \
     { probability >= 0.5 [ holds(c.v = 0) until holds(c.v = 1) ] }\n\
     probabilistic GhostTarget using inf on M over (c: M::Coin) under Even on origin \
     { probability >= 0.5 [ eventually holds(c.v = 0) ] }";

/// Checks FR-196 "Behavior": the claim's `over` instances are the subject's
/// universe, in order, and a state at which a predicate is undefined for an
/// instance ends the build. The second instance has no object in the initial
/// snapshot, so its predicate is undefined at position 0, with no step to
/// it.
#[trace("TC-641", "FR-196-AC-13")]
#[test]
fn an_instance_undefined_at_an_initial_state_ends_at_position_zero() {
    let world = World::with_universe(&prob_world::coin(GHOST), &["c", "ghost"]);
    let ProbBuild::Undefined(path) = build_product(
        world.request("Ghost", ModelCheckLimits::default()),
        &Cancel::new(),
    )
    .expect("the build ends with a refutation") else {
        panic!("an undefined path");
    };
    assert!(path.steps.is_empty());
    assert_eq!(path.position, 0);
    assert_eq!(path.initial, 0);
    assert_eq!(path.probability, ratio(1, 1));
    assert_eq!(path.binding, path.key.binding);
    assert!(path
        .key
        .monitor
        .is_some_and(|monitor| matches!(monitor, MonitorState::Formula(FormulaState::Running(_)))));
    assert!(path.key.accumulator.is_none());
    assert!(path.key.pending.is_none());
    assert!(!path.key.model.is_empty());
    assert!(matches!(path.read, UndefinedRead::Atom(_)));
    assert!(matches!(
        &path.binding,
        Some(binding) if binding.key == "ghost"
    ));
    let ProbBuild::Undefined(path) = build_product(
        world.request("GhostSteps", ModelCheckLimits::default()),
        &Cancel::new(),
    )
    .expect("the initial measure read is undefined") else {
        panic!("an undefined measure read");
    };
    assert_eq!(path.position, 0);
    assert_eq!(path.initial, 0);
    assert!(matches!(
        path.key.monitor,
        Some(MonitorState::Measure {
            remaining: 3,
            phase: MeasurePhase::Inactive
        })
    ));
    assert_eq!(
        path.key.accumulator,
        Some(Accumulator::Steps {
            activated: false,
            amount: Amount::Exact(ratio(0, 1))
        })
    );
    assert!(path.key.pending.is_none());
    // An unbounded form names the role of the predicate that has no value.
    for (claim, role) in [
        ("GhostFrom", PredicateRole::From),
        ("GhostTarget", PredicateRole::Target),
    ] {
        let ProbBuild::Undefined(path) = build_product(
            world.request(claim, ModelCheckLimits::default()),
            &Cancel::new(),
        )
        .expect("the build ends with a refutation") else {
            panic!("{claim}: an undefined path");
        };
        assert_eq!(path.read, UndefinedRead::Predicate(role), "{claim}");
    }
}

const STUTTER: &str = "probabilistic Steps using ev on M over (m: M::Msg) under Even on origin \
     { probability >= 0.5 [ steps from holds(m.attempts = 0) until holds(m.delivered and m.attempts = 0) \
     within 4 <= 10 ] }\n\
     probabilistic Cost using ev on M over (m: M::Msg) under every scheduler on origin \
     { probability >= 0.5 [ accumulate cost from holds(m.attempts = 0) \
     until holds(m.delivered and m.attempts = 0) within 4 >= 1 ] }";

/// Checks FR-196 "Behavior": a bounded measure continues on the terminal stutter
/// steps through its window, each a step of zero reward, and the window's end
/// decides it: here `B` never holds, so the measure ends censored.
#[trace("TC-631")]
#[test]
fn a_measure_continues_on_the_stutter_steps_to_the_end_of_its_window() {
    let world = World::new(&prob_world::link(STUTTER));
    for (claim, kind) in [("Steps", ProductKind::Dtmc), ("Cost", ProductKind::Mdp)] {
        let product = built(&world, claim);
        assert_eq!(product.kind, kind, "{claim}");
        let stutters: Vec<&ProbEdge> = product
            .edges
            .iter()
            .filter(|edge| matches!(edge.choice, ProductAction::TerminalStutter))
            .collect();
        assert!(!stutters.is_empty(), "{claim}");
        for edge in &stutters {
            let remaining = |state: ProbStateId| match product.states[state.index()].monitor {
                Some(MonitorState::Measure { remaining, .. }) => remaining,
                other => panic!("{claim}: a measure: {other:?}"),
            };
            assert_eq!(remaining(edge.to) + 1, remaining(edge.from), "{claim}");
            assert_eq!(edge.probability, ratio(1, 1), "{claim}");
            // A stutter step adds a step to `steps` and nothing to `cost`.
            let (before, after) = (
                &product.states[edge.from.index()].accumulator,
                &product.states[edge.to.index()].accumulator,
            );
            match (before, after, claim) {
                (
                    Some(Accumulator::Steps {
                        amount: Amount::Exact(before),
                        ..
                    }),
                    Some(Accumulator::Steps { amount, .. }),
                    "Steps",
                ) => {
                    if remaining(edge.to) == 0 {
                        assert_eq!(*amount, Amount::Censored);
                    } else {
                        assert_eq!(*amount, Amount::Exact(before.add(&ratio(1, 1))));
                    }
                }
                (
                    Some(Accumulator::Reward { amount: before, .. }),
                    Some(Accumulator::Reward { amount, .. }),
                    "Cost",
                ) => {
                    if remaining(edge.to) == 0 {
                        assert_eq!(*amount, Amount::Censored);
                    } else {
                        assert_eq!(amount, before);
                    }
                }
                other => panic!("{claim}: {other:?}"),
            }
        }
        let ended: Vec<_> = product
            .states
            .iter()
            .filter(|key| key.monitor.is_some_and(MonitorState::is_decided))
            .collect();
        assert!(!ended.is_empty(), "{claim}");
        for key in ended {
            assert!(
                matches!(
                    key.monitor,
                    Some(MonitorState::Measure {
                        remaining: 0,
                        phase: MeasurePhase::Censored
                    })
                ),
                "{claim}: {:?}",
                key.monitor
            );
        }
    }
}

const SPREAD: &str = "probabilistic Merge using inf on M over (n: M::Node) under every scheduler \
     on origin { probability >= 0.5 [ eventually holds(n.healthy) ] }\n\
     probabilistic Plain using inf on M over (n: M::Node) under Steady \
     on origin { probability >= 0.5 [ eventually holds(n.healthy) ] }";

const REWARDS: &str = "probabilistic Spread using ev on M over (s: M::Server) \
     under every scheduler on origin { long_run_fraction holds(s.phase = 0) >= 0.5 }";

/// Checks FR-196 "Behavior": over every scheduler, draws of one action that reach
/// the same post-state with the same rewards are one edge with their summed
/// probability and no draw; draws whose rewards differ stay apart, and under
/// a workload every draw is its own edge.
#[trace("TC-631")]
#[test]
fn draws_to_one_post_state_are_one_edge_over_every_scheduler() {
    let model = prob_world::health(
        "self.healthy = pre(self.healthy)",
        "true: 1, false: 3",
        false,
        SPREAD,
    );
    let world = World::new(&model);
    let merged = built(&world, "Merge");
    let first = start(&merged);
    let [edge] = merged.outgoing(first) else {
        panic!("one edge: {:?}", merged.outgoing(first));
    };
    assert_eq!(
        (edge.to, edge.probability.clone(), edge.draw.is_none()),
        (first, ratio(1, 1), true)
    );
    assert!(edge.action.is_some());
    let plain = built(&world, "Plain");
    let edges = plain.outgoing(start(&plain));
    assert_eq!(edges.len(), 2);
    assert_eq!(
        distinct(edges.iter().map(|edge| edge.probability.clone()).collect()),
        [ratio(1, 4), ratio(3, 4)]
    );
    assert!(edges.iter().all(|edge| edge.draw.is_some()));

    let service = World::new(&prob_world::service(REWARDS));
    let product = built(&service, "Spread");
    let by_edges: Vec<(usize, bool)> = product
        .ids()
        .map(|state| product.outgoing(state))
        .filter(|edges| edges.len() > 1)
        .map(|edges| (edges.len(), edges.iter().all(|edge| edge.draw.is_none())))
        .collect();
    // From attempt 0 the outcome decides the next state: six draws, six
    // edges. From attempt 1 both outcomes reach the same state, so the two
    // draws of one `d` are one edge.
    assert!(by_edges.contains(&(6, false)), "{by_edges:?}");
    assert!(by_edges.contains(&(3, true)), "{by_edges:?}");
    let merged_edge = product
        .edges
        .iter()
        .filter(|edge| edge.draw.is_none())
        .find(|edge| integer_of(&edge.rewards[0].1) == 3)
        .expect("a merged edge for d = 3");
    assert_eq!(merged_edge.probability, ratio(7, 100));
}

const REACH: &str =
    "probabilistic Terminates using inf on M over (p: M::Proc) under every scheduler \
     on origin { probability >= 0.99 [ eventually holds(p.done) ] }\n\
     probabilistic FairTerminates using inf on M over (p: M::Proc) \
     under every scheduler fair strong Proc::flip \
     on origin { probability >= 0.99 [ eventually holds(p.done) ] }\n\
     probabilistic WeakFairTerminates using inf on M over (p: M::Proc) \
     under every scheduler fair weak Proc::flip \
     on origin { probability >= 0.99 [ eventually holds(p.done) ] }\n\
     probabilistic FairMax using inf on M over (p: M::Proc) \
     under every scheduler fair strong Proc::wait \
     on origin { probability <= 0.99 [ eventually holds(p.done) ] }\n\
     probabilistic Unless using inf on M over (p: M::Proc) under Even on origin \
     { probability >= 0.9 [ holds(not p.done) until holds(p.done) ] }";

/// Checks FR-196 "Behavior", exact-only forms: an unbounded reachability claim uses
/// the MDP or DTMC itself as the product, with the claim's predicates as
/// state labels and no monitor, accumulator or pending draw in any key. The
/// fairness set belongs to the claim, not the product.
#[trace("TC-631")]
#[test]
fn an_unbounded_form_labels_the_model_states() {
    let world = World::new(&prob_world::process(REACH));
    let product = built(&world, "Terminates");
    assert_eq!(product.kind, ProductKind::Mdp);
    for key in &product.states {
        assert!(key.monitor.is_none() && key.accumulator.is_none() && key.pending.is_none());
    }
    assert_eq!(product.states.len(), 2);
    assert!(product.decided.is_empty());
    assert_eq!(product.labels, [[false], [true]]);
    let live = start(&product);
    let edges = product.outgoing(live);
    let flip: Vec<&ProbEdge> = edges
        .iter()
        .filter(|edge| {
            edge.action
                .as_ref()
                .is_some_and(|a| a.operation.contains("flip"))
        })
        .collect();
    let wait: Vec<&ProbEdge> = edges
        .iter()
        .filter(|edge| {
            edge.action
                .as_ref()
                .is_some_and(|a| a.operation.contains("wait"))
        })
        .collect();
    assert_eq!(flip.len(), 2);
    assert_eq!(wait.len(), 1);
    // `wait` repeats the state; `flip` ends it with probability 1/2.
    assert_eq!(
        (wait[0].to, wait[0].probability.clone()),
        (live, ratio(1, 1))
    );
    for edge in &flip {
        assert_eq!(edge.probability, ratio(1, 2));
        let heads = boolean_of(&edge.draw.as_ref().expect("a draw")[0]);
        assert_eq!(edge.to == live, !heads);
    }
    let done = flip.iter().find(|edge| edge.to != live).expect("done").to;
    let [stutter] = product.outgoing(done) else {
        panic!("the terminal state only stutters");
    };
    assert!(matches!(stutter.choice, ProductAction::TerminalStutter));
    assert_eq!(
        (stutter.to, stutter.probability.clone()),
        (done, ratio(1, 1))
    );

    let fair = built(&world, "FairTerminates");
    assert_eq!(fair.states, product.states);
    assert_eq!(fair.labels, product.labels);
    assert_eq!(fair.edges.len(), product.edges.len());

    let until = built(&world, "Unless");
    assert_eq!(until.kind, ProductKind::Dtmc);
    // `A` then `B`: not yet done, then done.
    assert_eq!(until.labels, [[true, false], [false, true]]);
}

/// FR-200-AC-1 (TC-635): fairness removes the live wait component for both
/// weak and strong fairness to `flip`, making termination's infimum one.
#[trace("TC-635", "FR-200-AC-1")]
#[test]
fn fair_coin_has_no_nonterminal_fair_end_component() {
    let world = World::new(&prob_world::process(REACH));
    for name in ["FairTerminates", "WeakFairTerminates"] {
        let product = built(&world, name);
        let target = product
            .labels
            .iter()
            .map(|labels| labels[0])
            .collect::<Vec<_>>();
        let components = maximal_fair_end_components(&product, &product.fairness)
            .expect("well-formed checked fairness");
        assert!(
            components
                .iter()
                .all(|component| component.states.iter().all(|state| target[state.index()])),
            "only target terminal stutter components may remain globally: {name}: {components:?}"
        );
        let result = decide_fair_reachability(
            &product,
            None,
            &target,
            UnboundedObjective::Min,
            &ExactProbLimits::default(),
        )
        .expect("exact fair reachability");
        assert!(
            result.components.is_empty(),
            "the non-target restriction has no fair end component"
        );
        assert_eq!(
            result.values[start(&product).index()],
            fsl_analyze::exact_probabilistic::ExtRational::Finite(ratio(1, 1)),
            "target={target:?}, product labels={:?}, fair states={:?}",
            product.labels,
            result.components
        );
        assert_eq!(result.fairness, product.fairness);
    }
    let plain = built(&world, "Terminates");
    let target = plain
        .labels
        .iter()
        .map(|labels| labels[0])
        .collect::<Vec<_>>();
    let plain_min = fsl_analyze::exact_probabilistic::decide_reachability(
        &plain,
        &target,
        UnboundedObjective::Min,
        &ExactProbLimits::default(),
    )
    .expect("exact unrestricted reachability");
    assert_eq!(
        plain_min.values[start(&plain).index()],
        fsl_analyze::exact_probabilistic::ExtRational::Finite(ratio(0, 1))
    );

    let maximizing = built(&world, "FairMax");
    let components = maximal_fair_end_components(&maximizing, &maximizing.fairness)
        .expect("well-formed fairness");
    assert!(components.iter().any(|component| {
        component.states.contains(&start(&maximizing))
            && component
                .actions
                .iter()
                .any(|(_, action)| action.operation.contains("wait"))
    }));
    let target = maximizing
        .labels
        .iter()
        .map(|labels| labels[0])
        .collect::<Vec<_>>();
    let maximum = decide_fair_reachability(
        &maximizing,
        None,
        &target,
        UnboundedObjective::Max,
        &ExactProbLimits::default(),
    )
    .expect("fair maximum reachability");
    assert_eq!(
        maximum.values[start(&maximizing).index()],
        fsl_analyze::exact_probabilistic::ExtRational::Finite(ratio(1, 1))
    );
    let flip_index = match maximum.policy[start(&maximizing).index()]
        .as_ref()
        .expect("a maximizing action")
    {
        fsl_analyze::exact_probabilistic::SchedulerChoice::Deterministic(
            fsl_analyze::exact_probabilistic::ActionChoice(index),
        ) => *index,
        other => panic!("the positive-value state takes FR-198's policy: {other:?}"),
    };
    let mut operations = Vec::new();
    for edge in maximizing.outgoing(start(&maximizing)) {
        if let Some(action) = edge.action.as_ref() {
            if !operations.iter().any(|seen| seen == action) {
                operations.push(action.clone());
            }
        }
    }
    assert!(operations[flip_index].operation.contains("flip"));
}

/// FSpec FR-411-AC-10 and FR-412-AC-7: EN-5 emits a fair-MDP certificate
/// for the fair Coin process and its independent checker accepts value one.
#[trace("FR-411-AC-10", "FR-412-AC-7")]
#[test]
fn fair_coin_analysis_emits_independently_checked_value_one() {
    use fsl_analyze::analyze::{analyze_items, exact_probabilistic_item, Engines};
    use fsl_foundation::RequestIndex;
    use fsl_replay::TerminalValue;

    let world = World::new(&prob_world::process(REACH));
    let records = analyze_items(
        &Engines::fsl().expect("FSL engines"),
        vec![exact_probabilistic_item(
            RequestIndex::new(0),
            world.request("FairTerminates", ModelCheckLimits::default()),
        )],
        &Cancel::new(),
    );
    let record = records.into_iter().next().expect("one analyzed claim");
    let terminal = record
        .terminal()
        .unwrap_or_else(|| panic!("fair claim did not settle: {:?}", record.settlement));
    assert!(matches!(terminal.value(), TerminalValue::Proved { .. }));
    let bytes = terminal
        .exact_certificate_document()
        .expect("independently checked exact certificate");
    let certificate = fsl_replay::read_probability_proof_document(
        bytes,
        fsl_replay::ProbabilityDocumentLimits::for_exact(
            &ExactProbLimits::default(),
            fsl_replay::AccountingLimits::default().0,
        ),
        &Cancel::new(),
    )
    .expect("strict fair-MDP certificate");
    assert!(certificate.is_fair_mdp());
    assert_eq!(certificate.document().values.len(), 2);
    assert!(certificate
        .document()
        .values
        .iter()
        .all(|entry| entry.value == ratio(1, 1)));
}

/// FR-1041-AC-1/2/4: W has `wait` at s and a fair coin `go` to terminal g/f;
/// the minimum and maximum are both 1/2, while malformed Bellman, rank and
/// fairness records are rejected at their specified locations.
#[trace("TC-3150", "FR-1041-AC-1", "FR-1041-AC-2", "FR-1041-AC-4")]
#[test]
fn fair_eventual_w_exact_minimum_maximum_and_forgery_vectors() {
    use fsl_analyze::analyze::{analyze_items, exact_probabilistic_item, Engines};
    use fsl_eval::simulation::model::ModelSystem;
    use fsl_foundation::RequestIndex;
    use fsl_replay::{
        ExactProbabilityCertificateCheck, ProbabilityCertificateLocus, ProbabilityCertificateRule,
        ProbabilityDocumentError, ProofBasis, TerminalValue,
    };

    const CLAIMS: &str = "probabilistic FairMin using inf on M over (w: M::W) \
         under every scheduler fair strong W::go on origin \
         { probability >= rational(1,2) [ eventually holds(w.g) ] }\n\
         probabilistic FairMax using inf on M over (w: M::W) \
         under every scheduler fair strong W::go on origin \
         { probability <= rational(1,2) [ eventually holds(w.g) ] }\n\
         probabilistic WeakFairMin using inf on M over (w: M::W) \
         under every scheduler fair weak W::go on origin \
         { probability >= rational(1,2) [ eventually holds(w.g) ] }\n\
         probabilistic AboveMax using inf on M over (w: M::W) \
         under every scheduler fair strong W::go on origin \
         { probability <= rational(2,5) [ eventually holds(w.g) ] }\n\
         probabilistic NoFairMin using inf on M over (w: M::W) under every scheduler on origin \
         { probability >= rational(1,2) [ eventually holds(w.g) ] }";
    let world = World::new(&prob_world::fair_bounds_w(CLAIMS));
    let records = analyze_items(
        &Engines::fsl().expect("EN-5 is admitted"),
        ["FairMin", "FairMax", "WeakFairMin", "AboveMax", "NoFairMin"]
            .into_iter()
            .enumerate()
            .map(|(index, name)| {
                exact_probabilistic_item(
                    RequestIndex::new(index),
                    world.request(name, ModelCheckLimits::default()),
                )
            })
            .collect(),
        &Cancel::new(),
    );
    let min = records[0].terminal().expect("AC-1 minimum settles");
    let TerminalValue::Proved {
        basis: ProofBasis::ExactValue { value, .. },
        ..
    } = min.value()
    else {
        panic!("the fair minimum is exact: {:#?}", records[0]);
    };
    assert_eq!(value.value(), &ratio(1, 2));
    let TerminalValue::Proved {
        basis: ProofBasis::ExactValue { value, .. },
        ..
    } = records[1].terminal().expect("AC-4 maximum settles").value()
    else {
        panic!("the fair maximum is exact: {:#?}", records[1]);
    };
    assert_eq!(value.value(), &ratio(1, 2));
    assert_eq!(
        records[3]
            .terminal()
            .unwrap_or_else(|| panic!("lower maximum bound record: {:#?}", records[3]))
            .value(),
        &TerminalValue::Refuted
    );
    assert_eq!(
        records[4]
            .terminal()
            .expect("the fairness-empty minimum settles")
            .value(),
        &TerminalValue::Refuted
    );
    let TerminalValue::Proved {
        basis: ProofBasis::ExactValue { value, .. },
        ..
    } = records[2]
        .terminal()
        .expect("weak fair minimum settles")
        .value()
    else {
        panic!("the weak fair minimum is exact: {:#?}", records[2]);
    };
    assert_eq!(value.value(), &ratio(1, 2));

    let limits = fsl_replay::ProbabilityDocumentLimits::for_exact(
        &ExactProbLimits::default(),
        fsl_replay::AccountingLimits::default().0,
    );
    let system = ModelSystem::new(world.subject().simulation).expect("checked W model");
    let identity =
        fsl_replay::probabilistic_claim_identity(world.claim("FairMin")).expect("AC-1 identity");
    let check = |bytes: &[u8]| {
        fsl_replay::check_probability_certificate_document(
            &system,
            system.input().probabilistic.claims(),
            &identity,
            bytes,
            limits,
            &Cancel::new(),
        )
    };
    let min_bytes = records[0]
        .terminal()
        .unwrap()
        .exact_certificate_document()
        .expect("AC-1 fair-minimum document");
    let min_check = check(min_bytes).expect("the strict reader checks AC-1 document");
    assert!(matches!(
        min_check,
        ExactProbabilityCertificateCheck::Accepted(_)
    ));
    let max_identity =
        fsl_replay::probabilistic_claim_identity(world.claim("FairMax")).expect("AC-4 identity");
    let max_bytes = records[1]
        .terminal()
        .unwrap()
        .exact_certificate_document()
        .expect("AC-4 fair-maximum document");
    assert!(matches!(
        fsl_replay::check_probability_certificate_document(
            &system,
            system.input().probabilistic.claims(),
            &max_identity,
            max_bytes,
            limits,
            &Cancel::new(),
        ),
        Ok(ExactProbabilityCertificateCheck::Accepted(_))
    ));
    let weak_bytes = records[2]
        .terminal()
        .unwrap()
        .exact_certificate_document()
        .expect("AC-1 weak-fair document");
    let weak_doc: serde_json::Value = serde_json::from_slice(weak_bytes).unwrap();
    let strong_doc: serde_json::Value = serde_json::from_slice(min_bytes).unwrap();
    assert_ne!(
        strong_doc["objective"]["fairness"],
        weak_doc["objective"]["fairness"]
    );
    let weak_identity = fsl_replay::probabilistic_claim_identity(world.claim("WeakFairMin"))
        .expect("weak fairness identity");
    assert!(matches!(
        fsl_replay::check_probability_certificate_document(
            &system,
            system.input().probabilistic.claims(),
            &weak_identity,
            weak_bytes,
            limits,
            &Cancel::new(),
        ),
        Ok(ExactProbabilityCertificateCheck::Accepted(_))
    ));
    let parsed = fsl_replay::read_probability_proof_document(min_bytes, limits, &Cancel::new())
        .expect("strict AC-1 document read");
    let document = parsed.document();
    assert_eq!(document.values.len(), 3);
    let mut values = document
        .values
        .iter()
        .map(|row| row.value.clone())
        .collect::<Vec<_>>();
    values.sort();
    assert_eq!(values, vec![ratio(0, 1), ratio(1, 2), ratio(1, 1)]);
    let mut ranks = document
        .ranking
        .iter()
        .map(|(_, rank)| *rank)
        .collect::<Vec<_>>();
    ranks.sort();
    assert_eq!(ranks, vec![0, 0, 1]);

    let base: serde_json::Value =
        serde_json::from_slice(min_bytes).expect("canonical AC-1 document JSON");
    let values = base["values"].as_array().unwrap();
    let state_key = |row: &serde_json::Value| {
        use base64::Engine;
        fsl_replay::ProbabilityProductKey::from_canonical_bytes(
            base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(row["state"].as_str().unwrap())
                .unwrap(),
        )
    };
    let s = values
        .iter()
        .find(|row| row["value"] == serde_json::json!({"numerator":"1","denominator":"2"}))
        .expect("s carries the literal 1/2 value");
    let s_key = state_key(s);
    let f = values
        .iter()
        .find(|row| row["value"] == serde_json::json!({"numerator":"0","denominator":"1"}))
        .expect("f carries the literal zero value");
    let f_key = state_key(f);
    let reject_at = |bytes: &[u8], rule, key: fsl_replay::ProbabilityProductKey| match check(bytes)
        .expect("a well-shaped forged value is rejected by the checker")
    {
        ExactProbabilityCertificateCheck::Rejected(rejection) => {
            assert_eq!(rejection.rule, rule);
            assert_eq!(rejection.at, ProbabilityCertificateLocus::ProductState(key));
        }
        other => panic!("forged AC-2 values must be rejected: {other:?}"),
    };
    for numerator in ["0", "1"] {
        let mut forged = base.clone();
        let row = forged["values"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|row| row["state"] == s["state"])
            .unwrap();
        row["value"] = if numerator == "0" {
            serde_json::json!({"numerator":"0","denominator":"1"})
        } else {
            serde_json::json!({"numerator":"1","denominator":"4"})
        };
        reject_at(
            &serde_json::to_vec(&forged).unwrap(),
            ProbabilityCertificateRule::RankBroken,
            s_key.clone(),
        );
    }
    let mut forged_f = base.clone();
    let row = forged_f["values"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["state"] == f["state"])
        .unwrap();
    row["value"] = serde_json::json!({"numerator":"1","denominator":"2"});
    reject_at(
        &serde_json::to_vec(&forged_f).unwrap(),
        ProbabilityCertificateRule::InequalityFails,
        f_key,
    );

    let mut without_fairness = base.clone();
    without_fairness["objective"]["fairness"] = serde_json::json!([]);
    let without_fairness_result = check(&serde_json::to_vec(&without_fairness).unwrap());
    assert!(
        matches!(without_fairness_result, Err(ProbabilityDocumentError::ObjectiveMismatch)),
        "removing the fairness record is rejected as objective mismatch: {without_fairness_result:?}"
    );
    let mut changed = serde_json::from_slice::<serde_json::Value>(min_bytes).unwrap();
    changed["objective"]["fairness"][0]["target"]["declaration"]["package"] =
        serde_json::json!("test/foreign-model");
    assert!(matches!(
        check(&serde_json::to_vec(&changed).unwrap()),
        Err(ProbabilityDocumentError::ObjectiveMismatch)
    ));
    let mut duplicate = serde_json::from_slice::<serde_json::Value>(min_bytes).unwrap();
    let row = duplicate["objective"]["fairness"][0].clone();
    duplicate["objective"]["fairness"]
        .as_array_mut()
        .unwrap()
        .push(row);
    let malformed = check(&serde_json::to_vec(&duplicate).unwrap())
        .expect("duplicate fairness rows are a checked malformed rejection");
    assert!(matches!(
        malformed,
        ExactProbabilityCertificateCheck::Rejected(ref rejection)
            if rejection.rule == ProbabilityCertificateRule::Malformed
                && rejection.at == ProbabilityCertificateLocus::Member("objective")
    ));
}

/// FSL FR-1042-AC-22 / FSpec FR-412-AC-13: EN-5 encloses the exact fair
/// minimum 1/2 under a one-bit rational budget and the independent reader
/// accepts both emitted value-bound documents.
#[trace("TC-3212", "FR-1042-AC-22", "FR-412-AC-13")]
#[test]
fn fair_eventual_value_bounds_are_independently_checked() {
    use fsl_analyze::analyze::{analyze_items, exact_probabilistic_item, Engines};
    use fsl_eval::simulation::model::ModelSystem;
    use fsl_foundation::RequestIndex;
    use fsl_replay::{ExactProbabilityCertificateCheck, ProofBasis, TerminalValue};

    const CLAIMS: &str = "probabilistic FairBound using inf on M over (w: M::W) \
         under every scheduler fair strong W::go on origin \
         { probability >= rational(2,5) [ eventually holds(w.g) ] }";
    let world = World::new(&prob_world::fair_bounds_w(CLAIMS));
    assert!(matches!(
        world.claim("FairBound").form,
        fsl_semantics::check::ProbForm::Reach {
            target: fsl_semantics::check::ReachTarget::Eventually(_),
            ..
        }
    ));
    let mut request = world.request("FairBound", ModelCheckLimits::default());
    request.limits.max_rational_bits = 1;
    request.limits.max_precision_bits = 16;
    let limits = request.limits.clone();
    let record = analyze_items(
        &Engines::fsl().expect("EN-5 is admitted"),
        vec![exact_probabilistic_item(RequestIndex::new(0), request)],
        &Cancel::new(),
    )
    .remove(0);
    let terminal = record
        .terminal()
        .unwrap_or_else(|| panic!("the fair claim settles: {record:#?}"));
    let TerminalValue::Proved {
        basis: ProofBasis::ValueBounds { lower, upper, .. },
        ..
    } = terminal.value()
    else {
        panic!("the fair value is enclosed by checked bounds: {record:#?}");
    };
    let lower_value = lower.value();
    let upper_value = match upper {
        fsl_replay::ExactBound::Finite(value) => value.value(),
        fsl_replay::ExactBound::Infinity => panic!("finite upper bound"),
    };
    assert!(lower_value <= &ratio(1, 2) && &ratio(1, 2) <= upper_value);
    assert!(lower_value >= &ratio(2, 5));

    let system = ModelSystem::new(world.subject().simulation).expect("checked fair model");
    let identity =
        fsl_replay::probabilistic_claim_identity(world.claim("FairBound")).expect("claim identity");
    let doc_limits =
        fsl_replay::ProbabilityDocumentLimits::for_interval(&limits, prob_world::UNLIMITED);
    for (bytes, role) in [
        (
            terminal
                .exact_certificate_document()
                .expect("lower document"),
            fsl_replay::CertificateRole::Claim,
        ),
        (
            terminal
                .exact_counterpart_document()
                .expect("upper document"),
            fsl_replay::CertificateRole::Counterpart,
        ),
    ] {
        assert!(matches!(
            fsl_replay::check_probability_certificate_document_as(
                &system,
                system.input().probabilistic.claims(),
                &identity,
                bytes,
                role,
                doc_limits,
                &Cancel::new(),
            ),
            Ok(ExactProbabilityCertificateCheck::Accepted(_))
        ));
    }
    let mut forged: serde_json::Value = serde_json::from_slice(
        terminal
            .exact_certificate_document()
            .expect("lower document"),
    )
    .expect("canonical lower JSON");
    let rows = forged["values"].as_array_mut().expect("base values");
    let old = rows[0]["value"].clone();
    rows[0]["value"]["numerator"] = serde_json::json!("1");
    rows[0]["value"]["denominator"] = serde_json::json!("3");
    assert_ne!(rows[0]["value"], old);
    assert!(matches!(
        fsl_replay::check_probability_certificate_document_as(
            &system,
            system.input().probabilistic.claims(),
            &identity,
            &serde_json::to_vec(&forged).unwrap(),
            fsl_replay::CertificateRole::Claim,
            doc_limits,
            &Cancel::new(),
        ),
        Ok(ExactProbabilityCertificateCheck::Rejected(_))
    ));
}

/// FSL FR-1042-AC-4 / FSpec FR-412-AC-20: the upper fair bound's `go` hint
/// selects a policy action with rank progress; a `wait` hint is rejected, and
/// omitting the hint lets the checker find the satisfying action itself.
#[trace("TC-3212", "FR-1042-AC-4", "FR-412-AC-20")]
#[test]
fn fair_eventual_upper_policy_hint_is_optional_but_independently_checked() {
    use fsl_analyze::analyze::{analyze_items, exact_probabilistic_item, Engines};
    use fsl_eval::simulation::{model::ModelSystem, TransitionSystem};
    use fsl_foundation::RequestIndex;
    use fsl_replay::{
        ExactProbabilityCertificateCheck, ProbabilityCertificateRule, ProofBasis, TerminalValue,
    };

    const CLAIMS: &str = "probabilistic FairBound using inf on M over (w: M::W) \
         under every scheduler fair strong W::go on origin \
         { probability >= rational(2,5) [ eventually holds(w.g) ] }";
    let world = World::new(&prob_world::fair_bounds_w(CLAIMS));
    let mut request = world.request("FairBound", ModelCheckLimits::default());
    request.limits.max_rational_bits = 1;
    request.limits.max_precision_bits = 16;
    let limits = request.limits.clone();
    let record = analyze_items(
        &Engines::fsl().expect("EN-5 is admitted"),
        vec![exact_probabilistic_item(RequestIndex::new(0), request)],
        &Cancel::new(),
    )
    .remove(0);
    let TerminalValue::Proved {
        basis: ProofBasis::ValueBounds { upper, .. },
        ..
    } = record.terminal().expect("the interval settles").value()
    else {
        panic!("the fair exact value is enclosed: {record:#?}");
    };
    assert!(match upper {
        fsl_replay::ExactBound::Finite(value) => value.value() >= &ratio(1, 2),
        fsl_replay::ExactBound::Infinity => false,
    });
    let upper_bytes = record
        .terminal()
        .unwrap()
        .exact_counterpart_document()
        .expect("upper value-bound document");
    let lower_bytes = record
        .terminal()
        .unwrap()
        .exact_certificate_document()
        .expect("lower value-bound document");
    let system = ModelSystem::new(world.subject().simulation).expect("checked fair model");
    let identity =
        fsl_replay::probabilistic_claim_identity(world.claim("FairBound")).expect("claim identity");
    let doc_limits =
        fsl_replay::ProbabilityDocumentLimits::for_interval(&limits, prob_world::UNLIMITED);
    let check = |bytes: &[u8], role| {
        fsl_replay::check_probability_certificate_document_as(
            &system,
            system.input().probabilistic.claims(),
            &identity,
            bytes,
            role,
            doc_limits,
            &Cancel::new(),
        )
    };
    let initial = system
        .initial()
        .into_iter()
        .next()
        .expect("one literal initial w state");
    let actions = system
        .actions(&initial)
        .expect("initial actions expand")
        .expect("initial actions are decided");
    let transition = |name: &str| {
        let expected = format!("ix://test/fair-bounds-w/W/{name}");
        let action = actions
            .iter()
            .find(|action| {
                serde_json::from_str::<serde_json::Value>(&action.identity.operation)
                    .ok()
                    .and_then(|identity| {
                        identity
                            .get("node")
                            .and_then(|node| node.as_str())
                            .map(str::to_owned)
                    })
                    .as_deref()
                    == Some(expected.as_str())
            })
            .unwrap_or_else(|| panic!("the literal {name} action is enabled"));
        system
            .scheduled_identity_key(&action.identity)
            .expect("scheduled transition identity is canonical")
    };
    let go = serde_json::from_slice::<serde_json::Value>(&transition("go"))
        .expect("go transition identity is canonical JSON");
    let wait = serde_json::from_slice::<serde_json::Value>(&transition("wait"))
        .expect("wait transition identity is canonical JSON");
    let mut upper: serde_json::Value =
        serde_json::from_slice(upper_bytes).expect("upper value-bound JSON is canonical");
    assert_eq!(
        upper["objective"]["base"]["profile"],
        "filament.probability-objective/fair-mdp-eventual/v1"
    );
    assert_eq!(upper["objective"]["base"]["optimization"], "minimum");
    let targets = upper["objective"]["base"]["targets"]
        .as_array()
        .expect("the base objective lists its literal target product states")
        .clone();
    let ranking = upper["ranking"].as_array().expect("upper ranking rows");
    let root = ranking
        .iter()
        .find(|row| row["rank"] == serde_json::json!(1))
        .expect("s has the literal progressing rank 1")["state"]
        .clone();
    let mut lower: serde_json::Value =
        serde_json::from_slice(lower_bytes).expect("lower value-bound JSON is canonical");
    let set_root_value = |document: &mut serde_json::Value| {
        let rows = document["values"]
            .as_array_mut()
            .expect("literal value rows");
        let row = rows
            .iter_mut()
            .find(|row| row["state"] == root)
            .expect("the same initial product state is named in the values");
        row["value"] = serde_json::json!({"numerator": "1", "denominator": "2"});
    };
    set_root_value(&mut lower);
    set_root_value(&mut upper);
    for row in upper["values"].as_array().unwrap() {
        let expected = if row["state"] == root {
            serde_json::json!({"numerator": "1", "denominator": "2"})
        } else if targets.iter().any(|target| target == &row["state"]) {
            serde_json::json!({"numerator": "1", "denominator": "1"})
        } else {
            serde_json::json!({"numerator": "0", "denominator": "1"})
        };
        assert_eq!(row["value"], expected);
    }
    assert_eq!(upper["ranking"].as_array().unwrap().len(), 3);
    for row in upper["ranking"].as_array().unwrap() {
        assert_eq!(row["rank"], if row["state"] == root { 1 } else { 0 });
    }
    assert!(lower.get("ranking").is_none());
    assert!(lower.get("policy").is_none());
    let lower_replay = fsl_replay::check_probability_certificate_document_as(
        &system,
        system.input().probabilistic.claims(),
        &identity,
        &serde_json::to_vec(&lower).unwrap(),
        fsl_replay::CertificateRole::Claim,
        doc_limits,
        &Cancel::new(),
    )
    .expect("literal lower bound is checked");
    assert!(matches!(
        lower_replay,
        ExactProbabilityCertificateCheck::Accepted(_)
    ));
    let upper_replay = fsl_replay::check_probability_certificate_document_as(
        &system,
        system.input().probabilistic.claims(),
        &identity,
        &serde_json::to_vec(&upper).unwrap(),
        fsl_replay::CertificateRole::Counterpart,
        doc_limits,
        &Cancel::new(),
    )
    .expect("literal upper bound is checked");
    assert!(matches!(
        upper_replay,
        ExactProbabilityCertificateCheck::Accepted(_)
    ));
    upper["policy"] = serde_json::json!([{"state": root, "choice": go}]);
    let supplied_go = fsl_replay::check_probability_certificate_document_as(
        &system,
        system.input().probabilistic.claims(),
        &identity,
        &serde_json::to_vec(&upper).unwrap(),
        fsl_replay::CertificateRole::Counterpart,
        doc_limits,
        &Cancel::new(),
    );
    assert!(
        matches!(
            supplied_go,
            Ok(ExactProbabilityCertificateCheck::Accepted(_))
        ),
        "the explicit go policy satisfies the upper bound: {supplied_go:?}"
    );
    upper["policy"][0]["choice"] = wait;
    let refused = check(
        &serde_json::to_vec(&upper).unwrap(),
        fsl_replay::CertificateRole::Counterpart,
    )
    .expect("a canonical but non-progressing policy is a checked rejection");
    assert!(matches!(
        refused,
        ExactProbabilityCertificateCheck::Rejected(ref rejection)
            if rejection.rule == ProbabilityCertificateRule::RankBroken
    ));
}

/// FR-412-AC-7 and FR-1041-AC-1/5: a forged value-one certificate with a
/// locally valid Bellman equation and progress rank must still be rejected
/// when an independently reconstructed fair end component can avoid target.
#[trace("FR-412-AC-7")]
#[test]
fn fair_mdp_checker_rejects_a_fair_non_target_end_component() {
    use fsl_analyze::analyze::{analyze_items, exact_probabilistic_item, Engines};
    use fsl_foundation::RequestIndex;
    use fsl_replay::{
        check_probability_certificate_document, probability_proof_document,
        write_probability_proof_document, ExactProbabilityCertificateCheck,
    };

    let claims = format!(
        "{REACH}\
         probabilistic FairWait using inf on M over (p: M::Proc) \
         under every scheduler fair strong Proc::wait on origin \
         {{ probability >= 0.99 [ eventually holds(p.done) ] }}\n"
    );
    let world = World::new(&prob_world::process(&claims));
    let valid = analyze_items(
        &Engines::fsl().expect("FSL engines"),
        vec![exact_probabilistic_item(
            RequestIndex::new(0),
            world.request("FairTerminates", ModelCheckLimits::default()),
        )],
        &Cancel::new(),
    );
    let genuine = valid[0]
        .terminal()
        .expect("the fair Coin proof settles")
        .exact_certificate()
        .expect("the independent checker accepted the fair Coin certificate")
        .clone();
    let system = fsl_eval::simulation::model::ModelSystem::new(world.subject().simulation)
        .expect("the checked model is admitted");
    let item = fsl_replay::probabilistic_claim_identity(world.claim("FairWait"))
        .expect("the fair-loop claim identity is canonical");
    let limits = fsl_replay::ProbabilityDocumentLimits::for_exact(
        &ExactProbLimits::default(),
        fsl_replay::AccountingLimits::default().0,
    );
    let forged = probability_proof_document(
        &system,
        world.package.graph().probabilistic_model().claims(),
        &item,
        &genuine,
        limits,
        &Cancel::new(),
    )
    .expect("the claim-shaped value-one candidate has valid local equations and ranks");
    assert!(forged.is_fair_mdp());
    assert!(forged
        .document()
        .values
        .iter()
        .all(|entry| entry.value == ratio(1, 1)));
    let bytes = write_probability_proof_document(&forged, limits, &Cancel::new())
        .expect("the forged fair-MDP document is well-formed");
    let checked = check_probability_certificate_document(
        &system,
        world.package.graph().probabilistic_model().claims(),
        &item,
        &bytes,
        limits,
        &Cancel::new(),
    );
    assert!(
        matches!(checked, Ok(ExactProbabilityCertificateCheck::Rejected(_))),
        "a fair wait self-loop is a non-target fair end component: {checked:?}"
    );
}

/// A terminal stutter-only component is fair when it remains in the
/// non-target graph, despite having no scheduled identities.
#[trace("TC-635", "FR-200-AC-5")]
#[test]
fn non_target_terminal_stutter_belongs_to_fair_reachability_u() {
    const CLAIM: &str = "probabilistic TerminalFair using inf on M over (t: M::TerminalFair) \
         under every scheduler fair strong TerminalFair::proceed on origin \
         { probability >= 0.5 [ eventually holds(t.done) ] }";
    let world = World::new(&prob_world::fair_terminal_stutter(CLAIM));
    let product = built(&world, "TerminalFair");
    let target = product
        .labels
        .iter()
        .map(|labels| labels[0])
        .collect::<Vec<_>>();
    let result = decide_fair_reachability(
        &product,
        None,
        &target,
        UnboundedObjective::Min,
        &ExactProbLimits::default(),
    )
    .expect("non-target terminal stutter is a fair end component");
    let terminal_stutter = result
        .components
        .iter()
        .find(|component| component.actions.is_empty())
        .expect("terminal stutter is included in U");
    assert!(terminal_stutter
        .states
        .iter()
        .any(|state| !target[state.index()]));
    assert_eq!(
        result.values[start(&product).index()],
        fsl_analyze::exact_probabilistic::ExtRational::Finite(ratio(0, 1))
    );
    let initial = start(&product);
    let selected = match result.policy[initial.index()]
        .as_ref()
        .expect("the initial state has a fair scheduler choice")
    {
        fsl_analyze::exact_probabilistic::SchedulerChoice::Deterministic(
            fsl_analyze::exact_probabilistic::ActionChoice(index),
        ) => vec![*index],
        fsl_analyze::exact_probabilistic::SchedulerChoice::Randomized(choices) => choices
            .iter()
            .filter(|(_, probability)| probability > &ratio(0, 1))
            .map(|(choice, _)| choice.0)
            .collect(),
    };
    let mut actions = Vec::new();
    for edge in product.outgoing(initial) {
        if let Some(identity) = edge.action.as_ref() {
            if !actions.iter().any(|seen| seen == identity) {
                actions.push(identity.clone());
            }
        }
    }
    assert!(selected
        .iter()
        .any(|index| actions[*index].operation.contains("abandon")));
}

/// Expected-reward minimization ranges over fair schedulers: `free` alone is
/// an unfair recurrent choice, so the unit-cost fair exit is required.
#[test]
fn fair_expected_reward_minimum_excludes_an_unfair_free_bottom_class() {
    const CLAIM: &str = "probabilistic FairMinimum using inf on M over (p: M::Proc) \
         under every scheduler fair strong Proc::costly on origin \
         { expected accumulate cost until holds(p.done) <= 1 }";
    let world = World::new(&prob_world::fair_min_reward(CLAIM));
    let product = built(&world, "FairMinimum");
    let target = product
        .labels
        .iter()
        .map(|labels| labels[0])
        .collect::<Vec<_>>();
    let unrestricted = fsl_analyze::exact_probabilistic::decide_expected_reward(
        &product,
        &target,
        "cost",
        UnboundedObjective::Min,
        &ExactProbLimits::default(),
    )
    .expect("FR-198 minimum over the same fixture");
    assert_eq!(
        unrestricted.values[start(&product).index()],
        fsl_analyze::exact_probabilistic::ExtRational::Finite(ratio(1, 1))
    );
    let unrestricted_choice =
        unrestricted.policy[start(&product).index()].expect("the live FR-198 state has a policy");
    let mut unrestricted_actions = Vec::new();
    for edge in product.outgoing(start(&product)) {
        if let Some(identity) = edge.action.as_ref() {
            if !unrestricted_actions.iter().any(|seen| seen == identity) {
                unrestricted_actions.push(identity.clone());
            }
        }
    }
    assert!(unrestricted_actions[unrestricted_choice]
        .operation
        .contains("costly"));
    let result = fsl_analyze::exact_probabilistic::decide_fair_expected_reward(
        &product,
        &target,
        "cost",
        UnboundedObjective::Min,
        &ExactProbLimits::default(),
    )
    .expect("fair minimum uses the unit-cost exit");
    assert_eq!(
        result.values[start(&product).index()],
        fsl_analyze::exact_probabilistic::ExtRational::Finite(ratio(1, 1))
    );
    let policy = result
        .policy
        .as_ref()
        .expect("the result carries a fair witness");
    let selected = match policy[start(&product).index()]
        .as_ref()
        .expect("the initial state has a scheduler choice")
    {
        fsl_analyze::exact_probabilistic::SchedulerChoice::Deterministic(
            fsl_analyze::exact_probabilistic::ActionChoice(index),
        ) => *index,
        other => panic!("the fair expected-reward minimum is deterministic: {other:?}"),
    };
    let mut actions = Vec::new();
    for edge in product.outgoing(start(&product)) {
        if let Some(identity) = edge.action.as_ref() {
            if !actions.iter().any(|seen| seen == identity) {
                actions.push(identity.clone());
            }
        }
    }
    assert!(
        actions[selected].operation.contains("costly"),
        "{actions:?}"
    );
}

/// FR-200-AC-2 and FR-1041-AC-3: a strong-fair scheduler can reach the stuck
/// state, where `stall` is the only action and the fairness constraint is
/// disabled; the cyclic refutation is emitted and independently replayed.
#[trace("TC-635", "FR-200-AC-2")]
#[test]
fn coin2_stall_is_a_fair_end_component_and_witness() {
    const CLAIM: &str = "probabilistic StuckFair using inf on M over (p: M::Proc) \
         under every scheduler fair strong Proc::flip on origin \
         { probability >= 0.99 [ eventually holds(p.done) ] }\n\
         probabilistic RewardFair using inf on M over (p: M::Proc) \
         under every scheduler fair strong Proc::flip on origin \
         { expected accumulate cost until holds(p.done) <= 3 }";
    let world = World::new(&prob_world::process_coin2(CLAIM));
    let product = built(&world, "StuckFair");
    let components = maximal_fair_end_components(&product, &product.fairness)
        .expect("well-formed checked fairness");
    let component = components
        .iter()
        .find(|component| {
            component
                .actions
                .iter()
                .any(|(_, identity)| identity.operation.contains("stall"))
        })
        .expect("the stuck fair component is present");
    assert_eq!(component.states.len(), 1);
    assert_eq!(component.actions.len(), 1);
    assert!(component.actions[0].1.operation.contains("stall"));

    let target = product
        .labels
        .iter()
        .map(|labels| labels[0])
        .collect::<Vec<_>>();
    let result = decide_fair_reachability(
        &product,
        None,
        &target,
        UnboundedObjective::Min,
        &ExactProbLimits::default(),
    )
    .expect("exact fair reachability");
    assert_eq!(
        result.values[start(&product).index()],
        fsl_analyze::exact_probabilistic::ExtRational::Finite(ratio(0, 1))
    );
    let analyzed = fsl_analyze::analyze::analyze_items(
        &fsl_analyze::analyze::Engines::fsl().expect("FSL engines"),
        vec![fsl_analyze::analyze::exact_probabilistic_item(
            fsl_foundation::RequestIndex::new(0),
            world.request("StuckFair", ModelCheckLimits::default()),
        )],
        &Cancel::new(),
    );
    let record = analyzed.first().expect("one fair refutation");
    let terminal = record.terminal().unwrap_or_else(|| {
        panic!(
            "cyclic fair refutation did not settle: {:?}",
            record.settlement
        )
    });
    assert_eq!(terminal.value(), &fsl_replay::TerminalValue::Refuted);
    match &world.claim("StuckFair").form {
        fsl_semantics::check::ProbForm::Reach {
            bound: fsl_semantics::check::Bound::AtLeast(value),
            target: fsl_semantics::check::ReachTarget::Eventually(_),
        } => assert_eq!(value, &ratio(99, 100)),
        other => panic!("unexpected FR-1041 claim shape: {other:?}"),
    }
    let accepted = record
        .probabilistic_counterexample
        .as_ref()
        .expect("the independent fair replay accepted a counterexample")
        .evidence();
    assert!(
        accepted.paths.is_empty(),
        "the witness uses the cyclic subsystem"
    );
    let system = fsl_eval::simulation::model::ModelSystem::new(world.subject().simulation)
        .expect("coin2 model system");
    let initial = &system.initial()[0];
    let identity = system
        .actions(initial)
        .expect("initial actions")
        .expect("resolved actions")
        .into_iter()
        .find(|action| action.identity.operation.contains("stall"))
        .expect("stall is enabled")
        .identity;
    let stall = fsl_replay::Schedule::of(&system, &identity).expect("canonical stall schedule");
    assert!(accepted.scheduler.iter().any(|(key, choice)| {
        key == &accepted.initial_key
            && matches!(choice, fsl_replay::ProbabilisticSchedulerChoice::Deterministic(action) if action == &fsl_replay::ProductAction::Identity(stall.clone()))
    }));

    let wait = system
        .actions(initial)
        .expect("initial actions")
        .expect("resolved actions")
        .into_iter()
        .find(|action| action.identity.operation.contains("wait"))
        .expect("wait is enabled");
    let unfair_item = fsl_analyze::analyze::exact_probabilistic_item_witness_tampered(
        fsl_foundation::RequestIndex::new(0),
        world.request("StuckFair", ModelCheckLimits::default()),
        move |witness| {
            let schedule = fsl_replay::Schedule::of(&system, &wait.identity).unwrap();
            let row = witness
                .scheduler
                .iter_mut()
                .find(|(key, _)| key == &witness.initial_key)
                .expect("initial scheduler row");
            row.1 = fsl_replay::ProbabilisticSchedulerChoice::Deterministic(
                fsl_replay::ProductAction::Identity(schedule),
            );
        },
    );
    let unfair = fsl_analyze::analyze::analyze_items(
        &fsl_analyze::analyze::Engines::fsl().expect("FSL engines"),
        vec![unfair_item],
        &Cancel::new(),
    )
    .remove(0);
    assert!(
        !matches!(
            unfair.terminal().map(|terminal| terminal.value()),
            Some(fsl_replay::TerminalValue::Refuted)
        ),
        "the strong-fairness checker rejects a wait-only cycle"
    );
    let live = start(&product);
    let selected = match result.policy[live.index()].as_ref().expect("live choice") {
        fsl_analyze::exact_probabilistic::SchedulerChoice::Deterministic(
            fsl_analyze::exact_probabilistic::ActionChoice(index),
        ) => *index,
        other => panic!("expected the exact reachability action, got {other:?}"),
    };
    let mut identities = Vec::new();
    for edge in product.outgoing(live) {
        if let Some(identity) = edge.action.as_ref() {
            if !identities.iter().any(|seen| seen == identity) {
                identities.push(identity.clone());
            }
        }
    }
    assert!(identities[selected].operation.contains("stall"));
    assert!(component.states.contains(&component.actions[0].0));

    let reward_product = built(&world, "RewardFair");
    let target = reward_product
        .labels
        .iter()
        .map(|labels| labels[0])
        .collect::<Vec<_>>();
    let maximum = fsl_analyze::exact_probabilistic::decide_fair_expected_reward(
        &reward_product,
        &target,
        "cost",
        UnboundedObjective::Max,
        &ExactProbLimits::default(),
    )
    .expect("the fair reward maximum is unbounded");
    assert_eq!(
        maximum.values[start(&reward_product).index()],
        fsl_analyze::exact_probabilistic::ExtRational::PlusInfinity
    );
    assert!(
        maximum.policy.is_some(),
        "the result carries a fair scheduler"
    );
    let stuck = component.actions[0].0;
    let operation_choices = |state| {
        let mut choices = Vec::new();
        for edge in reward_product.outgoing(state) {
            if let Some(identity) = edge.action.as_ref() {
                if !choices.iter().any(|seen| seen == identity) {
                    choices.push(identity.clone());
                }
            }
        }
        choices
    };
    let reward_policy = maximum.policy.as_ref().expect("a fair scheduler");
    let selected = match reward_policy[stuck.index()]
        .as_ref()
        .expect("stuck scheduler choice")
    {
        fsl_analyze::exact_probabilistic::SchedulerChoice::Deterministic(
            fsl_analyze::exact_probabilistic::ActionChoice(index),
        ) => vec![*index],
        fsl_analyze::exact_probabilistic::SchedulerChoice::Randomized(choices) => choices
            .iter()
            .filter(|(_, probability)| probability > &ratio(0, 1))
            .map(|(choice, _)| choice.0)
            .collect(),
    };
    assert!(selected
        .iter()
        .any(|index| operation_choices(stuck)[*index].operation.contains("stall")));
    let minimum = fsl_analyze::exact_probabilistic::decide_fair_expected_reward(
        &reward_product,
        &target,
        "cost",
        UnboundedObjective::Min,
        &ExactProbLimits::default(),
    )
    .expect("the fair reward minimum uses FR-198");
    let fr198_minimum = fsl_analyze::exact_probabilistic::decide_expected_reward(
        &reward_product,
        &target,
        "cost",
        UnboundedObjective::Min,
        &ExactProbLimits::default(),
    )
    .expect("the FR-198 minimum is exact");
    assert_eq!(minimum.values, fr198_minimum.values);
    assert!(
        minimum.policy.is_some(),
        "the result carries a fair scheduler"
    );
}

/// FR-1041-AC-3 (TC-2110 step 3): a fair minimum uses the exact values
/// s=0/u=0/g=1/f=0. The refutation witness chooses `drift` into u's fair
/// loop, while replay rejects a forged wait-only cycle at s.
#[trace("TC-2110", "FR-1041-AC-3", "FR-411-AC-10", "FR-412-AC-13")]
#[test]
fn fair_minimum_replays_drift_into_a_zero_value_fair_component() {
    use fsl_analyze::analyze::{analyze_items, exact_probabilistic_item, Engines};
    use fsl_foundation::RequestIndex;
    use fsl_replay::{ProbabilisticSchedulerChoice, TerminalValue};

    const CLAIMS: &str = "probabilistic DriftRefuted using inf on M over (w: M::W) \
         under every scheduler fair strong W::go on origin \
         { probability >= rational(1,4) [ eventually holds(w.g) ] }";
    let world = World::new(&prob_world::fair_drift(CLAIMS));

    let refuted = analyze_items(
        &Engines::fsl().expect("FSL engines"),
        vec![exact_probabilistic_item(
            RequestIndex::new(0),
            world.request("DriftRefuted", ModelCheckLimits::default()),
        )],
        &Cancel::new(),
    )
    .remove(0);
    assert_eq!(
        refuted
            .terminal()
            .expect("the AC-3 threshold settles")
            .value(),
        &TerminalValue::Refuted
    );
    match &world.claim("DriftRefuted").form {
        fsl_semantics::check::ProbForm::Reach {
            bound: fsl_semantics::check::Bound::AtLeast(value),
            target: fsl_semantics::check::ReachTarget::Eventually(_),
        } => assert_eq!(value, &ratio(1, 4)),
        other => panic!("unexpected AC-3 checked form: {other:?}"),
    }
    let witness = refuted
        .probabilistic_counterexample
        .as_ref()
        .expect("strict replay accepted the fair counterexample")
        .evidence();
    assert!(witness.paths.is_empty());
    let system = fsl_eval::simulation::model::ModelSystem::new(world.subject().simulation)
        .expect("checked W model");
    let initial = &system.initial()[0];
    let actions = system
        .actions(initial)
        .expect("initial actions")
        .expect("resolved initial actions");
    let schedule = |name: &str| {
        let identity = actions
            .iter()
            .find(|action| action.identity.operation.contains(name))
            .unwrap_or_else(|| panic!("{name} is enabled at s"));
        fsl_replay::Schedule::of(&system, &identity.identity).expect("canonical schedule")
    };
    let drift = schedule("drift");
    let wait = schedule("wait");
    assert!(witness.scheduler.iter().any(|(key, choice)| {
        key == &witness.initial_key
            && matches!(choice, ProbabilisticSchedulerChoice::Deterministic(action) if action == &fsl_replay::ProductAction::Identity(drift.clone()))
    }), "the replayed scheduler takes drift from s into u");

    let binding = fsl_replay::claim_binding(&system, world.claim("DriftRefuted"))
        .expect("claim binding")
        .expect("the W receiver is bound");
    let key_for = |state: &fsl_eval::simulation::model::ModelSimState| {
        let bytes = system.state_key(state).expect("canonical W state").0;
        fsl_replay::ProbabilityProductKey::for_model_state(Some(&binding.reference), &bytes)
            .expect("complete fair-minimum key")
    };
    let post_for = |name: &str| {
        let action = actions
            .iter()
            .find(|action| action.identity.operation.contains(name))
            .unwrap_or_else(|| panic!("{name} is enabled at s"));
        let [draw] = action.draws.as_slice() else {
            panic!("{name} has one admitted draw");
        };
        let [(_, post)] = draw.successors.as_slice() else {
            panic!("{name} has one admitted successor");
        };
        post
    };
    let s = key_for(initial);
    let u_state = post_for("drift");
    let u = key_for(u_state);
    let g = key_for(post_for("go"));
    let f = key_for(post_for("fail"));
    let loop_identity = system
        .actions(u_state)
        .expect("u actions")
        .expect("resolved u actions")
        .into_iter()
        .find(|action| action.identity.operation.contains("loop"))
        .expect("u has its loop")
        .identity;
    let loop_schedule =
        fsl_replay::Schedule::of(&system, &loop_identity).expect("canonical u-loop schedule");
    assert!(witness.scheduler.iter().any(|(key, choice)| {
        key == &u
            && matches!(choice, ProbabilisticSchedulerChoice::Deterministic(action) if action == &fsl_replay::ProductAction::Identity(loop_schedule.clone()))
    }), "replay reconstructs the u loop after drift");
    let mut exact_values = vec![
        fsl_replay::ExactProbabilityValue {
            state: s.clone(),
            value: ratio(0, 1),
        },
        fsl_replay::ExactProbabilityValue {
            state: u.clone(),
            value: ratio(0, 1),
        },
        fsl_replay::ExactProbabilityValue {
            state: g.clone(),
            value: ratio(1, 1),
        },
        fsl_replay::ExactProbabilityValue {
            state: f.clone(),
            value: ratio(0, 1),
        },
    ];
    exact_values.sort_by(|left, right| left.state.cmp(&right.state));
    let exact = fsl_replay::ExactProbabilityCertificate {
        values: exact_values,
    };
    let limits = fsl_replay::ProbabilityDocumentLimits::for_exact(
        &ExactProbLimits::default(),
        fsl_replay::AccountingLimits::default().0,
    );
    let item = fsl_replay::probabilistic_claim_identity(world.claim("DriftRefuted"))
        .expect("canonical FR-1041 identity");
    let proof = fsl_replay::probability_proof_document(
        &system,
        system.input().probabilistic.claims(),
        &item,
        &exact,
        limits,
        &Cancel::new(),
    )
    .expect("the fair-MDP builder accepts the literal exact values");
    assert!(proof.is_fair_mdp());
    let bytes = fsl_replay::write_probability_proof_document(&proof, limits, &Cancel::new())
        .expect("canonical exact fair-MDP certificate");
    let certificate = fsl_replay::read_probability_proof_document(&bytes, limits, &Cancel::new())
        .expect("independent reader accepts the fair-MDP certificate");
    assert!(
        matches!(
            fsl_replay::check_probability_certificate_document(
                &system,
                system.input().probabilistic.claims(),
                &item,
                &bytes,
                limits,
                &Cancel::new(),
            )
            .expect("independent fair-MDP value/rank checker"),
            fsl_replay::ExactProbabilityCertificateCheck::Rejected(ref rejection)
                if rejection.rule == fsl_replay::ProbabilityCertificateRule::ThresholdMissed
        ),
        "the exact rows and fair ranks pass before the asserted 1/4 bound is compared"
    );
    let checked_values = &certificate.document().values;
    let value = |key: &fsl_replay::ProbabilityProductKey| {
        &checked_values
            .iter()
            .find(|entry| &entry.state == key)
            .expect("the checker documents this complete state")
            .value
    };
    assert_eq!(value(&s), &ratio(0, 1));
    assert_eq!(value(&u), &ratio(0, 1));
    assert_eq!(value(&g), &ratio(1, 1));
    assert_eq!(value(&f), &ratio(0, 1));
    let rank = |key: &fsl_replay::ProbabilityProductKey| {
        certificate
            .document()
            .ranking
            .iter()
            .find(|(state, _)| state == key)
            .map(|(_, rank)| *rank)
            .expect("rank for each fair-minimum state")
    };
    assert_eq!(rank(&s), 1);
    assert_eq!(rank(&u), 0);
    assert_eq!(rank(&g), 0);
    assert_eq!(rank(&f), 0);
    let mut forged_rank = proof.clone();
    forged_rank
        .document_mut()
        .ranking
        .iter_mut()
        .find(|(key, _)| key == &s)
        .expect("the producer supplies s rank")
        .1 = 0;
    let forged_rank_bytes =
        fsl_replay::write_probability_proof_document(&forged_rank, limits, &Cancel::new())
            .expect("the forged rank is syntactically admitted");
    assert!(
        matches!(
            fsl_replay::check_probability_certificate_document(
                &system,
                system.input().probabilistic.claims(),
                &item,
                &forged_rank_bytes,
                limits,
                &Cancel::new(),
            )
            .expect("forged fair-rank check"),
            fsl_replay::ExactProbabilityCertificateCheck::Rejected(ref rejection)
                if rejection.rule == fsl_replay::ProbabilityCertificateRule::RankBroken
                    && rejection.at == fsl_replay::ProbabilityCertificateLocus::ProductState(s.clone())
        ),
        "s is zero-valued but is not itself in the independently reconstructed U"
    );

    let forged_wait = fsl_analyze::analyze::exact_probabilistic_item_witness_tampered(
        RequestIndex::new(0),
        world.request("DriftRefuted", ModelCheckLimits::default()),
        move |witness| {
            let row = witness
                .scheduler
                .iter_mut()
                .find(|(key, _)| key == &witness.initial_key)
                .expect("initial action row");
            row.1 = ProbabilisticSchedulerChoice::Deterministic(
                fsl_replay::ProductAction::Identity(wait.clone()),
            );
        },
    );
    let forged = analyze_items(
        &Engines::fsl().expect("FSL engines"),
        vec![forged_wait],
        &Cancel::new(),
    )
    .remove(0);
    assert!(
        !matches!(
            forged.terminal().map(|terminal| terminal.value()),
            Some(TerminalValue::Refuted)
        ),
        "the independent fairness checker rejects the wait-only cycle"
    );
}

/// `each` fairness creates an obligation for every scheduled identity,
/// including the receiver component of an otherwise shared operation.
#[test]
fn fair_each_refines_one_operation_into_per_receiver_identities() {
    const CLAIM: &str = "probabilistic EachIdentity using inf on M over (p: M::Proc) \
         under every scheduler fair strong each Proc::wait on origin \
         { probability >= 0.5 [ eventually holds(p.done) ] }";
    let fixture = prob_world::process(CLAIM);
    let world = World::symmetric_objects(&fixture, &["a", "b"]);
    let product = built(&world, "EachIdentity");
    assert_eq!(product.fairness.len(), 1);
    assert_eq!(
        product.fairness[0].granularity(),
        filament_semantic_value::fairness::FairnessGranularity::Each
    );

    let components = maximal_fair_end_components(&product, &product.fairness)
        .expect("checked per-identity fairness");
    let wait_identities = components
        .iter()
        .flat_map(|component| component.actions.iter())
        .filter(|(_, identity)| identity.operation.contains("wait"))
        .map(|(_, identity)| identity.receiver.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(wait_identities.len(), 2, "{components:?}");
}

#[trace("TC-635", "FR-200-AC-6")]
#[test]
fn fair_weak_each_refines_one_operation_into_per_receiver_identities() {
    const CLAIM: &str = "probabilistic WeakEachIdentity using inf on M over (p: M::Pair) \
         under every scheduler fair weak each Pair::tick on origin \
         { probability >= 0.5 [ eventually holds(p.live) ] }";
    let fixture = prob_world::pair_tick(CLAIM);
    let world = World::symmetric_objects(&fixture, &["a", "b"]);
    let product = built(&world, "WeakEachIdentity");
    let components = maximal_fair_end_components(&product, &product.fairness)
        .expect("checked per-identity weak fairness");
    let wait_identities = components
        .iter()
        .flat_map(|component| component.actions.iter())
        .filter(|(_, identity)| identity.operation.contains("tick"))
        .map(|(_, identity)| identity.receiver.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(wait_identities.len(), 2, "{components:?}");
}

/// A randomized split reaches either of two distinct fair bottom SCCs; the
/// witness is checked against both components.
#[trace("TC-635", "FR-200-AC-7")]
#[test]
fn fair_witness_validates_multiple_bottom_sccs() {
    const CLAIM: &str = "probabilistic SplitFair using inf on M over (s: M::Split) \
         under every scheduler fair strong Split::step on origin \
         { probability >= 0.5 [ eventually holds(s.right) ] }";
    let world = World::new(&prob_world::fair_two_bottoms(CLAIM));
    let product = built(&world, "SplitFair");
    let components = maximal_fair_end_components(&product, &product.fairness)
        .expect("two disjoint fair end components");
    assert_eq!(components.len(), 2, "{components:?}");
    assert!(
        components.iter().all(|component| component
            .actions
            .iter()
            .any(|(_, identity)| identity.operation.contains("step"))),
        "components={components:?} states={:?} edges={:?}",
        product.states,
        product.edges
    );

    let target = product
        .labels
        .iter()
        .map(|labels| labels[0])
        .collect::<Vec<_>>();
    let result = decide_fair_reachability(
        &product,
        None,
        &target,
        UnboundedObjective::Min,
        &ExactProbLimits::default(),
    )
    .expect("the returned scheduler is checked over every bottom SCC");
    assert_eq!(
        result.values[start(&product).index()],
        fsl_analyze::exact_probabilistic::ExtRational::Finite(ratio(1, 2))
    );
    let bottom_states = components
        .iter()
        .map(|component| *component.states.first().expect("a bottom state"))
        .collect::<BTreeSet<_>>();
    assert_eq!(bottom_states.len(), 2);
    let mut reachable = BTreeSet::from([start(&product)]);
    let mut pending = VecDeque::from([start(&product)]);
    while let Some(state) = pending.pop_front() {
        for edge in product.outgoing(state) {
            if reachable.insert(edge.to) {
                pending.push_back(edge.to);
            }
        }
    }
    for bottom in bottom_states {
        assert!(reachable.contains(&bottom));
        assert!(result.policy[bottom.index()].is_some());
    }
}

const COST: &str = "probabilistic Cost using ev on M over (m: M::Msg) under Even on origin \
     { expected accumulate cost until holds(m.delivered or m.attempts = 2) <= rational(6, 5) }";

/// Checks FR-196 "Behavior", expected reward: the DTMC itself is the product, the
/// target is the one state label and each send carries its `cost` on its
/// edge; the stutter step of a terminal state carries cost 0.
#[trace("TC-631")]
#[test]
fn an_expected_reward_product_keeps_rewards_on_edges() {
    let world = World::new(&prob_world::link(COST));
    let product = built(&world, "Cost");
    assert_eq!(product.kind, ProductKind::Dtmc);
    let cost = |edge: &ProbEdge| {
        let [(name, value)] = &edge.rewards[..] else {
            panic!("one reward: {:?}", edge.rewards);
        };
        assert_eq!(name.as_str(), "cost");
        integer_of(value)
    };
    let first = start(&product);
    assert_eq!(product.labels[first.index()], [false]);
    let sends = product.outgoing(first);
    assert_eq!(sends.len(), 4);
    assert!(sends.iter().all(|edge| cost(edge) == 1));
    let stutters: Vec<&ProbEdge> = product
        .edges
        .iter()
        .filter(|edge| matches!(edge.choice, ProductAction::TerminalStutter))
        .collect();
    assert!(!stutters.is_empty());
    assert!(stutters.iter().all(|edge| cost(edge) == 0));
    for edge in stutters {
        assert_eq!(
            product.labels[edge.from.index()],
            [true],
            "terminal states hold B"
        );
        assert_eq!(edge.to, edge.from);
    }
}

const LONG_RUN: &str =
    "probabilistic LongRun using ev on M over (s: M::Server) under Steady on origin \
     { long_run_fraction holds(s.phase = 0) >= 0.5 }";

/// Checks FR-196 "Behavior", long-run fraction: the DTMC itself is the product with
/// the predicate as the state label and no control in any key.
#[trace("TC-631")]
#[test]
fn a_long_run_fraction_product_is_the_chain_with_labels() {
    let world = World::new(&prob_world::service(LONG_RUN));
    let product = built(&world, "LongRun");
    assert_eq!(product.kind, ProductKind::Dtmc);
    assert_eq!(product.labels.len(), product.states.len());
    for state in product.ids() {
        let edges = product.outgoing(state);
        assert_eq!(total(edges), ratio(1, 1), "state {state:?}");
        let key = &product.states[state.index()];
        assert!(key.monitor.is_none() && key.accumulator.is_none());
    }
    // The chain cycles through Idle: some state is labelled and some is not.
    let idle = product.labels.iter().filter(|label| label[0]).count();
    assert!(idle >= 1 && idle < product.labels.len());
}

const MEASURES: &str = "probabilistic Steps using ev on M over (s: M::Server) under Steady on origin \
     { probability >= 0.5 [ steps from holds(s.phase = 1) until holds(s.phase = 2) within 10 <= 1 ] }\n\
     probabilistic Count using ev on M over (s: M::Server) under Steady on origin \
     { probability >= 0.5 [ fraction holds(s.phase = 1) over [0, 2] >= rational(1, 2) ] }\n\
     probabilistic Weighted using ev on M over (s: M::Server) under Steady on origin \
     { probability >= 0.5 [ fraction holds(s.phase = 1 and s.attempts = 1) over [0, 2] \
     weighted by duration >= rational(1, 2) ] }\n\
     probabilistic Short using ev on M over (s: M::Server) under Steady on origin \
     { probability >= 0.5 [ steps from holds(s.phase = 1) until holds(s.phase = 2) within 1 <= 5 ] }";

/// Checks FR-196 "Behavior", bounded measures: a `steps` measure counts one per
/// step and saturates above its threshold; a fraction counts the included
/// positions, each decided at the step that leaves it, and a weighted
/// fraction keeps both sums exactly; a measure whose window ends first is
/// censored.
#[trace("TC-631")]
#[test]
fn steps_and_fractions_accumulate_exactly_through_their_windows() {
    let world = World::new(&prob_world::service(MEASURES));

    let steps = built(&world, "Steps");
    let held = distinct(
        steps
            .states
            .iter()
            .map(|key| match &key.accumulator {
                Some(Accumulator::Steps { activated, amount }) => (*activated, amount.clone()),
                other => panic!("a steps accumulator: {other:?}"),
            })
            .collect(),
    );
    let exact = |value| (true, Amount::Exact(ratio(value, 1)));
    assert_eq!(
        held,
        distinct(vec![
            (false, Amount::Exact(ratio(0, 1))),
            exact(0),
            exact(1),
            (true, Amount::AboveBound),
        ])
    );

    let count = built(&world, "Count");
    let complete: BTreeSet<(u64, u64)> = count
        .states
        .iter()
        .filter(|key| key.monitor == Some(MonitorState::Fraction { remaining: 0 }))
        .map(|key| match key.accumulator {
            Some(Accumulator::CountFraction {
                true_count,
                position_count,
            }) => (true_count, position_count),
            ref other => panic!("a count accumulator: {other:?}"),
        })
        .collect();
    assert_eq!(complete, BTreeSet::from([(1, 3), (2, 3)]));
    assert!(count
        .ids()
        .filter(|state| is_decided(&count, *state))
        .all(|state| count.outgoing(state).is_empty()));

    let weighted = built(&world, "Weighted");
    let sums = distinct(
        weighted
            .states
            .iter()
            .filter(|key| key.monitor == Some(MonitorState::Fraction { remaining: 0 }))
            .map(|key| match &key.accumulator {
                Some(Accumulator::WeightedFraction {
                    true_sum,
                    total_sum,
                }) => (true_sum.clone(), total_sum.clone()),
                other => panic!("a weighted accumulator: {other:?}"),
            })
            .collect(),
    );
    assert!(sums.contains(&(ratio(0, 1), ratio(1, 1))), "{sums:?}");
    assert!(sums.contains(&(ratio(3, 1), ratio(4, 1))), "{sums:?}");

    // `within 1` ends the window after one step: a measure activated at
    // position 1 and not ended is censored.
    let short = built(&world, "Short");
    let censored: Vec<_> = short
        .states
        .iter()
        .filter(|key| {
            matches!(
                key.monitor,
                Some(MonitorState::Measure {
                    phase: MeasurePhase::Censored,
                    ..
                })
            )
        })
        .collect();
    assert!(!censored.is_empty());
    for key in censored {
        assert_eq!(
            key.accumulator,
            Some(Accumulator::Steps {
                activated: true,
                amount: Amount::Censored
            })
        );
    }
}

/// Checks FR-196 "Behavior": reaching `max_states`, `max_transitions` or
/// `max_automaton_states`, or a cancellation, stops the build and names it.
#[trace("TC-631")]
#[test]
fn a_budget_or_a_cancellation_stops_the_build() {
    let world = World::new(&prob_world::no_fault(NO_FAULT));
    let stopped = |limits: ModelCheckLimits| match build_product(
        world.request("NoFault", limits),
        &Cancel::new(),
    ) {
        Err(ProductRefusal::Stopped(stop)) => stop,
        other => panic!("expected a stop: {other:?}"),
    };
    let defaults = ModelCheckLimits::default();
    assert_eq!(
        stopped(ModelCheckLimits {
            limits: Limits {
                max_states: 5,
                ..defaults.limits
            },
            ..defaults
        }),
        ProductStop::Limit {
            limit: ModelCheckLimit::ExactProbMaxStates,
            value: 5
        }
    );
    assert_eq!(
        stopped(ModelCheckLimits {
            limits: Limits {
                max_transitions: 3,
                ..defaults.limits
            },
            ..defaults
        }),
        ProductStop::Limit {
            limit: ModelCheckLimit::ExactProbMaxTransitions,
            value: 3
        }
    );
    assert_eq!(
        stopped(ModelCheckLimits {
            max_automaton_states: 2,
            ..defaults
        }),
        ProductStop::Limit {
            limit: ModelCheckLimit::ExactProbMaxAutomatonStates,
            value: 2
        }
    );
    let setting_of = |limits: ModelCheckLimits| match stopped(limits) {
        ProductStop::Limit { limit, value } => (limit.setting(), value),
        other => panic!("expected a limit stop: {other:?}"),
    };
    assert_eq!(
        setting_of(ModelCheckLimits {
            limits: Limits {
                max_states: 5,
                ..defaults.limits
            },
            ..defaults
        }),
        ("exact_prob.max_states", 5)
    );
    assert_eq!(
        setting_of(ModelCheckLimits {
            limits: Limits {
                max_transitions: 3,
                ..defaults.limits
            },
            ..defaults
        }),
        ("exact_prob.max_transitions", 3)
    );
    assert_eq!(
        setting_of(ModelCheckLimits {
            max_automaton_states: 2,
            ..defaults
        }),
        ("exact_prob.max_automaton_states", 2)
    );
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    assert!(matches!(
        build_product(world.request("NoFault", defaults), &cancel),
        Err(ProductRefusal::Stopped(ProductStop::Cancelled(
            CancelCause::Requested
        )))
    ));
}

const NEGOTIATED: &str = "probabilistic Seeded using ev on M over (p: M::Proc) under Even \
     on origin with alpha 0.01, beta 0.01, indifference 0.002 \
     { probability >= 0.9 [ always[0,3] holds(not p.done) ] }\n\
     probabilistic Reached using inf on M over (p: M::Proc) under Even \
     on origin with alpha 0.01, beta 0.01, indifference 0.002 \
     { probability >= 0.9 [ eventually holds(p.done) ] }\n\
     probabilistic Anywhere using ev on M over (p: M::Proc) under every scheduler \
     on origin with alpha 0.01, beta 0.01, indifference 0.002 \
     { probability >= 0.9 [ always[0,3] holds(not p.done) ] }\n\
     probabilistic Unseeded using ev on M over (p: M::Proc) under Even on origin \
     { probability >= 0.9 [ always[0,3] holds(not p.done) ] }";

/// Checks FR-195-AC-4, refusal half only (TC-630 step 4; the manifest and the
/// FR-192 settlement are not built): a claim only exact evidence decides is
/// refused at negotiation, before any sample, with the cause that names why:
/// `EveryScheduler` for `FairTerminates`, `ExactOnlyForm` for an
/// expected-reward claim under a workload, `MissingConfidence` for a bounded
/// event with no confidence parameters; a statistical claim passes.
#[trace("TC-630")]
#[test]
fn statistical_evidence_is_refused_for_a_claim_only_exact_evidence_decides() {
    use fsl_analyze::statistical::require_statistical;
    use fsl_replay::statistical::StatisticalRefusal;

    let process = World::new(&prob_world::process(&format!("{REACH}\n{NEGOTIATED}")));
    let verdict = |claim: &str| require_statistical(process.claim(claim));
    // The gate refuses exactly the claims `exact_only` marks.
    for name in [
        "FairTerminates",
        "Terminates",
        "Anywhere",
        "Reached",
        "Unless",
        "Unseeded",
        "Seeded",
    ] {
        assert_eq!(
            verdict(name).is_err(),
            process.claim(name).exact_only(),
            "{name}"
        );
    }
    assert_eq!(
        verdict("FairTerminates"),
        Err(StatisticalRefusal::EveryScheduler)
    );
    assert_eq!(
        verdict("Terminates"),
        Err(StatisticalRefusal::EveryScheduler)
    );
    assert_eq!(verdict("Anywhere"), Err(StatisticalRefusal::EveryScheduler));
    // A form only EN-5 decides is refused whatever its scheduler and
    // confidence say.
    assert_eq!(verdict("Reached"), Err(StatisticalRefusal::ExactOnlyForm));
    assert_eq!(verdict("Unless"), Err(StatisticalRefusal::ExactOnlyForm));
    assert_eq!(
        verdict("Unseeded"),
        Err(StatisticalRefusal::MissingConfidence)
    );
    assert_eq!(verdict("Seeded"), Ok(()));

    let link = World::new(&prob_world::link(COST));
    assert_eq!(
        require_statistical(link.claim("Cost")),
        Err(StatisticalRefusal::ExactOnlyForm)
    );
}

const ACTIVATED_AT_TERMINAL: &str = "probabilistic Late using ev on M over (m: M::Msg) under Even \
     on origin { probability >= 0.5 [ steps from holds(m.delivered) \
     until holds(not m.delivered) within 4 <= 10 ] }";

const HORIZON: &str =
    "probabilistic Idle using ev on M over (s: M::Server) under Steady on origin \
     { probability >= 0.5 [ steps from holds(s.phase = 1 and s.attempts = 2) \
     until holds(s.phase = 2) within 3 <= 5 ] }\n\
     probabilistic Source using ev on M over (s: M::Server) under Steady on origin \
     { probability >= 0.5 [ fraction holds(s.phase = 0) over [0, 2] >= rational(1, 2) ] }";

/// Checks FR-196 "Behavior" (TC-631): a `steps` measure whose `A` never holds ends
/// every path at `Measure{remaining: 0, phase: Unactivated}` with accumulator
/// `Steps{activated: false, amount: 0}`, decided and not expanded; a `steps`
/// measure activated at a terminal model state counts each stutter step up to
/// its horizon; a fraction reads its predicate at the position the step
/// leaves, not at the position it reaches.
#[trace("TC-631")]
#[test]
fn an_unactivated_measure_ends_decided_and_a_fraction_reads_the_source() {
    let world = World::new(&prob_world::service(HORIZON));
    let idle = built(&world, "Idle");
    let ended: Vec<_> = idle
        .states
        .iter()
        .filter(|key| key.monitor.is_some_and(MonitorState::is_decided))
        .collect();
    assert!(!ended.is_empty());
    for key in ended {
        assert_eq!(
            key.monitor,
            Some(MonitorState::Measure {
                remaining: 0,
                phase: MeasurePhase::Unactivated
            })
        );
        assert_eq!(
            key.accumulator,
            Some(Accumulator::Steps {
                activated: false,
                amount: Amount::Exact(ratio(0, 1))
            })
        );
    }
    // Every path ends there, and a decided state is not expanded.
    for state in idle.ids() {
        assert_eq!(
            idle.outgoing(state).is_empty(),
            is_decided(&idle, state),
            "{:?}",
            idle.states[state.index()]
        );
    }

    // Idle holds only at the first position; the positions reached never do.
    let source = built(&world, "Source");
    let complete = distinct(
        source
            .states
            .iter()
            .filter(|key| key.monitor == Some(MonitorState::Fraction { remaining: 0 }))
            .map(|key| match key.accumulator {
                Some(Accumulator::CountFraction {
                    true_count,
                    position_count,
                }) => (true_count, position_count),
                ref other => panic!("a count accumulator: {other:?}"),
            })
            .collect(),
    );
    assert_eq!(complete, [(1, 3)]);

    // `Late` activates where the message is delivered, a terminal model
    // state: each stutter step from there adds one step until the window
    // ends censored.
    let late = World::new(&prob_world::link(ACTIVATED_AT_TERMINAL));
    let product = built(&late, "Late");
    let steps = |state: ProbStateId| match &product.states[state.index()] {
        ProbKey {
            monitor: Some(MonitorState::Measure { remaining, .. }),
            accumulator: Some(Accumulator::Steps { activated, amount }),
            ..
        } => (*remaining, *activated, amount.clone()),
        other => panic!("a steps measure: {other:?}"),
    };
    let activated_terminal: Vec<ProbStateId> = product
        .edges
        .iter()
        .filter(|edge| matches!(edge.choice, ProductAction::TerminalStutter))
        .map(|edge| edge.from)
        .filter(|state| {
            let (_, activated, amount) = steps(*state);
            activated && amount == Amount::Exact(ratio(0, 1))
        })
        .collect();
    assert!(!activated_terminal.is_empty());
    for start in activated_terminal {
        let mut chain = vec![steps(start)];
        let mut at = start;
        while let [edge] = product.outgoing(at) {
            assert!(matches!(edge.choice, ProductAction::TerminalStutter));
            at = edge.to;
            chain.push(steps(at));
        }
        let remaining = chain[0].0;
        let expected: Vec<_> = (0..=remaining)
            .map(|taken| {
                let amount = if taken == remaining {
                    Amount::Censored
                } else {
                    Amount::Exact(ratio(i64::try_from(taken).expect("small"), 1))
                };
                (remaining - taken, true, amount)
            })
            .collect();
        assert_eq!(chain, expected);
    }
}

const LOOSE_COST: &str = "reward cost on Node::tick = 2;\n\
     probabilistic Cost using ev on M over (n: M::Node) under every scheduler on origin \
     { probability >= 0.5 [ accumulate cost from holds(n.healthy) \
     until holds(not n.healthy) within 3 >= 5 ] }";

/// Checks FR-196 "Behavior", SCH-2: a draw with several post-states advances the
/// accumulator once, at the resolution, with the draw's rewards; the edge
/// that took the draw carries them, and the resolution edges carry every
/// reward at 0.
#[trace("TC-631")]
#[test]
fn a_resolution_advances_the_accumulator_with_the_draws_rewards() {
    let world = World::new(&prob_world::health(
        "self.healthy = (pre(self.healthy) and not fault)",
        "true: 1, false: 9999999",
        true,
        LOOSE_COST,
    ));
    let product = built(&world, "Cost");
    let held = distinct(
        product
            .states
            .iter()
            .filter(|key| key.monitor.is_some_and(MonitorState::is_decided))
            .filter_map(|key| match &key.accumulator {
                Some(Accumulator::Reward { amount, .. }) => Some(amount.clone()),
                _ => None,
            })
            .collect(),
    );
    // A fault at the first tick ends at 2, at the second at 4; a third tick
    // without a fault is above 5.
    for value in [2, 4] {
        assert!(
            held.contains(&Amount::Exact(ratio(value, 1))),
            "{value}: {held:?}"
        );
    }
    let cost = |edge: &ProbEdge| {
        let [(name, value)] = &edge.rewards[..] else {
            panic!("one reward: {:?}", edge.rewards);
        };
        assert_eq!(name.as_str(), "cost");
        integer_of(value)
    };
    let mut draws = 0;
    for edge in &product.edges {
        match &edge.choice {
            ProductAction::Identity(_) => {
                draws += 1;
                assert_eq!(cost(edge), 2);
            }
            ProductAction::PostState(_) => assert_eq!(cost(edge), 0),
            ProductAction::TerminalStutter => {}
            other => panic!("an untimed product has no digital action: {other:?}"),
        }
    }
    assert!(draws > 0);
}

const BINDINGS: &str = "probabilistic Terminates using inf on M over (p: M::Proc) \
     under every scheduler on origin \
     { probability >= 0.99 [ eventually holds(p.done) ] }";

/// Checks FR-196-AC-10 (TC-631): over a population of two live objects, a
/// claim `over` the object type builds two initial product states for the
/// subject's one initial state, one per binding; the same model state under
/// the same monitor state and the two bindings is two product states whose
/// keys differ in `binding`, and the two have different labels.
#[trace("TC-631", "FR-196-AC-10")]
#[test]
fn each_over_binding_has_its_own_states() {
    let world = World::with_objects(&prob_world::process(BINDINGS), &["p", "q"]);
    let product = built(&world, "Terminates");
    let [(initial_a, first, a), (initial_b, second, b)] = &product.initial[..] else {
        panic!("one initial state per binding: {:?}", product.initial);
    };
    assert_eq!(initial_a, initial_b, "both start at the one initial state");
    assert_ne!(a, b);
    assert!(first.is_some() && second.is_some() && first != second);
    assert_eq!(product.states[a.index()].binding, *first);
    assert_eq!(product.states[b.index()].binding, *second);
    let (x, y) = (&product.states[a.index()], &product.states[b.index()]);
    assert_eq!((&x.model, &x.monitor), (&y.model, &y.monitor));
    assert_ne!(x, y);
    // One model state, two readings: where `p` is done and `q` is not.
    let differing = product.ids().any(|left| {
        product.ids().any(|right| {
            let (x, y) = (
                &product.states[left.index()],
                &product.states[right.index()],
            );
            x.model == y.model
                && x.monitor == y.monitor
                && x.binding != y.binding
                && x != y
                && product.labels[left.index()] != product.labels[right.index()]
        })
    });
    assert!(differing);
}

/// FR-196-AC-10 (TC-631): each of two subject initial states is represented
/// once per live binding, and equal model and monitor states under those
/// bindings have different product keys.
#[trace("TC-631", "FR-196-AC-10")]
#[test]
fn each_over_binding_has_its_own_initial_for_every_subject_initial() {
    let model = prob_world::process(BINDINGS);
    let world = World::with_object_initials(
        &model,
        &["p", "q"],
        &[
            serde_json::json!({"done": {"boolean": false}}),
            serde_json::json!({"done": {"boolean": true}}),
        ],
    );
    let product = built(&world, "Terminates");

    assert_eq!(product.initial.len(), 4, "two bindings per subject initial");
    for subject_initial in [0, 1] {
        let entries: Vec<_> = product
            .initial
            .iter()
            .filter(|(initial, _, _)| *initial == subject_initial)
            .collect();
        let [(left_initial, left_binding, left_state), (right_initial, right_binding, right_state)] =
            entries.as_slice()
        else {
            panic!("two binding initials for subject state {subject_initial}: {entries:?}");
        };
        assert_eq!(*left_initial, *right_initial);
        let (Some(left_binding), Some(right_binding)) = (left_binding, right_binding) else {
            panic!("each initial carries its over binding: {entries:?}");
        };
        assert_ne!(left_binding, right_binding);

        let (left_key, right_key) = (
            &product.states[left_state.index()],
            &product.states[right_state.index()],
        );
        assert_eq!(
            (&left_key.model, &left_key.monitor),
            (&right_key.model, &right_key.monitor)
        );
        assert_eq!(left_key.binding.as_ref(), Some(left_binding));
        assert_eq!(right_key.binding.as_ref(), Some(right_binding));
        assert_ne!(left_key, right_key, "binding changes the product key");
    }
}

/// Checks FR-196 "Behavior": reaching `max_states` or `max_transitions` stops the
/// build, and a build of exactly that size completes.
#[trace("TC-631")]
#[test]
fn a_budget_equal_to_the_product_size_completes() {
    let world = World::new(&prob_world::no_fault(NO_FAULT));
    let product = built(&world, "NoFault");
    let (states, edges) = (product.states.len(), product.edges.len());
    assert_eq!((states, edges), (1002, 2000));
    let defaults = ModelCheckLimits::default();
    let with = |max_states, max_transitions| ModelCheckLimits {
        limits: Limits {
            max_states,
            max_transitions,
        },
        ..defaults
    };
    let ran = |limits| build_product(world.request("NoFault", limits), &Cancel::new());
    assert!(matches!(ran(with(states, edges)), Ok(ProbBuild::Built(_))));
    assert!(matches!(
        ran(with(states - 1, edges)),
        Err(ProductRefusal::Stopped(ProductStop::Limit {
            limit: ModelCheckLimit::ExactProbMaxStates,
            ..
        }))
    ));
    assert!(matches!(
        ran(with(states, edges - 1)),
        Err(ProductRefusal::Stopped(ProductStop::Limit {
            limit: ModelCheckLimit::ExactProbMaxTransitions,
            ..
        }))
    ));
}

/// TC-631: an unbounded exact claim retains real None monitor and accumulator
/// members in its key; exhausting the state budget returns a typed stop and
/// does not manufacture a partial product. This does not cover FR-196-AC-9's
/// timed-key clock/deadline retention or malformed-key admission negatives.
#[trace("TC-631")]
#[test]
fn exact_unbounded_keys_keep_none_and_budget_stops_have_no_product() {
    let world = World::new(&prob_world::process(REACH));
    let product = built(&world, "Terminates");
    let initial = &product.states[start(&product).index()];
    assert_eq!(initial.monitor, None);
    assert_eq!(initial.accumulator, None);

    let limits = ModelCheckLimits {
        limits: Limits {
            max_states: 1,
            ..ModelCheckLimits::default().limits
        },
        ..ModelCheckLimits::default()
    };
    let result = build_product(world.request("Terminates", limits), &Cancel::new());
    assert!(
        matches!(
            result,
            Err(ProductRefusal::Stopped(ProductStop::Limit {
                limit: ModelCheckLimit::ExactProbMaxStates,
                value: 1,
            }))
        ),
        "typed budget refusal: {result:?}"
    );
}

/// FR-196-AC-17: the admitted event-position profile builds its formula
/// monitor into EN-5's bounded product. A raw model self-cycle at the first
/// loss therefore has a distinct remaining-horizon key, while the exact
/// eventual and always values match their independent finite-horizon cases.
#[trace("TC-631", "FR-196-AC-17")]
#[test]
fn bounded_event_position_formulas_use_formula_keys_and_exact_extrema() {
    use fsl_analyze::analyze::{analyze_items, exact_probabilistic_item, Engines};
    use fsl_foundation::RequestIndex;
    use fsl_replay::{ModelCheckLimits, ProofBasis, TerminalValue};

    let eventual = World::new(&prob_world::Model {
        package: "test/bounded-event-position",
        object: "T",
        population: "ts",
        key: "x",
        integers: vec![],
        fields: vec![("delivered", prob_world::BOOLEAN.into())],
        operations: vec![prob_world::Operation {
            name: "step",
            parameters: vec![("outcome", prob_world::BOOLEAN.into())],
            modifies: vec!["delivered"],
            deletes: false,
        }],
        body: "pre Active using v on M::T::step { not self.delivered }\n\
               post Stepped using v on M::T::step { self.delivered = outcome }\n\
               random T::step(outcome) ~ { true: 1, false: 1 };\n\
               terminal Done using v on M when { self.delivered };\n\
               workload Steady on M { weight T::step = 1; }\n\
               probabilistic WithinTwo using ev on M over (t: M::T) under Steady on origin { probability >= rational(3, 4) [ eventually[0, 2] holds(t.delivered) ] }"
            .to_owned(),
        initial: serde_json::json!({"delivered": {"boolean": false}}),
    });
    let product = built(&eventual, "WithinTwo");
    let initial_id = start(&product);
    let initial = &product.states[initial_id.index()];
    assert!(matches!(
        initial.monitor,
        Some(MonitorState::Formula(FormulaState::Running(_)))
    ));
    let loss = product
        .outgoing(initial_id)
        .iter()
        .find(|edge| product.states[edge.to.index()].model == initial.model)
        .expect("the undelivered half-probability branch returns to the same model state");
    assert_eq!(loss.probability, ratio(1, 2));
    let one_loss = &product.states[loss.to.index()];
    assert_ne!(
        initial, one_loss,
        "remaining horizon is part of the full key"
    );
    assert_ne!(initial.monitor, one_loss.monitor);
    assert!(matches!(
        one_loss.monitor,
        Some(MonitorState::Formula(FormulaState::Running(_)))
    ));

    let always = World::new(&prob_world::Model {
        package: "test/bounded-always",
        object: "T",
        population: "ts",
        key: "x",
        integers: vec![("Phase", 0, 2)],
        fields: vec![
            ("phase", "ix://test/bounded-always/Phase".into()),
            ("p", prob_world::BOOLEAN.into()),
        ],
        operations: vec![
            prob_world::Operation {
                name: "a",
                parameters: vec![("outcome", prob_world::BOOLEAN.into())],
                modifies: vec!["phase", "p"],
                deletes: false,
            },
            prob_world::Operation {
                name: "b",
                parameters: vec![("outcome", prob_world::BOOLEAN.into())],
                modifies: vec!["phase", "p"],
                deletes: false,
            },
            prob_world::Operation {
                name: "recover",
                parameters: vec![],
                modifies: vec!["phase", "p"],
                deletes: false,
            },
        ],
        body: "pre A using v on M::T::a { self.phase = 0 }\n\
               post ADone using v on M::T::a { (outcome and self.phase = 1 and self.p = true) or (not outcome and self.phase = 2 and self.p = false) }\n\
               pre B using v on M::T::b { self.phase = 0 }\n\
               post BDone using v on M::T::b { (outcome and self.phase = 1 and self.p = true) or (not outcome and self.phase = 2 and self.p = false) }\n\
               pre Recover using v on M::T::recover { self.phase = 2 }\n\
               post Recovered using v on M::T::recover { self.phase = 1 and self.p = true }\n\
               random T::a(outcome) ~ { true: 1, false: 3 };\n\
               random T::b(outcome) ~ { true: 3, false: 1 };\n\
               terminal Done using v on M when { self.phase = 1 };\n\
               probabilistic Minimum using ev on M over (t: M::T) under every scheduler on origin { probability >= rational(1, 4) [ always[0, 1] holds(t.p) ] }\n\
               probabilistic Maximum using ev on M over (t: M::T) under every scheduler on origin { probability <= rational(3, 4) [ always[0, 1] holds(t.p) ] }"
            .to_owned(),
        initial: serde_json::json!({
            "phase": {"integer": "0"},
            "p": {"boolean": true},
        }),
    });

    let engines = Engines::fsl().expect("exact EN-5 is registered");
    let exact_value = |world: &World, name: &str| {
        let item = exact_probabilistic_item(
            RequestIndex::new(0),
            world.request(name, ModelCheckLimits::default()),
        );
        let mut records = analyze_items(&engines, vec![item], &Cancel::new());
        assert_eq!(records.len(), 1);
        let record = records.remove(0);
        let terminal = record.terminal().expect("exact analysis settles");
        let TerminalValue::Proved {
            basis: ProofBasis::ExactValue { value, .. },
            ..
        } = terminal.value()
        else {
            panic!(
                "{name} is proved with an exact formula value: {:?}",
                terminal.value()
            );
        };
        value.value().clone()
    };
    assert_eq!(exact_value(&eventual, "WithinTwo"), ratio(3, 4));
    assert_eq!(exact_value(&always, "Minimum"), ratio(1, 4));
    assert_eq!(exact_value(&always, "Maximum"), ratio(3, 4));
}

/// Observe the real relation and full-target construction boundaries, rather
/// than inferring admission from the final product's size.
#[cfg(feature = "test-support")]
#[trace("TC-631", "FR-203-AC-14")]
#[test]
fn untimed_exact_admits_edges_before_relation_and_target_construction() {
    use fsl_analyze::exact_probabilistic::{
        product_construction_counts, reset_product_construction_counts,
    };
    let world = World::new(&prob_world::health(
        "self.healthy = (pre(self.healthy) and not fault)",
        "true: 1, false: 1",
        false,
        HEALTH,
    ));
    for claim in ["Steadily", "Anyhow"] {
        for max_transitions in [0, 1] {
            reset_product_construction_counts();
            let mut limits = ModelCheckLimits::default();
            limits.limits.max_transitions = max_transitions;
            let result = build_product(world.request(claim, limits), &Cancel::new());
            assert!(
                matches!(
                    result,
                    Err(ProductRefusal::Stopped(ProductStop::Limit {
                        limit: ModelCheckLimit::ExactProbMaxTransitions,
                        ..
                    }))
                ),
                "{claim}: {result:?}"
            );
            let counts = product_construction_counts();
            assert_eq!(
                counts.relations,
                usize::from(max_transitions != 0),
                "{claim}"
            );
            assert_eq!(counts.edges, max_transitions, "{claim}");
            assert_eq!(
                counts.targets,
                1 + max_transitions,
                "{claim}: the unpaid target is never materialized"
            );
        }
    }
    // The first two edges are pending-draw states. The third resolves one
    // actual post-state; the fourth must stop before retaining its target.
    let world = loose_health();
    reset_product_construction_counts();
    let mut limits = ModelCheckLimits::default();
    limits.limits.max_transitions = 3;
    let result = build_product(world.request("Anyhow", limits), &Cancel::new());
    assert!(
        matches!(
            result,
            Err(ProductRefusal::Stopped(ProductStop::Limit {
                limit: ModelCheckLimit::ExactProbMaxTransitions,
                ..
            }))
        ),
        "{result:?}"
    );
    let counts = product_construction_counts();
    assert_eq!(counts.edges, 3);
    assert_eq!(counts.targets, 4);
    assert_eq!(
        counts.relations, 1,
        "resolution reuses the admitted relation"
    );
}

#[cfg(feature = "test-support")]
#[trace("TC-631", "FR-203-AC-14")]
#[test]
fn untimed_exact_counts_distinct_pending_keys_and_reuses_full_state_capacity() {
    use fsl_analyze::exact_probabilistic::{
        product_construction_counts, reset_product_construction_counts,
    };
    let extra = "probabilistic Reuse using inf on M over (n: M::Node) under every scheduler on origin { probability >= 0.5 [ eventually holds(n.healthy) ] }";
    let mut model = prob_world::health("self.healthy = false", "true: 1, false: 1", false, extra);
    model.operations.push(prob_world::Operation {
        name: "other",
        parameters: vec![("fault", prob_world::BOOLEAN.to_owned())],
        modifies: vec!["healthy"],
        deletes: false,
    });
    model.body.push_str("\npost Other using v on M::Node::other { self.healthy = false }\nrandom Node::other(fault) ~ { true: 1, false: 1 };\n");
    model.body = model.body.replace(
        "workload Steady on M { weight Node::tick = 1; }",
        "workload Steady on M { weight Node::tick = 1; weight Node::other = 1; }",
    );
    let world = World::new(&model);
    let mut limits = ModelCheckLimits::default();
    limits.limits.max_states = 2;
    reset_product_construction_counts();
    let result = build_product(world.request("Reuse", limits), &Cancel::new());
    let Ok(ProbBuild::Built(product)) = result else {
        panic!("two actions share the same pending key, then reuse it at full capacity: {result:?}")
    };
    assert_eq!(product.states.len(), 2);
    assert_eq!(product.edges.len(), 4);
    assert_eq!(product_construction_counts().edges, 4);

    // Distinct intermediate draw keys must each reserve a new slot even
    // though none of the planned targets has been interned yet.
    let world = loose_health();
    reset_product_construction_counts();
    let result = build_product(world.request("Anyhow", limits), &Cancel::new());
    assert!(
        matches!(
            result,
            Err(ProductRefusal::Stopped(ProductStop::Limit {
                limit: ModelCheckLimit::ExactProbMaxStates,
                value: 2
            }))
        ),
        "{result:?}"
    );
    let counts = product_construction_counts();
    assert_eq!(
        counts.targets, 2,
        "initial plus the first admitted pending key"
    );
    assert_eq!(
        counts.edges, 1,
        "the next distinct key is denied before its edge is retained"
    );

    reset_product_construction_counts();
    limits.limits.max_states = 1;
    let result = build_product(world.request("Anyhow", limits), &Cancel::new());
    assert!(
        matches!(
            result,
            Err(ProductRefusal::Stopped(ProductStop::Limit {
                limit: ModelCheckLimit::ExactProbMaxStates,
                value: 1
            }))
        ),
        "{result:?}"
    );
    assert_eq!(
        product_construction_counts().targets,
        1,
        "a genuinely new key cannot materialize a target at full capacity"
    );
}

const TOSS: &str = "probabilistic Tossed using ev on M over (c: M::Coin) under every scheduler \
     on origin { probability >= 0.5 [ always[0,3] holds(c.v = 0) ] }\n\
     probabilistic Plain using ev on M under every scheduler on origin \
     { probability >= 0.5 [ always[0,2] holds(true) ] }";

/// Checks FR-196 "Behavior", SCH-2: an undefined path over every scheduler names
/// the post-state the scheduler took at a draw with several post-states, so
/// it replays to one state.
#[trace("TC-641", "FR-196-AC-12")]
#[test]
fn an_mdp_undefined_path_names_the_post_state_taken_at_each_draw() {
    let world = World::new(&prob_world::tossed_coin(TOSS));
    let ProbBuild::Undefined(path) = build_product(
        world.request("Tossed", ModelCheckLimits::default()),
        &Cancel::new(),
    )
    .expect("the build ends with a refutation") else {
        panic!("an undefined path");
    };
    let [step] = &path.steps[..] else {
        panic!("one step: {:?}", path.steps);
    };
    assert_eq!(path.position, 1);
    assert_eq!(step.probability, ratio(1, 2));
    let resolved = step.resolved.as_ref().expect("the post-state taken");
    // The coin is gone: a complete product key for the selected post-state,
    // and not the initial one.
    let plain = built(&world, "Plain");
    let initial = &plain.states[start(&plain).index()].model;
    assert_ne!(&resolved.model, initial);
    assert!(plain.states.iter().any(|key| key.model == resolved.model));
    assert_eq!(step.target, *resolved);
}

#[trace("TC-633", "FR-201-AC-15")]
#[test]
fn until_minimum_retains_absorbing_failure_in_qualitative_analysis() {
    use fsl_analyze::exact_probabilistic::{decide_until, ExtRational};
    let world = World::new(&prob_world::link(
        "probabilistic FirstSend using inf on M over (m: M::Msg) under every scheduler on origin \
         { probability >= 0.5 [ holds(m.attempts = 0) until holds(m.delivered) ] }",
    ));
    let product = built(&world, "FirstSend");
    let from = product
        .labels
        .iter()
        .map(|labels| labels[0])
        .collect::<Vec<_>>();
    let target = product
        .labels
        .iter()
        .map(|labels| labels[1])
        .collect::<Vec<_>>();
    for (objective, numerator, denominator) in [
        (UnboundedObjective::Min, 4, 5),
        (UnboundedObjective::Max, 9, 10),
    ] {
        let result = decide_until(
            &product,
            &from,
            &target,
            objective,
            &ExactProbLimits::default(),
        )
        .unwrap();
        assert_eq!(
            result.values[start(&product).index()],
            ExtRational::Finite(ratio(numerator, denominator))
        );
        for (index, labels) in product.labels.iter().enumerate() {
            if !labels[0] && !labels[1] {
                assert_eq!(result.values[index], ExtRational::Finite(ratio(0, 1)));
            }
        }
    }
}

#[trace("TC-633", "FR-201-AC-15")]
#[test]
fn until_avoidance_cannot_cross_an_already_reached_target() {
    use fsl_analyze::exact_probabilistic::{decide_until, ExtRational};
    let world = World::new(&prob_world::link(
        "probabilistic ForGraph using inf on M over (m: M::Msg) under every scheduler on origin \
         { probability >= 0.5 [ eventually holds(m.delivered) ] }",
    ));
    let seed = built(&world, "ForGraph");
    let ids = seed.ids().take(3).collect::<Vec<_>>();
    let states = seed.states.into_iter().take(3).collect();
    let edge = |from: usize, to: usize| ProbEdge {
        from: ids[from],
        to: ids[to],
        choice: ProductAction::TerminalStutter,
        action: None,
        draw: None,
        probability: ratio(1, 1),
        elapsed: ratio(0, 1),
        rewards: Vec::new(),
    };
    let product = ProbProduct {
        states,
        kind: ProductKind::Mdp,
        edges: vec![edge(0, 1), edge(1, 2), edge(2, 2)],
        initial: Vec::new(),
        decided: Vec::new(),
        labels: Vec::new(),
        fairness_operations: Vec::new(),
        fairness: Vec::new(),
    };
    let result = decide_until(
        &product,
        &[true, true, true],
        &[false, true, false],
        UnboundedObjective::Min,
        &ExactProbLimits::default(),
    )
    .unwrap();
    assert_eq!(
        result.values,
        vec![
            ExtRational::Finite(ratio(1, 1)),
            ExtRational::Finite(ratio(1, 1)),
            ExtRational::Finite(ratio(0, 1))
        ]
    );
}
