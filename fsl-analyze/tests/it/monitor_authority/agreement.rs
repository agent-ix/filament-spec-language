// SPDX-License-Identifier: AGPL-3.0-or-later
//! T-064 (FR-861-AC-1, FR-861-AC-2): on the same cases, the answers the
//! evaluator reports for the partial-value order and for a completeness
//! revision equal the answers the `filament-observation` owner modules give,
//! and a disagreement is a failure naming the case and both answers, never an
//! answer one side is preferred for.

use filament_observation::authority::completeness::{
    check_revision, FactStatus, HeldFact, ListedContradiction, ListedFact, RetainedHolding, State,
};
use filament_observation::authority::{partial, Limits};
use filament_observation::Identity;
use fsl_eval::temporal::{
    ClockCover, Containment, ContradictedDimension, LedgerBinding, LedgerRefusal, ObservedPosition,
    ProgressAssertion, ProgressClosure, ProgressCompleteness, ProgressLedger, ProgressValue,
};
use ix_trace_rs::trace;

fn id(value: &str) -> Identity {
    Identity::new(value)
}

/// One case on which the evaluator and the owner each give an answer.
#[derive(Debug, Eq, PartialEq)]
struct Disagreement {
    case: String,
    evaluator: String,
    owner: String,
}

/// The one comparison every agreement test makes: equal answers pass, and any
/// other pair is reported with the case and both answers, whichever side is
/// the one the evaluator's report would follow.
fn agree(case: &str, evaluator: &str, owner: &str) -> Result<(), Disagreement> {
    if evaluator == owner {
        return Ok(());
    }
    Err(Disagreement {
        case: case.to_owned(),
        evaluator: evaluator.to_owned(),
        owner: owner.to_owned(),
    })
}

fn expect_agreement(results: Vec<Result<(), Disagreement>>) {
    let failures: Vec<_> = results.into_iter().filter_map(Result::err).collect();
    assert!(
        failures.is_empty(),
        "the evaluator and the owner disagree: {failures:?}"
    );
}

// --- Partial: the order of two uncertain event-time intervals (FR-861-AC-1).

/// The instants, in quarter ticks, a reading may lie at under `cover`
/// (FR-944): a brute-force oracle independent of both implementations.
fn instants(cover: ClockCover, reading: i128) -> std::ops::RangeInclusive<i128> {
    match cover {
        ClockCover::Exact => 4 * reading..=4 * reading,
        ClockCover::Counter { uncertainty } => {
            let u = i128::from(uncertainty);
            4 * (reading - u)..=4 * (reading + 1 + u) - 1
        }
        ClockCover::Bounded { uncertainty } => {
            let u = i128::from(uncertainty);
            4 * (reading - u)..=4 * (reading + u)
        }
    }
}

fn owner_interval(cover: ClockCover, reading: i128, shift: i128) -> partial::EventTimeInterval {
    let span = instants(cover, reading);
    partial::EventTimeInterval::new(
        id("clock:agreement"),
        id("1"),
        id("tick/4"),
        span.start() + shift,
        span.end() + shift,
        Limits::owner_max(),
    )
    .expect("an owner interval")
}

fn owner_before(cover: ClockCover, a: i128, b: i128) -> bool {
    owner_interval(cover, a, 0)
        .relation_to(&owner_interval(cover, b, 0), Limits::owner_max())
        .expect("an owner order")
        .definitely_before()
}

fn owner_within(cover: ClockCover, bound: i64, request: i128, ack: i128) -> Containment {
    let shifted = owner_interval(cover, request, 4 * i128::from(bound));
    let relation = owner_interval(cover, ack, 0)
        .relation_to(&shifted, Limits::owner_max())
        .expect("an owner order");
    if !relation.after() {
        Containment::True
    } else if !relation.before() && !relation.equal() {
        Containment::False
    } else {
        Containment::Indeterminate
    }
}

fn oracle_before(cover: ClockCover, a: i128, b: i128) -> bool {
    instants(cover, a).end() < instants(cover, b).start()
}

fn oracle_within(cover: ClockCover, bound: i64, request: i128, ack: i128) -> Containment {
    let limit = 4 * i128::from(bound);
    let (mut holds, mut fails) = (false, false);
    for r in instants(cover, request) {
        for a in instants(cover, ack) {
            if a - r <= limit {
                holds = true;
            } else {
                fails = true;
            }
        }
    }
    match (holds, fails) {
        (true, false) => Containment::True,
        (false, true) => Containment::False,
        _ => Containment::Indeterminate,
    }
}

#[test]
#[trace("TC-2442", "FR-861-AC-1")]
fn the_evaluator_orders_uncertain_intervals_as_the_owner_partial_module_does() {
    let mut results = Vec::new();
    // FR-501-AC-13's counter cover (`Counter { uncertainty: 1 }`, request at
    // tick 0, acknowledgements at 3001 and 3002) and the exact and bounded
    // covers over every pair of readings 0 to 6 and bounds 0 to 6.
    for cover in [
        ClockCover::Exact,
        ClockCover::Counter { uncertainty: 1 },
        ClockCover::Bounded { uncertainty: 1 },
    ] {
        for a in 0..=6_u64 {
            for b in 0..=6_u64 {
                let case = format!("{cover:?} definitely_before({a}, {b})");
                let evaluator = cover.definitely_before(a, b);
                results.push(agree(
                    &format!("{case} against the owner"),
                    &evaluator.to_string(),
                    &owner_before(cover, i128::from(a), i128::from(b)).to_string(),
                ));
                results.push(agree(
                    &format!("{case} against the instants"),
                    &evaluator.to_string(),
                    &oracle_before(cover, i128::from(a), i128::from(b)).to_string(),
                ));
                for bound in 0..=6_i64 {
                    let case = format!("{cover:?} within_bound({bound}, {a}, {b})");
                    let evaluator = cover.within_bound(bound, a, b);
                    results.push(agree(
                        &format!("{case} against the owner"),
                        &format!("{evaluator:?}"),
                        &format!("{:?}", owner_within(cover, bound, a.into(), b.into())),
                    ));
                    results.push(agree(
                        &format!("{case} against the instants"),
                        &format!("{evaluator:?}"),
                        &format!("{:?}", oracle_within(cover, bound, a.into(), b.into())),
                    ));
                }
            }
        }
    }
    let counter = ClockCover::Counter { uncertainty: 1 };
    for ack in [3001_u64, 3002] {
        results.push(agree(
            &format!("Counter {{ uncertainty: 1 }} within_bound(3000, 0, {ack})"),
            &format!("{:?}", counter.within_bound(3000, 0, ack)),
            &format!("{:?}", owner_within(counter, 3000, 0, i128::from(ack))),
        ));
    }
    expect_agreement(results);
}

// --- Completeness: accepting or refusing a revision (FR-861-AC-2).

/// The retained covered set as the owner's `check_revision` reads it, by
/// identity alone, written here independently of the ledger's own mapping.
struct Holding(Vec<ObservedPosition>);

impl RetainedHolding for Holding {
    type Key = str;

    fn held(&self, key: &str) -> Option<HeldFact<'_>> {
        self.0
            .iter()
            .find(|position| position.identity == key)
            .map(|position| HeldFact {
                status: FactStatus::Available,
                observation_identity: position.fingerprint.as_deref(),
            })
    }
}

fn position(label: &str, stamp: u64) -> ObservedPosition {
    ObservedPosition {
        identity: label.to_owned(),
        stamp,
        fingerprint: None,
    }
}

fn ledger_binding() -> LedgerBinding {
    LedgerBinding {
        package: "package:agreement".to_owned(),
        clause: filament_exact::NodeKey::new(
            "package:agreement",
            vec![filament_exact::PathSegment::Text("Clause".to_owned())],
            0,
        )
        .expect("a clause identity"),
        occurrence: 0,
        clock: "clock:agreement".to_owned(),
        profile: "finite-window".to_owned(),
    }
}

fn assertion(completeness: ProgressCompleteness) -> ProgressAssertion {
    ProgressAssertion {
        watermark: 100,
        completeness,
        scope: ProgressClosure::Open,
        execution: ProgressClosure::Open,
    }
}

fn retained_positions() -> Vec<ObservedPosition> {
    vec![position("a", 0), position("b", 50), position("c", 100)]
}

/// What the owner's `check_revision` answers for `asserted` over `positions`
/// after `Complete { through: 100 }` over the retained positions, as FR-944
/// "Completeness" maps the arguments: `accepted`, or the typed cause with
/// the value FR-502's contradiction carries for it.
fn owner_answer(asserted: ProgressCompleteness, positions: &[ObservedPosition]) -> String {
    let (successor_upper, successor_state) = match asserted {
        ProgressCompleteness::Complete { through } => (Some(i128::from(through)), State::Complete),
        ProgressCompleteness::Incomplete => (Some(100), State::Incomplete),
    };
    let listed: Vec<ListedFact<'_, str>> = positions
        .iter()
        .map(|position| ListedFact {
            key: position.identity.as_str(),
            status: FactStatus::Available,
            observation_identity: position.fingerprint.as_deref(),
            placed: i128::from(position.stamp),
        })
        .collect();
    match check_revision(
        Some(100),
        State::Complete,
        &Holding(retained_positions()),
        successor_upper,
        successor_state,
        &listed,
    ) {
        Ok(()) => "accepted".to_owned(),
        Err(ListedContradiction::BoundaryRegressed { .. }) => {
            format!("BoundaryRegressed: Completeness({asserted:?})")
        }
        Err(ListedContradiction::CompleteRegressed { .. }) => {
            format!("CompleteRegressed: Completeness({asserted:?})")
        }
        Err(ListedContradiction::UncoveredMember { listed }) => {
            format!("UncoveredMember: Position({:?})", positions[listed])
        }
        Err(ListedContradiction::MemberObservationChanged { listed, .. }) => format!(
            "MemberObservationChanged: Position({:?})",
            ObservedPosition {
                fingerprint: None,
                ..positions[listed].clone()
            }
        ),
    }
}

/// What the evaluator's ledger answers for the same request, the typed cause
/// read from the contradiction it reports.
fn ledger_answer(asserted: ProgressCompleteness, positions: &[ObservedPosition]) -> String {
    let binding = ledger_binding();
    let ledger = ProgressLedger::new()
        .admit(
            &binding,
            assertion(ProgressCompleteness::Complete { through: 100 }),
            &retained_positions(),
            1000,
        )
        .expect("the retained assertion");
    let Err(refusal) = ledger.admit(&binding, assertion(asserted), positions, 1000) else {
        return "accepted".to_owned();
    };
    let LedgerRefusal::Contradiction(contradiction) = refusal else {
        panic!("a contradiction: {refusal:?}");
    };
    assert_eq!(contradiction.dimension, ContradictedDimension::Completeness);
    assert_eq!(
        contradiction.retained,
        ProgressValue::Completeness(ProgressCompleteness::Complete { through: 100 })
    );
    match contradiction.asserted {
        ProgressValue::Completeness(value @ ProgressCompleteness::Complete { .. }) => {
            format!("BoundaryRegressed: Completeness({value:?})")
        }
        ProgressValue::Completeness(value @ ProgressCompleteness::Incomplete) => {
            format!("CompleteRegressed: Completeness({value:?})")
        }
        ProgressValue::Position(position) => {
            let held = retained_positions()
                .iter()
                .any(|held| held.identity == position.identity);
            let cause = if held {
                "MemberObservationChanged"
            } else {
                "UncoveredMember"
            };
            format!("{cause}: Position({position:?})", position = *position)
        }
        other => panic!("a completeness or position value: {other:?}"),
    }
}

/// FR-502-AC-2's cases with the answer FR-944 states for each.
fn revision_cases() -> Vec<(
    &'static str,
    ProgressCompleteness,
    Vec<ObservedPosition>,
    String,
)> {
    let held = retained_positions();
    let mut uncovered = held.clone();
    uncovered.push(position("d", 80));
    vec![
        (
            "Complete through 100",
            ProgressCompleteness::Complete { through: 100 },
            held.clone(),
            "accepted".to_owned(),
        ),
        (
            "Complete through 140",
            ProgressCompleteness::Complete { through: 140 },
            held.clone(),
            "accepted".to_owned(),
        ),
        (
            "Complete through 60",
            ProgressCompleteness::Complete { through: 60 },
            held.clone(),
            "BoundaryRegressed: Completeness(Complete { through: 60 })".to_owned(),
        ),
        (
            "Incomplete",
            ProgressCompleteness::Incomplete,
            held,
            "CompleteRegressed: Completeness(Incomplete)".to_owned(),
        ),
        (
            "an uncovered position stamped 80",
            ProgressCompleteness::Complete { through: 100 },
            uncovered,
            format!("UncoveredMember: Position({:?})", position("d", 80)),
        ),
    ]
}

#[test]
#[trace("TC-2442", "FR-861-AC-2")]
fn the_ledger_answers_a_completeness_revision_as_the_owners_check_does() {
    let mut results = Vec::new();
    for (case, asserted, positions, expected) in revision_cases() {
        let owner = owner_answer(asserted, &positions);
        let evaluator = ledger_answer(asserted, &positions);
        results.push(agree(
            &format!("{case}: the owner against FR-944's mapping"),
            &owner,
            &expected,
        ));
        results.push(agree(
            &format!("{case}: the evaluator and the owner"),
            &evaluator,
            &owner,
        ));
    }
    expect_agreement(results);
}

#[test]
#[trace("TC-2442", "FR-861-AC-4")]
fn an_injected_disagreement_fails_naming_the_case_and_both_answers() {
    // A partial answer that differs from the owner's, and a completeness
    // answer that differs from the owner's: neither is passed on.
    let owner = owner_before(ClockCover::Counter { uncertainty: 1 }, 0, 6);
    let injected = !owner;
    assert_eq!(
        agree(
            "Counter { uncertainty: 1 } definitely_before(0, 6)",
            &injected.to_string(),
            &owner.to_string()
        ),
        Err(Disagreement {
            case: "Counter { uncertainty: 1 } definitely_before(0, 6)".to_owned(),
            evaluator: injected.to_string(),
            owner: owner.to_string(),
        })
    );
    let refused = owner_answer(
        ProgressCompleteness::Complete { through: 60 },
        &retained_positions(),
    );
    assert_eq!(
        agree("Complete through 60", "accepted", &refused),
        Err(Disagreement {
            case: "Complete through 60".to_owned(),
            evaluator: "accepted".to_owned(),
            owner: "BoundaryRegressed: Completeness(Complete { through: 60 })".to_owned(),
        })
    );
    assert!(std::panic::catch_unwind(|| {
        expect_agreement(vec![agree("a case", "one answer", "another answer")]);
    })
    .is_err());
}
