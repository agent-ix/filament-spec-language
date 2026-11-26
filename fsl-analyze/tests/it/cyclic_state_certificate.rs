// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-201's closed cyclic until and always proof profiles: emission by EN-5,
//! the independent checker's own component classification, and limits.
use crate::long_run_support::limits;
use crate::prob_world::{Model, Operation, World};
use filament_exact::{Cancel, CancelCause, Integer, Rational};
use filament_semantic_value::checking::CheckMode;
use fsl_analyze::analyze::{analyze_items, exact_probabilistic_item, Engines};
use fsl_analyze::exact_probabilistic::{
    build_product, decide_until, ProbBuild, UnboundedError, UnboundedObjective,
};
use fsl_eval::simulation::model::ModelSystem;
use fsl_foundation::RequestIndex;
use fsl_replay::{
    check_cyclic_mdp_probability_certificate_observing, check_probability_certificate_document,
    probabilistic_claim_identity, read_probability_proof_document,
    write_probability_proof_document, BoundsMethod, CyclicMdpProbabilityPhase as P, ExactBound,
    ExactProbabilityCertificateCheck as Check, IncompleteCause, ModelCheckLimit, ModelCheckLimits,
    ProbabilityBoundSide, ProbabilityCertificateLocus as Locus, ProbabilityCertificateRule as Rule,
    ProbabilityDocumentError, ProbabilityDocumentLimit, ProbabilityObjective,
    ProbabilityStateEvent, ProofBasis, TerminalValue,
};
use ix_trace_rs::trace;
use serde_json::Value;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};

const DTMC: &str = "filament.probability-objective/cyclic-dtmc-state-predicate/v1";
const MDP: &str = "filament.probability-objective/cyclic-mdp-state-predicate/v1";
const UNTIL: &str = "holds(x.phase <= 3) until holds(x.phase = 2)";
const ALWAYS: &str = "always holds(x.phase <= 1 or x.phase = 3)";

fn ratio(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).unwrap()
}

/// `phase` 0 moves by `a` (and, with `b`, by `b`) to itself, to the two-state
/// cycle of phases 1 and 3, or to the terminal phase 2.
fn model(with_b: bool, claims: &str) -> Model {
    let package = "test/cyclic-state";
    let post = "self.phase - outcome = 0";
    let enabled = if with_b { "self.phase = 0" } else { "false" };
    Model {
        package, object:"Item", population:"items", key:"x",
        integers:vec![("Phase",0,3),("Outcome",0,2)],
        fields:vec![("phase",format!("ix://{package}/Phase"))],
        operations:vec![
            Operation {name:"a",parameters:vec![("outcome",format!("ix://{package}/Outcome"))],modifies:vec!["phase"],deletes:false},
            Operation {name:"b",parameters:vec![("outcome",format!("ix://{package}/Outcome"))],modifies:vec!["phase"],deletes:false},
            Operation {name:"after",parameters:vec![],modifies:vec!["phase"],deletes:false}],
        body:format!("pre A using v on M::Item::a {{ self.phase = 0 }}\npost AD using v on M::Item::a {{ {post} }}\npre B using v on M::Item::b {{ {enabled} }}\npost BD using v on M::Item::b {{ {post} }}\npre After using v on M::Item::after {{ self.phase = 1 or self.phase = 3 }}\npost AfterD using v on M::Item::after {{ self.phase - (if pre(self.phase) = 1 then 3 else 1) = 0 }}\nrandom Item::a(outcome) ~ {{ 0: 5, 1: 2, 2: 3 }};\nrandom Item::b(outcome) ~ {{ 0: 5, 1: 4, 2: 1 }};\nterminal Stop using v on M when {{ self.phase = 2 }};\n{claims}"),
        initial:serde_json::json!({"phase":{"integer":"0"}}),
    }
}

fn claim(name: &str, bound: &str, event: &str) -> String {
    format!("probabilistic {name} using inf on M over (x: M::Item) under every scheduler on origin {{ probability {bound} [ {event} ] }}\n")
}

fn produce(world: &World, name: &str, limits: ModelCheckLimits) -> Vec<u8> {
    let record = settle(world, name, limits, |_| {});
    let terminal = record.terminal().expect("a settled claim");
    assert!(
        matches!(terminal.value(), TerminalValue::Proved { .. }),
        "{record:#?}"
    );
    terminal
        .exact_certificate_document()
        .expect("a proof carries its document")
        .to_vec()
}

fn settle(
    world: &World,
    name: &str,
    limits: ModelCheckLimits,
    adjust: impl FnOnce(&mut fsl_analyze::exact_probabilistic::ProductRequest<'_>),
) -> fsl_analyze::analyze::AnalyzeRecord {
    let mut request = world.request(name, limits);
    adjust(&mut request);
    analyze_items(
        &Engines::fsl().unwrap(),
        vec![exact_probabilistic_item(RequestIndex::new(0), request)],
        &Cancel::new(),
        fsl_replay::CertificationPolicy::AdmitAny,
    )
    .remove(0)
}

fn check(world: &World, name: &str, bytes: &[u8]) -> Result<Check, ProbabilityDocumentError> {
    check_with(world, name, bytes, limits())
}

fn check_with(
    world: &World,
    name: &str,
    bytes: &[u8],
    bound: fsl_replay::ProbabilityDocumentLimits,
) -> Result<Check, ProbabilityDocumentError> {
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let item = probabilistic_claim_identity(world.claim(name)).unwrap();
    check_probability_certificate_document(
        &system,
        world.package.graph().probabilistic_model().claims(),
        &item,
        bytes,
        bound,
        &Cancel::new(),
    )
}

fn json(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap()
}

/// Offer `bytes` for the claim `name`, edited as unchecked evidence.
fn offered(
    world: &World,
    name: &str,
    bytes: &[u8],
    edit: impl FnOnce(&mut fsl_replay::ProbabilityCertificateDocument),
) -> Vec<u8> {
    let mut document = read_probability_proof_document(bytes, limits(), &Cancel::new()).unwrap();
    let inner = document.document_mut();
    inner.identity = probabilistic_claim_identity(world.claim(name)).unwrap();
    edit(inner);
    write_probability_proof_document(&document, limits(), &Cancel::new()).unwrap()
}

/// Edit the canonical document's objective and value rows as JSON.
fn edited(bytes: &[u8], edit: impl FnOnce(&mut Value, &mut Vec<Value>)) -> Vec<u8> {
    let mut document = json(bytes);
    let mut values = document["values"].as_array().unwrap().clone();
    edit(&mut document["objective"], &mut values);
    document["values"] = Value::Array(values);
    serde_json::to_vec(&document).unwrap()
}

fn rejection(result: Result<Check, ProbabilityDocumentError>) -> (Rule, Locus) {
    match result {
        Ok(Check::Rejected(rejection)) => (rejection.rule, rejection.at),
        other => panic!("expected a rejection: {other:?}"),
    }
}

struct Case {
    with_b: bool,
    event: &'static str,
    operator: &'static str,
    threshold: (i64, i64),
    profile: &'static str,
    targets: usize,
    side: ProbabilityBoundSide,
}

fn cases() -> Vec<Case> {
    let mut cases = Vec::new();
    for (with_b, profile, rows) in [
        (false, DTMC, [(3, 5, 1), (3, 5, 1), (2, 5, 1), (2, 5, 1)]),
        (true, MDP, [(1, 5, 1), (3, 5, 1), (2, 5, 1), (4, 5, 1)]),
    ] {
        for (index, (event, operator, side)) in [
            (UNTIL, ">=", ProbabilityBoundSide::AtLeast),
            (UNTIL, "<=", ProbabilityBoundSide::AtMost),
            (ALWAYS, ">=", ProbabilityBoundSide::AtLeast),
            (ALWAYS, "<=", ProbabilityBoundSide::AtMost),
        ]
        .into_iter()
        .enumerate()
        {
            let (numerator, denominator, targets) = rows[index];
            cases.push(Case {
                with_b,
                event,
                operator,
                threshold: (numerator, denominator),
                profile,
                targets,
                side,
            });
        }
    }
    cases
}

fn world_of(case: &Case, claims: &str) -> World {
    World::new(&model(case.with_b, claims))
}

fn bound_text(case: &Case) -> String {
    format!(
        "{} rational({},{})",
        case.operator, case.threshold.0, case.threshold.1
    )
}

#[trace("TC-3184", "TC-3186", "FR-201-AC-62", "FR-201-AC-64")]
#[test]
fn cyclic_until_and_always_claims_prove_with_their_closed_profile_at_equality() {
    for case in cases() {
        let world = world_of(&case, &claim("Likely", &bound_text(&case), case.event));
        let bytes = produce(&world, "Likely", ModelCheckLimits::default());
        let document = json(&bytes);
        let objective = &document["objective"];
        assert_eq!(objective["profile"], case.profile);
        assert_eq!(
            objective["event"],
            if case.event == UNTIL {
                "until"
            } else {
                "always"
            }
        );
        assert_eq!(objective["optimization"], "maximum");
        assert!(objective.get("failures").is_none());
        assert_eq!(objective["fairness"], serde_json::json!([]));
        assert_eq!(objective["targets"].as_array().unwrap().len(), case.targets);
        let parsed = read_probability_proof_document(&bytes, limits(), &Cancel::new()).unwrap();
        assert_eq!(parsed.is_cyclic_mdp(), case.with_b);
        assert_eq!(
            write_probability_proof_document(&parsed, limits(), &Cancel::new()).unwrap(),
            bytes,
            "byte for byte"
        );
        let Ok(Check::Accepted(accepted)) = check(&world, "Likely", &bytes) else {
            panic!("the emitted document is accepted");
        };
        assert_eq!(accepted.value(), &ratio(case.threshold.0, case.threshold.1));
        assert_eq!(accepted.side(), case.side);
        // The accepted proof record is consistent: the basis value, the
        // token's value and the initial row of its certificate are one number.
        let initial = accepted
            .certificate()
            .values
            .iter()
            .find(|row| phase_of(&row.state) == 0)
            .expect("the initial row");
        assert_eq!(&initial.value, accepted.value());
        let record = settle(&world, "Likely", ModelCheckLimits::default(), |_| {});
        let terminal = record.terminal().unwrap();
        let TerminalValue::Proved {
            basis: ProofBasis::ExactValue { value, entries, .. },
            ..
        } = terminal.value()
        else {
            panic!("a proof with an exact basis: {record:#?}");
        };
        assert_eq!(value.value(), accepted.value());
        assert_eq!(entries.len(), 1);
        assert_eq!(
            terminal.exact_certificate().unwrap().values[0].value,
            initial.value
        );
    }
}

#[trace(
    "TC-3180",
    "TC-3181",
    "TC-3182",
    "FR-201-AC-58",
    "FR-201-AC-59",
    "FR-201-AC-60"
)]
#[test]
fn a_stricter_threshold_misses_at_the_initial_state_and_wrong_ranks_break() {
    for case in cases() {
        let strict = match (case.operator, case.threshold) {
            (">=", (n, d)) => format!(">= rational({},{})", n * 5 + 1, d * 5),
            (_, (n, d)) => format!("<= rational({},{})", n * 5 - 1, d * 5),
        };
        let claims =
            claim("Likely", &bound_text(&case), case.event) + &claim("Strict", &strict, case.event);
        let world = world_of(&case, &claims);
        let bytes = produce(&world, "Likely", ModelCheckLimits::default());
        let for_strict = offered(&world, "Strict", &bytes, |_| {});
        let (rule, locus) = rejection(check(&world, "Strict", &for_strict));
        assert_eq!(rule, Rule::ThresholdMissed);
        let Locus::ProductState(at) = locus else {
            panic!("a product state locus")
        };
        let document = read_probability_proof_document(&bytes, limits(), &Cancel::new()).unwrap();
        let row = document
            .document()
            .values
            .iter()
            .find(|row| row.state == at)
            .expect("the locus is a document row");
        assert!(row.value > ratio(0, 1) && row.value < ratio(1, 1));
        // Every rank at 0 breaks progress at the live initial state.
        let flat = offered(&world, "Likely", &bytes, |document| {
            for (_, rank) in &mut document.ranking {
                *rank = 0;
            }
        });
        let (rule, locus) = rejection(check(&world, "Likely", &flat));
        assert_eq!(rule, Rule::RankBroken);
        assert_eq!(locus, Locus::ProductState(at.clone()));
        // A rank above the shortest distance breaks the DTMC ranking.
        if !case.with_b {
            let raised = offered(&world, "Likely", &bytes, |document| {
                for (key, rank) in &mut document.ranking {
                    if *key == at {
                        *rank += 1;
                    }
                }
            });
            assert_eq!(
                rejection(check(&world, "Likely", &raised)),
                (Rule::RankBroken, Locus::ProductState(at.clone()))
            );
        }
        // Every value 1 fails the maximum equality at a live state.
        let ones = offered(&world, "Likely", &bytes, |document| {
            for row in &mut document.values {
                row.value = ratio(1, 1);
            }
        });
        // All ones is a fixed point of the maximum equation where nothing
        // ranks toward a target, so the MDP check stops at progress.
        let (rule, _) = rejection(check(&world, "Likely", &ones));
        assert_eq!(
            rule,
            if case.with_b && ((case.event == UNTIL) == (case.operator == "<=")) {
                Rule::RankBroken
            } else {
                Rule::InequalityFails
            },
            "{} {} {}",
            case.with_b,
            case.event,
            case.operator
        );
        // The initial value replaced by 1/2 fails.
        let half = offered(&world, "Likely", &bytes, |document| {
            for row in &mut document.values {
                if row.state == at {
                    row.value = ratio(1, 2);
                }
            }
        });
        assert_eq!(
            rejection(check(&world, "Likely", &half)),
            (Rule::InequalityFails, Locus::ProductState(at)),
            "{} {} {}",
            case.with_b,
            case.event,
            case.operator
        );
    }
}

#[trace(
    "TC-3181",
    "TC-3183",
    "TC-3184",
    "FR-201-AC-59",
    "FR-201-AC-61",
    "FR-201-AC-62"
)]
#[test]
fn producer_targets_objectives_events_and_rows_are_never_evidence() {
    for case in cases() {
        let world = world_of(&case, &claim("Likely", &bound_text(&case), case.event));
        let bytes = produce(&world, "Likely", ModelCheckLimits::default());
        let mismatch = |bytes: Vec<u8>| {
            assert!(
                matches!(
                    check(&world, "Likely", &bytes),
                    Err(ProbabilityDocumentError::ObjectiveMismatch)
                ),
                "{}",
                String::from_utf8_lossy(&bytes)
            );
        };
        mismatch(edited(&bytes, |objective, values| {
            objective["targets"] = serde_json::json!([]);
            for row in values {
                row["value"] = serde_json::json!({"numerator": "0", "denominator": "1"});
            }
        }));
        mismatch(edited(&bytes, |objective, _| {
            objective["targets"] = serde_json::json!([]);
        }));
        for optimization in ["minimum", "workload"] {
            mismatch(edited(&bytes, |objective, _| {
                objective["optimization"] = optimization.into();
            }));
        }
        mismatch(edited(&bytes, |objective, _| {
            objective["event"] = if case.event == UNTIL {
                "always"
            } else {
                "until"
            }
            .into();
        }));
        // A dropped, duplicated or reordered row never accepts.
        for edit in [0, 1, 2] {
            let changed = edited(&bytes, |_, values| match edit {
                0 => {
                    values.pop();
                }
                1 => {
                    let first = values[0].clone();
                    values.push(first);
                }
                _ => values.reverse(),
            });
            assert!(
                !matches!(check(&world, "Likely", &changed), Ok(Check::Accepted(_))),
                "row edit {edit}"
            );
        }
    }
}

#[trace("TC-3183", "FR-201-AC-61")]
#[test]
fn the_checker_derives_and_classifies_end_components_itself() {
    // `phase` 1 and 3 form an end component with no decided state: it
    // rejects `until` and accepts `always`, whatever the producer lists.
    let until = World::new(&model(true, &claim("Likely", ">= rational(1,5)", UNTIL)));
    let bytes = produce(&until, "Likely", ModelCheckLimits::default());
    let omitted = offered(&until, "Likely", &bytes, |document| {
        // A producer that does not list the cycle claims it can reach phase 2.
        document.targets.clear();
        for row in &mut document.values {
            row.value = ratio(0, 1);
        }
    });
    assert!(matches!(
        check(&until, "Likely", &omitted),
        Err(ProbabilityDocumentError::ObjectiveMismatch)
    ));
    let always = World::new(&model(true, &claim("Likely", ">= rational(2,5)", ALWAYS)));
    let bytes = produce(&always, "Likely", ModelCheckLimits::default());
    let document = json(&bytes);
    // The accepting cycle lies outside the rejecting union, so it is not a target.
    assert_eq!(
        document["objective"]["targets"].as_array().unwrap().len(),
        1
    );
}

#[trace("TC-3184", "FR-201-AC-62")]
#[test]
fn acyclic_subjects_keep_the_state_predicate_profile_and_cyclic_ones_refuse_it() {
    let claims = claim("Likely", ">= rational(3,5)", UNTIL);
    let world = World::new(&crate::prob_world::link(
        "probabilistic Likely using inf on M over (m: M::Msg) under every scheduler on origin { probability >= 0.95 [ holds(m.attempts <= 2) until holds(m.delivered) ] }\n",
    ));
    let bytes = produce(&world, "Likely", ModelCheckLimits::default());
    assert_eq!(
        json(&bytes)["objective"]["profile"],
        "filament.probability-objective/state-predicate/v1"
    );
    let cyclic = World::new(&model(false, &claims));
    let cyclic_bytes = produce(&cyclic, "Likely", ModelCheckLimits::default());
    let mut document = json(&cyclic_bytes);
    document["objective"]["profile"] = "filament.probability-objective/state-predicate/v1".into();
    document["objective"]["failures"] = serde_json::json!([]);
    let result = check(&cyclic, "Likely", &serde_json::to_vec(&document).unwrap());
    assert!(
        !matches!(result, Ok(Check::Accepted(_))),
        "a cyclic subject is not certified under state-predicate/v1: {result:?}"
    );
}

#[trace("TC-3187", "FR-201-AC-65")]
#[test]
fn a_violated_quantity_past_the_rational_budget_settles_incomplete() {
    let world = World::new(&model(false, &claim("Likely", ">= rational(3,5)", UNTIL)));
    let record = settle(&world, "Likely", ModelCheckLimits::default(), |request| {
        request.limits.max_rational_bits = 1;
    });
    assert_eq!(
        record.terminal().unwrap().value(),
        &TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::ExactProbMaxRationalBits,
            value: 1,
            required: None,
        }),
        "{record:#?}"
    );
    assert!(record
        .terminal()
        .unwrap()
        .exact_certificate_document()
        .is_none());
    assert!(record.exact_probabilistic.is_none());
    let record = settle(&world, "Likely", ModelCheckLimits::default(), |_| {});
    assert!(matches!(
        record.terminal().unwrap().value(),
        TerminalValue::Proved { .. }
    ));
}

/// One byte short of the storage the run needs to reach the rational budget's
/// settlement, it stops on the storage budget, although the first solve fits
/// that storage and reaches the rational budget itself. The refused bytes are
/// therefore the cyclic-subject check's, which reads the graph after the solve.
#[trace("TC-3187", "FR-201-AC-65", "FR-203")]
#[test]
fn a_storage_stop_while_reading_the_cyclic_subject_names_the_storage_budget() {
    let world = World::new(&model(false, &claim("Likely", ">= rational(3,5)", UNTIL)));
    let settled = |storage_bytes: u64| {
        let record = settle(&world, "Likely", ModelCheckLimits::default(), |request| {
            request.limits.max_rational_bits = 1;
            request.limits.document.storage_bytes = storage_bytes;
        });
        assert!(record.exact_probabilistic.is_none());
        record.terminal().unwrap().value().clone()
    };
    let stopped = |limit, value| {
        TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit,
            value,
            required: None,
        })
    };
    let rational = stopped(ModelCheckLimit::ExactProbMaxRationalBits, 1);
    let mut request = world.request("Likely", ModelCheckLimits::default());
    let (mut short, mut enough) = (0, request.limits.document.storage_bytes);
    assert_eq!(settled(enough), rational);
    while enough - short > 1 {
        let middle = short + (enough - short) / 2;
        if settled(middle) == rational {
            enough = middle;
        } else {
            short = middle;
        }
    }
    assert_eq!(
        settled(short),
        stopped(ModelCheckLimit::CertificateMaxWorkingBytes, short)
    );

    request.limits.max_rational_bits = 1;
    request.limits.document.storage_bytes = short;
    let limits = request.limits;
    let ProbBuild::Built(product) = build_product(request, &Cancel::new()).unwrap() else {
        panic!("the cyclic product builds");
    };
    let operand = |at: usize| {
        product
            .labels
            .iter()
            .map(|labels| labels[at])
            .collect::<Vec<_>>()
    };
    let solved = decide_until(
        &product,
        &operand(0),
        &operand(1),
        UnboundedObjective::Min,
        &limits,
        &Cancel::new(),
    );
    assert!(
        matches!(solved, Err(UnboundedError::MaxRationalBits)),
        "the first solve fits {short} bytes: {solved:?}"
    );
}

/// Past the rational budget the run's last readings admit the subject again
/// and walk its graph; a cancellation at any of them keeps its cause rather
/// than reporting the budget.
#[trace("TC-3185", "FR-201-AC-63", "FR-201-AC-65")]
#[test]
fn cancelling_the_cyclic_subject_check_past_the_rational_budget_keeps_its_cause() {
    let world = World::new(&model(false, &claim("Likely", ">= rational(3,5)", UNTIL)));
    let engines = Engines::fsl().unwrap();
    let request = || {
        let mut request = world.request("Likely", ModelCheckLimits::default());
        request.limits.max_rational_bits = 1;
        request
    };
    let run = |cancel: &Cancel| {
        let item = exact_probabilistic_item(RequestIndex::new(0), request());
        analyze_items(&engines, vec![item], cancel, fsl_replay::CertificationPolicy::AdmitAny).remove(0)
    };
    let live = Cancel::new();
    let _counting = live.count_charges();
    run(&live);
    let last = live.charges();
    let building = Cancel::new();
    let _building = building.count_charges();
    build_product(request(), &building).unwrap();
    // The check is one admission and one walk of the same graph the product
    // build admits and walks, so twice the build's readings cover it.
    let first = last.saturating_sub(building.charges().saturating_mul(2));
    assert!(first > 0 && first < last);
    for at in first..=last {
        let cause = if at % 2 == 0 {
            CancelCause::Requested
        } else {
            CancelCause::Deadline
        };
        let slot = Arc::new(OnceLock::<Cancel>::new());
        let weak = Arc::downgrade(&slot);
        let polls = AtomicU32::new(0);
        let cancel = Cancel::observing(move || {
            if polls.fetch_add(1, Ordering::Relaxed) + 1 == at {
                weak.upgrade().unwrap().get().unwrap().cancel(cause);
            }
        });
        slot.set(cancel.clone()).unwrap();
        let record = run(&cancel);
        assert_eq!(
            record.terminal().expect("a terminal record").value(),
            &TerminalValue::Incomplete(IncompleteCause::Cancelled { source: cause }),
            "reading {at} of {last}"
        );
        assert!(record.exact_probabilistic.is_none());
    }
}

/// The budget bounds each part of a rational separately: both parts of
/// `Link`'s least value 24/25 take five bits, so 8 bits keeps the exact
/// answer although numerator and denominator together are longer.
#[trace("TC-3187", "FR-201")]
#[test]
fn an_acyclic_value_whose_parts_fit_the_rational_budget_is_exact() {
    let world = World::new(&crate::prob_world::link(
        "probabilistic Likely using inf on M over (m: M::Msg) under every scheduler on origin \
         { probability >= 0.95 [ holds(m.attempts <= 2) until holds(m.delivered) ] }\n",
    ));
    let record = settle(&world, "Likely", ModelCheckLimits::default(), |request| {
        request.limits.max_rational_bits = 8;
        request.limits.precision_bits = 4;
        request.limits.max_precision_bits = 4;
    });
    let TerminalValue::Proved {
        basis: ProofBasis::ExactValue { value, .. },
        ..
    } = record.terminal().unwrap().value()
    else {
        panic!("{record:#?}");
    };
    assert_eq!(value.value(), &ratio(24, 25));
}

/// `Link`'s `always holds(m.attempts <= 1 or m.delivered)` fails only by
/// losing both sends, so its least value is `1 - 1/25 = 24/25`. At 8 bits
/// both parts fit and the first solve is exact. At 4 bits 25 does not fit: the
/// threshold solve runs on the complement and stops on its interval, which is
/// reported as the claim's own.
#[trace("TC-633", "FR-198-AC-9")]
#[test]
fn an_always_claim_past_the_rational_budget_settles_on_its_own_value() {
    let world = World::new(&crate::prob_world::link(
        "probabilistic Likely using inf on M over (m: M::Msg) under every scheduler on origin \
         { probability >= 0.95 [ always holds(m.attempts <= 1 or m.delivered) ] }\n",
    ));
    let run = |max_rational_bits| {
        settle(&world, "Likely", ModelCheckLimits::default(), |request| {
            request.limits.max_rational_bits = max_rational_bits;
            request.limits.max_iterations = 100;
            request.limits.precision_bits = 4;
            request.limits.max_precision_bits = 4;
        })
    };
    let record = run(8);
    let TerminalValue::Proved {
        basis: ProofBasis::ExactValue { value, .. },
        ..
    } = record.terminal().unwrap().value()
    else {
        panic!("{record:#?}");
    };
    assert_eq!(value.value(), &ratio(24, 25));

    let record = run(4);
    let TerminalValue::Incomplete(IncompleteCause::PrecisionBudget { lower, upper }) =
        record.terminal().unwrap().value()
    else {
        panic!("{record:#?}");
    };
    assert!(
        lower.value() <= &ratio(24, 25) && &ratio(24, 25) <= upper.value(),
        "{lower:?} {upper:?}"
    );
    let terminal = record.terminal().unwrap();
    assert!(terminal.exact_certificate_document().is_none());
    assert!(terminal.exact_counterpart_document().is_none());
}

/// Each threshold is the value itself, so no interval decides it; 24 and 25
/// take five bits each, within a budget of 8.
#[trace("TC-633", "FR-198-AC-7")]
#[test]
fn link_values_whose_parts_fit_the_rational_budget_prove_at_equality() {
    for event in [
        "holds(m.attempts <= 2) until holds(m.delivered)",
        "always holds(m.attempts <= 1 or m.delivered)",
    ] {
        let world = World::new(&crate::prob_world::link(&format!(
            "probabilistic Likely using inf on M over (m: M::Msg) under every scheduler on origin \
             {{ probability >= rational(24,25) [ {event} ] }}\n"
        )));
        let record = settle(&world, "Likely", ModelCheckLimits::default(), |request| {
            request.limits.max_rational_bits = 8;
            request.limits.max_iterations = 3;
        });
        let terminal = record.terminal().unwrap();
        let TerminalValue::Proved {
            basis: ProofBasis::ExactValue { value, .. },
            ..
        } = terminal.value()
        else {
            panic!("{event}: {record:#?}");
        };
        assert_eq!(value.value(), &ratio(24, 25), "{event}");
        assert!(terminal.exact_counterpart_document().is_none(), "{event}");
    }
}

/// `Loop`'s value 1/2 has a two-bit denominator, past a budget of one bit, so
/// only interval iteration can settle the item: at precision 8 it separates
/// 12/25, and held at precision 4 it cannot.
#[trace("TC-633", "FR-198-AC-8")]
#[test]
fn an_item_past_the_rational_budget_settles_from_interval_iteration() {
    let world = World::new(&chain(
        &[act("go", 0, &[(0, 1), (1, 1), (2, 1)])],
        2,
        &claim(
            "Likely",
            ">= rational(12,25)",
            "eventually holds(x.phase = 1)",
        ),
    ));
    let run = |max_precision_bits| {
        settle(&world, "Likely", ModelCheckLimits::default(), |request| {
            request.limits.max_rational_bits = 1;
            request.limits.max_iterations = 100;
            request.limits.precision_bits = 4;
            request.limits.max_precision_bits = max_precision_bits;
        })
    };
    let record = run(16);
    let TerminalValue::Proved {
        basis:
            ProofBasis::ValueBounds {
                lower,
                upper: ExactBound::Finite(upper),
                method,
                ..
            },
        ..
    } = record.terminal().unwrap().value()
    else {
        panic!("{record:#?}");
    };
    assert_eq!(
        method,
        &BoundsMethod::IntervalIteration {
            precision_bits: 8,
            sweeps: 103,
        }
    );
    assert!(&ratio(12, 25) < lower.value(), "{lower:?}");
    assert!(lower.value() <= &ratio(1, 2), "{lower:?}");
    assert!(&ratio(1, 2) <= upper.value(), "{upper:?}");
    assert!(lower.value() < upper.value(), "{lower:?} {upper:?}");

    let record = run(4);
    let terminal = record.terminal().unwrap();
    let TerminalValue::Incomplete(IncompleteCause::PrecisionBudget { lower, upper }) =
        terminal.value()
    else {
        panic!("{record:#?}");
    };
    assert!(
        lower.value() <= &ratio(12, 25) && &ratio(12, 25) <= upper.value(),
        "{lower:?} {upper:?}"
    );
    assert!(terminal.exact_certificate_document().is_none());
    assert!(terminal.exact_counterpart_document().is_none());
}

/// FR-201-AC-59's violated quantity 3/4 has parts of two and three bits, so a
/// budget of 3 keeps its exact proof although the two together are longer.
#[trace("TC-3187", "FR-201-AC-65")]
#[test]
fn a_cyclic_violated_quantity_whose_parts_fit_the_rational_budget_is_proved() {
    let event = format!("{} until {}", at(&[0, 3, 4]), at(&[1]));
    let world = World::new(&until_chain(&claim("Ge", ">= rational(1,4)", &event)));
    let record = settle(&world, "Ge", ModelCheckLimits::default(), |request| {
        request.limits.max_rational_bits = 3;
    });
    let TerminalValue::Proved {
        basis: ProofBasis::ExactValue { value, .. },
        ..
    } = record.terminal().unwrap().value()
    else {
        panic!("{record:#?}");
    };
    assert_eq!(value.value(), &ratio(1, 4));
}

#[trace("TC-3185", "FR-201-AC-63")]
#[test]
fn cancellation_and_limits_keep_their_cause_across_every_cyclic_state_stage() {
    for (with_b, bound, event) in [
        (true, ">= rational(1,5)", UNTIL),
        (true, "<= rational(4,5)", ALWAYS),
        (false, ">= rational(3,5)", UNTIL),
        (false, "<= rational(2,5)", ALWAYS),
    ] {
        let world = World::new(&model(with_b, &claim("Likely", bound, event)));
        let bytes = produce(&world, "Likely", ModelCheckLimits::default());
        let system = ModelSystem::new(world.subject().simulation).unwrap();
        let item = probabilistic_claim_identity(world.claim("Likely")).unwrap();
        let claims = world.package.graph().probabilistic_model().claims();
        for cause in [CancelCause::Requested, CancelCause::Deadline] {
            let cancel = Cancel::new();
            cancel.cancel(cause);
            assert_eq!(
                check_probability_certificate_document(
                    &system,
                    claims,
                    &item,
                    &bytes,
                    limits(),
                    &cancel
                ),
                Err(ProbabilityDocumentError::Cancelled(cause))
            );
            if !with_b {
                continue;
            }
            for phase in [
                P::OriginalAdmission,
                P::Classification,
                P::Components,
                P::CertificateGraph,
                P::Rows,
                P::ActionSums,
                P::Progress,
            ] {
                let cancel = Cancel::new();
                let mut reached = false;
                let result = check_cyclic_mdp_probability_certificate_observing(
                    &system,
                    claims,
                    &item,
                    &bytes,
                    limits(),
                    &cancel,
                    &mut |actual| {
                        if actual == phase {
                            reached = true;
                            cancel.cancel(cause);
                        }
                    },
                );
                assert!(reached, "{phase:?}");
                assert_eq!(result, Err(ProbabilityDocumentError::Cancelled(cause)));
            }
        }
        let mut small = limits();
        small.states = 1;
        assert!(matches!(
            check_with(&world, "Likely", &bytes, small),
            Err(ProbabilityDocumentError::Limit {
                limit: ProbabilityDocumentLimit::States,
                ..
            })
        ));
        let mut small = limits();
        small.edges = 1;
        assert!(matches!(
            check_with(&world, "Likely", &bytes, small),
            Err(ProbabilityDocumentError::Limit {
                limit: ProbabilityDocumentLimit::Edges,
                ..
            })
        ));
        let mut small = limits();
        small.accounting.integer_bits = 1;
        assert!(matches!(
            check_with(&world, "Likely", &bytes, small),
            Err(ProbabilityDocumentError::Accounting(_))
        ));
    }
}

#[trace("TC-3185", "FR-201-AC-63")]
#[test]
fn an_undefined_state_predicate_is_not_read_as_false() {
    for with_b in [false, true] {
        let valid = World::new_with_mode(
            &model(with_b, &claim("Likely", ">= rational(1,5)", UNTIL)),
            CheckMode::Kernel,
        );
        let valid = if with_b {
            valid
        } else {
            World::new_with_mode(
                &model(false, &claim("Likely", ">= rational(3,5)", UNTIL)),
                CheckMode::Kernel,
            )
        };
        let bytes = produce(&valid, "Likely", ModelCheckLimits::default());
        let document = read_probability_proof_document(&bytes, limits(), &Cancel::new()).unwrap();
        let certificate = fsl_replay::ExactProbabilityCertificate {
            values: document.document().values.clone(),
        };
        let threshold = if with_b { "1,5" } else { "3,5" };
        let undefined = World::new_with_mode(
            &model(
                with_b,
                &claim(
                    "Likely",
                    &format!(">= rational({threshold})"),
                    "holds(x.phase <= 3) until holds(if x.phase = 3 then zero_mod(0) else x.phase = 2)",
                ),
            )
            .with_zero_mod(),
            CheckMode::Kernel,
        );
        let system = ModelSystem::new(undefined.subject().simulation).unwrap();
        let item = probabilistic_claim_identity(undefined.claim("Likely")).unwrap();
        let result = fsl_replay::probability_proof_document(
            &system,
            undefined.package.graph().probabilistic_model().claims(),
            &item,
            &certificate,
            limits(),
            &Cancel::new(),
        );
        assert!(
            matches!(result, Err(ProbabilityDocumentError::UndefinedPredicate(_))),
            "{result:?}"
        );
    }
}

trait WithZeroMod {
    fn with_zero_mod(self) -> Model;
}
impl WithZeroMod for Model {
    fn with_zero_mod(mut self) -> Model {
        self.body.insert_str(
            0,
            "function zero_mod using vc(n: Int[0, 0]): Boolean pure { 1 mod n = 0 }\n",
        );
        self
    }
}

/// One action of a chain fixture: from `from`, the weighted outcomes `to`.
struct Act {
    name: &'static str,
    from: i64,
    to: Vec<(i64, u32)>,
}

fn act(name: &'static str, from: i64, to: &[(i64, u32)]) -> Act {
    Act {
        name,
        from,
        to: to.to_vec(),
    }
}

/// The spec's chains, as a model over one `phase` field starting at 0. A
/// state without an action is declared terminal.
fn chain(acts: &[Act], max: i64, claims: &str) -> Model {
    let package = "test/cyclic-chain";
    let mut integers = vec![("Phase", 0, max)];
    let mut operations = Vec::new();
    let mut body = String::new();
    let mut live = std::collections::BTreeSet::new();
    for act in acts {
        live.insert(act.from);
        let name = act.name;
        let mut parameters = Vec::new();
        let target = if act.to.len() == 1 {
            act.to[0].0.to_string()
        } else {
            let kind: &'static str = Box::leak(format!("O{name}").into_boxed_str());
            integers.push((kind, 0, act.to.len() as i64 - 1));
            parameters.push(("outcome", format!("ix://{package}/{kind}")));
            let mut text = act.to.last().unwrap().0.to_string();
            for (index, (to, _)) in act.to.iter().enumerate().rev().skip(1) {
                text = format!("if outcome = {index} then {to} else {text}");
            }
            let weights = act
                .to
                .iter()
                .enumerate()
                .map(|(index, (_, weight))| format!("{index}: {weight}"))
                .collect::<Vec<_>>()
                .join(", ");
            body.push_str(&format!(
                "random Item::{name}(outcome) ~ {{ {weights} }};\n"
            ));
            format!("({text})")
        };
        operations.push(Operation {
            name,
            parameters,
            modifies: vec!["phase"],
            deletes: false,
        });
        body.push_str(&format!(
            "pre P_{name} using v on M::Item::{name} {{ self.phase = {} }}\npost Q_{name} using v on M::Item::{name} {{ self.phase - {target} = 0 }}\n",
            act.from
        ));
    }
    let stops = (0..=max)
        .filter(|phase| !live.contains(phase))
        .map(|phase| format!("self.phase = {phase}"))
        .collect::<Vec<_>>()
        .join(" or ");
    if !stops.is_empty() {
        body.push_str(&format!("terminal Stop using v on M when {{ {stops} }};\n"));
    }
    body.push_str(claims);
    Model {
        package,
        object: "Item",
        population: "items",
        key: "x",
        integers,
        fields: vec![("phase", format!("ix://{package}/Phase"))],
        operations,
        body,
        initial: serde_json::json!({"phase":{"integer":"0"}}),
    }
}

fn at(phases: &[i64]) -> String {
    let text = phases
        .iter()
        .map(|phase| format!("x.phase = {phase}"))
        .collect::<Vec<_>>()
        .join(" or ");
    format!("holds({text})")
}

type Row = (i64, Rational, u64);

const PHASE: &str = r#""name":"phase","value":{"type":"integer","value":""#;

fn phase_of(key: &fsl_replay::ProbabilityProductKey) -> i64 {
    let text = String::from_utf8_lossy(key.as_bytes());
    let at = text.find(PHASE).expect("a phase field") + PHASE.len();
    text[at..].split('"').next().unwrap().parse().unwrap()
}

/// The same complete key with another `phase`, for states the producer did
/// not list.
fn key_at(
    template: &fsl_replay::ProbabilityProductKey,
    phase: i64,
) -> fsl_replay::ProbabilityProductKey {
    let text = String::from_utf8_lossy(template.as_bytes()).into_owned();
    let at = text.find(PHASE).expect("a phase field") + PHASE.len();
    let end = at + text[at..].find('"').unwrap();
    fsl_replay::ProbabilityProductKey::from_canonical_bytes(
        format!("{}{}{}", &text[..at], phase, &text[end..]).into_bytes(),
    )
}

/// The targets, then the value rows with their ranks, by phase.
fn shape(bytes: &[u8]) -> (Vec<i64>, Vec<Row>) {
    let parsed = read_probability_proof_document(bytes, limits(), &Cancel::new()).unwrap();
    let document = parsed.document();
    let targets = document.targets.iter().map(phase_of).collect();
    let rows = document
        .values
        .iter()
        .map(|row| {
            let phase = phase_of(&row.state);
            let rank = document
                .ranking
                .iter()
                .find(|(key, _)| *key == row.state)
                .expect("a rank per row")
                .1;
            (phase, row.value.clone(), rank)
        })
        .collect();
    (targets, rows)
}

fn row(phase: i64, value: (i64, i64), rank: u64) -> Row {
    (phase, ratio(value.0, value.1), rank)
}

/// An unchecked document for the claim `name` with the given targets and
/// rows, laid over the keys of `template` (a producer document of any
/// provable claim over these states).
fn forged(
    world: &World,
    name: &str,
    template: &[u8],
    event: ProbabilityStateEvent,
    mdp: bool,
    targets: &[i64],
    rows: &[Row],
) -> Vec<u8> {
    let bytes = offered(world, name, template, |document| {
        let key = document.values[0].state.clone();
        let base = document.values[0].clone();
        document.event = Some(event);
        document.optimization = ProbabilityObjective::Maximum;
        document.targets = targets.iter().map(|phase| key_at(&key, *phase)).collect();
        document.values = rows
            .iter()
            .map(|(phase, value, _)| {
                let mut row = base.clone();
                row.state = key_at(&key, *phase);
                row.value = value.clone();
                row
            })
            .collect();
        document.ranking = rows
            .iter()
            .map(|(phase, _, rank)| (key_at(&key, *phase), *rank))
            .collect();
    });
    if !mdp {
        return bytes;
    }
    let mut document = json(&bytes);
    document["objective"]["profile"] = MDP.into();
    serde_json::to_vec(&document).unwrap()
}

fn reject(result: Result<Check, ProbabilityDocumentError>) -> (Rule, i64) {
    match rejection(result) {
        (rule, Locus::ProductState(key)) => (rule, phase_of(&key)),
        (rule, locus) => panic!("{rule:?} at {locus:?}"),
    }
}

fn mismatch(result: Result<Check, ProbabilityDocumentError>) {
    assert!(
        matches!(result, Err(ProbabilityDocumentError::ObjectiveMismatch)),
        "{result:?}"
    );
}

fn accepted(
    result: Result<Check, ProbabilityDocumentError>,
) -> fsl_replay::ExactProbabilityAcceptance {
    match result {
        Ok(Check::Accepted(accepted)) => accepted,
        other => panic!("expected acceptance: {other:?}"),
    }
}

fn claims_over(event: &str, list: &[(&str, &str)]) -> String {
    list.iter()
        .map(|(name, bound)| claim(name, bound, event))
        .collect()
}

/// FR-201-AC-58's chain: `s` (satisfies P) moves to `t` and to `u` with
/// probability 1/2 each; `t` (not P) returns to `s`; `u` (P) loops.
fn always_chain(claims: &str) -> Model {
    chain(
        &[
            act("go", 0, &[(1, 1), (2, 1)]),
            act("back", 1, &[(0, 1)]),
            act("stay", 2, &[(2, 1)]),
        ],
        2,
        claims,
    )
}

#[trace("TC-3180", "FR-201-AC-58")]
#[test]
fn a_decided_state_with_a_return_edge_absorbs_and_both_always_sides_certify() {
    let always = format!("always {}", at(&[0, 2]));
    let world = World::new(&always_chain(&claims_over(
        &always,
        &[
            ("Ge", ">= rational(1,2)"),
            ("Le", "<= rational(1,2)"),
            ("GeMore", ">= rational(3,5)"),
            ("LeLess", "<= rational(2,5)"),
        ],
    )));
    let ge = produce(&world, "Ge", ModelCheckLimits::default());
    let le = produce(&world, "Le", ModelCheckLimits::default());
    // `t` has a return edge to `s` yet is decided, so it is the target.
    assert_eq!(
        shape(&ge),
        (
            vec![1],
            vec![row(0, (1, 2), 1), row(1, (1, 1), 0), row(2, (0, 1), 0)]
        )
    );
    assert_eq!(
        shape(&le),
        (
            vec![2],
            vec![row(0, (1, 2), 1), row(1, (0, 1), 0), row(2, (1, 1), 0)]
        )
    );
    assert_eq!(json(&ge)["objective"]["optimization"], "maximum");
    assert_eq!(json(&le)["objective"]["optimization"], "maximum");
    assert_eq!(accepted(check(&world, "Ge", &ge)).value(), &ratio(1, 2));
    assert_eq!(accepted(check(&world, "Le", &le)).value(), &ratio(1, 2));
    // The same documents at `>= 3/5` and `<= 2/5` miss at `s`.
    assert_eq!(
        reject(check(
            &world,
            "GeMore",
            &offered(&world, "GeMore", &ge, |_| {})
        )),
        (Rule::ThresholdMissed, 0)
    );
    assert_eq!(
        reject(check(
            &world,
            "LeLess",
            &offered(&world, "LeLess", &le, |_| {})
        )),
        (Rule::ThresholdMissed, 0)
    );
    // Every value 1 under `{t}` fails the equation at `u`.
    let ones = forged(
        &world,
        "Ge",
        &ge,
        ProbabilityStateEvent::Always,
        false,
        &[1],
        &[row(0, (1, 1), 1), row(1, (1, 1), 0), row(2, (1, 1), 0)],
    );
    assert_eq!(
        reject(check(&world, "Ge", &ones)),
        (Rule::InequalityFails, 2)
    );
    // The decided `t` is a target: a value that lets it continue is refused.
    let leaking = forged(
        &world,
        "Ge",
        &ge,
        ProbabilityStateEvent::Always,
        false,
        &[1],
        &[row(0, (1, 2), 1), row(1, (1, 2), 0), row(2, (0, 1), 0)],
    );
    // `s = 1/2` no longer equals the average of its successors' values.
    assert_eq!(
        reject(check(&world, "Ge", &leaking)),
        (Rule::InequalityFails, 0)
    );
    // Empty targets with every value 0 refuse before any row is read.
    let empty = forged(
        &world,
        "LeLess",
        &le,
        ProbabilityStateEvent::Always,
        false,
        &[],
        &[row(0, (0, 1), 0), row(1, (0, 1), 0), row(2, (0, 1), 0)],
    );
    mismatch(check(&world, "LeLess", &empty));
}

#[trace("TC-3186", "FR-201-AC-64")]
#[test]
fn a_cyclic_dtmc_document_is_maximum_on_both_sides_and_never_minimum_or_workload() {
    let always = format!("always {}", at(&[0, 2]));
    let world = World::new(&always_chain(&claims_over(
        &always,
        &[("Ge", ">= rational(1,2)"), ("Le", "<= rational(1,2)")],
    )));
    for (name, profile) in [("Ge", DTMC), ("Le", DTMC)] {
        let bytes = produce(&world, name, ModelCheckLimits::default());
        assert_eq!(json(&bytes)["objective"]["profile"], profile);
        assert_eq!(json(&bytes)["objective"]["optimization"], "maximum");
        for optimization in ["minimum", "workload"] {
            mismatch(check(
                &world,
                name,
                &edited(&bytes, |objective, _| {
                    objective["optimization"] = optimization.into();
                }),
            ));
        }
    }
}

/// FR-201-AC-59's chain: `s` (A, not B) moves to `b` (B) with 1/4, to `n`
/// (neither) with 1/4 and to `c` with 1/2; `b` and `n` return to `s`; `c`
/// and `d` (A, not B) alternate.
fn until_chain(claims: &str) -> Model {
    chain(
        &[
            act("go", 0, &[(1, 1), (2, 1), (3, 2)]),
            act("rb", 1, &[(0, 1)]),
            act("rn", 2, &[(0, 1)]),
            act("cd", 3, &[(4, 1)]),
            act("dc", 4, &[(3, 1)]),
        ],
        4,
        claims,
    )
}

#[trace("TC-3181", "FR-201-AC-59")]
#[test]
fn a_bottom_component_without_a_decided_state_rejects_until_and_stops_the_union() {
    let event = format!("{} until {}", at(&[0, 3, 4]), at(&[1]));
    let world = World::new(&until_chain(&claims_over(
        &event,
        &[
            ("Ge", ">= rational(1,4)"),
            ("Le", "<= rational(1,4)"),
            ("GeMore", ">= rational(3,10)"),
            ("LeLess", "<= rational(1,5)"),
            ("GeHalf", ">= rational(1,2)"),
        ],
    )));
    let ge = produce(&world, "Ge", ModelCheckLimits::default());
    let le = produce(&world, "Le", ModelCheckLimits::default());
    // The violating union {c, d} is a stop: `c` is a target and there is no
    // row for `d`, which only the union reaches.
    assert_eq!(
        shape(&ge),
        (
            vec![2, 3],
            vec![
                row(0, (3, 4), 1),
                row(1, (0, 1), 0),
                row(2, (1, 1), 0),
                row(3, (1, 1), 0)
            ]
        )
    );
    // At `<= 1/4` no component accepts, so `c` and `d` are ordinary states.
    assert_eq!(
        shape(&le),
        (
            vec![1],
            vec![
                row(0, (1, 4), 1),
                row(1, (1, 1), 0),
                row(2, (0, 1), 0),
                row(3, (0, 1), 0),
                row(4, (0, 1), 0)
            ]
        )
    );
    assert_eq!(accepted(check(&world, "Ge", &ge)).value(), &ratio(1, 4));
    assert_eq!(accepted(check(&world, "Le", &le)).value(), &ratio(1, 4));
    assert_eq!(
        reject(check(
            &world,
            "GeMore",
            &offered(&world, "GeMore", &ge, |_| {})
        )),
        (Rule::ThresholdMissed, 0)
    );
    assert_eq!(
        reject(check(
            &world,
            "LeLess",
            &offered(&world, "LeLess", &le, |_| {})
        )),
        (Rule::ThresholdMissed, 0)
    );
    // Rank 2 at `s` breaks the ranking at `s`.
    let raised = offered(&world, "Ge", &ge, |document| {
        let initial = document.values[0].state.clone();
        for (key, rank) in &mut document.ranking {
            if *key == initial {
                *rank = 2;
            }
        }
    });
    assert_eq!(reject(check(&world, "Ge", &raised)), (Rule::RankBroken, 0));
    // A false claim `>= 1/2` with the producer's harmonic targets `{n}` is
    // refused: the checker derives {c, n}, whatever the initial value says.
    let false_claim = forged(
        &world,
        "GeHalf",
        &ge,
        ProbabilityStateEvent::Until,
        false,
        &[2],
        &[
            row(0, (1, 4), 1),
            row(1, (0, 1), 0),
            row(2, (1, 1), 0),
            row(3, (0, 1), 0),
            row(4, (0, 1), 0),
        ],
    );
    mismatch(check(&world, "GeHalf", &false_claim));
}

/// FR-201-AC-60's chain: `s` (P) has `a`, to the terminal P state `g` with
/// 3/4 and to `f` with 1/4, and `b`, to `s` with 1/2, `f` and `g` with 1/4
/// each; `f` (not P) returns to `s`.
fn mdp_chain(claims: &str) -> Model {
    chain(
        &[
            act("a", 0, &[(1, 3), (2, 1)]),
            act("b", 0, &[(0, 2), (2, 1), (1, 1)]),
            act("ret", 2, &[(0, 1)]),
        ],
        2,
        claims,
    )
}

#[trace("TC-3182", "FR-201-AC-60")]
#[test]
fn the_cyclic_mdp_profile_certifies_the_maximum_over_every_action_on_both_sides() {
    let always = format!("always {}", at(&[0, 1]));
    let world = World::new(&mdp_chain(&claims_over(
        &always,
        &[
            ("Ge", ">= rational(1,2)"),
            ("Le", "<= rational(3,4)"),
            ("GeMore", ">= rational(3,5)"),
            ("LeLess", "<= rational(7,10)"),
            ("GeHalf", ">= rational(3,4)"),
        ],
    )));
    let ge = produce(&world, "Ge", ModelCheckLimits::default());
    let le = produce(&world, "Le", ModelCheckLimits::default());
    assert_eq!(json(&ge)["objective"]["profile"], MDP);
    assert_eq!(
        shape(&ge),
        (
            vec![2],
            vec![row(0, (1, 2), 1), row(1, (0, 1), 0), row(2, (1, 1), 0)]
        )
    );
    assert_eq!(
        shape(&le),
        (
            vec![1],
            vec![row(0, (3, 4), 1), row(1, (1, 1), 0), row(2, (0, 1), 0)]
        )
    );
    assert_eq!(accepted(check(&world, "Ge", &ge)).value(), &ratio(1, 2));
    assert_eq!(accepted(check(&world, "Le", &le)).value(), &ratio(3, 4));
    assert_eq!(
        reject(check(
            &world,
            "GeMore",
            &offered(&world, "GeMore", &ge, |_| {})
        )),
        (Rule::ThresholdMissed, 0)
    );
    assert_eq!(
        reject(check(
            &world,
            "LeLess",
            &offered(&world, "LeLess", &le, |_| {})
        )),
        (Rule::ThresholdMissed, 0)
    );
    // A single-policy value `s = 1/4` passes `>= 3/4` but not the maximum
    // Bellman equation: `Q_b = 3/8`.
    let single = forged(
        &world,
        "GeHalf",
        &ge,
        ProbabilityStateEvent::Always,
        true,
        &[2],
        &[row(0, (1, 4), 1), row(1, (0, 1), 0), row(2, (1, 1), 0)],
    );
    assert_eq!(
        reject(check(&world, "GeHalf", &single)),
        (Rule::InequalityFails, 0)
    );
    let flat = offered(&world, "Le", &le, |document| {
        let initial = document.values[0].state.clone();
        for (key, rank) in &mut document.ranking {
            if *key == initial {
                *rank = 0;
            }
        }
    });
    assert_eq!(reject(check(&world, "Le", &flat)), (Rule::RankBroken, 0));
    mismatch(check(
        &world,
        "Ge",
        &edited(&ge, |objective, _| {
            objective["optimization"] = "workload".into();
        }),
    ));
}

#[trace("TC-3183", "FR-201-AC-61")]
#[test]
fn end_components_made_by_a_scheduler_choice_are_derived_and_classified_by_the_checker() {
    // (i) `wait` loops at `s`, `go` reaches the B state `b`.
    let until = format!("{} until {}", at(&[0]), at(&[1]));
    let waiting = World::new(&chain(
        &[act("wait", 0, &[(0, 1)]), act("go", 0, &[(1, 1)])],
        1,
        &claim("Ge", ">= rational(1,2)", &until),
    ));
    // No document certifies `>= 1/2` here: `wait` forever violates.
    let record = settle(&waiting, "Ge", ModelCheckLimits::default(), |_| {});
    assert!(
        !matches!(
            record.terminal().unwrap().value(),
            TerminalValue::Proved { .. }
        ),
        "{record:#?}"
    );
    // The documents borrow the keys of a provable claim over the same states.
    let always = format!("always {}", at(&[0, 2]));
    let template_world = World::new(&always_chain(&claim("Ge", ">= rational(1,2)", &always)));
    let template = produce(&template_world, "Ge", ModelCheckLimits::default());
    let correct = forged(
        &waiting,
        "Ge",
        &template,
        ProbabilityStateEvent::Until,
        true,
        &[0],
        &[row(0, (1, 1), 0)],
    );
    assert_eq!(
        reject(check(&waiting, "Ge", &correct)),
        (Rule::ThresholdMissed, 0)
    );
    let empty = forged(
        &waiting,
        "Ge",
        &template,
        ProbabilityStateEvent::Until,
        true,
        &[],
        &[row(0, (0, 1), 0), row(1, (0, 1), 0)],
    );
    mismatch(check(&waiting, "Ge", &empty));
    // (ii) `stay` keeps `ok`; `leave` reaches `down`, which returns.
    let always = format!("always {}", at(&[0]));
    let staying = World::new(&chain(
        &[
            act("stay", 0, &[(0, 1)]),
            act("leave", 0, &[(1, 1)]),
            act("ret", 1, &[(0, 1)]),
        ],
        1,
        &(claim("Le", "<= rational(1,2)", &always) + &claim("Ge", ">= rational(1,2)", &always)),
    ));
    let correct = forged(
        &staying,
        "Le",
        &template,
        ProbabilityStateEvent::Always,
        true,
        &[0],
        &[row(0, (1, 1), 0)],
    );
    assert_eq!(
        reject(check(&staying, "Le", &correct)),
        (Rule::ThresholdMissed, 0)
    );
    let empty = forged(
        &staying,
        "Le",
        &template,
        ProbabilityStateEvent::Always,
        true,
        &[],
        &[row(0, (0, 1), 0), row(1, (0, 1), 0)],
    );
    mismatch(check(&staying, "Le", &empty));
    // `ok` lies in the accepting end component {ok} outside the union.
    let outside = forged(
        &staying,
        "Ge",
        &template,
        ProbabilityStateEvent::Always,
        true,
        &[1],
        &[row(0, (1, 1), 1), row(1, (1, 1), 0)],
    );
    assert_eq!(
        reject(check(&staying, "Ge", &outside)),
        (Rule::ThresholdMissed, 0)
    );
    let zero = forged(
        &staying,
        "Ge",
        &template,
        ProbabilityStateEvent::Always,
        true,
        &[1],
        &[row(0, (0, 1), 0), row(1, (1, 1), 0)],
    );
    assert_eq!(
        reject(check(&staying, "Ge", &zero)),
        (Rule::InequalityFails, 0)
    );
}

#[trace("TC-3183", "FR-201-AC-61")]
#[test]
fn an_end_component_made_by_a_scheduler_choice_prunes_its_leaving_action_and_certifies() {
    // `wait` is a rejecting end component at `s` once `go`, which leaves it,
    // is pruned; the value still comes from `go`.
    let until = format!("{} until {}", at(&[0]), at(&[1]));
    let waiting = World::new(&chain(
        &[act("wait", 0, &[(0, 1)]), act("go", 0, &[(1, 1), (2, 1)])],
        2,
        &claim("Le", "<= rational(1,2)", &until),
    ));
    let bytes = produce(&waiting, "Le", ModelCheckLimits::default());
    assert_eq!(json(&bytes)["objective"]["profile"], MDP);
    assert_eq!(
        shape(&bytes),
        (
            vec![1],
            vec![row(0, (1, 2), 1), row(1, (1, 1), 0), row(2, (0, 1), 0)]
        )
    );
    assert_eq!(
        accepted(check(&waiting, "Le", &bytes)).value(),
        &ratio(1, 2)
    );
    // `stay` is an accepting end component outside the violating union.
    let always = format!("always {}", at(&[0, 1]));
    let staying = World::new(&chain(
        &[
            act("stay", 0, &[(0, 1)]),
            act("leave", 0, &[(1, 1), (2, 1)]),
        ],
        2,
        &claim("Ge", ">= rational(1,2)", &always),
    ));
    let bytes = produce(&staying, "Ge", ModelCheckLimits::default());
    assert_eq!(json(&bytes)["objective"]["profile"], MDP);
    assert_eq!(
        shape(&bytes),
        (
            vec![2],
            vec![row(0, (1, 2), 1), row(1, (0, 1), 0), row(2, (1, 1), 0)]
        )
    );
    assert_eq!(
        accepted(check(&staying, "Ge", &bytes)).value(),
        &ratio(1, 2)
    );
}

fn describe(result: &Result<Check, ProbabilityDocumentError>) -> String {
    match result {
        Ok(Check::Accepted(_)) => "accepted".to_owned(),
        Ok(Check::Rejected(rejection)) => match &rejection.at {
            Locus::ProductState(key) => format!("{:?}@{}", rejection.rule, phase_of(key)),
            other => format!("{:?}@{other:?}", rejection.rule),
        },
        Ok(other) => format!("other:{other:?}"),
        Err(error) => format!("error:{error:?}"),
    }
}

/// Re-encode every state key of the document through `change`.
fn rekeyed(bytes: &[u8], change: impl Fn(&mut Value)) -> Vec<u8> {
    use base64::Engine as _;
    let engine = base64::engine::general_purpose::STANDARD;
    let mut document = json(bytes);
    let alter = |text: &mut Value| {
        let mut key: Value =
            serde_json::from_slice(&engine.decode(text.as_str().unwrap()).unwrap()).unwrap();
        change(&mut key);
        *text = Value::String(engine.encode(serde_json::to_vec(&key).unwrap()));
    };
    for row in document["values"].as_array_mut().unwrap() {
        alter(&mut row["state"]);
    }
    for row in document["ranking"].as_array_mut().unwrap() {
        alter(&mut row["state"]);
    }
    for target in document["objective"]["targets"].as_array_mut().unwrap() {
        alter(target);
    }
    serde_json::to_vec(&document).unwrap()
}

fn until_world() -> World {
    let event = format!("{} until {}", at(&[0, 3, 4]), at(&[1]));
    World::new(&until_chain(
        &(claim("Ge", ">= rational(1,4)", &event) + &claim("Other", ">= rational(1,8)", &event)),
    ))
}

fn mdp_world() -> World {
    let event = format!("always {}", at(&[0, 1]));
    World::new(&mdp_chain(
        &(claim("Ge", ">= rational(1,2)", &event) + &claim("Other", ">= rational(1,3)", &event)),
    ))
}

#[trace("TC-3184", "FR-201-AC-62")]
#[test]
fn every_mutation_of_a_cyclic_document_keeps_its_existing_rule_and_locus() {
    for (name, world) in [("dtmc", until_world()), ("mdp", mdp_world())] {
        let bytes = produce(&world, "Ge", ModelCheckLimits::default());
        let run = |_: &str, mutated: Vec<u8>| describe(&check(&world, "Ge", &mutated));
        let other = |bytes: &[u8]| {
            let mut document =
                read_probability_proof_document(bytes, limits(), &Cancel::new()).unwrap();
            document.document_mut().identity =
                probabilistic_claim_identity(world.claim("Other")).unwrap();
            write_probability_proof_document(&document, limits(), &Cancel::new()).unwrap()
        };
        let kind = |kind: &str| {
            let mut document = json(&bytes);
            document["kind"] = kind.into();
            serde_json::to_vec(&document).unwrap()
        };
        let table: Vec<(&str, Vec<u8>, &str)> = vec![
            ("foreign identity", other(&bytes), "error:IdentityMismatch"),
            (
                "model-only key",
                rekeyed(&bytes, |key| key[1] = Value::Null),
                "error:Malformed",
            ),
            (
                "foreign binding",
                rekeyed(&bytes, |key| key[1]["identity"] = "y".into()),
                "error:ObjectiveMismatch",
            ),
            (
                "fairness",
                edited(&bytes, |objective, _| {
                    objective["fairness"] = serde_json::json!(["weak"])
                }),
                "error:Unsupported",
            ),
            (
                "state-predicate profile",
                {
                    let mut document = json(&bytes);
                    document["objective"]["profile"] =
                        "filament.probability-objective/state-predicate/v1".into();
                    document["objective"]["failures"] = serde_json::json!([]);
                    serde_json::to_vec(&document).unwrap()
                },
                "error:Unsupported",
            ),
            (
                "foreign event",
                edited(&bytes, |objective, _| {
                    objective["event"] = "eventually".into()
                }),
                "error:Unsupported",
            ),
            (
                "nonreduced rational",
                edited(&bytes, |_, values| {
                    values[0]["value"] = serde_json::json!({"numerator": "2", "denominator": "4"})
                }),
                "error:Malformed",
            ),
            (
                "noncanonical bytes",
                serde_json::to_vec_pretty(&json(&bytes)).unwrap(),
                "error:Noncanonical",
            ),
            (
                "dropped row",
                edited(&bytes, |_, values| {
                    values.pop();
                }),
                "Malformed@Member(\"values\")",
            ),
            (
                "duplicate row",
                edited(&bytes, |_, values| {
                    let first = values[0].clone();
                    values.push(first);
                }),
                if name == "dtmc" {
                    "error:Malformed"
                } else {
                    "Malformed@Member(\"values\")"
                },
            ),
            (
                "reordered rows",
                edited(&bytes, |_, values| values.reverse()),
                if name == "dtmc" {
                    "error:Malformed"
                } else {
                    "Malformed@Member(\"values\")"
                },
            ),
            ("lower document", kind("lower"), "error:Unsupported"),
            ("upper document", kind("upper"), "error:Unsupported"),
            ("interval document", kind("interval"), "error:Unsupported"),
        ];
        for (label, mutated, expected) in table {
            assert_eq!(run(label, mutated), expected, "{name}: {label}");
        }
        // The subject selects the profile (FR-201): a changed profile is an
        // objective mismatch in both directions, never an acceptance.
        let other = if name == "dtmc" { MDP } else { DTMC };
        let relabelled = edited(&bytes, |objective, _| objective["profile"] = other.into());
        assert_eq!(
            run("profile", relabelled),
            "error:ObjectiveMismatch",
            "{name}"
        );
    }
}

type Setter = fn(&mut fsl_replay::ProbabilityDocumentLimits, u64);

fn setters() -> Vec<(&'static str, Setter)> {
    vec![
        ("bytes", |l, v| l.bytes = v),
        ("storage", |l, v| l.storage_bytes = v),
        ("states", |l, v| l.states = v as usize),
        ("edges", |l, v| l.edges = v as usize),
        ("scalar bits", |l, v| l.scalar_bits = v),
        ("work", |l, v| l.accounting.work_units = v),
        ("integer bits", |l, v| l.accounting.integer_bits = v),
    ]
}

fn is_cause(label: &str, error: &ProbabilityDocumentError) -> bool {
    use ProbabilityDocumentLimit as L;
    match (label, error) {
        ("bytes", ProbabilityDocumentError::Limit { limit, .. }) => *limit == L::Bytes,
        ("storage", ProbabilityDocumentError::Limit { limit, .. }) => *limit == L::StorageBytes,
        ("states", ProbabilityDocumentError::Limit { limit, .. }) => *limit == L::States,
        ("edges", ProbabilityDocumentError::Limit { limit, .. }) => *limit == L::Edges,
        ("scalar bits", ProbabilityDocumentError::Limit { limit, .. }) => *limit == L::RationalBits,
        ("scalar bits", ProbabilityDocumentError::IntegerText { .. }) => true,
        ("work" | "integer bits", ProbabilityDocumentError::Accounting(_)) => true,
        _ => false,
    }
}

/// One run under `bound`: the verdict and the last stage an MDP run entered.
fn run_with(
    world: &World,
    bytes: &[u8],
    bound: fsl_replay::ProbabilityDocumentLimits,
    mdp: bool,
) -> (Result<Check, ProbabilityDocumentError>, Option<P>) {
    if !mdp {
        return (check_with(world, "Ge", bytes, bound), None);
    }
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let item = probabilistic_claim_identity(world.claim("Ge")).unwrap();
    let mut last = None;
    let result = check_cyclic_mdp_probability_certificate_observing(
        &system,
        world.package.graph().probabilistic_model().claims(),
        &item,
        bytes,
        bound,
        &Cancel::new(),
        &mut |phase| last = Some(phase),
    );
    (result, last)
}

fn stages_of(world: &World, mdp: bool) -> Vec<(&'static str, Vec<String>)> {
    let bytes = produce(world, "Ge", ModelCheckLimits::default());
    let mut found = Vec::new();
    for (label, set) in setters() {
        let accepts = |value: u64| {
            let mut bound = limits();
            set(&mut bound, value);
            matches!(
                run_with(world, &bytes, bound, mdp).0,
                Ok(Check::Accepted(_))
            )
        };
        // The least accepted value, by bisection (acceptance is monotone).
        let (mut low, mut high) = (0_u64, 1_u64 << 24);
        assert!(accepts(high), "{label}");
        while low < high {
            let middle = low + (high - low) / 2;
            if accepts(middle) {
                high = middle;
            } else {
                low = middle + 1;
            }
        }
        let least = low;
        let mut stages = std::collections::BTreeSet::new();
        let stride = (least / 1500).max(1);
        let samples = (0..=least.min(48))
            .chain((0..least).step_by(stride as usize))
            .chain(least.checked_sub(1));
        for value in samples.filter(|value| *value < least) {
            let mut bound = limits();
            set(&mut bound, value);
            let (result, stage) = run_with(world, &bytes, bound, mdp);
            match result {
                Err(error) if is_cause(label, &error) => {
                    stages.insert(format!("{stage:?}"));
                }
                other => panic!("{label} {value}: not the typed cause: {other:?}"),
            }
        }
        found.push((label, stages.into_iter().collect()));
    }
    found
}

#[trace("TC-3185", "FR-201-AC-63")]
#[test]
fn exhausting_a_ceiling_stops_with_its_own_typed_cause_at_every_observable_stage() {
    // The DTMC profile has no stage observer: a ceiling is only seen to stop
    // the check with its own cause (never a rejection or another cause).
    for (label, stages) in stages_of(&until_world(), false) {
        assert_eq!(stages, ["None"], "dtmc {label}");
    }
    let observed = stages_of(&mdp_world(), true)
        .into_iter()
        .collect::<std::collections::BTreeMap<_, _>>();
    let has = |label: &str, wanted: &[&str]| {
        for stage in wanted {
            assert!(
                observed[label].iter().any(|seen| seen.contains(stage)),
                "{label} stopped in {stage}: {:?}",
                observed[label]
            );
        }
    };
    // Admission-time ceilings stop before the first stage or in original
    // admission; the cumulative ceilings stop in every later stage that
    // allocates or sums.
    has("bytes", &["None"]);
    has("states", &["None"]);
    has("edges", &["OriginalAdmission"]);
    has(
        "storage",
        &[
            "OriginalAdmission",
            "Components",
            "CertificateGraph",
            "Rows",
            "ActionSums",
        ],
    );
    has(
        "work",
        &["OriginalAdmission", "Components", "ActionSums", "Progress"],
    );
    has("scalar bits", &["OriginalAdmission"]);
    has("integer bits", &["OriginalAdmission"]);
}

#[trace("TC-3185", "FR-201-AC-63")]
#[test]
fn a_checker_given_an_undefined_state_predicate_refuses_instead_of_reading_false() {
    for with_b in [false, true] {
        let valid = World::new(&model(
            with_b,
            &claim(
                "Likely",
                if with_b {
                    ">= rational(1,5)"
                } else {
                    ">= rational(3,5)"
                },
                UNTIL,
            ),
        ));
        let bytes = produce(&valid, "Likely", ModelCheckLimits::default());
        let undefined = World::new_with_mode(
            &model(
                with_b,
                &claim(
                    "Likely",
                    if with_b { ">= rational(1,5)" } else { ">= rational(3,5)" },
                    "holds(x.phase <= 3) until holds(if x.phase = 3 then zero_mod(0) else x.phase = 2)",
                ),
            )
            .with_zero_mod(),
            CheckMode::Kernel,
        );
        let offer = offered(&undefined, "Likely", &bytes, |_| {});
        let result = check(&undefined, "Likely", &offer);
        assert!(
            matches!(result, Err(ProbabilityDocumentError::UndefinedPredicate(_))),
            "{with_b}: {result:?}"
        );
    }
}

#[trace("TC-3184", "FR-201-AC-62")]
#[test]
fn claim_forms_outside_the_state_predicate_profiles_issue_no_proof_in_them() {
    let a = at(&[0, 3, 4]);
    let b = at(&[1]);
    let head = "probabilistic V using inf on M over (x: M::Item)";
    let forms = [
        (
            "fair",
            format!("{head} under every scheduler fair weak Item::go on origin {{ probability >= rational(1,4) [ {a} until {b} ] }}\n"),
        ),
        (
            "reward",
            format!("reward cost on Item::go = 1;\n{head} under every scheduler on origin {{ expected accumulate cost until {b} <= rational(5,1) }}\n"),
        ),
        (
            "event predicate",
            format!("{head} under every scheduler on origin {{ probability >= rational(1,4) [ eventually holds(x.phase = 1) ] }}\n"),
        ),
    ];
    for (label, claims) in forms {
        let world = World::new(&until_chain(&claims));
        let record = settle(&world, "V", ModelCheckLimits::default(), |_| {});
        let profile = record
            .terminal()
            .and_then(|terminal| terminal.exact_certificate_document())
            .map(|bytes| {
                json(bytes)["objective"]["profile"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            });
        match (label, profile) {
            ("fair", profile) => {
                assert!(profile.is_none(), "{label}: {profile:?}");
                assert!(
                    !matches!(
                        record.terminal().map(|terminal| terminal.value()),
                        Some(TerminalValue::Proved { .. })
                    ),
                    "{label}: {record:#?}"
                );
            }
            ("reward", profile) => assert!(profile.is_none(), "{label}: {profile:?}"),
            (_, profile) => assert_eq!(
                profile.as_deref(),
                Some("filament.probability-objective/cyclic-dtmc-eventual/v1"),
                "{label}"
            ),
        }
    }
}

fn profile_of(record: &fsl_analyze::analyze::AnalyzeRecord) -> Option<String> {
    record
        .terminal()
        .and_then(|terminal| terminal.exact_certificate_document())
        .map(|bytes| {
            json(bytes)["objective"]["profile"]
                .as_str()
                .unwrap()
                .to_owned()
        })
}

fn unsupported(record: &fsl_analyze::analyze::AnalyzeRecord) -> bool {
    matches!(
        &record.settlement,
        fsl_analyze::analyze::Settlement::Disposed(disposed)
            if disposed.kind == fsl_analyze::analyze::DispositionKind::Unsupported
    )
}

/// FR-201-AC-62: a timed (clock) subject gets no proof in these profiles.
#[trace("TC-3184", "FR-201-AC-62")]
#[test]
fn a_timed_subject_issues_no_proof_in_the_cyclic_state_predicate_profiles() {
    let profiles = "profile ev = \"filament.temporal.timed/v1\";\n\
         profile inf = \"filament.temporal.infinite-trace/v1\";\n";
    let document = crate::timed_world::document(&["x"], &[("send", &["late", "x"])]);
    for formula in [
        "holds(not c.late) until holds(c.late)",
        "always holds(not c.late)",
    ] {
        let body = format!(
            "{}pre Send using v on M::Call::send {{ not self.late and self.x >= 1 ms }}\n\
             post Sent using v on M::Call::send {{ self.late and self.x = 0 ms }}\n\
             time invariant Deadline when {{ not self.late }} {{ self.x <= 2 ms }}\n\
             probabilistic V using inf on M over (c: M::Call) under every scheduler on origin {{ \
             probability >= rational(1,4) [ {formula} ] }}\n",
            crate::timed_world::DENSE,
        );
        let world = crate::timed_world::world(&document, profiles, &body);
        let system = world.system();
        let model = world.package.graph().probabilistic_model();
        let item = probabilistic_claim_identity(model.claim("V").unwrap()).unwrap();
        let result = fsl_replay::probability_proof_document(
            &system,
            model.claims(),
            &item,
            &fsl_replay::ExactProbabilityCertificate { values: Vec::new() },
            limits(),
            &Cancel::new(),
        );
        assert!(
            matches!(&result, Err(ProbabilityDocumentError::Unsupported)),
            "{formula}: {:?}",
            result.map(|_| ())
        );
    }
}

#[trace("TC-3184", "FR-201-AC-62")]
#[test]
fn bounded_nested_multiple_and_unresolved_forms_issue_no_proof_in_these_profiles() {
    let until = |scope: &str| {
        format!(
            "probabilistic V using inf on M over (x: M::Item) under {scope} on origin {{ probability >= rational(1,4) [ {} until {} ] }}\n",
            at(&[0, 3, 4]),
            at(&[1])
        )
    };
    let ev = "probabilistic V using ev on M over (x: M::Item) under every scheduler on origin";
    let settle_v = |world: &World| settle(world, "V", ModelCheckLimits::default(), |_| {});
    // Bounded forms are not these profiles: they keep the bounded document.
    for form in [
        format!("always[0,3] {}", at(&[0, 3, 4])),
        format!("eventually[0,3] {}", at(&[1])),
    ] {
        let world = World::new(&until_chain(&format!(
            "{ev} {{ probability >= rational(1,4) [ {form} ] }}\n"
        )));
        let profile = profile_of(&settle_v(&world)).expect("a bounded proof");
        assert!(!profile.contains("state-predicate"), "{form}: {profile}");
        assert_eq!(
            profile, "filament.probability-objective/reachability/v1",
            "{form}"
        );
    }
    // A nested temporal form over an event-position claim is unsupported.
    let nested = World::new(&until_chain(&format!(
        "{ev} {{ probability >= rational(1,4) [ eventually[0,3] always[0,1] {} ] }}\n",
        at(&[1])
    )));
    let record = settle_v(&nested);
    assert!(
        unsupported(&record) && profile_of(&record).is_none(),
        "{record:#?}"
    );
    // An unbounded nested form is refused by the front end before any proof.
    let refusals = World::try_new(&until_chain(&format!(
        "probabilistic V using inf on M over (x: M::Item) under every scheduler on origin {{ probability >= rational(1,4) [ always eventually {} ] }}\n",
        at(&[1])
    )))
    .err()
    .expect("the front end refuses an unbounded nested form");
    assert!(
        format!("{refusals:?}").contains("NotStatePredicate"),
        "{refusals:?}"
    );
    // Multiple bindings and multiple initial states are unsupported.
    let claims = until("every scheduler");
    let record = settle_v(&World::with_objects(&until_chain(&claims), &["x", "y"]));
    assert!(
        unsupported(&record) && profile_of(&record).is_none(),
        "{record:#?}"
    );
    let model = until_chain(&claims);
    let initials = [
        model.initial.clone(),
        serde_json::json!({"phase":{"integer":"3"}}),
    ];
    let record = settle_v(&World::with_initials(&model, &initials));
    assert!(
        unsupported(&record) && profile_of(&record).is_none(),
        "{record:#?}"
    );
    // A workload that leaves an operation unweighted is refused by the front end.
    let partial = until_chain(&format!(
        "workload Even on M {{ weight Item::go = 1; }}\n{}",
        until("Even")
    ));
    let refusals = World::try_new(&partial)
        .err()
        .expect("an incomplete workload is refused");
    assert!(
        format!("{refusals:?}").contains("IncompleteWorkload"),
        "{refusals:?}"
    );
    // A workload that names no declaration is refused by the front end. (One
    // that resolves every live state to one distribution is a supported
    // DTMC subject, as AC-58 and AC-64 state.)
    let refusals = World::try_new(&mdp_chain(&claim_text(
        "Missing",
        &format!("always {}", at(&[0, 1])),
    )))
    .err()
    .expect("an unresolved workload is refused");
    let text = format!("{refusals:?}");
    assert!(
        text.contains("UnresolvedWorkload") && text.contains("Missing"),
        "{text}"
    );
}

fn claim_text(scope: &str, event: &str) -> String {
    format!("probabilistic V using inf on M over (x: M::Item) under {scope} on origin {{ probability >= rational(1,2) [ {event} ] }}\n")
}

/// The least `accounting.work_units` under which `completes` holds.
fn least_work_units(completes: impl Fn(u64) -> bool) -> u64 {
    let (mut short, mut enough) = (0, 1_000_000);
    assert!(completes(enough), "the default allowance completes");
    while enough - short > 1 {
        let middle = short + (enough - short) / 2;
        if completes(middle) {
            enough = middle;
        } else {
            short = middle;
        }
    }
    enough
}

/// An allowance that pays the product build of `name`, and pays the reading of
/// whether its subject is acyclic, but not the one after the other.
fn one_short_of_the_build_and_the_cyclic_check(world: &World, name: &str) -> u64 {
    let build = least_work_units(|work_units| {
        let mut request = world.request(name, ModelCheckLimits::default());
        request.subject.simulation.meter.work_units = work_units;
        build_product(request, &Cancel::new()).is_ok()
    });
    let check = least_work_units(|work_units| {
        let mut request = world.request(name, ModelCheckLimits::default());
        request.subject.simulation.meter.work_units = work_units;
        let limits = request.limits;
        ModelSystem::new(request.subject.simulation).is_ok_and(|system| {
            fsl_replay::admit_acyclic_probabilistic_refutation_subject(
                &system,
                &limits,
                &Cancel::new(),
            )
            .is_ok()
        })
    });
    build + check - 1
}

fn assert_work_units_stop(record: &fsl_analyze::analyze::AnalyzeRecord, work_units: u64) {
    assert_eq!(
        record.terminal().expect("a settled claim").value(),
        &TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::EvaluationMeter {
                counter: filament_exact::LimitKind::WorkUnits
            },
            value: work_units,
            required: None,
        }),
        "{record:#?}"
    );
    assert!(record.exact_probabilistic.is_none());
}

/// Past the rational budget, the cyclic until item reads its subject's shape
/// under what its build left of `accounting.work_units`, not under a second
/// allowance.
#[trace("TC-638", "FR-203-AC-13")]
#[test]
fn the_cyclic_check_past_the_rational_budget_spends_the_items_work_allowance() {
    let world = World::new(&model(false, &claim("Likely", ">= rational(3,5)", UNTIL)));
    let work_units = one_short_of_the_build_and_the_cyclic_check(&world, "Likely");
    let record = settle(&world, "Likely", ModelCheckLimits::default(), |request| {
        request.limits.max_rational_bits = 1;
        request.subject.simulation.meter.work_units = work_units;
    });
    assert_work_units_stop(&record, work_units);
}

/// A failing unwindowed `always` reads its subject's shape, to choose its
/// refutation, under what its build left of `accounting.work_units`.
#[trace("TC-638", "FR-203-AC-13")]
#[test]
fn the_cyclic_check_of_a_refutation_spends_the_items_work_allowance() {
    let world = World::new(&model(
        false,
        &claim("Surely", ">= rational(4,5)", "always holds(x.phase <= 1)"),
    ));
    let record = settle(&world, "Surely", ModelCheckLimits::default(), |_| {});
    assert!(
        matches!(
            record.terminal().expect("a settled claim").value(),
            TerminalValue::Refuted
        ),
        "{record:#?}"
    );
    let work_units = one_short_of_the_build_and_the_cyclic_check(&world, "Surely");
    let record = settle(&world, "Surely", ModelCheckLimits::default(), |request| {
        request.subject.simulation.meter.work_units = work_units;
    });
    assert_work_units_stop(&record, work_units);
}
