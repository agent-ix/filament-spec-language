// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-201's closed cyclic until and always proof profiles: emission by EN-5,
//! the independent checker's own component classification, and limits.
use super::long_run_certificate::limits;
use crate::prob_world::{Model, Operation, World};
use filament_exact::{Cancel, CancelCause, Integer, Rational};
use filament_semantic_value::checking::CheckMode;
use fsl_analyze::analyze::{analyze_items, exact_probabilistic_item, Engines};
use fsl_eval::simulation::model::ModelSystem;
use fsl_replay::{
    check_cyclic_mdp_probability_certificate_observing, check_probability_certificate_document,
    probabilistic_claim_identity, read_probability_proof_document,
    write_probability_proof_document, CyclicMdpProbabilityPhase as P,
    ExactProbabilityCertificateCheck as Check, IncompleteCause, ModelCheckLimit, ModelCheckLimits,
    ProbabilityBoundSide, ProbabilityCertificateLocus as Locus, ProbabilityCertificateRule as Rule,
    ProbabilityDocumentError, ProbabilityDocumentLimit, ProbabilityObjective,
    ProbabilityStateEvent, TerminalValue,
};
use ix_trace_rs::trace;
use serde_json::Value;

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
        vec![exact_probabilistic_item(
            fsl_foundation::RequestIndex::new(0),
            request,
        )],
        &Cancel::new(),
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
        for tiny in [0, 1, 64, 512] {
            let mut small = limits();
            small.storage_bytes = tiny;
            assert!(
                !matches!(
                    check_with(&world, "Likely", &bytes, small),
                    Ok(Check::Accepted(_))
                ),
                "storage {tiny}"
            );
            let mut small = limits();
            small.bytes = tiny;
            assert!(
                !matches!(
                    check_with(&world, "Likely", &bytes, small),
                    Ok(Check::Accepted(_))
                ),
                "bytes {tiny}"
            );
        }
        let mut small = limits();
        small.accounting.integer_bits = 1;
        assert!(matches!(
            check_with(&world, "Likely", &bytes, small),
            Err(ProbabilityDocumentError::Accounting(_))
        ));
        let mut small = limits();
        small.accounting.work_units = 1;
        assert!(check_with(&world, "Likely", &bytes, small).is_err());
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
