// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-213 (TC-658): interval operators over a protocol subject measure
//! distance in counted steps (ADR-027 PB-2, PB-3).
//!
//! ADR-027 §7's repaired `Fill` (`TwoPre: self.v <= 1`) and §7.1's `Pay`
//! (`bal` is `Cell.v`, see `crate::protocol_compensation`) are read as
//! behaviours of `ProtocolSystem<Sc>`: the evaluator reads each position's
//! atom through the state and each step's `counts` flag, and EN-1 checks the
//! same claim forms on the product with the claim's counted monitor.

use filament_exact::{Cancel, Meter};
use fsl_analyze::analyze::ProtocolItem;
use fsl_analyze::model_check::{
    check_model, check_protocol, ModelCheckRequest, ProtocolCheckRequest,
};
use fsl_eval::model_subject::{AtomStop, ModelInstance, ModelLetters, ProtocolSubject};
use fsl_eval::simulation::model::{ExpansionLimits, ModelSimState};
use fsl_eval::simulation::protocol::{
    PositionAnchor, ProtocolState, ProtocolSystem, ProtocolTransition, Sc, StepKind,
};
use fsl_eval::simulation::{SubjectSystem, TransitionSystem};
use fsl_eval::temporal::{
    evaluate_at, AtomReading, LetterStop, TemporalLetters, TemporalOutcome, TemporalPosition,
    TemporalTrace,
};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_package::CheckedPackage;
use fsl_replay::{
    check_protocol_closure, replay_protocol_trace_with_temporal_selections, settle_protocol_check,
    CertificateLocus, CertificateRejection, CertificateRule, Certification, ClauseIdentity,
    ClosureCertificate, IncompleteCause, ModelCheckItem, ModelCheckLimit, ModelCheckLimits,
    ModelCheckOutcome, ModelCheckRecord, ModelTraceSubject, ProductStateRef, ProofBasis,
    ProofCertificate, ProtocolCertificateRequest, ProtocolTraceSubject, ReplayLimits, ResultLabel,
    TemporalCertificateCheck, TemporalCounterexample, TerminalValue,
};
use fsl_semantics::check::{CheckedTemporalClause, FormulaNodeId};
use fsl_semantics::model::observation::DocumentRef;
use ix_trace_rs::trace;

use crate::counter::UNLIMITED;
use crate::fill::{
    attempt, contracts, document, fill_unit, instance, model_subject, package, protocol, reachable,
    sc_system, subject, unit_text, universe, v, Documents, Reached,
};
use crate::model_settle::{envelope_at, request};
use crate::protocol_compensation::pay;

/// A claim over `k: M::Cell`, `on origin`, under event-position
/// false-extension.
pub(crate) fn claim(formula: &str) -> String {
    format!(
        "temporal Claim using tb over (k: M::Cell) clock \"model-steps\" on origin \
         {{ {formula} }}\n"
    )
}

/// `body` from one initial snapshot, `c.v = 0`, with its checked package.
pub(crate) struct Case {
    pub(crate) body: String,
    pub(crate) documents: Documents,
    pub(crate) initial: Vec<DocumentRef>,
    pub(crate) checked: CheckedPackage,
    pub(crate) temporal_clock_selections: Vec<fsl_package::TemporalClockSelection>,
}

impl Case {
    pub(crate) fn new(body: &str) -> Self {
        let mut documents = Documents::new();
        let initial = vec![documents.start("s0", &[("c", 0)])];
        let checked = package(body);
        let temporal_clock_selections = crate::fill::tcb_01_event_position_selection(&checked);
        Self {
            body: body.to_owned(),
            documents,
            initial,
            checked,
            temporal_clock_selections,
        }
    }

    pub(crate) fn subject(&self, protocol: &str) -> ProtocolSubject<'_> {
        subject(
            &self.checked,
            &self.documents,
            self.initial.clone(),
            protocol,
            "c",
        )
    }

    pub(crate) fn clause(&self) -> &CheckedTemporalClause {
        self.checked.graph().temporal_clauses()[0].clause()
    }

    /// EN-1 over `protocol`'s claim, read through `letters`.
    pub(crate) fn check(
        &self,
        protocol: &str,
        letters: &dyn ModelLetters,
    ) -> ModelCheckOutcome<ProtocolTransition> {
        check_protocol(
            ProtocolCheckRequest {
                temporal_clock_selections: &self.temporal_clock_selections,
                subject: self.subject(protocol),
                item: ModelCheckItem::Clause {
                    clause: self.clause(),
                    letters,
                },
                max_depth: None,
                max_store_buffer: None,
                max_messages: None,
                max_live_instances: None,
                limits: ModelCheckLimits::default(),
                ignore_memory_constraints: false,
            },
            &Cancel::new(),
        )
        .expect("the protocol subject checks")
    }

    /// FR-127's label of `outcome`, its counterexample replayed by FR-217.
    pub(crate) fn label(
        &self,
        protocol: &str,
        letters: &dyn ModelLetters,
        outcome: &ModelCheckOutcome<ProtocolTransition>,
    ) -> ResultLabel {
        self.record(protocol, letters, outcome).label
    }

    /// FR-127's record of `outcome`, its counterexample replayed by FR-217.
    pub(crate) fn record(
        &self,
        protocol: &str,
        letters: &dyn ModelLetters,
        outcome: &ModelCheckOutcome<ProtocolTransition>,
    ) -> ModelCheckRecord<ProtocolTransition> {
        let unit = unit_text(&self.body).into_bytes();
        let node = &self.checked.graph().temporal_clauses()[0];
        let clause_node = node.identity().clone();
        let occurrence = OccurrenceKey::new(clause_node.clone(), node.claim().clone());
        let replay = |payload: &TemporalCounterexample<ProtocolTransition>| {
            let subject = self.subject(protocol);
            replay_protocol_trace_with_temporal_selections(
                request(
                    &self.checked,
                    &self.temporal_clock_selections,
                    &document(),
                    self.documents.snapshots.iter(),
                    &unit,
                ),
                &ProtocolTraceSubject {
                    model: ModelTraceSubject {
                        timed: None,
                        initial: self.initial.clone(),
                        universes: universe(&["c"]),
                        expansion: ExpansionLimits {
                            max_candidates: 1_000_000,
                        },
                        stutter_closure: false,
                    },
                    protocol: protocol.to_owned(),
                    over: instance("c"),
                    roles: subject.roles,
                    capacities: Vec::new(),
                    memory: subject.memory,
                    max_store_buffer: None,
                    max_messages: None,
                    max_live_instances: None,
                },
                &envelope_at(payload.clone(), clause_node.clone(), occurrence.clone()),
                letters,
                &self.temporal_clock_selections,
                ReplayLimits::default(),
            )
        };
        settle_protocol_check(
            outcome,
            &ModelCheckLimits::default(),
            &ProtocolCertificateRequest {
                temporal_clock_selections: &self.temporal_clock_selections,
                subject: self.subject(protocol),
                item: ModelCheckItem::Clause {
                    clause: self.clause(),
                    letters,
                },
                max_store_buffer: None,
                max_messages: None,
                max_live_instances: None,
            },
            &Cancel::new(),
            replay,
        )
    }
}

impl Case {
    /// The `temporal-satisfaction` request item EN-1 runs over `protocol`'s
    /// claim; its replay is for a claim that holds, so it is never called.
    fn protocol_item<'a>(
        &'a self,
        protocol: &str,
        letters: &'a dyn ModelLetters,
    ) -> ProtocolItem<'a> {
        let item = || ModelCheckItem::Clause {
            clause: self.clause(),
            letters,
        };
        ProtocolItem {
            request: ProtocolCheckRequest {
                temporal_clock_selections: &self.temporal_clock_selections,
                subject: self.subject(protocol),
                item: item(),
                max_depth: None,
                max_store_buffer: None,
                max_messages: None,
                max_live_instances: None,
                limits: ModelCheckLimits::default(),
                ignore_memory_constraints: false,
            },
            certificate: ProtocolCertificateRequest {
                temporal_clock_selections: &self.temporal_clock_selections,
                subject: self.subject(protocol),
                item: item(),
                max_store_buffer: None,
                max_messages: None,
                max_live_instances: None,
            },
            replay: Box::new(|_| panic!("a claim that holds has no counterexample")),
        }
    }
}

impl Case {
    /// [`Self::protocol_item`] with the production replay of FR-217 behind
    /// it, so a refutation settles `refuted` through `analyze_items`.
    fn replaying_protocol_item<'a>(
        &'a self,
        protocol: &'a str,
        letters: &'a dyn ModelLetters,
    ) -> ProtocolItem<'a> {
        let mut item = self.protocol_item(protocol, letters);
        let unit = unit_text(&self.body).into_bytes();
        let node = &self.checked.graph().temporal_clauses()[0];
        let clause_node = node.identity().clone();
        let occurrence = OccurrenceKey::new(clause_node.clone(), node.claim().clone());
        item.replay = Box::new(move |payload| {
            let subject = self.subject(protocol);
            replay_protocol_trace_with_temporal_selections(
                request(
                    &self.checked,
                    &self.temporal_clock_selections,
                    &document(),
                    self.documents.snapshots.iter(),
                    &unit,
                ),
                &ProtocolTraceSubject {
                    model: ModelTraceSubject {
                        timed: None,
                        initial: self.initial.clone(),
                        universes: universe(&["c"]),
                        expansion: ExpansionLimits {
                            max_candidates: 1_000_000,
                        },
                        stutter_closure: false,
                    },
                    protocol: protocol.to_owned(),
                    over: instance("c"),
                    roles: subject.roles,
                    capacities: Vec::new(),
                    memory: subject.memory,
                    max_store_buffer: None,
                    max_messages: None,
                    max_live_instances: None,
                },
                &envelope_at(payload.clone(), clause_node.clone(), occurrence.clone()),
                letters,
                &self.temporal_clock_selections,
                ReplayLimits::default(),
            )
        });
        item
    }
}

/// FR-281 over a protocol subject: the refuted `Fill` clause settles through
/// `analyze_items` with its one counterexample, the replayed protocol
/// `TemporalCounterexample`, written as FR-433 gives it: the model-check
/// members, the `subject` naming the protocol and its `over` reference, steps
/// carrying protocol transition identities, and `max_live_instances`.
#[trace("TC-770", "FR-286-AC-14")]
#[test]
fn a_refuted_protocol_clause_carries_its_spelled_counterexample() {
    use fsl_analyze::analyze::{analyze_items, protocol_item, split_records, Engines};
    use fsl_foundation::RequestIndex;
    use fsl_replay::TerminalValue;

    let case = repaired_fill("eventually[0,1] holds(k.v = 3)");
    let letters = CellV(|value| value == 3);
    let item = protocol_item(
        RequestIndex::new(0),
        case.replaying_protocol_item("Fill", &letters),
    )
    .expect("a clause run");
    let records = analyze_items(
        &Engines::fsl().expect("the linked engines register"),
        vec![item],
        &Cancel::new(),
    );
    let (terminals, found) = split_records(records, &case.checked);
    assert_eq!(terminals[0].value(), &TerminalValue::Refuted);
    let [found] = found.as_slice() else {
        panic!("one counterexample: {found:?}");
    };
    let wire = found
        .wire
        .as_ref()
        .expect("a protocol counterexample is spelled");
    assert_eq!(wire["trace"], "model");
    assert_eq!(wire["kind"], "formula");
    assert_eq!(wire["subject"]["type"], "protocol-subject");
    assert_eq!(wire["subject"]["protocol"], "Fill");
    assert_eq!(wire["subject"]["over"]["type"], "reference");
    // `Fill` activates `on origin`, so no instance budget applies (FR-433).
    assert!(wire.get("max_live_instances").is_none());
    assert!(wire.get("blocked").is_none());
    let steps = wire["prefix"].as_array().expect("steps");
    assert!(!steps.is_empty());
    for step in steps {
        assert_eq!(step["transition"]["type"], "protocol-transition");
        assert!(step["post_state"]["key"].is_string());
    }
}

/// FSL FR-507 (TC-1302): a protocol clause item is a `temporal-trace` claim
/// (its clause is a `temporal_clause` node, FSpec FR-290), so
/// `temporal-satisfaction` applies and EN-1 settles it; read as a `protocol`
/// claim, `analyze` would refuse it `inapplicable-kind`.
#[test]
fn a_protocol_clause_item_is_settled_by_en1_not_refused() {
    use fsl_analyze::analyze::{analyze_items, protocol_item, Engines, Settlement};
    use fsl_foundation::RequestIndex;

    let case = repaired_fill("eventually[0,2] holds(k.v >= 2)");
    let letters = CellV(at_least_two);
    let item = protocol_item(RequestIndex::new(0), case.protocol_item("Fill", &letters))
        .expect("a clause run");
    let record = analyze_items(
        &Engines::fsl().expect("the linked engines register"),
        vec![item],
        &Cancel::new(),
    )
    .remove(0);
    assert!(
        matches!(record.settlement, Settlement::Terminal(_)),
        "{:?}",
        record.settlement
    );
}

/// The claim's one atom, a predicate of the bound cell's `v`.
pub(crate) struct CellV(pub(crate) fn(i64) -> bool);

impl ModelLetters for CellV {
    crate::units::unguarded!();

    fn reads(&self, _atom: FormulaNodeId) -> crate::units::Reads {
        crate::units::unread()
    }

    fn atom(
        &self,
        _atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        Ok(if (self.0)(v(state, &instance.key)) {
            AtomReading::True
        } else {
            AtomReading::False
        })
    }
}

/// One behaviour of a protocol system: its states and the steps between
/// them, read by the evaluator as a finite trace.
struct Behaviour {
    states: Vec<ModelSimState>,
    steps: Vec<ProtocolTransition>,
    holds: fn(i64) -> bool,
    /// Whether the letter reports the steps' own counts, or counts every
    /// step as a model subject does.
    by_kind: bool,
}

impl Behaviour {
    /// The behaviour of `system` from its initial state taking, at each
    /// state, the step `pick` chooses among those enabled there.
    fn follow(
        system: &ProtocolSystem<'_, Sc>,
        holds: fn(i64) -> bool,
        pick: impl Fn(&[(ProtocolTransition, usize)]) -> usize,
    ) -> Self {
        let reached = reachable(system);
        let atoms = |reached: &Reached| system.atom_state(&reached.state).into_owned();
        let mut at = 0;
        let mut states = vec![atoms(&reached[0])];
        let mut steps = Vec::new();
        while !reached[at].steps.is_empty() {
            let (transition, next) = &reached[at].steps[pick(&reached[at].steps)];
            steps.push(transition.clone());
            states.push(atoms(&reached[*next]));
            at = *next;
        }
        Self {
            states,
            steps,
            holds,
            by_kind: true,
        }
    }

    fn at(&self, clause: &CheckedTemporalClause, position: u64) -> TemporalOutcome {
        let len = u64::try_from(self.states.len()).expect("a small trace");
        evaluate_at(
            clause,
            TemporalTrace::Finite { len },
            TemporalPosition::new(position),
            &mut self.letters(),
            &mut Meter::new(UNLIMITED),
        )
        .expect("the trace is readable")
    }

    fn letters(&self) -> Letters<'_> {
        Letters(self)
    }

    /// The positions after the first at which a counted step was taken.
    fn counted(&self) -> Vec<bool> {
        self.steps.iter().map(ProtocolTransition::counts).collect()
    }
}

struct Letters<'a>(&'a Behaviour);

impl TemporalLetters for Letters<'_> {
    fn atom(
        &mut self,
        _atom: FormulaNodeId,
        position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        let index = usize::try_from(position.index()).expect("a small position");
        Ok(if (self.0.holds)(v(&self.0.states[index], "c")) {
            AtomReading::True
        } else {
            AtomReading::False
        })
    }

    fn activation(
        &mut self,
        _position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        panic!("these claims have no `when` condition");
    }

    fn counts_step(&self, position: TemporalPosition) -> bool {
        let step = usize::try_from(position.index()).expect("a small position") - 1;
        !self.0.by_kind || self.0.steps[step].counts()
    }
}

fn violated(position: u64) -> TemporalOutcome {
    TemporalOutcome::Violated {
        at: TemporalPosition::new(position),
    }
}

/// The step whose node ends with `suffix`, else the first.
fn preferring(suffix: &'static str) -> impl Fn(&[(ProtocolTransition, usize)]) -> usize {
    move |steps| {
        steps
            .iter()
            .position(|(step, _)| step.node().is_some_and(|node| node.ends_with(suffix)))
            .unwrap_or(0)
    }
}

fn is_three(value: i64) -> bool {
    value == 3
}

fn at_least_two(value: i64) -> bool {
    value >= 2
}

fn is_zero(value: i64) -> bool {
    value == 0
}

fn repaired_fill(formula: &str) -> Case {
    Case::new(&(fill_unit("self.v <= 1", "") + &claim(formula)))
}

/// FR-213-AC-1 (TC-658): on the behaviour of the repaired `Fill` that takes
/// `B` before `A`, positions 1 (`fork`), 4 (`join`) and 5 (`finish`) carry
/// `protocol` anchors and positions 2 and 3 the anchors of `setTwo` and
/// `inc`; the counted steps are `attempt(B)` and `attempt(A)`, so
/// `eventually[0,2] holds(k.v = 3)` holds at position 0 and
/// `eventually[0,1] holds(k.v = 3)` does not. Read as a model subject reads
/// it, with every step counted, `k.v = 3` is three steps away and
/// `eventually[0,2]` fails.
#[trace("TC-658", "FR-213-AC-1")]
#[test]
fn counted_steps_decide_the_interval_over_the_repaired_fill() {
    let two = repaired_fill("eventually[0,2] holds(k.v = 3)");
    let subject = two.subject("Fill");
    let system = sc_system(&subject);
    let mut behaviour = Behaviour::follow(&system, is_three, preferring("/B"));
    let anchors: Vec<PositionAnchor<'_>> = behaviour.steps.iter().map(|s| s.anchor()).collect();
    assert_eq!(anchors.len(), 5, "{anchors:?}");
    for position in [0, 3, 4] {
        assert!(
            matches!(anchors[position], PositionAnchor::Protocol { .. }),
            "step {position}: {:?}",
            anchors[position]
        );
    }
    assert_eq!(
        [
            behaviour.steps[0].kind(),
            behaviour.steps[3].kind(),
            behaviour.steps[4].kind()
        ],
        [StepKind::Fork, StepKind::Join, StepKind::Finish]
    );
    for (position, operation) in [(1, "setTwo"), (2, "inc")] {
        assert!(
            matches!(anchors[position], PositionAnchor::Operation(name) if name.contains(operation)),
            "step {position}: {:?}",
            anchors[position]
        );
    }
    assert_eq!(behaviour.counted(), [false, true, true, false, false]);
    assert_eq!(behaviour.at(two.clause(), 0), TemporalOutcome::Holds);
    let one = repaired_fill("eventually[0,1] holds(k.v = 3)");
    assert_eq!(behaviour.at(one.clause(), 0), violated(0));
    behaviour.by_kind = false;
    assert_eq!(behaviour.at(two.clause(), 0), violated(0));
}

/// FR-213-AC-2 (TC-658): over §7.1's `Pay`, on the failure path
/// `eventually[0,2] holds(k.bal = 0)` holds at position 2 and fails at
/// position 1, whose distance to the refund is 3 with `cend` and `finish`
/// uncounted; on the success path it fails at every position from 1.
#[trace("TC-658", "FR-213-AC-2")]
#[test]
fn counted_steps_decide_the_interval_over_the_compensation_paths() {
    let case = Case::new(&(pay("true") + &claim("eventually[0,2] holds(k.v = 0)")));
    let subject = case.subject("Pay");
    let system = sc_system(&subject);
    let protocol = &system;
    let failing = |value: bool| {
        move |steps: &[(ProtocolTransition, usize)]| {
            let event = |step: &ProtocolTransition| {
                let key = protocol.transition_key(step).expect("an identity");
                let identity: serde_json::Value = serde_json::from_slice(&key).expect("JSON");
                step.kind() == StepKind::Event && identity["record"]["value"] == value
            };
            steps
                .iter()
                .position(|(step, _)| {
                    event(step) && step.node().is_some_and(|node| node.ends_with("Fail"))
                })
                .unwrap_or(0)
        }
    };
    let failure = Behaviour::follow(&system, is_zero, failing(true));
    assert_eq!(
        failure.steps.iter().map(|s| s.kind()).collect::<Vec<_>>(),
        [
            StepKind::Attempt,
            StepKind::Event,
            StepKind::Event,
            StepKind::CAttempt,
            StepKind::CEnd,
            StepKind::Finish
        ]
    );
    assert_eq!(
        failure.counted(),
        [true, true, true, true, false, false],
        "the refund is the fourth counted step"
    );
    assert_eq!(failure.at(case.clause(), 2), TemporalOutcome::Holds);
    assert_eq!(failure.at(case.clause(), 1), violated(1));
    let success = Behaviour::follow(&system, is_zero, failing(false));
    for position in 1..u64::try_from(success.states.len()).expect("a small trace") {
        assert_eq!(success.at(case.clause(), position), violated(position));
    }
}

/// FR-213-AC-3 (TC-658): EN-1 reproduces AC-1. Over the repaired `Fill`,
/// `eventually[0,2] holds(k.v >= 2)` under event-position false-extension
/// settles `proved`, its closure certificate checked by the core (FR-338);
/// `eventually[0,1] holds(k.v = 3)` settles `refuted` with a prefix on which
/// no position at counted distance 0 or 1 from position 0 has `k.v = 3`, and
/// that prefix replays.
#[trace("TC-658", "FR-213-AC-3")]
#[test]
fn the_model_checker_counts_steps_over_the_repaired_fill() {
    let proved = repaired_fill("eventually[0,2] holds(k.v >= 2)");
    let letters = CellV(at_least_two);
    let outcome = proved.check("Fill", &letters);
    assert!(
        matches!(outcome, ModelCheckOutcome::Holds { .. }),
        "{outcome:?}"
    );
    assert_eq!(
        proved.label("Fill", &letters, &outcome),
        ResultLabel::Proved
    );

    let refuted = repaired_fill("eventually[0,1] holds(k.v = 3)");
    let letters = CellV(|value| value == 3);
    let outcome = refuted.check("Fill", &letters);
    let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
        panic!("{outcome:?}");
    };
    let subject = refuted.subject("Fill");
    let system = sc_system(&subject);
    let mut state = system.initial().remove(0);
    let (mut distance, mut values) = (0, vec![(0, v(&system.atom_state(&state), "c"))]);
    for step in &counterexample.prefix {
        let next = system
            .successors(&state)
            .expect("expands")
            .successors
            .into_iter()
            .find(|(transition, _)| *transition == step.transition)
            .expect("the step is enabled")
            .1;
        distance += i64::from(step.transition.counts());
        values.push((distance, v(&system.atom_state(&next), "c")));
        state = next;
    }
    assert!(
        values
            .iter()
            .all(|(distance, value)| *distance > 1 || *value != 3),
        "{values:?}"
    );
    assert_eq!(values.last().map(|(distance, _)| *distance), Some(2));
    assert_eq!(
        refuted.label("Fill", &letters, &outcome),
        ResultLabel::Refuted
    );
}

/// FR-213-AC-3 (TC-658), the model-subject half: over the `Fill` model
/// subject every step counts (ADR-018's reading), so
/// `eventually[0,1] holds(k.v = 3)` is refuted by the one step that leaves
/// the last position of its window, and `eventually[0,2] holds(k.v >= 2)` by
/// two steps (`touch` leaves `k.v` at 0), not by the protocol's counted steps.
#[trace("TC-658", "FR-213-AC-3")]
#[test]
fn a_model_subject_counts_every_step() {
    for (formula, holds, steps) in [
        (
            "eventually[0,1] holds(k.v = 3)",
            is_three as fn(i64) -> bool,
            1,
        ),
        ("eventually[0,2] holds(k.v >= 2)", at_least_two, 2),
    ] {
        let case = repaired_fill(formula);
        let letters = CellV(holds);
        let temporal_clock_selections = crate::fill::tcb_01_event_position_selection(&case.checked);
        let outcome = check_model(
            ModelCheckRequest {
                temporal_clock_selections: &temporal_clock_selections,
                subject: model_subject(
                    &case.checked,
                    &case.documents,
                    case.initial.clone(),
                    &["c"],
                ),
                item: ModelCheckItem::Clause {
                    clause: case.clause(),
                    letters: &letters,
                },
                max_depth: None,
                limits: ModelCheckLimits::default(),
                reductions: Default::default(),
            },
            &Cancel::new(),
        )
        .expect("the model subject checks");
        let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
            panic!("{formula}: {outcome:?}");
        };
        assert_eq!(counterexample.prefix.len(), steps, "{formula}");
    }
}

/// FR-213-AC-2 (TC-658), a vector that tells counted steps from positions:
/// on the failure path of `Pay`, `cend` and `finish` are uncounted and the
/// false extension's stutter is the first counted step past them, so
/// `always[0,1] holds(k.bal = 0)` is violated at position 4 (the refund's
/// post-state), though it holds there if every step counts. The spec's own
/// AC-2 vector gives the same verdicts either way (QSL-396).
#[trace("TC-658", "FR-213-AC-2")]
#[test]
fn an_uncounted_cend_and_finish_end_the_window_after_the_refund() {
    let case = Case::new(&(pay("true") + &claim("always[0,1] holds(k.v = 0)")));
    let subject = case.subject("Pay");
    let system = sc_system(&subject);
    let protocol = &system;
    let failed = |steps: &[(ProtocolTransition, usize)]| {
        steps
            .iter()
            .position(|(step, _)| {
                let key = protocol.transition_key(step).expect("an identity");
                let identity: serde_json::Value = serde_json::from_slice(&key).expect("JSON");
                step.kind() == StepKind::Event
                    && step.node().is_some_and(|node| node.ends_with("Fail"))
                    && identity["record"]["value"] == true
            })
            .unwrap_or(0)
    };
    let mut behaviour = Behaviour::follow(&system, is_zero, failed);
    assert_eq!(behaviour.at(case.clause(), 4), violated(4));
    behaviour.by_kind = false;
    assert_eq!(behaviour.at(case.clause(), 4), TemporalOutcome::Holds);
}

/// FR-338 distance-history forms (FR-213, TC-658): a past operator with an
/// interval measures its window backwards in counted steps. On the repaired
/// `Fill` taking `B` before `A`, `k.v` is 0, 0 (`fork`), 2 (`setTwo`), 3
/// (`inc`), 3 (`join`), 3 (`finish`): the one counted step back from `join`
/// and from `finish` is `setTwo`'s position, where `k.v` is 2, so
/// `always[0,5] (holds(k.v = 3) implies not once[1,1] holds(k.v = 3))` holds
/// at position 0. Counting every step, `join`'s one step back is `inc`'s
/// position, where `k.v = 3`, and the claim fails. EN-1 decides the claim on
/// the counted monitor over every interleaving, and refutes the claim with
/// `not` removed on a prefix that replays.
#[trace("TC-658")]
#[test]
fn a_past_window_is_measured_in_counted_steps() {
    let case = repaired_fill("always[0,5] (holds(k.v = 3) implies not once[1,1] holds(k.v = 3))");
    let subject = case.subject("Fill");
    let system = sc_system(&subject);
    let mut behaviour = Behaviour::follow(&system, is_three, preferring("/B"));
    assert_eq!(behaviour.counted(), [false, true, true, false, false]);
    assert_eq!(behaviour.at(case.clause(), 0), TemporalOutcome::Holds);
    behaviour.by_kind = false;
    assert!(matches!(
        behaviour.at(case.clause(), 0),
        TemporalOutcome::Violated { .. }
    ));

    let letters = CellV(is_three);
    let outcome = case.check("Fill", &letters);
    assert!(
        matches!(outcome, ModelCheckOutcome::Holds { .. }),
        "{outcome:?}"
    );

    let missed = repaired_fill("always[0,5] (holds(k.v = 3) implies once[1,1] holds(k.v = 3))");
    let outcome = missed.check("Fill", &letters);
    assert!(
        matches!(outcome, ModelCheckOutcome::Violated { .. }),
        "{outcome:?}"
    );
    assert_eq!(
        missed.label("Fill", &letters, &outcome),
        ResultLabel::Refuted
    );
}

/// The claim `eventually[0,2] holds(k.v = 1)` over a protocol that sends
/// on a channel whose delivery is `unknown`, so `duplicate` then `lose`
/// returns to the state it left, and then runs `A`, which makes `k.v` 1. The
/// product state of the cycle coalesces and the monitor never rejects.
pub(crate) fn lossy_channel(members: &str) -> Case {
    let members = format!(
        "channel ch from w to w carries Boolean ordering fifo by (m: Boolean) \
         {{ true }} delivery unknown capacity 2 overflow block; {members}"
    );
    let run = format!(
        "sequence Main {{ send S1 via ch as (s1: Boolean) {{ s1 }}; \
         {} }}",
        attempt("A", "inc", "IncPre, IncPost", "a", "true")
    );
    let unit = contracts("self.v <= 1") + &protocol("P", &members, &run, "d");
    Case::new(&(unit + &claim("eventually[0,2] holds(k.v = 1)")))
}

/// ADR-027 PB-3 (FR-338): a cycle of uncounted steps keeps an interval that
/// needs a counted step open, so it is not met on that lasso, but the lasso
/// is a behaviour only when it is fair. With the default scheduler
/// constraints the thread at `A` can still move, `A` is enabled at every
/// state of the `duplicate`/`lose` cycle and never taken, so the cycle is
/// unfair, every fair behaviour takes `A`, and the claim holds: EN-1 settles
/// `proved`, the core's closure checker having run the same search.
#[trace("TC-658", "FR-338-AC-6")]
#[test]
fn an_uncounted_cycle_that_a_thread_can_leave_is_not_a_fair_behaviour() {
    let case = lossy_channel("");
    let letters = CellV(|value| value == 1);
    let outcome = case.check("P", &letters);
    assert!(
        matches!(outcome, ModelCheckOutcome::Holds { .. }),
        "{:?}",
        outcome.stats()
    );
    assert_eq!(case.label("P", &letters, &outcome), ResultLabel::Proved);
}

/// ADR-027 PB-3 (FR-338): under `scheduling adversarial` the same cycle is a
/// fair behaviour, and it is the counterexample: the lasso whose stem is the
/// `send` and whose loop is `duplicate` then `lose`, the thread resting
/// before `A` throughout. The claim is refuted, and the lasso replays, read in
/// counted steps with a loop that has none.
#[trace("TC-658", "FR-338-AC-6")]
#[test]
fn an_uncounted_cycle_that_keeps_an_interval_open_refutes_it_under_adversarial_scheduling() {
    let case = lossy_channel("scheduling adversarial;");
    let letters = CellV(|value| value == 1);
    let outcome = case.check("P", &letters);
    let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
        panic!("{:?}", outcome.stats());
    };
    let lasso = counterexample.lasso.as_ref().expect("a lasso");
    assert!(!lasso.terminal_stutter);
    assert!(counterexample.fairness.is_empty());
    let (stem, cycle) = counterexample.prefix.split_at(lasso.entry);
    assert_eq!(
        stem.iter()
            .map(|step| step.transition.kind())
            .collect::<Vec<_>>(),
        [StepKind::Send]
    );
    assert_eq!(
        cycle
            .iter()
            .map(|step| step.transition.kind())
            .collect::<Vec<_>>(),
        [StepKind::Duplicate, StepKind::Lose]
    );
    assert!(cycle.iter().all(|step| !step.transition.counts()));
    assert_eq!(case.label("P", &letters, &outcome), ResultLabel::Refuted);
}

/// Offer `certificate` to the core checker as the proof of `case`'s claim.
fn closure_check(
    case: &Case,
    letters: &dyn ModelLetters,
    certificate: &ProofCertificate,
) -> TemporalCertificateCheck {
    let ProofCertificate::Closure(certificate) = certificate else {
        panic!("a safety clause holds with a closure: {certificate:?}");
    };
    closure_check_over(case, "P", letters, certificate)
}

/// [`closure_check`] over `protocol`, for a closure `certificate`.
pub(crate) fn closure_check_over(
    case: &Case,
    protocol: &str,
    letters: &dyn ModelLetters,
    certificate: &ClosureCertificate,
) -> TemporalCertificateCheck {
    closure_check_with(case, protocol, letters, certificate, &ModelCheckLimits::default())
}

/// [`closure_check_over`] under `limits`.
fn closure_check_with(
    case: &Case,
    protocol: &str,
    letters: &dyn ModelLetters,
    certificate: &ClosureCertificate,
    limits: &ModelCheckLimits,
) -> TemporalCertificateCheck {
    check_protocol_closure(
        &ProtocolCertificateRequest {
            temporal_clock_selections: &case.temporal_clock_selections,
            subject: case.subject(protocol),
            item: ModelCheckItem::Clause {
                clause: case.clause(),
                letters,
            },
            max_store_buffer: None,
            max_messages: None,
            max_live_instances: None,
        },
        certificate,
        limits,
        &Cancel::new(),
    )
    .expect("the request is checked")
}

/// Whether `state` is a bounded monitor state that still owes its interval.
fn owes_interval(state: &ProductStateRef) -> bool {
    String::from_utf8_lossy(state.automaton_state.as_bytes()).contains("\"bounded\"")
}

/// FR-338 "Protocol non-progress acceptance": a closure that reaches the same
/// product states is no proof when the cycle of uncounted steps is a fair
/// behaviour. The certificate the default package's proof carries, offered
/// for the adversarial package's claim over the same states, is rejected
/// `WitnessFails` at the first product state of the cycle in product-state
/// order, not at an earlier state that owes the interval but is not on the
/// cycle; offered for the default package it is accepted, the thread at `A`
/// excluding the cycle.
#[trace("TC-658", "FR-338-AC-6")]
#[test]
fn a_closure_with_a_fair_uncounted_cycle_is_rejected_as_a_proof() {
    let fair = lossy_channel("");
    let letters = CellV(|value| value == 1);
    let ModelCheckOutcome::Holds { certificate, .. } = fair.check("P", &letters) else {
        panic!("the default scheduler excludes the cycle");
    };
    assert_eq!(
        closure_check(&fair, &letters, &certificate),
        TemporalCertificateCheck::Accepted
    );
    let adversarial = lossy_channel("scheduling adversarial;");
    let TemporalCertificateCheck::Rejected(CertificateRejection {
        rule: CertificateRule::WitnessFails,
        at: CertificateLocus::ProductState(at),
    }) = closure_check(&adversarial, &letters, &certificate)
    else {
        panic!("the fair cycle is a witness");
    };
    // The refutation's lasso names the cycle: its stem is the `send` and its
    // loop the `duplicate` and `lose` that return to the state after it.
    let ModelCheckOutcome::Violated { counterexample, .. } = adversarial.check("P", &letters)
    else {
        panic!("the adversarial scheduler admits the cycle");
    };
    let cycle: Vec<_> = counterexample
        .prefix
        .iter()
        .map(|step| step.post_state.clone())
        .collect();
    let ProofCertificate::Closure(certificate) = &certificate else {
        panic!("a safety clause holds with a closure: {certificate:?}");
    };
    let owing: Vec<&ProductStateRef> = certificate
        .states
        .iter()
        .filter(|s| owes_interval(s))
        .collect();
    // The initial product state owes the interval too, and is off the cycle.
    assert!(owing
        .iter()
        .any(|state| !cycle.contains(&state.model_state)));
    let expected = owing
        .iter()
        .filter(|state| cycle.contains(&state.model_state))
        .min()
        .copied()
        .expect("a state of the cycle");
    assert_eq!(&at, expected);
}

/// A protocol `P` over `k` whose `run` is `run`, with `members` before it
/// and the claim `eventually[0,2] holds(k.v = 1)`.
fn counted_case(members: &str, run: &str) -> Case {
    let unit = contracts("self.v <= 1") + &protocol("P", members, run, "d");
    Case::new(&(unit + &claim("eventually[0,2] holds(k.v = 1)")))
}

/// Settle `case`'s claim `proved` and have the core accept its certificate.
fn assert_proved_and_accepted(case: &Case) {
    let letters = CellV(|value| value == 1);
    let outcome = case.check("P", &letters);
    let ModelCheckOutcome::Holds { certificate, .. } = &outcome else {
        panic!("{:?}", outcome.stats());
    };
    assert_eq!(
        closure_check(case, &letters, certificate),
        TemporalCertificateCheck::Accepted
    );
    assert_eq!(case.label("P", &letters, &outcome), ResultLabel::Proved);
}

/// FR-338-AC-6, the paired oracle that "prevents rejecting every pending
/// clock indiscriminately": under `scheduling adversarial` a zero-counted
/// cycle reached after the debt is met is no non-progress, since the
/// monitor has accepted. `A` runs first, then a `send` whose `duplicate` and
/// `lose` cycle is fair and never ends.
#[trace("TC-658", "FR-338-AC-6")]
#[test]
fn an_uncounted_cycle_after_the_debt_is_met_is_not_a_refutation() {
    let members = "channel ch from w to w carries Boolean ordering fifo by (m: Boolean) \
                   { true } delivery unknown capacity 2 overflow block; scheduling adversarial;";
    let run = format!(
        "sequence Main {{ {} send S1 via ch as (s1: Boolean) {{ s1 }}; }}",
        attempt("A", "inc", "IncPre, IncPost", "a", "true")
    );
    assert_proved_and_accepted(&counted_case(members, &run));
}

/// FR-338-AC-6, the other paired oracle: a cycle that takes a counted step
/// passes no non-progress check. A `repeat` of `touch` attempts never ends
/// and each pass counts, so after `A` met the debt the claim is proved under
/// `scheduling adversarial`; with `A` absent the same cycle exhausts the
/// interval in counted steps, and the refutation is the finite bad prefix of
/// that expiry, not a zero-progress lasso.
#[trace("TC-658", "FR-338-AC-6")]
#[test]
fn a_cycle_that_takes_a_counted_step_is_not_a_zero_progress_refutation() {
    let looping = format!(
        "repeat Loop by w visible () while {{ true }} {}",
        attempt("T", "touch", "TouchPre", "t", "true")
    );
    let met = format!(
        "sequence Main {{ {} {looping} }}",
        attempt("A", "inc", "IncPre, IncPost", "a", "true")
    );
    assert_proved_and_accepted(&counted_case("scheduling adversarial;", &met));

    let case = counted_case(
        "scheduling adversarial;",
        &format!("sequence Main {{ {looping} }}"),
    );
    let letters = CellV(|value| value == 1);
    let outcome = case.check("P", &letters);
    let ModelCheckOutcome::Violated { counterexample, .. } = &outcome else {
        panic!("{:?}", outcome.stats());
    };
    assert!(counterexample.lasso.is_none(), "{counterexample:?}");
    assert!(counterexample
        .prefix
        .iter()
        .all(|step| step.transition.counts()));
    assert_eq!(case.label("P", &letters, &outcome), ResultLabel::Refuted);
}

/// ADR-027 PB-3: an operation, an event (an `activate` included), a `send`
/// and a `receive` count; every other step kind, `spawn`, `retire`, `fence`
/// and a memory step included, moves the position and not the distance.
#[trace("TC-658")]
#[test]
fn only_operations_events_sends_and_receives_count() {
    use StepKind::*;
    for kind in [Attempt, CAttempt, Event, Activate, Send, Receive] {
        assert!(kind.counts(), "{kind:?}");
    }
    for kind in [
        Duplicate, Lose, Fork, Join, Timeout, Finish, Memory, Fence, CEnd, Spawn, Retire,
    ] {
        assert!(!kind.counts(), "{kind:?}");
    }
}

/// FR-338 "Guarded `on each` activation": a protocol subject's steps do not
/// all count, so the bounded `on each` translation, whose `Idle` key
/// `[[],[],[],[]]` is no state there, is refused for it. A certificate that
/// carries the translation is never accepted: the checker refuses the item
/// before it reads one state.
#[trace("TC-525", "FR-338-AC-10")]
#[test]
fn a_protocol_subject_certificate_with_the_bounded_each_translation_is_refused() {
    let case = Case::new(
        &(fill_unit("self.v <= 1", "")
            + "temporal Claim using tb over (k: M::Cell) clock \"model-steps\" \
               on each (q: M::Cell) { always[0,1] holds(k.v >= 0) }\n"),
    );
    let letters = CellV(|_| true);
    let request = ProtocolCertificateRequest {
        temporal_clock_selections: &case.temporal_clock_selections,
        subject: case.subject("Fill"),
        item: ModelCheckItem::Clause {
            clause: case.clause(),
            letters: &letters,
        },
        max_store_buffer: None,
        max_messages: None,
        max_live_instances: None,
    };
    let refused = check_protocol_closure(
        &request,
        &fsl_replay::ClosureCertificate { states: Vec::new() },
        &ModelCheckLimits::default(),
        &Cancel::new(),
    );
    assert!(
        matches!(
            refused,
            Err(fsl_replay::ClosureRefusal::Item(
                fsl_replay::ItemRefusal::Translation(_)
            ))
        ),
        "{refused:?}"
    );
}

/// FR-338-AC-31: a TP-3 clause over a protocol subject carries a certificate
/// the core accepts. `always (p implies eventually[0,5] p)` holds over the
/// repaired `Fill` with a non-empty closure certificate, and FR-127 settles it
/// `proved`, `Certified`; the empty certificate is no proof of it.
#[trace("TC-525", "FR-338-AC-31")]
#[test]
fn a_tp3_proof_over_a_protocol_subject_settles_certified() {
    let case = Case::new(
        &(fill_unit("self.v <= 1", "")
            + &tp3(
                "on origin",
                "always (holds(k.v <= 3) implies eventually[0,5] holds(k.v <= 3))",
            )),
    );
    let letters = CellV(|_| true);
    let outcome = case.check("Fill", &letters);
    let ModelCheckOutcome::Holds { certificate, .. } = &outcome else {
        panic!("{:?}", outcome.stats());
    };
    let ProofCertificate::Closure(certificate) = certificate else {
        panic!("a safety clause holds with a closure: {certificate:?}");
    };
    assert!(!certificate.states.is_empty(), "{certificate:?}");
    assert_eq!(
        closure_check_over(&case, "Fill", &letters, certificate),
        TemporalCertificateCheck::Accepted
    );
    assert_eq!(
        case.record("Fill", &letters, &outcome).value,
        TerminalValue::Proved {
            basis: ProofBasis::Exhaustive,
            certification: Certification::Certified,
        }
    );
    let emptied = ClosureCertificate { states: Vec::new() };
    assert!(matches!(
        closure_check_over(&case, "Fill", &letters, &emptied),
        TemporalCertificateCheck::Rejected(CertificateRejection {
            rule: CertificateRule::InitialMissing,
            ..
        })
    ));
}

/// A `Claim` over `k: M::Cell` under the infinite-trace profile, with
/// `activation` (`on origin`, `on each (q: M::Cell)`).
pub(crate) fn tp3(activation: &str, formula: &str) -> String {
    format!(
        "temporal Claim using t0 over (k: M::Cell) clock \"model-steps\" {activation} \
         {{ {formula} }}\n"
    )
}

fn at_least_one(value: i64) -> bool {
    value >= 1
}

/// Letters whose atom `i` of the clause (in the clause's atom order) reads
/// `reads[i]` of the bound cell's `v`.
pub(crate) struct CellAtoms {
    atoms: Vec<FormulaNodeId>,
    reads: Vec<fn(i64) -> bool>,
}

impl CellAtoms {
    pub(crate) fn of(clause: &CheckedTemporalClause, reads: &[fn(i64) -> bool]) -> Self {
        let automaton =
            fsl_eval::automaton::translate(clause, 256).expect("the clause translates");
        assert_eq!(automaton.atoms().len(), reads.len(), "one reading per atom");
        Self {
            atoms: automaton.atoms().to_vec(),
            reads: reads.to_vec(),
        }
    }

    fn truth(&self, atom: FormulaNodeId, value: i64) -> bool {
        let index = self
            .atoms
            .iter()
            .position(|known| *known == atom)
            .expect("the clause's own atom");
        (self.reads[index])(value)
    }
}

impl ModelLetters for CellAtoms {
    crate::units::unguarded!();

    fn reads(&self, _atom: FormulaNodeId) -> crate::units::Reads {
        crate::units::unread()
    }

    fn atom(
        &self,
        atom: FormulaNodeId,
        instance: &ModelInstance,
        state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        Ok(if self.truth(atom, v(state, &instance.key)) {
            AtomReading::True
        } else {
            AtomReading::False
        })
    }
}

/// The JSON of the key `state` carries.
pub(crate) fn key_json(state: &ProductStateRef) -> serde_json::Value {
    serde_json::from_slice(state.automaton_state.as_bytes()).expect("engine JSON key")
}

/// The distances of every `distance-history` bucket the key carries, sorted.
fn bucket_distances(state: &ProductStateRef) -> Vec<u64> {
    let key = key_json(state);
    let mut distances = Vec::new();
    for branch in key[2][1].as_array().expect("safety branches") {
        for memory in branch[1].as_array().expect("a branch's past memory") {
            if memory[1] != "distance-history" {
                continue;
            }
            for bucket in memory[2].as_array().expect("buckets") {
                let distance = bucket[0].as_str().expect("a decimal distance");
                distances.push(distance.parse().expect("a small distance"));
            }
        }
    }
    distances.sort_unstable();
    distances
}

/// The entries of `certificate` whose model state is `state`'s, under the
/// cycle keys the checker writes.
fn entries_at(
    certificate: &ClosureCertificate,
    system: &ProtocolSystem<'_, Sc>,
    state: &ProtocolState<Sc>,
) -> Vec<ProductStateRef> {
    let (key, _) = system.state_key(state).expect("encodes");
    let key = fsl_eval::simulation::StateKey::from_canonical_bytes(key);
    certificate
        .states
        .iter()
        .filter(|entry| entry.model_state == key)
        .cloned()
        .collect()
}

/// The initial state of `system` and the one `kind` step from `state`.
fn after(
    system: &ProtocolSystem<'_, Sc>,
    state: &ProtocolState<Sc>,
    kind: StepKind,
) -> ProtocolState<Sc> {
    system
        .successors(state)
        .expect("expansion")
        .successors
        .into_iter()
        .find(|(step, _)| step.kind() == kind)
        .unwrap_or_else(|| panic!("a {kind:?} step is enabled"))
        .1
}

/// The product states of `case`'s TP-3 clause over `protocol` that a
/// breadth-first walk of the protocol system reaches before the clause's
/// monitor rejects, each keyed under the clause's own context, and the
/// rejecting key `["safety",[]]`: a certificate that holds every state the
/// walk of a violated clause reached, which the checker rejects `BadState`.
fn walked_before_violation(
    case: &Case,
    protocol: &str,
    reads: &[fn(i64) -> bool],
) -> (ClosureCertificate, fsl_eval::automaton::AutomatonStateKey) {
    use fsl_eval::automaton::{
        translate_counted, Atom, ContextBinding, Letter, Monitor, SafetyStates, StateContext,
        SubjectKind,
    };
    let subject = case.subject(protocol);
    let system = sc_system(&subject).with_cycle_keys(std::collections::BTreeSet::new());
    let clause = case.clause();
    let identity = ClauseIdentity::resolve(
        &case.checked,
        &case.documents.packages,
        clause,
        &case.temporal_clock_selections,
        &Cancel::new(),
        fsl_package::EmitLimits::fixture(),
    )
    .expect("the package declares the clause");
    let binding = vec![ContextBinding::instance(
        identity.over().expect("an over node"),
        "c",
    )];
    let context = StateContext::clause(clause, identity.occurrence().clone(), binding);
    let keys = SafetyStates::clause(clause, &context, SubjectKind::Protocol).expect("translates");
    let mut monitor = Monitor::bad_prefix(
        translate_counted(clause, 256).expect("translates"),
        256,
    )
    .expect("a safety automaton");
    let uncounted = monitor.automaton().uncounted();
    let letter = |state: &ProtocolState<Sc>, counted: bool| {
        let model = system.atom_state(state).into_owned();
        let mut letter = Letter::default();
        for (index, read) in reads.iter().enumerate() {
            if read(v(&model, "c")) {
                letter = letter.with(Atom::new(index).expect("an atom"));
            }
        }
        match uncounted {
            Some(uncounted) if !counted => letter.with(uncounted),
            _ => letter,
        }
    };
    let model_key = |state: &ProtocolState<Sc>| {
        let (key, _) = system.state_key(state).expect("encodes");
        fsl_eval::simulation::StateKey::from_canonical_bytes(key)
    };
    let rejecting = keys.key(&[]).expect("encodes");
    let start = monitor.initial().expect("initial state");
    let mut states = std::collections::BTreeSet::new();
    let mut seen = std::collections::BTreeSet::new();
    let mut queue = std::collections::VecDeque::new();
    let mut reach = |monitor: &mut Monitor<fsl_eval::automaton::PropertyAutomaton>,
                     state: ProtocolState<Sc>,
                     before: fsl_eval::automaton::StateId,
                     counted: bool,
                     queue: &mut std::collections::VecDeque<(
        ProtocolState<Sc>,
        fsl_eval::automaton::StateId,
    )>| {
        let next = monitor
            .step(before, letter(&state, counted))
            .expect("steps");
        if monitor.is_rejecting(next).expect("a known state") {
            return;
        }
        let configurations = monitor
            .configurations(next, keys.max_key_bytes())
            .expect("resolves");
        let at = ProductStateRef {
            model_state: model_key(&state),
            automaton_state: keys.key(&configurations).expect("encodes"),
        };
        if seen.insert(at.clone()) {
            states.insert(at);
            queue.push_back((state, next));
        }
    };
    for initial in system.initial() {
        reach(&mut monitor, initial, start, true, &mut queue);
    }
    while let Some((state, at)) = queue.pop_front() {
        let successors = system.successors(&state).expect("expansion").successors;
        if successors.is_empty() {
            reach(&mut monitor, state, at, true, &mut queue);
            continue;
        }
        for (transition, next) in successors {
            reach(&mut monitor, next, at, transition.counts(), &mut queue);
        }
    }
    (
        ClosureCertificate {
            states: states.into_iter().collect(),
        },
        rejecting,
    )
}

fn repaired_tp3(activation: &str, formula: &str) -> Case {
    Case::new(&(fill_unit("self.v <= 1", "") + &tp3(activation, formula)))
}

/// FR-338-AC-31 (past): over the repaired `Fill`, whose counted steps are
/// `attempt(B)` and `attempt(A)` and whose `fork`, `join` and `finish` are
/// uncounted, `always (holds(k.v = 0) implies not once[1,2] holds(k.v >= 1))`
/// returns `Holds` with a certificate whose keys carry a `distance-history`
/// entry for `once[1,2]` in the past memory, accepted by `check_protocol_closure`,
/// so FR-127 settles `proved`, `Certified`. The key after `fork` is the key
/// before it (an uncounted step keeps every bucket distance), the bucket
/// distances the keys carry are `[]` before an attempt, `[0]` after one and
/// `[0, 1]` after two (a counted step raises each by one and adds a
/// distance-zero bucket), and the certificate with the key after the second
/// `attempt` replaced by the key after the first is rejected `SuccessorMissing`
/// at the reached state. `max_states` one below the closure's product states
/// stops `LimitReached{MaxStates, ..}`, settled `incomplete`.
#[trace("TC-525", "FR-338-AC-31")]
#[trace("TC-525", "FR-127-AC-1")]
#[test]
fn a_past_tp3_clause_over_the_repaired_fill_keys_distance_history() {
    let case = repaired_tp3(
        "on origin",
        "always (holds(k.v = 0) implies not once[1,2] holds(k.v >= 1))",
    );
    let letters = CellAtoms::of(case.clause(), &[is_zero, at_least_one]);
    let outcome = case.check("Fill", &letters);
    let ModelCheckOutcome::Holds {
        certificate: ProofCertificate::Closure(certificate),
        ..
    } = &outcome
    else {
        panic!("{:?}", outcome.stats());
    };
    assert_eq!(
        closure_check_over(&case, "Fill", &letters, certificate),
        TemporalCertificateCheck::Accepted
    );
    assert_eq!(
        case.record("Fill", &letters, &outcome).value,
        TerminalValue::Proved {
            basis: ProofBasis::Exhaustive,
            certification: Certification::Certified,
        }
    );
    assert!(certificate
        .states
        .iter()
        .any(|state| String::from_utf8_lossy(state.automaton_state.as_bytes())
            .contains("distance-history")));

    let distances: std::collections::BTreeSet<Vec<u64>> =
        certificate.states.iter().map(bucket_distances).collect();
    for expected in [vec![], vec![0], vec![0, 1]] {
        assert!(distances.contains(&expected), "{expected:?} in {distances:?}");
    }

    let subject = case.subject("Fill");
    let system = sc_system(&subject).with_cycle_keys(std::collections::BTreeSet::new());
    let origin = system.initial().remove(0);
    let forked = after(&system, &origin, StepKind::Fork);
    let at_origin = entries_at(certificate, &system, &origin);
    let at_fork = entries_at(certificate, &system, &forked);
    assert_eq!((at_origin.len(), at_fork.len()), (1, 1));
    assert_eq!(at_origin[0].automaton_state, at_fork[0].automaton_state);

    let one_attempt = certificate
        .states
        .iter()
        .find(|state| bucket_distances(state) == [0])
        .expect("a state after one attempt")
        .clone();
    let two_attempts = certificate
        .states
        .iter()
        .find(|state| bucket_distances(state) == [0, 1])
        .expect("a state after two attempts")
        .clone();
    let mut replaced = certificate.clone();
    for state in &mut replaced.states {
        if *state == two_attempts {
            state.automaton_state = one_attempt.automaton_state.clone();
        }
    }
    assert_eq!(
        closure_check_over(&case, "Fill", &letters, &replaced),
        TemporalCertificateCheck::Rejected(CertificateRejection {
            rule: CertificateRule::SuccessorMissing,
            at: CertificateLocus::ProductState(two_attempts),
        })
    );

    let mut limits = ModelCheckLimits::default();
    limits.limits.max_states = certificate.states.len() - 1;
    let value = u64::try_from(certificate.states.len() - 1).expect("a small closure");
    assert_eq!(
        closure_check_with(&case, "Fill", &letters, certificate, &limits),
        TemporalCertificateCheck::Stopped(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::MaxStates,
            value,
        })
    );
}

/// FR-338-AC-31 (future): over the repaired `Fill`, `always (holds(k.v = 0)
/// implies eventually[0,2] holds(k.v >= 2))` returns `Holds` whose key after
/// `fork` carries the `deadline` window `"2"` of its older activation, equal
/// to the initial key, accepted. The clause with `eventually[0,1] holds(k.v =
/// 3)` returns `Violated`, refuted through the production replay, and the
/// certificate a walk of it reached is rejected `BadState` under
/// `["safety",[]]`. The accepted certificate with every key's activation
/// replaced by `"each"` is rejected `ShapeMismatch`.
#[trace("TC-525", "FR-338-AC-31")]
#[test]
fn a_future_tp3_clause_over_the_repaired_fill_keeps_the_older_deadline() {
    let case = repaired_tp3(
        "on origin",
        "always (holds(k.v = 0) implies eventually[0,2] holds(k.v >= 2))",
    );
    let letters = CellAtoms::of(case.clause(), &[is_zero, at_least_two]);
    let outcome = case.check("Fill", &letters);
    let ModelCheckOutcome::Holds {
        certificate: ProofCertificate::Closure(certificate),
        ..
    } = &outcome
    else {
        panic!("{:?}", outcome.stats());
    };
    assert_eq!(
        closure_check_over(&case, "Fill", &letters, certificate),
        TemporalCertificateCheck::Accepted
    );
    assert_eq!(
        case.record("Fill", &letters, &outcome).value,
        TerminalValue::Proved {
            basis: ProofBasis::Exhaustive,
            certification: Certification::Certified,
        }
    );
    let subject = case.subject("Fill");
    let system = sc_system(&subject).with_cycle_keys(std::collections::BTreeSet::new());
    let origin = system.initial().remove(0);
    let forked = after(&system, &origin, StepKind::Fork);
    let at_origin = entries_at(certificate, &system, &origin);
    let at_fork = entries_at(certificate, &system, &forked);
    assert_eq!((at_origin.len(), at_fork.len()), (1, 1));
    assert_eq!(at_origin[0].automaton_state, at_fork[0].automaton_state);
    assert!(
        String::from_utf8_lossy(at_fork[0].automaton_state.as_bytes())
            .contains("\"deadline\",\"2\""),
    );

    let mut each = certificate.clone();
    for state in &mut each.states {
        let mut key = key_json(state);
        key[1][2] = serde_json::json!("each");
        state.automaton_state = fsl_eval::automaton::AutomatonStateKey::imported(
            serde_json::to_vec(&key).expect("array-only key"),
        );
    }
    let least = each.states.iter().min().cloned().expect("a state");
    assert_eq!(
        closure_check_over(&case, "Fill", &letters, &each),
        TemporalCertificateCheck::Rejected(CertificateRejection {
            rule: CertificateRule::ShapeMismatch,
            at: CertificateLocus::ProductState(least),
        })
    );

    let short = repaired_tp3(
        "on origin",
        "always (holds(k.v = 0) implies eventually[0,1] holds(k.v = 3))",
    );
    let reads: [fn(i64) -> bool; 2] = [is_zero, is_three];
    let letters = CellAtoms::of(short.clause(), &reads);
    let outcome = short.check("Fill", &letters);
    assert!(
        matches!(outcome, ModelCheckOutcome::Violated { .. }),
        "{:?}",
        outcome.stats()
    );
    assert_eq!(
        short.label("Fill", &letters, &outcome),
        ResultLabel::Refuted
    );
    let (walked, rejecting) = walked_before_violation(&short, "Fill", &reads);
    let TemporalCertificateCheck::Rejected(CertificateRejection {
        rule: CertificateRule::BadState,
        at: CertificateLocus::ProductState(at),
    }) = closure_check_over(&short, "Fill", &letters, &walked)
    else {
        panic!("the walked states end at a bad state");
    };
    assert_eq!(at.automaton_state, rejecting);
}
