// SPDX-License-Identifier: AGPL-3.0-or-later
//! T-064 (FR-861-AC-1, FR-861-AC-2): on the same cases, the answers the
//! evaluator reports for the partial-value order and for a completeness
//! revision equal the answers the `filament-observation` owner modules give,
//! and a disagreement is a failure naming the case and both answers, never an
//! answer one side is preferred for.

use filament_observation::authority::{
    completeness, partial, AuthoritySelection, Context, History, Limits, SubjectSelection,
    TemporalBoundary,
};
use filament_observation::{
    admit, AdmissionOutcome, AdmissionRequest, AdmittedRecord, Anchor, ClockRange, Digest,
    Identity, Member, ObservationBinding, PackageSelection, ProducerConfiguration,
    QualifiedObservation, QualifiedSubject, ResourceLimits, Revision, ScopeKind, ScopeSelection,
    StaticProducerBundle, SubjectIdentity, SubjectKind, ValueState, Visibility,
    NATIVE_LINKED_PACKAGE_FORMAT, PRODUCER_INTERFACE_VERSION,
};
use fsl_eval::temporal::{
    ClockCover, Containment, LedgerBinding, ObservedPosition, ProgressAssertion, ProgressClosure,
    ProgressCompleteness, ProgressLedger,
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

/// An admitted owner history of readings, each `(label, nanosecond anchor)`,
/// with the owner selections its completeness documents are derived under.
struct Held {
    qualified: Box<QualifiedObservation>,
    authority: AuthoritySelection,
    subject: SubjectSelection,
    members: Vec<(Identity, Identity)>,
}

fn held(readings: &[(&str, i128)]) -> Held {
    let limits = Limits::owner_max();
    let mut candidate = StaticProducerBundle {
        bundle_identity: Some("bundle:agreement".to_owned()),
        bundle_revision: Some(Revision::producer("1")),
        interface_version: Some(PRODUCER_INTERFACE_VERSION.to_owned()),
        configuration: Some(ProducerConfiguration {
            configuration_identity: "configuration:agreement".to_owned(),
            digest: StaticProducerBundle::default()
                .canonical_digest_selection()
                .expect("a default digest"),
        }),
        ..StaticProducerBundle::default()
    };
    candidate.digest = Some(candidate.canonical_digest_selection().expect("a digest"));
    let producer = candidate.admit().expect("a static producer");
    let subject = QualifiedSubject::new(
        &producer,
        SubjectKind::new(b"type:agreement".to_vec()).expect("a kind"),
        SubjectIdentity::new(b"object:agreement".to_vec()).expect("an identity"),
    );
    let source = id("source:agreement");
    let records: Vec<AdmittedRecord> = readings
        .iter()
        .map(|(label, at)| AdmittedRecord {
            identity: id("unassigned"),
            binding_identity: id("binding:agreement"),
            source_identity: source.clone(),
            schema_identity: id("schema:agreement"),
            subject: subject.clone(),
            signal_identity: id("signal:agreement"),
            trigger_identity: id("trigger:agreement"),
            unit: id("unit:reading"),
            value: ValueState::Present {
                value_type: id("type:reading"),
                canonical_value: format!("\"{label}\""),
            },
            visibility: Visibility::External,
            anchor: Anchor::TimestampNanos(*at),
            event_time_nanos: *at,
            ingestion_time_nanos: Some(at + 1),
            causal_relationship_identity: None,
            clock_identity: id("clock:agreement"),
            clock_revision: id("1"),
            clock_uncertainty_nanos: 0,
        })
        .collect();
    let mut request = AdmissionRequest {
        package: PackageSelection {
            format: NATIVE_LINKED_PACKAGE_FORMAT.to_owned(),
            identity: id("package:agreement"),
            revision: id("1"),
            digest: Digest::new([1; 32]),
        },
        producer,
        binding: ObservationBinding {
            identity: id("binding:agreement"),
            source_identity: source.clone(),
            schema_identity: id("schema:agreement"),
            signal_identity: id("signal:agreement"),
            trigger_identity: id("trigger:agreement"),
            unit: id("unit:reading"),
            subject_kind: subject.kind().clone(),
            required: true,
        },
        expected_subject: subject.clone(),
        relationships: vec![],
        required_relationships: vec![],
        scope: ScopeSelection {
            population_identity: id("unassigned"),
            membership_rule_identity: id("unassigned"),
            membership_digest: Digest::new([0; 32]),
            membership_document: vec![],
            required_member_identities: vec![],
            observation_sources: vec![source.clone()],
            completeness_dependencies: vec![id("completeness:agreement")],
            progress_dependencies: vec![id("progress:agreement")],
            clock_identity: id("clock:agreement"),
            clock_revision: id("1"),
            membership_complete: true,
            closure_identity: Some(id("closure:agreement")),
            closure_digest: Some(Digest::new([2; 32])),
            kind: ScopeKind::Window {
                window_identity: id("window:agreement"),
            },
            range: ClockRange::Timestamp {
                start_nanos: 0,
                end_nanos: 1000,
            },
            members: vec![],
        },
        records,
        limits: ResourceLimits {
            max_records: readings.len(),
            max_members: readings.len(),
            max_relationships: 0,
            max_required_relationships: 0,
        },
    };
    let mut members = Vec::new();
    for (index, (label, at)) in readings.iter().enumerate() {
        let record = filament_observation::authority::observation::record_identity(
            &request.records[index],
            limits,
        )
        .expect("a record identity");
        request.records[index].identity = record.clone();
        let object = id(&format!("member:{label}"));
        request.scope.members.push(Member {
            object_identity: object.clone(),
            record_identity: record.clone(),
            anchor: Anchor::TimestampNanos(*at),
        });
        members.push((object, record));
    }
    members.sort();
    request.scope.required_member_identities =
        members.iter().map(|(object, _)| object.clone()).collect();
    filament_observation::authority::population::assign_request_identities(&mut request, limits)
        .expect("population identities");
    let qualified = match admit(request) {
        AdmissionOutcome::Available { observation } => observation,
        other => panic!("the observation owner refused the fixture: {other:?}"),
    };
    let subject = SubjectSelection {
        scope_identity: id("window:agreement"),
        population_identity: qualified.scope().population_identity.clone(),
    };
    Held {
        qualified,
        authority: AuthoritySelection {
            definition_identity: id("definition:agreement-completeness"),
            definition_revision: id("1"),
            definition_digest: Digest::new([3; 32]),
        },
        subject,
        members,
    }
}

impl Held {
    fn context<'a>(
        &'a self,
        revision: u64,
        predecessor: Option<&'a filament_observation::authority::Document>,
    ) -> Context<'a> {
        Context::new(
            History::batch(&self.qualified),
            &self.authority,
            &self.subject,
            revision,
            predecessor,
        )
    }

    /// FR-944's mapping of an assertion: every admitted member, `Available`
    /// under `Complete` and `Incomplete` naming no observation otherwise, on a
    /// boundary at `upper` nanoseconds.
    fn selection(&self, complete: bool, upper: i128) -> completeness::Selection {
        completeness::Selection::new(
            id(&format!("through:{upper}")),
            TemporalBoundary::TimestampedEvent {
                lower_nanos: upper,
                upper_inclusive_nanos: upper,
                carrier_end_exclusive_nanos: upper + 1,
                watermark_nanos: upper,
            },
            self.members
                .iter()
                .map(|(object, record)| {
                    completeness::Fact::new(
                        object.clone(),
                        complete.then(|| record.clone()),
                        if complete {
                            completeness::FactStatus::Available
                        } else {
                            completeness::FactStatus::Incomplete
                        },
                    )
                })
                .collect(),
        )
    }
}

/// What the owner's revision rule answers: `accepted`, or the contradiction's
/// stable code.
fn owner_revision(
    prior_history: &Held,
    successor_history: &Held,
    asserted: ProgressCompleteness,
) -> String {
    let limits = Limits::owner_max();
    let prior_selection = prior_history.selection(true, 100);
    let prior_context = prior_history.context(1, None);
    let document =
        completeness::derive(prior_context, &prior_selection, limits).expect("a prior document");
    let prior = completeness::read(document.bytes(), prior_context, &prior_selection, limits)
        .expect("a prior view");
    let selection = match asserted {
        ProgressCompleteness::Complete { through } => {
            successor_history.selection(true, i128::from(through))
        }
        ProgressCompleteness::Incomplete => successor_history.selection(false, 100),
    };
    match completeness::revise(
        &prior,
        successor_history.context(2, Some(&document)),
        &selection,
        limits,
    ) {
        Ok(_) => "accepted".to_owned(),
        Err(error) => match error.cause() {
            completeness::RevisionCause::Contradiction(contradiction) => {
                contradiction.code().to_owned()
            }
            completeness::RevisionCause::Authority(cause) => {
                format!("owner refusal {cause:?}")
            }
        },
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

/// What the evaluator's ledger answers for `asserted` over `positions` after
/// `Complete { through: 100 }` over the positions `a`, `b` and `c`: `accepted`
/// or `contradiction`.
fn ledger_revision(asserted: ProgressCompleteness, positions: &[ObservedPosition]) -> String {
    let binding = ledger_binding();
    let held = [position("a", 0), position("b", 50), position("c", 100)];
    let ledger = ProgressLedger::new()
        .admit(
            &binding,
            assertion(ProgressCompleteness::Complete { through: 100 }),
            &held,
            1000,
        )
        .expect("the retained assertion");
    match ledger.admit(&binding, assertion(asserted), positions, 1000) {
        Ok(_) => "accepted".to_owned(),
        Err(_) => "contradiction".to_owned(),
    }
}

/// FR-502-AC-2's cases: the owner code each contradiction carries, or `None`
/// where the revision is accepted.
fn revision_cases() -> Vec<(&'static str, ProgressCompleteness, Vec<ObservedPosition>, &'static str)>
{
    let held = vec![position("a", 0), position("b", 50), position("c", 100)];
    let mut uncovered = held.clone();
    uncovered.push(position("d", 80));
    vec![
        (
            "Complete through 100",
            ProgressCompleteness::Complete { through: 100 },
            held.clone(),
            "accepted",
        ),
        (
            "Complete through 140",
            ProgressCompleteness::Complete { through: 140 },
            held.clone(),
            "accepted",
        ),
        (
            "Complete through 60",
            ProgressCompleteness::Complete { through: 60 },
            held.clone(),
            "QOBS-REV-BOUNDARY-REGRESSED",
        ),
        (
            "Incomplete",
            ProgressCompleteness::Incomplete,
            held,
            "QOBS-REV-COMPLETE-REGRESSED",
        ),
        (
            "an uncovered position stamped 80",
            ProgressCompleteness::Complete { through: 100 },
            uncovered,
            "QOBS-REV-UNCOVERED-MEMBER",
        ),
    ]
}

#[test]
#[trace("TC-2442", "FR-861-AC-2")]
fn the_evaluator_accepts_and_refuses_a_completeness_revision_as_the_owner_does() {
    let prior = held(&[("a", 0), ("b", 50), ("c", 100)]);
    let with_uncovered = held(&[("a", 0), ("b", 50), ("c", 100), ("d", 80)]);
    let mut results = Vec::new();
    for (case, asserted, positions, expected) in revision_cases() {
        let successor = if positions.len() == 4 {
            &with_uncovered
        } else {
            &prior
        };
        let owner = owner_revision(&prior, successor, asserted);
        let evaluator = ledger_revision(asserted, &positions);
        // The two sides answer in their own vocabulary, so each is first read
        // against the answer FR-944 states for the case, then against the
        // other: an accept is an accept, a contradiction the owner's refusal.
        results.push(agree(
            &format!("{case}: the owner against FR-944's rule"),
            &owner,
            expected,
        ));
        results.push(agree(
            &format!("{case}: the evaluator and the owner"),
            if evaluator == "accepted" {
                "accepted"
            } else {
                "refused"
            },
            if owner == "accepted" {
                "accepted"
            } else {
                "refused"
            },
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
    let prior = held(&[("a", 0), ("b", 50), ("c", 100)]);
    let refused = owner_revision(&prior, &prior, ProgressCompleteness::Complete { through: 60 });
    assert_eq!(
        agree("Complete through 60", "accepted", &refused),
        Err(Disagreement {
            case: "Complete through 60".to_owned(),
            evaluator: "accepted".to_owned(),
            owner: "QOBS-REV-BOUNDARY-REGRESSED".to_owned(),
        })
    );
    assert!(std::panic::catch_unwind(|| {
        expect_agreement(vec![agree("a case", "one answer", "another answer")]);
    })
    .is_err());
}
