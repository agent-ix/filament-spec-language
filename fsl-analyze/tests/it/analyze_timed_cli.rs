// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-632 and FR-626 (TC-1592, TC-1586) over the timed `Rpc` subject through
//! `fsl analyze`: MITL claims decided on zones with a certificate, derived
//! deadlock-freedom and time-lock-freedom items, budgets, and the exact
//! rational delays of a refutation.

use std::cell::Cell;
use std::cell::RefCell;
use std::collections::BTreeMap;

use filament_exact::{Cancel, Meter, Undefined};
use fsl_analyze::model_check::digital::{check_digital, DigitalLimits, DigitalRequest};
use fsl_eval::model_subject::{AtomStop, ModelInstance, ModelLetters};
use fsl_eval::simulation::model::{ModelSimState, TimedSystem};
use fsl_eval::temporal::{AtomReading, CaptureReading, UndefinedCause};
use fsl_replay::{CounterexampleKind, ModelCheckOutcome};
use fsl_semantics::check::FormulaNodeId;
use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{analyze, label, request_over, Workspace};
use crate::analyze_family_support::{rpc_snapshot, rpc_unit, rpc_unit_with};

/// ADR-026 section 11's `Settles`: every `Waiting` position is followed within
/// `interval` by a position that is not `Waiting`.
fn settles(interval: &str) -> String {
    format!(
        "temporal Settles using tt over (c: M::Call) clock \"model-time\" on origin \
         {{ always (holds(c.phase = 1) implies eventually{interval} holds(not (c.phase = 1))) }}\n"
    )
}

fn run(name: &str, unit: &str, items: Value) -> (crate::analyze_cli_support::Run, Vec<Value>) {
    run_with_snapshots(name, unit, &[rpc_snapshot()], items)
}

fn run_with_snapshots(
    name: &str,
    unit: &str,
    snapshots: &[Vec<u8>],
    items: Value,
) -> (crate::analyze_cli_support::Run, Vec<Value>) {
    let ws = Workspace::new(name);
    let run = analyze(&request_over(
        &ws,
        unit,
        &crate::zone_search::rpc_document(),
        snapshots,
        json!({"items": items}),
    ));
    let items = run.document()["items"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    (run, items)
}

/// FR-632-AC-1: the MITL claim proves with basis `zone-certified` and
/// certification `certified` when the reply fits its interval, and refutes
/// with a counterexample whose delays are exact rationals when the interval
/// is cut short; the counterexample replays.
#[trace("TC-1592", "FR-632-AC-1")]
#[test]
fn an_mitl_claim_proves_certified_and_refutes_with_exact_delays() {
    let (run, items) = run(
        "timed-mitl",
        &rpc_unit(
            4,
            &format!(
                "{}{}",
                settles("[0 ms, 3 ms]").replace("Settles", "Within"),
                settles("[0 ms, 3 ms)").replace("Settles", "Short")
            ),
        ),
        json!([{"claim": "Within"}, {"claim": "Short"}]),
    );
    assert_eq!(label(&items[0]), "proved");
    assert_eq!(items[0]["record"]["basis"]["type"], "zone-certified");
    assert_eq!(items[0]["record"]["certification"], "certified");
    assert_eq!(label(&items[1]), "refuted");
    assert_eq!(items[1]["replay"]["reproduced"], true);
    let steps = items[1]["counterexample"]["prefix"]
        .as_array()
        .expect("a finite prefix");
    // FR-241 takes the least admissible delays: `send` at 0, then `reply` at 3 (the
    // violating position of `[0 ms, 3 ms)`). The lasso tail is the engine's.
    let head: Vec<(String, String, String)> = steps
        .iter()
        .take(2)
        .map(|step| {
            let delay = &step["delay"];
            (
                delay["numerator"].as_str().unwrap_or("-").to_owned(),
                delay["denominator"].as_str().unwrap_or("-").to_owned(),
                step["transition"]["operation"]
                    .as_str()
                    .and_then(|node| node.rsplit('/').next())
                    .unwrap_or("-")
                    .to_owned(),
            )
        })
        .collect();
    let expected: Vec<(String, String, String)> = [("0", "1", "send"), ("3", "1", "reply")]
        .iter()
        .map(|(n, d, op)| ((*n).to_owned(), (*d).to_owned(), (*op).to_owned()))
        .collect();
    assert_eq!(head, expected, "{}", items[1]["counterexample"]);
    assert_eq!(run.exit, 10);
}

/// FR-626-AC-3: the derived time-lock-freedom item of the timed subject
/// proves when every reachable state lets time diverge, and refutes with a
/// concretised, replayed counterexample when a strict guard stalls it.
#[trace("TC-1586", "FR-626-AC-3")]
#[test]
fn time_lock_freedom_proves_and_refutes_over_a_timed_subject() {
    let item = json!([{"claim": "M", "temporal": {"item_kind": "time-lock-freedom"}}]);
    let (_, items) = run("timed-lock-free", &rpc_unit(4, ""), item.clone());
    assert_eq!(label(&items[0]), "proved");
    assert_eq!(items[0]["record"]["certification"], "certified");
    let (run, items) = run("timed-lock", &rpc_unit_with(3, "> 3", ">", ""), item);
    assert_eq!(label(&items[0]), "refuted");
    assert_eq!(items[0]["counterexample"]["kind"], "time-lock");
    assert_eq!(items[0]["replay"]["reproduced"], true);
    assert_eq!(run.exit, 10);
}

/// FR-626: the derived deadlock-freedom item of the timed subject is decided
/// on zones as well.
#[trace("TC-1586", "FR-626-AC-1")]
#[test]
fn deadlock_freedom_is_decided_over_a_timed_subject() {
    let (_, items) = run(
        "timed-deadlock",
        &rpc_unit(4, ""),
        json!([{"claim": "M", "temporal": {"item_kind": "deadlock-freedom"}}]),
    );
    assert_eq!(label(&items[0]), "proved", "{}", items[0]);
    assert_eq!(items[0]["request"]["candidates"]["backends"][0], "en-6");
}

/// FR-625-AC-3 on zones: a symbolic-state budget returns `incomplete` naming
/// the limit, never a verdict.
#[trace("TC-1592", "FR-632-AC-1")]
#[test]
fn a_zone_state_budget_is_incomplete() {
    let (run, items) = run(
        "timed-budget",
        &rpc_unit(4, &settles("[0 ms, 3 ms]")),
        json!([{"claim": "Settles", "temporal": {"limits": {"max_states": 2}}}]),
    );
    assert_eq!(label(&items[0]), "incomplete");
    assert_eq!(run.exit, 22);
}

/// QSL-2129: a clock bound of the wrong dimension (a bare number against a
/// `Time` clock) is refused at check, before any item runs.
#[trace("TC-1592", "FR-632-AC-1")]
#[test]
fn a_wrong_dimension_clock_bound_is_refused_at_check() {
    let unit = rpc_unit(4, "").replace("self.x <= 3 ms", "self.x <= 3");
    let ws = Workspace::new("timed-dimension");
    let run = analyze(&request_over(
        &ws,
        &unit,
        &crate::zone_search::rpc_document(),
        &[rpc_snapshot()],
        json!({"items": []}),
    ));
    assert_eq!(run.exit, 20);
    assert_eq!(run.document()["items"], json!([]));
    assert_ne!(run.document()["last_stage"], Value::Null);
}

fn run_over(
    name: &str,
    unit: &str,
    document: &[u8],
    items: Value,
) -> (crate::analyze_cli_support::Run, Vec<Value>) {
    let ws = Workspace::new(name);
    let run = analyze(&request_over(
        &ws,
        unit,
        document,
        &[rpc_snapshot()],
        json!({"items": items}),
    ));
    let items = run.document()["items"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    (run, items)
}

/// QSL-2129: a timed subject whose clock owner takes an unbounded `Integer`
/// parameter needs a proof bound, as zone search reports it; the request's
/// subject cannot make the item an engine fault.
#[trace("TC-694")]
#[test]
fn a_timed_subject_with_an_unbounded_root_requires_a_bound() {
    let document = crate::timed_world::document_unbounded(
        &["x"],
        &[
            ("send", &["phase", "inflight", "late", "x"]),
            ("reply", &["phase", "inflight", "late"]),
            ("timeout", &["phase"]),
            ("reset", &["phase"]),
        ],
    );
    let (run, items) = run_over(
        "timed-unbounded",
        &rpc_unit(4, &settles("[0 ms, 3 ms]")),
        &document,
        json!([{"claim": "Settles"}]),
    );
    let item = &items[0];
    assert_eq!(label(item), "unsupported", "{item}");
    assert_eq!(item["record"]["diagnostic"]["cause"], "unbounded-extent");
    assert_eq!(run.exit, 21);
}

/// A snapshot of two `Call` objects, so the subject holds two live clocks.
fn two_call_snapshot() -> Vec<u8> {
    let mut snapshot: Value =
        serde_json::from_slice(&rpc_snapshot()).expect("the snapshot is JSON");
    let objects = snapshot["populations"][0]["objects"]
        .as_array_mut()
        .expect("objects");
    let mut second = objects[0].clone();
    second["key"] = json!("d");
    objects.push(second);
    snapshot.to_string().into_bytes()
}

/// W6 FR-632-AC-3: `max_clocks` is read by a timed subject. Over two calls, a
/// budget of 1 stops EN-6 `incomplete` with `LimitReached{max_clocks}` and
/// value 1; on an untimed subject no route reads it, so the item is
/// `unsupported`.
#[trace("TC-1592", "FR-632-AC-3")]
#[test]
fn max_clocks_is_read_on_a_timed_subject_and_unrouted_on_an_untimed_one() {
    let ws = Workspace::new("timed-max-clocks");
    let timed = analyze(&request_over(
        &ws,
        &rpc_unit(4, &settles("[0 ms, 3 ms]")),
        &crate::zone_search::rpc_document(),
        &[two_call_snapshot()],
        json!({"items": [{"claim": "Settles", "temporal": {"limits": {"max_clocks": 1}}}]}),
    ));
    let item = timed.item(0);
    assert_eq!(label(&item), "incomplete", "{item}");
    assert_eq!(item["record"]["cause"]["kind"], "limit-reached", "{item}");
    assert_eq!(item["record"]["cause"]["limit"], "max_clocks", "{item}");
    assert_eq!(item["record"]["cause"]["value"], 1, "{item}");
    let ws = Workspace::new("untimed-max-clocks");
    let untimed = analyze(&crate::analyze_cli_support::counter_request(
        &ws,
        &crate::analyze_cli_support::bounded(),
        json!([{"claim": "Bounded", "temporal": {"limits": {"max_clocks": 1}}}]),
    ));
    assert_eq!(untimed.item(0)["record"]["result"], "unsupported");
}

/// FR-852: a state-graph claim over a timed subject routes to EN-6 and
/// proves with a certified zone graph and an exact-delay witness.
#[trace("TC-2412", "FR-852")]
#[test]
fn a_state_graph_claim_over_a_timed_subject_proves_on_zones() {
    let (run, items) = run(
        "timed-state-graph",
        &rpc_unit(
            4,
            "possible Reaches using v over (c: M::Call) { c.phase = 1 }\n",
        ),
        json!([{"claim": "Reaches"}]),
    );
    let item = &items[0];
    assert_eq!(label(item), "proved", "{item}");
    assert_eq!(item["record"]["backend"], "en-6", "{item}");
    assert_eq!(item["record"]["basis"]["type"], "zone-certified", "{item}");
    assert_eq!(item["record"]["certification"], "certified", "{item}");
    assert_eq!(item["certificate"]["accepted"], true, "{item}");
    assert_eq!(item["certificate"]["checker"], "zone-certificate", "{item}");
    assert_eq!(
        item["certificate"]["document"]["state_graph"]["form"], "possible",
        "{item}"
    );
    let evidence = &item["record"]["evidence"];
    assert_eq!(evidence["arm"], "witness", "{item}");
    assert_eq!(
        evidence["paths"].as_array().map(Vec::len),
        Some(1),
        "{item}"
    );
    let steps = evidence["paths"][0]["path"]["steps"]
        .as_array()
        .expect("a concrete witness path");
    assert_eq!(steps.len(), 1, "{item}");
    assert_eq!(
        steps[0]["transition"]["operation"],
        r#"{"node":"ix://test/timed/Call/send","package":"test/timed"}"#,
        "{item}"
    );
    assert_eq!(
        steps[0]["delay"],
        json!({"numerator": "0", "denominator": "1"}),
        "{item}"
    );
    assert_eq!(run.exit, 0);
}

/// An item carrying a member no route reads (`witness_samples` on a clause)
/// settles `unsupported` `unsupported-requested-capability` even where its
/// subject is unbounded: it is never told to add a bound that would still
/// leave it unsupported.
#[trace("TC-762")]
#[test]
fn an_unread_member_settles_unsupported_not_requires_bound() {
    let document = crate::timed_world::document_unbounded(
        &["x"],
        &[
            ("send", &["phase", "inflight", "late", "x"]),
            ("reply", &["phase", "inflight", "late"]),
            ("timeout", &["phase"]),
            ("reset", &["phase"]),
        ],
    );
    let (run, items) = run_over(
        "timed-unread",
        &rpc_unit(4, &settles("[0 ms, 3 ms]")),
        &document,
        json!([{"claim": "Settles", "temporal": {"witness_samples": 2}}]),
    );
    let item = &items[0];
    assert_eq!(label(item), "unsupported", "{item}");
    assert_eq!(
        item["record"]["diagnostic"]["cause"], "unsupported-requested-capability",
        "{item}"
    );
    assert_eq!(run.exit, 21);
}

/// The first two steps of `item`'s counterexample: each delay's numerator and
/// denominator and its operation's name.
fn head(item: &Value) -> Vec<(String, String, String)> {
    item["counterexample"]["prefix"]
        .as_array()
        .expect("a finite prefix")
        .iter()
        .take(2)
        .map(|step| {
            let delay = &step["delay"];
            (
                delay["numerator"].as_str().unwrap_or("-").to_owned(),
                delay["denominator"].as_str().unwrap_or("-").to_owned(),
                step["transition"]["operation"]
                    .as_str()
                    .and_then(|node| node.rsplit('/').next())
                    .unwrap_or("-")
                    .to_owned(),
            )
        })
        .collect()
}

/// FR-632-AC-2: with `options.method` `digital-clocks` the closed `Rpc`
/// subject settles the MITL claims as the zone route does, on backend
/// `en-1`: `eventually[0 ms, 3 ms]` proved with basis `exhaustive`, and the
/// closed `eventually[0 ms, 2 ms]` refuted by `send` at 0 then `reply` after
/// the integer delay 3, a counterexample that replays.
#[trace("TC-1592", "FR-632-AC-2")]
#[test]
fn digital_clocks_settle_mitl_claims_as_zones_do() {
    let unit = rpc_unit(
        4,
        &format!(
            "{}{}",
            settles("[0 ms, 3 ms]").replace("Settles", "Within"),
            settles("[0 ms, 2 ms]").replace("Settles", "Early")
        ),
    );
    let digital = json!({"method": "digital-clocks"});
    let (run, items) = run(
        "timed-digital-mitl",
        &unit,
        json!([{"claim": "Within", "options": digital.clone()},
            {"claim": "Early", "options": digital},
            {"claim": "Within"}, {"claim": "Early"}]),
    );
    assert_eq!(label(&items[0]), "proved", "{}", items[0]);
    assert_eq!(items[0]["record"]["backend"], "en-1", "{}", items[0]);
    let exploration = &items[0]["exploration"];
    assert_eq!(exploration["automaton_states"], "3", "{}", items[0]);
    assert!(
        exploration["states"]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap()
            > exploration["automaton_states"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap(),
        "automaton states count distinct claim states, not product states: {}",
        items[0]
    );
    assert_eq!(
        items[0]["record"]["basis"]["type"], "exhaustive",
        "{}",
        items[0]
    );
    assert_eq!(label(&items[1]), "refuted", "{}", items[1]);
    assert_eq!(items[1]["record"]["backend"], "en-1", "{}", items[1]);
    assert_eq!(items[1]["replay"]["reproduced"], true, "{}", items[1]);
    let expected: Vec<(String, String, String)> = [("0", "1", "send"), ("3", "1", "reply")]
        .iter()
        .map(|(n, d, op)| ((*n).to_owned(), (*d).to_owned(), (*op).to_owned()))
        .collect();
    assert_eq!(head(&items[1]), expected, "{}", items[1]["counterexample"]);
    // The zone route agrees.
    assert_eq!(label(&items[2]), "proved", "{}", items[2]);
    assert_eq!(items[2]["record"]["backend"], "en-6", "{}", items[2]);
    assert_eq!(label(&items[3]), "refuted", "{}", items[3]);
    assert_eq!(run.exit, 10);
}

/// FR-126's EN-1 statistics report the greatest automaton-state count of one
/// binding, even when different bindings reach disjoint claim states.
#[trace("TC-1592", "FR-632-AC-2")]
#[test]
fn digital_automaton_states_are_the_maximum_per_binding() {
    let body = format!(
        "{}\
         pre Idle using v on M::Call::idle {{ self.phase = 0 and self.phase = 1 }}\n\
         temporal BindingStates using tt over (c: M::Call) on origin \
         {{ always holds(c.phase = 0) }}\n",
        crate::timed_world::DENSE,
    );
    let world = crate::timed_world::world_over(
        &crate::timed_world::document(&[], &[("idle", &[])]),
        crate::zone_search::TIMED,
        &body,
        &["a", "z"],
    );
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("timed model");
    let node = &world.package.graph().temporal_clauses()[0];
    let inner = system.clause_letters(node).expect("clause letters");
    let letters = DifferentBindingAtoms {
        inner: &inner,
        reads: RefCell::new(BTreeMap::new()),
    };
    let request = DigitalRequest {
        subject: &timed,
        clause: node.clause(),
        letters: &letters,
        limits: DigitalLimits {
            max_states: u64::MAX,
            max_transitions: u64::MAX,
            max_automaton_states: u64::MAX,
        },
    };
    let outcome = check_digital(&request, &Cancel::new()).expect("digital route accepts");
    let ModelCheckOutcome::Violated { stats, .. } = outcome.outcome else {
        panic!("the false binding refutes: {outcome:?}");
    };
    assert_eq!(
        letters.reads.borrow().keys().cloned().collect::<Vec<_>>(),
        ["a", "z"]
    );
    assert_eq!(stats.automaton_states, 1);
}

/// Per-binding atoms put the two instances in disjoint one-state regions of
/// the same claim automaton.
struct DifferentBindingAtoms<'a> {
    inner: &'a dyn ModelLetters,
    reads: RefCell<BTreeMap<String, u64>>,
}

impl ModelLetters for DifferentBindingAtoms<'_> {
    fn atom(
        &self,
        _atom: FormulaNodeId,
        instance: &ModelInstance,
        _state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        *self
            .reads
            .borrow_mut()
            .entry(instance.key.clone())
            .or_default() += 1;
        Ok(if instance.key == "a" {
            AtomReading::True
        } else {
            AtomReading::False
        })
    }

    fn activation(
        &self,
        instance: &ModelInstance,
        state: &ModelSimState,
        meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        self.inner.activation(instance, state, meter)
    }

    fn captures(
        &self,
        instance: &ModelInstance,
        state: &ModelSimState,
        meter: &mut Meter,
    ) -> Result<CaptureReading, AtomStop> {
        self.inner.captures(instance, state, meter)
    }

    fn reads(&self, atom: FormulaNodeId) -> crate::units::Reads {
        self.inner.reads(atom)
    }

    fn activation_reads(&self) -> crate::units::Reads {
        self.inner.activation_reads()
    }
}

/// A test letter seam that returns `Undefined` on the second activation read:
/// the initial position is defined and the quiescent idle-tail read is not.
struct UndefinedOnIdleActivation<'a> {
    inner: &'a dyn ModelLetters,
    activation_reads: Cell<u8>,
}

impl ModelLetters for UndefinedOnIdleActivation<'_> {
    fn atom(
        &self,
        atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        self.inner.atom(atom, instance, state, meter)
    }

    fn activation(
        &self,
        instance: &ModelInstance,
        state: &ModelSimState,
        meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        let reads = self.activation_reads.get();
        self.activation_reads.set(reads.saturating_add(1));
        if reads == 0 {
            self.inner.activation(instance, state, meter)
        } else {
            Ok(AtomReading::Undefined(UndefinedCause::Kernel(
                Undefined::DivisionByZero,
            )))
        }
    }

    fn captures(
        &self,
        instance: &ModelInstance,
        state: &ModelSimState,
        meter: &mut Meter,
    ) -> Result<CaptureReading, AtomStop> {
        self.inner.captures(instance, state, meter)
    }

    fn reads(&self, atom: FormulaNodeId) -> crate::units::Reads {
        self.inner.reads(atom)
    }

    fn activation_reads(&self) -> crate::units::Reads {
        self.inner.activation_reads()
    }
}

/// FR-126's idle-tail expansion treats an undefined letter as an
/// `UndefinedEvaluation` violation, preserving the site's locus and cause.
#[trace("TC-1592", "FR-632-AC-2")]
#[test]
fn digital_idle_tail_undefined_reading_is_a_violation() {
    let body = format!(
        "{}\
         pre SendIdle using v on M::Call::send {{ self.phase = 0 and self.phase = 1 }}\n\
         temporal IdleTailUndefined using tt over (c: M::Call) on each (x: M::Call) when (true) \
         {{ always holds(true) }}\n",
        crate::timed_world::DENSE,
    );
    let world = crate::timed_world::world(
        &crate::zone_search::rpc_document(),
        crate::zone_search::TIMED,
        &body,
    );
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("timed model");
    let node = &world.package.graph().temporal_clauses()[0];
    let letters = system.clause_letters(node).expect("clause letters");
    let letters = UndefinedOnIdleActivation {
        inner: &letters,
        activation_reads: Cell::new(0),
    };
    let request = DigitalRequest {
        subject: &timed,
        clause: node.clause(),
        letters: &letters,
        limits: DigitalLimits {
            max_states: u64::MAX,
            max_transitions: u64::MAX,
            max_automaton_states: u64::MAX,
        },
    };
    let outcome = check_digital(&request, &Cancel::new()).expect("digital route accepts");
    let ModelCheckOutcome::Violated { counterexample, .. } = outcome.outcome else {
        panic!("the idle-tail read is a violation: {outcome:?}");
    };
    let CounterexampleKind::UndefinedEvaluation(undefined) = &counterexample.kind else {
        panic!("the counterexample names the undefined evaluation: {counterexample:?}");
    };
    assert_eq!(undefined.site.locus, fsl_replay::UndefinedLocus::Activation);
    assert_eq!(
        undefined.cause,
        UndefinedCause::Kernel(Undefined::DivisionByZero)
    );
}

/// QSL-2129: an item naming `options.method` `zones` negotiates to EN-6 and
/// one naming a value no backend publishes has no candidate.
#[trace("TC-2290", "FR-243-AC-4")]
#[test]
fn the_method_option_is_negotiated_by_its_published_values() {
    let (_, items) = run(
        "timed-method-values",
        &rpc_unit(4, &settles("[0 ms, 3 ms]")),
        json!([{"claim": "Settles", "options": {"method": "zones"}},
            {"claim": "Settles", "options": {"method": "regions"}}]),
    );
    assert_eq!(label(&items[0]), "proved", "{}", items[0]);
    assert_eq!(items[0]["record"]["backend"], "en-6", "{}", items[0]);
    assert_eq!(label(&items[1]), "unsupported", "{}", items[1]);
}

/// FR-625-AC-3 on the digital-clock route: a state budget returns
/// `incomplete` naming the limit, never a verdict, on backend `en-1`.
#[trace("TC-1592", "FR-632-AC-2")]
#[test]
fn a_digital_clock_state_budget_is_incomplete() {
    let (run, items) = run(
        "timed-digital-budget",
        &rpc_unit(4, &settles("[0 ms, 3 ms]")),
        json!([{"claim": "Settles", "options": {"method": "digital-clocks"},
            "temporal": {"limits": {"max_states": 2}}}]),
    );
    assert_eq!(label(&items[0]), "incomplete", "{}", items[0]);
    assert_eq!(
        items[0]["record"]["cause"]["limit"], "max_states",
        "{}",
        items[0]
    );
    assert_eq!(items[0]["record"]["backend"], "en-1", "{}", items[0]);
    assert_eq!(run.exit, 22);
}

/// FR-235-AC-8: the zone and digital-clock routes write the same FSpec
/// FR-416 `non_zeno` member for FR-632-AC-2's MITL claims.
#[trace("TC-3021", "FR-235-AC-8")]
#[test]
fn both_timed_routes_state_the_non_zeno_premise() {
    let unit = rpc_unit(
        4,
        &format!(
            "{}{}",
            settles("[0 ms, 3 ms]").replace("Settles", "Within"),
            settles("[0 ms, 2 ms]").replace("Settles", "Early")
        ),
    );
    let digital = json!({"method": "digital-clocks"});
    let (_, items) = run(
        "timed-non-zeno",
        &unit,
        json!([{"claim": "Within"}, {"claim": "Within", "options": digital.clone()},
            {"claim": "Early"}, {"claim": "Early", "options": digital}]),
    );
    let zone = &items[0]["record"]["non_zeno"];
    assert!(zone == "not-required" || zone == "assumed", "{}", items[0]);
    assert_eq!(&items[1]["record"]["non_zeno"], zone, "{}", items[1]);
    assert_eq!(items[2]["record"]["non_zeno"], "checked", "{}", items[2]);
    assert_eq!(items[3]["record"]["non_zeno"], "checked", "{}", items[3]);
}

/// A snapshot of the one call `c` of the domain package `package`.
fn call_snapshot(package: &str, identity: &str) -> Value {
    json!({
        "format": "filament.state.snapshot/v1",
        "identity": {"authority": "test", "identity": identity},
        "observation": "current", "model": {"identity": package},
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [{"population": format!("ix://{package}/calls"), "complete": true,
            "objects": [{"key": "c", "type": format!("ix://{package}/Call"),
                "fields": {"phase": {"integer": "0"}, "inflight": {"boolean": false},
                    "late": {"boolean": false}, "retried": {"boolean": false}}}]}],
    })
}

/// The untimed claim `Plain` over model `A` (`test/req`), run through
/// `fsl analyze` in a unit that also selects the timed model `M`
/// (`test/timed`, `time M dense`) when `timed_m`; `item` is the request
/// item; `scratch` names the test's own scratch directory. The one snapshot
/// covers `A` only, so the subject reads no timed model.
fn untimed_beside_timed(
    scratch: &str,
    timed_m: bool,
    item: Value,
) -> crate::analyze_cli_support::Run {
    use crate::timed_refinement::{req, treq};
    let time = if timed_m {
        "dimension Time;\nunit s : Time = rational(1, 1);\n\
         unit ms : Time = rational(1, 1000) * s;\ntime M dense unit ms;\n"
    } else {
        ""
    };
    let m_decl = if timed_m {
        "model M = \"test/timed\";\n"
    } else {
        ""
    };
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\n\
         profile inf = \"filament.temporal.infinite-trace/v1\";\n\
         profile t = \"filament.temporal.complete/v1\";\n\
         profile tt = \"filament.temporal.timed/v1\";\nmodel A = \"test/req\";\n{m_decl}{time}\
         pre Send using v on A::Call::send {{ not self.inflight }}\n\
         post Sent using v on A::Call::send {{ self.inflight }}\n\
         temporal Plain using inf over (c: A::Call) clock \"model-steps\" on origin \
         {{ always holds(c.phase <= 3) }}\n"
    );
    let ws = Workspace::new(scratch);
    ws.write("program.fsl", unit.as_bytes());
    ws.write("a.json", &req());
    ws.write("m.json", &treq());
    ws.write(
        "snap-a.json",
        call_snapshot("test/req", "sa").to_string().as_bytes(),
    );
    let source = |file: &str, identity: &str, document: &str| {
        json!({"format": "semantic-ir/2.0.0", "source": {"file": file, "authority": "test",
            "identity": identity, "document": document, "formal_revision": 1}})
    };
    let mut models = vec![source("a.json", "a-model", "A")];
    let documents = vec![json!({"file": "snap-a.json"})];
    if timed_m {
        models.push(source("m.json", "m-model", "M"));
    }
    let mut request = crate::analyze_cli_support::request_members(Some(json!([item])));
    request["models"] = json!(models);
    request["documents"] = json!(documents);
    analyze(
        &ws.write(
            "analyze.json",
            json!({"format": "native-analyze/1", "request": request})
                .to_string()
                .as_bytes(),
        ),
    )
}

/// QSL-2654, FR-243: a clause over an untimed model routes by the models its
/// subject reads, so a `time` declaration of another model of the unit leaves
/// it on EN-1: same backend, same verdict and same closure certificate as
/// without the timed model, never EN-6's zone product.
#[trace("TC-1592", "FR-243-AC-4")]
#[test]
fn an_untimed_claim_stays_on_en_1_beside_a_timed_model() {
    let mut outcomes = Vec::new();
    for timed_m in [false, true] {
        let run = untimed_beside_timed("untimed-en-1", timed_m, json!({"claim": "Plain"}));
        let item = run.item(0);
        assert_eq!(run.exit, 0, "timed_m={timed_m}: {item}");
        outcomes.push((
            item["record"].clone(),
            item["certificate"]["checker"].clone(),
            item["certificate"]["accepted"].clone(),
        ));
    }
    assert_eq!(outcomes[0].0["backend"], "en-1");
    assert_eq!(outcomes[0].0["result"], "proved");
    assert_eq!(outcomes[0], outcomes[1], "the timed model changes nothing");
}

/// QSL-2654, FSpec FR-415: `max_clocks` is read by a timed subject only. An
/// untimed claim beside a timed model it does not read ignores no option
/// silently: EN-1 does not read it, so the item is `unsupported`.
#[trace("TC-1592", "FR-632-AC-3")]
#[test]
fn max_clocks_on_an_untimed_claim_beside_a_timed_model_is_unrouted() {
    let item = json!({"claim": "Plain", "temporal": {"limits": {"max_clocks": 1}}});
    let run = untimed_beside_timed("untimed-max-clocks", true, item);
    assert_eq!(
        run.item(0)["record"]["result"],
        "unsupported",
        "{}",
        run.item(0)
    );
}

/// FR-632-AC-1: `always holds(true)` over one object whose `ping` resets `x`
/// under `time invariant x <= 1 ms` is true, so EN-6 proves it with an
/// accepted certificate.
#[trace("FR-632-AC-1")]
#[test]
fn zones_certify_a_safety_claim_under_a_resetting_deadline() {
    use crate::timed_world::{document, unit_text, DENSE, PACKAGE};
    let unit = unit_text(
        PACKAGE,
        crate::zone_search::TIMED,
        &format!(
            "{DENSE}post Pinged using v on M::Call::ping {{ self.x = 0 ms }}\n\
             time invariant Stall when {{ true }} {{ self.x <= 1 ms }}\n\
             temporal Safe using tt over (c: M::Call) clock \"model-time\" on origin \
             {{ always holds(true) }}\n"
        ),
    );
    let ws = Workspace::new("timed-resetting-safety");
    let run = analyze(&request_over(
        &ws,
        &unit,
        &document(&["x"], &[("ping", &["x"])]),
        &[rpc_snapshot()],
        json!({"items": [{"claim": "Safe"}]}),
    ));
    let item = run.item(0);
    assert_eq!(label(&item), "proved", "{item}");
}
