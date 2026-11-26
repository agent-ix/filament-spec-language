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
use fsl_analyze::zone_check::{ZoneCheckLimits, ZoneCheckOutcome, ZoneUndecided};
use fsl_eval::model_subject::{AtomStop, ModelInstance, ModelLetters};
use fsl_eval::simulation::model::TimedVacuity;
use fsl_eval::simulation::model::{ModelSimState, TimedSystem};
use fsl_eval::temporal::{AtomReading, CaptureReading, UndefinedCause};
use fsl_replay::ZoneCertificateRefusal;
use fsl_replay::{CounterexampleKind, ModelCheckOutcome};
use fsl_semantics::check::FormulaNodeId;
use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{analyze, label, request_over, Workspace};
use crate::analyze_family_support::{rpc_snapshot, rpc_unit, rpc_unit_with};
use crate::zone_replay::{replay, reproduced};
use crate::zone_search::{assert_edges_cover, claim, operations, proved, violated};

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

/// One `Call` object whose `ping` changes no data field, under
/// `time invariant x <= 1 ms`, with the temporal clauses `Safe`, `Live` and
/// `Wrong`; `resets` makes `ping` reset `x`. The items are the three claims
/// on EN-6, then on EN-1 `digital-clocks`; each result is backend, value and
/// cause kind.
fn stalling(name: &str, resets: bool) -> Vec<(String, String, String)> {
    use crate::timed_world::{document, unit_text, DENSE, PACKAGE};
    let (post, writes): (&str, &[&str]) = if resets {
        (
            "post Pinged using v on M::Call::ping { self.x = 0 ms }\n",
            &["x"],
        )
    } else {
        ("", &[])
    };
    let claims: String = [
        ("Safe", "always holds(true)"),
        ("Live", "always eventually holds(true)"),
        ("Wrong", "always holds(false)"),
    ]
    .iter()
    .map(|(claim, body)| {
        format!(
            "temporal {claim} using tt over (c: M::Call) clock \"model-time\" on origin \
             {{ {body} }}\n"
        )
    })
    .collect();
    let unit = unit_text(
        PACKAGE,
        crate::zone_search::TIMED,
        &format!(
            "{DENSE}{post}time invariant Stall when {{ true }} {{ self.x <= 1 ms }}\n{claims}"
        ),
    );
    let digital = json!({"method": "digital-clocks"});
    let ws = Workspace::new(name);
    let run = analyze(&request_over(
        &ws,
        &unit,
        &document(&["x"], &[("ping", writes)]),
        &[rpc_snapshot()],
        json!({"items": [{"claim": "Safe"}, {"claim": "Live"}, {"claim": "Wrong"},
            {"claim": "Safe", "options": digital.clone()},
            {"claim": "Live", "options": digital.clone()},
            {"claim": "Wrong", "options": digital}]}),
    ));
    (0..6)
        .map(|n| {
            let item = run.item(n);
            (
                item["record"]["backend"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
                label(&item).to_owned(),
                item["record"]["cause"]["kind"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
            )
        })
        .collect()
}

fn expected_on_both_routes<const N: usize>(
    per_claim: [(&str, &str); N],
) -> Vec<(String, String, String)> {
    ["en-6", "en-1"]
        .iter()
        .flat_map(|backend| {
            per_claim.iter().map(|(value, cause)| {
                (
                    (*backend).to_owned(),
                    (*value).to_owned(),
                    (*cause).to_owned(),
                )
            })
        })
        .collect()
}

/// FR-232 "Vacuity", FR-240 "Vacuity": FR-232-AC-3's `Stall` is closed and has
/// no time-divergent behaviour, so every claim over it, the false one
/// included, is `inconclusive` `no-admitted-behaviour` on both routes, with
/// no fairness declared.
#[trace("FR-232-AC-3", "FR-240-AC-4")]
#[test]
fn digital_clocks_prove_nothing_over_a_stalled_subject() {
    assert_eq!(
        stalling("timed-digital-stall", false),
        expected_on_both_routes([("inconclusive", "no-admitted-behaviour"); 3])
    );
}

/// The same model with `ping` resetting `x`: time diverges, so the true claims
/// are proved and the false one refuted on both routes. The first result,
/// EN-6 on `always holds(true)`, is not asserted: EN-6 rejects its own
/// certificate there (QSL-3486). A control for the stall test above; no
/// criterion states these verdicts, so it carries no trace tag.
#[test]
fn digital_clocks_decide_claims_once_time_can_diverge() {
    assert_eq!(
        stalling("timed-digital-stall-reset", true)[1..],
        expected_on_both_routes([("proved", ""), ("proved", ""), ("refuted", "")])[1..]
    );
}

/// Each claim of `claims` over `model`'s timed model with `body`'s
/// operations, on both timed routes: backend, value and cause kind. `analyze`
/// refuses a package selection of a timed model, so the source program is the
/// one input; the tests over [`partial_world`] read checked packages.
fn routes_over(
    name: &str,
    model: &[u8],
    body: &str,
    claims: &[(&str, &str)],
    snapshots: &[Vec<u8>],
    subject: Option<Value>,
) -> Vec<(String, String, String)> {
    use crate::timed_world::{unit_text, DENSE, PACKAGE};
    let temporal: String = claims
        .iter()
        .map(|(claim, formula)| {
            format!(
                "temporal {claim} using tt over (c: M::Call) clock \"model-time\" on origin \
                 {{ {formula} }}\n"
            )
        })
        .collect();
    let unit = unit_text(
        PACKAGE,
        crate::zone_search::TIMED,
        &format!("{DENSE}{body}{temporal}"),
    );
    let digital = json!({"method": "digital-clocks"});
    let items: Vec<Value> = claims
        .iter()
        .map(|(claim, _)| json!({"claim": claim}))
        .chain(
            claims
                .iter()
                .map(|(claim, _)| json!({"claim": claim, "options": digital.clone()})),
        )
        .collect();
    let mut extra = json!({"items": items});
    if let Some(subject) = subject {
        extra["subject"] = subject;
    }
    let ws = Workspace::new(name);
    let run = analyze(&request_over(&ws, &unit, model, snapshots, extra));
    (0..claims.len() * 2)
        .map(|n| {
            let item = run.item(n);
            (
                item["record"]["backend"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
                label(&item).to_owned(),
                item["record"]["cause"]["kind"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
            )
        })
        .collect()
}

/// From `phase` 0, `block` enters `phase` 2, where the invariant lets no time
/// pass and no step is enabled, and `go` enters the quiescent `phase` 1.
const LOCKING: &str = "pre Block using v on M::Call::block { self.phase = 0 }\n\
     post Blocked using v on M::Call::block { self.phase = 2 }\n\
     pre Go using v on M::Call::go { self.phase = 0 }\n\
     post Gone using v on M::Call::go { self.phase = 1 }\n\
     time invariant Locked when { self.phase = 2 } { self.x <= 0 ms }\n";

/// `6 mod (2 - phase)`, undefined only at `phase` 2. Only a package checked
/// under `Kernel` holds it: a source program is refused before it runs.
const MOD_DEFINED: &str = "function mod_defined using v(n: Int[0, 3]): Boolean pure \
     { 6 mod (2 - n) >= 0 }\n";

fn locking_document() -> Vec<u8> {
    crate::timed_world::document(&["x"], &[("block", &["phase"]), ("go", &["phase"])])
}

/// [`rpc_snapshot`] as the snapshot `identity` with `phase`.
fn snapshot_at(identity: &str, phase: u8) -> Vec<u8> {
    let mut snapshot: Value = serde_json::from_slice(&rpc_snapshot()).expect("the snapshot");
    snapshot["identity"]["identity"] = json!(identity);
    snapshot["populations"][0]["objects"][0]["fields"]["phase"] =
        json!({"integer": phase.to_string()});
    snapshot.to_string().into_bytes()
}

/// FR-239-AC-5's time-lock rule on both routes: `phase != 2` is false only on
/// the time-locked state, so it is proved, and the search goes on from there
/// to `go`, where `phase = 0` is false.
#[trace("FR-239-AC-5")]
#[test]
fn a_false_reading_on_a_time_locked_branch_refutes_nothing_on_either_route() {
    let expected = expected_on_both_routes([("proved", ""), ("refuted", "")]);
    let found = routes_over(
        "timed-lock-branch",
        &locking_document(),
        LOCKING,
        &[
            ("NotLocked", "always holds(c.phase != 2)"),
            ("Wrong", "always holds(c.phase = 0)"),
        ],
        &[rpc_snapshot()],
        None,
    );
    assert_eq!(found, expected);
}

/// The same rule over two initial states, one healthy (`phase` 0) and one
/// time-locked from the start (`phase` 2): `phase != 2` is proved, and
/// `phase != 1` is refuted after `go`.
#[trace("FR-239-AC-5")]
#[test]
fn a_locked_initial_state_beside_a_healthy_one_refutes_nothing() {
    let reference = |identity: &str| json!({"authority": "test", "identity": identity});
    let expected = expected_on_both_routes([("proved", ""), ("refuted", "")]);
    let found = routes_over(
        "timed-lock-initial",
        &locking_document(),
        LOCKING,
        &[
            ("NotLocked", "always holds(c.phase != 2)"),
            ("Wrong", "always holds(c.phase != 1)"),
        ],
        &[snapshot_at("s0", 0), snapshot_at("s1", 2)],
        Some(json!({"initial_states": [reference("s1"), reference("s0")],
            "universes": [reference("s0")], "proof_bounds": []})),
    );
    assert_eq!(found, expected);
}

/// The Kernel-checked package of one claim over [`LOCKING`].
fn partial_world(formula: &str) -> crate::timed_world::World {
    crate::timed_world::world_with_mode(
        &locking_document(),
        crate::zone_search::TIMED,
        &format!(
            "{}{MOD_DEFINED}{LOCKING}temporal Partial using tt over (c: M::Call) clock \
             \"model-time\" on origin {{ {formula} }}\n",
            crate::timed_world::DENSE
        ),
        filament_semantic_value::checking::CheckMode::Kernel,
    )
}

/// [`check_digital`] on `system`'s timed model and the claim of `world`.
fn digital_on(
    world: &crate::timed_world::World,
    system: &fsl_eval::simulation::model::ModelSystem<'_>,
) -> ModelCheckOutcome {
    let timed = TimedSystem::new(system, "M").expect("timed model");
    let node = &world.package.graph().temporal_clauses()[0];
    let letters = system.clause_letters(node).expect("clause letters");
    let request = DigitalRequest {
        subject: &timed,
        clause: node.clause(),
        letters: &letters,
        limits: UNLIMITED,
    };
    check_digital(&request, &Cancel::new())
        .expect("digital route accepts")
        .outcome
}

const UNLIMITED: DigitalLimits = DigitalLimits {
    max_states: u64::MAX,
    max_transitions: u64::MAX,
    max_automaton_states: u64::MAX,
};

/// QSL-3485 over the production evaluator and a checked package: the reading
/// undefined only on the locked branch hides nothing. By hand `phase = 0` is
/// false after `go` (so the conjunction refutes there), on both engines, and
/// both counterexamples replay.
#[trace("FR-239-AC-5")]
#[test]
fn an_undefined_reading_on_a_time_locked_branch_hides_no_violation() {
    let world = partial_world("always holds(mod_defined(c.phase) and c.phase = 0)");
    let system = world.system();
    let ModelCheckOutcome::Violated { counterexample, .. } = digital_on(&world, &system) else {
        panic!("en-1: `go` violates the claim");
    };
    let zones = violated(claim(&world, ZoneCheckLimits::default(), &Cancel::new()));
    for counterexample in [*counterexample, zones] {
        assert_eq!(counterexample.kind, CounterexampleKind::Formula);
        assert_eq!(operations(&counterexample), ["go"]);
        let settlement = replay(&world, &counterexample).expect("replays");
        assert!(reproduced(&settlement), "{settlement:?}");
    }
}

/// Where the claim holds on every admitted behaviour, the reading undefined
/// on the locked one leaves it proved: EN-1 holds, and EN-6's certificate
/// passes the independent checker.
#[trace("FR-239-AC-5")]
#[test]
fn an_undefined_reading_only_on_a_time_locked_branch_leaves_the_claim_proved() {
    let world = partial_world("always holds(mod_defined(c.phase) and c.phase <= 1)");
    let system = world.system();
    let digital = digital_on(&world, &system);
    assert!(
        matches!(digital, ModelCheckOutcome::HoldsReduced { .. }),
        "{digital:?}"
    );
    let certificate = proved(claim(&world, ZoneCheckLimits::default(), &Cancel::new()));
    assert_edges_cover(&world, &certificate);
}

/// An initial state that is time-locked and reads undefined, listed before a
/// healthy one: EN-1 goes on to the healthy one and proves.
#[trace("FR-239-AC-5")]
#[test]
fn digital_clocks_pass_an_undefined_time_locked_initial_state() {
    let world = partial_world("always holds(mod_defined(c.phase))");
    let locked = snapshot_at("locked", 2);
    let mut snapshots = world.snapshots.clone();
    snapshots.insert(
        fsl_semantics::model::observation::document_reference(&locked)
            .expect("a document names itself"),
        locked,
    );
    let mut request = world.request();
    request.snapshots = &snapshots;
    request.initial.insert(
        0,
        fsl_semantics::model::observation::DocumentRef {
            authority: "test".to_owned(),
            identity: "locked".to_owned(),
        },
    );
    let system = fsl_eval::simulation::model::ModelSystem::new(request).expect("admits");
    assert_eq!(system.initial_states().len(), 2);
    let outcome = digital_on(&world, &system);
    assert!(
        matches!(outcome, ModelCheckOutcome::HoldsReduced { .. }),
        "{outcome:?}"
    );
}

/// FR-125-AC-13: a timed subject with no initial state is `inconclusive`
/// `no-initial-state` on both routes, never proved, and ahead of the
/// instance set, which is empty too.
#[trace("FR-125-AC-13")]
#[test]
fn a_timed_claim_over_no_initial_state_is_never_proved() {
    let start = json!({"authority": "test", "identity": "s0"});
    let expected = expected_on_both_routes([("inconclusive", "no-initial-state"); 2]);
    let found = routes_over(
        "timed-no-initial",
        &locking_document(),
        LOCKING,
        &[
            ("Safe", "always holds(true)"),
            ("Wrong", "always holds(false)"),
        ],
        &[rpc_snapshot()],
        Some(json!({"initial_states": [], "universes": [start], "proof_bounds": []})),
    );
    assert_eq!(found, expected);
}

/// Counts the atom reads and reads as `inner` does, but for the first false
/// atom when `undefine_first_false` is set.
struct CountedLetters<'a> {
    inner: &'a dyn ModelLetters,
    atoms: Cell<u64>,
    undefine_first_false: Cell<bool>,
}

impl<'a> CountedLetters<'a> {
    fn new(inner: &'a dyn ModelLetters, undefine_first_false: bool) -> Self {
        Self {
            inner,
            atoms: Cell::new(0),
            undefine_first_false: Cell::new(undefine_first_false),
        }
    }
}

impl ModelLetters for CountedLetters<'_> {
    fn atom(
        &self,
        atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        self.atoms.set(self.atoms.get() + 1);
        let reading = self.inner.atom(atom, instance, state, meter)?;
        if matches!(reading, AtomReading::False) && self.undefine_first_false.replace(false) {
            return Ok(AtomReading::Undefined(UndefinedCause::Kernel(
                Undefined::DivisionByZero,
            )));
        }
        Ok(reading)
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

/// [`check_digital`] on the checked package of `body`'s one claim over
/// `document`, with the letters [`CountedLetters`] wraps: the outcome and the
/// atom reads.
fn digital_over(model: &[u8], body: &str, undefine_first_false: bool) -> (ModelCheckOutcome, u64) {
    let world = crate::timed_world::world(
        model,
        crate::zone_search::TIMED,
        &format!("{}{body}", crate::timed_world::DENSE),
    );
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("timed model");
    let node = &world.package.graph().temporal_clauses()[0];
    let inner = system.clause_letters(node).expect("clause letters");
    let letters = CountedLetters::new(&inner, undefine_first_false);
    let request = DigitalRequest {
        subject: &timed,
        clause: node.clause(),
        letters: &letters,
        limits: UNLIMITED,
    };
    let outcome = check_digital(&request, &Cancel::new()).expect("digital route accepts");
    (outcome.outcome, letters.atoms.get())
}

/// QSL-3485: `block`, searched first, locks time, and the reading there is
/// made undefined; `go` still refutes `always holds(c.phase = 0)`.
#[trace("FR-239-AC-5")]
#[test]
fn digital_undefined_at_a_time_lock_does_not_hide_a_violation() {
    let body = "pre Block using v on M::Call::block { self.phase = 0 }\n\
         post Blocked using v on M::Call::block { self.phase = 2 }\n\
         pre Go using v on M::Call::go { self.phase = 0 }\n\
         post Gone using v on M::Call::go { self.phase = 1 }\n\
         time invariant Locked when { self.phase = 2 } { self.x <= 0 ms }\n\
         temporal Stays using tt over (c: M::Call) clock \"model-time\" on origin \
         { always holds(c.phase = 0) }\n";
    let (outcome, _) = digital_over(&locking_document(), body, true);
    let ModelCheckOutcome::Violated { counterexample, .. } = outcome else {
        panic!("`go` violates the claim: {outcome:?}");
    };
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    assert_eq!(operations(&counterexample), ["go"]);
}

/// FR-232 "Vacuity": the subject's admitted behaviour is classified before
/// any claim product, so over the stalled subject no claim atom is read.
#[trace("FR-232-AC-3")]
#[test]
fn digital_clocks_read_no_claim_over_a_stalled_subject() {
    let body = "time invariant Stall when { true } { self.x <= 1 ms }\n\
         temporal Wrong using tt over (c: M::Call) clock \"model-time\" on origin \
         { always holds(c.phase = 1) }\n";
    let (outcome, atoms) = digital_over(
        &crate::timed_world::document(&["x"], &[("ping", &[])]),
        body,
        false,
    );
    assert!(
        matches!(
            outcome,
            ModelCheckOutcome::Undecided {
                cause: fsl_replay::UndecidedCause::NoAdmittedBehaviour,
                ..
            }
        ),
        "{outcome:?}"
    );
    assert_eq!(atoms, 0);
}

/// FR-125 "Instancing": a claim whose `over` type admits no key settles
/// `inconclusive` `no-instance`, the false one included, on both timed
/// routes.
#[trace("FR-125-AC-7")]
#[test]
fn timed_claims_over_no_instance_are_never_proved() {
    let mut snapshot: Value = serde_json::from_slice(&rpc_snapshot()).expect("the snapshot");
    snapshot["populations"][0]["objects"] = json!([]);
    let expected = expected_on_both_routes([
        ("inconclusive", "no-instance"),
        ("inconclusive", "no-instance"),
    ]);
    let found = routes_over(
        "timed-no-instance",
        &locking_document(),
        LOCKING,
        &[
            ("Safe", "always holds(true)"),
            ("Never", "always holds(false)"),
        ],
        &[snapshot.to_string().into_bytes()],
        None,
    );
    assert_eq!(found, expected);
}

/// The EN-6 certificate checker accepts no certificate over a subject with no
/// instance: the proof of a claim over one object, checked where the
/// population is empty.
#[trace("FR-125-AC-7")]
#[test]
fn a_zone_certificate_over_no_instance_is_refused() {
    let world = partial_world("always holds(c.phase <= 2)");
    let certificate = proved(claim(&world, ZoneCheckLimits::default(), &Cancel::new()));
    assert_edges_cover(&world, &certificate);
    let mut snapshot: Value = serde_json::from_slice(&rpc_snapshot()).expect("the snapshot");
    snapshot["populations"][0]["objects"] = json!([]);
    snapshot["identity"]["identity"] = json!("empty");
    let empty = snapshot.to_string().into_bytes();
    let reference = fsl_semantics::model::observation::document_reference(&empty)
        .expect("a document names itself");
    let mut snapshots = world.snapshots.clone();
    snapshots.insert(reference, empty);
    let mut request = world.request();
    request.snapshots = &snapshots;
    request.initial = vec![fsl_semantics::model::observation::DocumentRef {
        authority: "test".to_owned(),
        identity: "empty".to_owned(),
    }];
    let checked = crate::zone_search::check_over(
        &world,
        request,
        &certificate,
        fsl_replay::CertificateCheckLimits::default(),
    );
    assert_eq!(
        checked,
        Err(ZoneCertificateRefusal::Vacuous(TimedVacuity::NoInstance))
    );
}

/// The Kernel-checked package of one claim over the operations `operations`
/// with the clauses `model`, [`MOD_DEFINED`] in scope.
fn kernel_world(
    operations: &[(&str, &[&str])],
    model: &str,
    formula: &str,
) -> crate::timed_world::World {
    crate::timed_world::world_with_mode(
        &crate::timed_world::document(&["x"], operations),
        crate::zone_search::TIMED,
        &format!(
            "{}{MOD_DEFINED}{model}temporal Partial using tt over (c: M::Call) clock \
             \"model-time\" on origin {{ {formula} }}\n",
            crate::timed_world::DENSE
        ),
        filament_semantic_value::checking::CheckMode::Kernel,
    )
}

/// [`LOCKING`] with `phase` 2 let sit until `x` is 1 and `spin` looping there
/// in zero time: time never passes 1 ms from it, so it is a time-lock that a
/// delay enters.
const ZENO_LOCK: &str = "pre Block using v on M::Call::block { self.phase = 0 }\n\
     post Blocked using v on M::Call::block { self.phase = 2 }\n\
     pre Spin using v on M::Call::spin { self.phase = 2 }\n\
     post Spun using v on M::Call::spin { self.phase = 2 }\n\
     pre Go using v on M::Call::go { self.phase = 0 }\n\
     post Gone using v on M::Call::go { self.phase = 1 }\n\
     time invariant Locked when { self.phase = 2 } { self.x <= 1 ms }\n";

fn zeno_lock_world(formula: &str) -> crate::timed_world::World {
    kernel_world(
        &[
            ("block", &["phase"]),
            ("spin", &["phase"]),
            ("go", &["phase"]),
        ],
        ZENO_LOCK,
        formula,
    )
}

/// By hand: the reading is undefined only at `phase` 2, which is time-locked,
/// and `phase` 0 and 1 read true, so both routes prove the claim.
#[trace("FR-239-AC-5")]
#[test]
fn a_zero_time_loop_entered_by_a_delay_is_a_time_lock() {
    let world = zeno_lock_world("always holds(mod_defined(c.phase) and c.phase <= 1)");
    let system = world.system();
    let digital = digital_on(&world, &system);
    assert!(
        matches!(digital, ModelCheckOutcome::HoldsReduced { .. }),
        "{digital:?}"
    );
    let certificate = proved(claim(&world, ZoneCheckLimits::default(), &Cancel::new()));
    assert_edges_cover(&world, &certificate);
}

/// By hand: `go` makes `phase = 0` false, and the undefined reading on the
/// time-locked `phase` 2, found first, hides nothing.
#[trace("FR-239-AC-5")]
#[test]
fn a_zero_time_loop_entered_by_a_delay_hides_no_violation() {
    let world = zeno_lock_world("always holds(mod_defined(c.phase) and c.phase = 0)");
    let system = world.system();
    let ModelCheckOutcome::Violated { counterexample, .. } = digital_on(&world, &system) else {
        panic!("en-1: `go` violates the claim");
    };
    let zones = violated(claim(&world, ZoneCheckLimits::default(), &Cancel::new()));
    for counterexample in [*counterexample, zones] {
        assert_eq!(counterexample.kind, CounterexampleKind::Formula);
        assert_eq!(operations(&counterexample), ["go"]);
    }
}

/// FSpec FR-161 "Vacuity": `go` is enabled at every instant of the one
/// divergent behaviour (the `idle` loop) and leads to a time-lock, so under
/// `fair weak go` the subject admits no behaviour: every claim, the false one
/// included, is `inconclusive` `no-admitted-behaviour` on both routes.
#[trace("FSpec-FR-161-AC-15")]
#[test]
fn a_timed_claim_with_no_fair_behaviour_is_never_proved() {
    let model = "pre Idle using v on M::Call::idle { self.phase = 0 }\n\
         post Idled using v on M::Call::idle { self.x = 0 ms }\n\
         pre Go using v on M::Call::go { self.phase = 0 }\n\
         post Gone using v on M::Call::go { self.phase = 2 and self.x = 0 ms }\n\
         time invariant Busy when { self.phase = 0 } { self.x <= 1 ms }\n\
         time invariant Done when { self.phase = 2 } { self.x <= 0 ms }\n";
    let expected = expected_on_both_routes([("inconclusive", "no-admitted-behaviour"); 3]);
    let found = routes_over(
        "timed-no-fair-behaviour",
        &crate::timed_world::document(&["x"], &[("idle", &["x"]), ("go", &["phase", "x"])]),
        model,
        &[
            (
                "Invariant",
                "fair weak M::Call::go; always holds(c.phase != 1)",
            ),
            (
                "Window",
                "fair weak M::Call::go; always[0 ms, 1 ms] holds(c.phase != 1)",
            ),
            ("Never", "fair weak M::Call::go; always holds(false)"),
        ],
        &[rpc_snapshot()],
        None,
    );
    assert_eq!(found, expected);
}

/// FSpec FR-161 "Fairness over time": `phase` 3 is left only unfairly (the
/// `idle` loop, with `go` always enabled) or into a time-lock, so a claim
/// false only there holds on every fair behaviour; idling in `phase` 0 is one.
#[trace("FSpec-FR-161-AC-16")]
#[test]
fn a_violation_only_on_unfair_behaviours_refutes_nothing() {
    let model = "pre Bad using v on M::Call::block { self.phase = 0 }\n\
         post Badly using v on M::Call::block { self.phase = 3 }\n\
         pre Idle using v on M::Call::idle { self.phase = 3 }\n\
         post Idled using v on M::Call::idle { self.x = 0 ms }\n\
         pre Go using v on M::Call::go { self.phase = 3 }\n\
         post Gone using v on M::Call::go { self.phase = 2 and self.x = 0 ms }\n\
         time invariant Busy when { self.phase = 3 } { self.x <= 1 ms }\n\
         time invariant Done when { self.phase = 2 } { self.x <= 0 ms }\n";
    let expected = expected_on_both_routes([("proved", "")]);
    let found = routes_over(
        "timed-unfair-violation",
        &crate::timed_world::document(
            &["x"],
            &[
                ("block", &["phase"]),
                ("idle", &["x"]),
                ("go", &["phase", "x"]),
            ],
        ),
        model,
        &[(
            "Invariant",
            "fair weak M::Call::go; always holds(c.phase != 3)",
        )],
        &[rpc_snapshot()],
        None,
    );
    assert_eq!(found, expected);
}

/// FSpec FR-161 "Vacuity" ahead of an undefined reading: the reading is
/// undefined at `phase` 2, from which time diverges only on the `spin` loop
/// that `fair weak go` rejects, and no behaviour at all is fair. Both routes
/// settle `no-admitted-behaviour`, not `undefined-evaluation`.
#[trace("FSpec-FR-161-AC-15")]
#[test]
fn no_fair_behaviour_comes_before_an_undefined_refutation() {
    let world = kernel_world(
        &[
            ("block", &["phase"]),
            ("spin", &["x"]),
            ("go", &["phase", "x"]),
        ],
        "pre Block using v on M::Call::block { self.phase = 0 }\n\
         post Blocked using v on M::Call::block { self.phase = 2 }\n\
         pre Spin using v on M::Call::spin { self.phase = 2 }\n\
         post Spun using v on M::Call::spin { self.x = 0 ms }\n\
         pre Go using v on M::Call::go { self.phase = 2 }\n\
         post Gone using v on M::Call::go { self.phase = 1 and self.x = 0 ms }\n\
         time invariant Start when { self.phase = 0 } { self.x <= 1 ms }\n\
         time invariant Busy when { self.phase = 2 } { self.x <= 1 ms }\n\
         time invariant Done when { self.phase = 1 } { self.x <= 0 ms }\n",
        "fair weak M::Call::go; always holds(mod_defined(c.phase))",
    );
    let system = world.system();
    let digital = digital_on(&world, &system);
    assert!(
        matches!(
            digital,
            ModelCheckOutcome::Undecided {
                cause: fsl_replay::UndecidedCause::NoAdmittedBehaviour,
                ..
            }
        ),
        "{digital:?}"
    );
    let zones = claim(&world, ZoneCheckLimits::default(), &Cancel::new());
    assert!(
        matches!(
            zones,
            ZoneCheckOutcome::Undecided {
                cause: ZoneUndecided::NoAdmittedBehaviour,
                ..
            }
        ),
        "{zones:?}"
    );
}

/// A temporal claim and a state-graph claim, each on both timed routes, over
/// `model` with `body`, no initial state and a universe snapshot listing no
/// population: each item's result, diagnostic code and cause.
fn over_no_universe(name: &str, model: &[u8], body: &str) -> Vec<(String, String, String)> {
    use crate::timed_world::{unit_text, DENSE, PACKAGE};
    let unit = unit_text(
        PACKAGE,
        crate::zone_search::TIMED,
        &format!(
            "{DENSE}{body}temporal Safe using tt over (c: M::Call) clock \"model-time\" on \
             origin {{ always holds(true) }}\n\
             possible Reaches using v over (c: M::Call) {{ c.phase = 1 }}\n"
        ),
    );
    let digital = json!({"method": "digital-clocks"});
    let ws = Workspace::new(name);
    let mut universe: Value = serde_json::from_slice(&rpc_snapshot()).expect("the snapshot");
    universe["identity"]["identity"] = json!("u0");
    universe["populations"] = json!([]);
    let run = analyze(&request_over(
        &ws,
        &unit,
        model,
        &[rpc_snapshot(), universe.to_string().into_bytes()],
        json!({"items": [{"claim": "Safe"}, {"claim": "Safe", "options": digital.clone()},
            {"claim": "Reaches"}, {"claim": "Reaches", "options": digital}],
            "subject": {"initial_states": [], "universes": [{"authority": "test", "identity": "u0"}],
                "proof_bounds": []}}),
    ));
    (0..4)
        .map(|n| {
            let record = &run.item(n)["record"];
            let text = |value: &Value| value.as_str().unwrap_or_default().to_owned();
            (
                text(&record["result"]),
                text(&record["diagnostic"]["code"]),
                text(&record["diagnostic"]["cause"]),
            )
        })
        .collect()
}

fn requires_bound_on_all_four() -> Vec<(String, String, String)> {
    vec![
        (
            "unsupported".to_owned(),
            "unsupported_projection".to_owned(),
            "unbounded-extent".to_owned(),
        );
        4
    ]
}

/// FR-125-AC-13: a subject with no clock whose population has no universe is
/// refused for the missing bound, ahead of its empty initial list.
#[trace("FR-125-AC-13")]
#[test]
fn a_timed_claim_over_no_universe_requires_a_bound() {
    let model = crate::timed_world::document(&[], &[("go", &["phase"])]);
    assert_eq!(
        over_no_universe("timed-no-universe", &model, ""),
        requires_bound_on_all_four()
    );
}

/// FR-125-AC-13 with a clock the population's objects own: its clocks cannot
/// be listed without the universe, which is a missing bound, not a bad value.
#[trace("FR-125-AC-13")]
#[test]
fn a_clock_owner_with_no_universe_requires_a_bound() {
    assert_eq!(
        over_no_universe("timed-no-universe-clocked", &locking_document(), LOCKING),
        requires_bound_on_all_four()
    );
}
