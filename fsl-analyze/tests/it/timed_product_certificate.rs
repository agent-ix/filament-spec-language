// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-920, FR-921, FR-922, FR-923, FR-924 and FR-877 (TC-2489 to TC-2494;
//! FSpec FR-725, FR-751, FR-752, FR-753): EN-5's timed-product certificate
//! over ADR-028 §15.5's `Retx`, and the core checker that rebuilds the
//! digital or region product itself.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use filament_exact::{Cancel, Integer, Rational};
use fsl_analyze::analyze::{
    analyze_items, exact_probabilistic_item, exact_probabilistic_item_tampered, AnalyzeRecord,
    DispositionKind, Engines, Settlement,
};
use fsl_analyze::exact_probabilistic::{ExactProbLimits, ProductRequest};
use fsl_eval::model_subject::ModelSubject;
use fsl_foundation::RequestIndex;
use fsl_replay::{
    check_probability_certificate_document, CertificateRule, ExactProbabilityCertificate,
    ExactProbabilityCertificateCheck, InconclusiveCause, ProbabilityCertificateLocus,
    ProbabilityCertificateRule, ProbabilityDocumentError, ProofBasis, TerminalValue,
};
use ix_trace_rs::trace;
use serde_json::{json, Value};

use super::long_run_certificate::limits;
use crate::timed_world::{self, World};

const PROFILES: &str = "profile ev = \"filament.temporal.timed/v1\";\n\
     profile inf = \"filament.temporal.infinite-trace/v1\";\n";

const GUARD: &str = " and self.x >= 1 ms";
const STRICT: &str = " and self.x > 1 ms";
const TWO_DELAYS: &str = "delay M::Call::send ~ discrete { 1 ms: 1, 2 ms: 1 };\n";
const TOGGLE: &str = "pre Toggle using v on M::Call::toggle { true }\n\
     post Toggled using v on M::Call::toggle { self.retried = not self.retried }\n";
const ELAPSED: &str = "elapsed from holds(not c.late) until holds(c.late) within 4 ms";

fn ratio(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("nonzero")
}

fn deadline(name: &str, scheduler: &str, bound: &str) -> String {
    format!(
        "probabilistic {name} using ev on M over (c: M::Call) {scheduler} on origin {{ \
         probability {bound} [ eventually[0 ms, 4 ms] holds(c.late) ] }}\n"
    )
}

fn every_deadlines() -> String {
    format!(
        "{}{}{}",
        deadline("Deadline", "under every scheduler", ">= 0.99"),
        deadline("Max", "under every scheduler", "<= 0.9999"),
        deadline("Strict", "under every scheduler", ">= 0.995"),
    )
}

fn quantile(name: &str, q: &str, comparison: &str) -> String {
    format!(
        "probabilistic {name} using ev on M over (c: M::Call) under every scheduler on origin \
         {{ quantile {q} of ({ELAPSED}) {comparison} }}\n"
    )
}

/// `Retx` over the operations `operations` (`send` first): `late` is
/// `delivered`, `send(lost)` needs `x` past `guard`, resets `x` when
/// `reset`, loses with `1/10`, and the invariant `x <= 2 ms` holds until
/// delivery.
fn retx_with(
    operations: &[(&str, &[&str])],
    guard: &str,
    reset: bool,
    declarations: &str,
    claims: &str,
) -> World {
    retx_over(&["x"], operations, guard, reset, declarations, claims)
}

/// [`retx_with`] over the clocks `clocks`, `x` first.
fn retx_over(
    clocks: &[&str],
    operations: &[(&str, &[&str])],
    guard: &str,
    reset: bool,
    declarations: &str,
    claims: &str,
) -> World {
    let mut document: Value =
        serde_json::from_slice(&timed_world::document(clocks, operations)).expect("JSON");
    document["types"][1]["operations"][0]["params"] = json!([{
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
    let reset = if reset { " and self.x = 0 ms" } else { "" };
    let body = format!(
        "{}\
         pre Send using v on M::Call::send {{ not self.late{guard} }}\n\
         post Sent using v on M::Call::send {{ self.late = not lost{reset} }}\n\
         random M::Call::send(lost) ~ {{ true: 1, false: 9 }};\n\
         time invariant Deadline when {{ not self.late }} {{ self.x <= 2 ms }}\n\
         {declarations}{claims}",
        timed_world::DENSE,
    );
    timed_world::world(
        &serde_json::to_vec(&document).expect("serializes"),
        PROFILES,
        &body,
    )
}

fn retx(guard: &str, reset: bool, declarations: &str, claims: &str) -> World {
    retx_with(
        &[("send", &["late", "x"])],
        guard,
        reset,
        declarations,
        claims,
    )
}

fn request<'a>(world: &'a World, claim: &str) -> ProductRequest<'a> {
    ProductRequest {
        subject: ModelSubject {
            simulation: world.request(),
            proof_bounds: Vec::new(),
        },
        claim: world
            .package
            .graph()
            .probabilistic_model()
            .claim(claim)
            .expect("the claim checks"),
        limits: ExactProbLimits::default(),
    }
}

fn settle(world: &World, claim: &str) -> AnalyzeRecord {
    let engines = Engines::fsl().expect("the linked engines are admitted");
    let item = exact_probabilistic_item(RequestIndex::new(0), request(world, claim));
    analyze_items(&engines, vec![item], &Cancel::new()).remove(0)
}

fn settle_tampered(
    world: &World,
    claim: &str,
    tamper: impl Fn(&mut ExactProbabilityCertificate) + 'static,
) -> AnalyzeRecord {
    let engines = Engines::fsl().expect("the linked engines are admitted");
    let item =
        exact_probabilistic_item_tampered(RequestIndex::new(0), request(world, claim), tamper);
    analyze_items(&engines, vec![item], &Cancel::new()).remove(0)
}

/// The exact value of a `proved` record.
fn proved_value(record: &AnalyzeRecord) -> Rational {
    let terminal = record
        .terminal()
        .unwrap_or_else(|| panic!("expected a terminal record, got {record:#?}"));
    match terminal.value() {
        TerminalValue::Proved {
            basis: ProofBasis::ExactValue { value, .. },
            ..
        } => value.value().clone(),
        other => panic!("expected an exact proof, got {other:?}"),
    }
}

fn is_refuted(record: &AnalyzeRecord) -> bool {
    matches!(
        record.terminal().map(|terminal| terminal.value()),
        Some(TerminalValue::Refuted)
    )
}

/// The `timed-product/v1` document a proof retains.
fn document(record: &AnalyzeRecord) -> Value {
    let bytes = record
        .terminal()
        .and_then(|terminal| terminal.exact_certificate_document())
        .expect("every timed proof retains its certificate document");
    let document: Value = serde_json::from_slice(bytes).expect("a JSON document");
    assert_eq!(
        document["objective"]["profile"],
        "filament.probability-objective/timed-product/v1"
    );
    document
}

fn check(
    world: &World,
    claim: &str,
    document: &Value,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    let identity = fsl_replay::probabilistic_claim_identity(
        world
            .package
            .graph()
            .probabilistic_model()
            .claim(claim)
            .expect("the claim checks"),
    )
    .expect("an identity");
    check_probability_certificate_document(
        &world.system(),
        world.package.graph().probabilistic_model().claims(),
        &identity,
        &serde_json::to_vec(document).expect("serializes"),
        // The two-clock region product of FR-924-AC-6 has 217 states, more
        // than the 128 the shared document limits admit.
        fsl_replay::ProbabilityDocumentLimits {
            states: 4096,
            edges: 65_536,
            ..limits()
        },
        &Cancel::new(),
    )
}

fn accepted(world: &World, claim: &str, document: &Value) -> bool {
    matches!(
        check(world, claim, document),
        Ok(ExactProbabilityCertificateCheck::Accepted(_))
    )
}

fn rejected_rule(world: &World, claim: &str, document: &Value) -> ProbabilityCertificateRule {
    match check(world, claim, document) {
        Ok(ExactProbabilityCertificateCheck::Rejected(rejection)) => rejection.rule,
        other => panic!("expected a rejection, got {other:?}"),
    }
}

/// The certificate rule an item settled `inconclusive` with.
fn rejected_item_rule(record: &AnalyzeRecord) -> CertificateRule {
    match record.terminal().map(|terminal| terminal.value()) {
        Some(TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected {
            rule, ..
        })) => *rule,
        other => panic!("expected CertificateRejected, got {other:?}"),
    }
}

fn first_value_altered(document: &Value) -> Value {
    let mut altered = document.clone();
    altered["values"][0]["value"] = json!({"numerator": "1", "denominator": "7"});
    altered
}

/// Every value strictly between 0 and 1 set to `to`.
fn set_interior_values(to: Rational) -> impl Fn(&mut ExactProbabilityCertificate) {
    move |certificate| {
        for entry in &mut certificate.values {
            if entry.value > ratio(0, 1) && entry.value < ratio(1, 1) {
                entry.value = to.clone();
            }
        }
    }
}

// --- FR-920 (TC-2489) -------------------------------------------------------

/// FR-920-AC-1: the digital document names its route, unit, scale and form,
/// spans the clock ranges and is accepted.
#[trace("TC-2489", "FR-920-AC-1")]
#[trace("FSpec-TC-1767", "FSpec-FR-725-AC-1")]
#[test]
fn digital_document_names_route_scale_and_clocks() {
    let world = retx(GUARD, true, "", &every_deadlines());
    let record = settle(&world, "Deadline");
    assert_eq!(proved_value(&record), ratio(99, 100));
    let doc = document(&record);
    let timed = &doc["objective"]["timed"];
    assert_eq!(timed["route"], "digital");
    assert_eq!(timed["scale"], "1");
    assert_eq!(timed["form"], "deadline");
    assert!(accepted(&world, "Deadline", &doc));
}

/// FR-920-AC-2: a strict guard gives a region document whose keys carry a
/// fraction member.
#[trace("TC-2489", "FR-920-AC-2")]
#[trace("FSpec-TC-1767", "FSpec-FR-725-AC-2")]
#[test]
fn region_document_keeps_fraction_order() {
    let world = retx(STRICT, true, "", &every_deadlines());
    let doc = document(&settle(&world, "Deadline"));
    assert_eq!(doc["objective"]["timed"]["route"], "region");
    let rows = doc["values"].as_array().expect("values");
    let keys = rows
        .iter()
        .map(|row| {
            let encoded = row["state"].as_str().expect("encoded product key");
            let bytes = URL_SAFE_NO_PAD.decode(encoded).expect("base64 product key");
            serde_json::from_slice::<Value>(&bytes).expect("canonical product key")
        })
        .collect::<Vec<_>>();
    let unique: std::collections::BTreeSet<String> =
        keys.iter().map(serde_json::Value::to_string).collect();
    assert_eq!(unique.len(), rows.len());
    assert!(keys.iter().all(|key| key[4][1][0] == "region"));
    assert!(keys.iter().all(|key| {
        key[4][1][4].as_array().is_some_and(|clocks| {
            clocks.iter().all(|clock| {
                clock
                    .as_array()
                    .is_some_and(|parts| parts.len() == 4 && parts[3].as_str().is_some())
            })
        })
    }));
}

/// FR-920-AC-3: a document has no component member; adding one rejects.
#[trace("TC-2489", "FR-920-AC-3")]
#[trace("FSpec-TC-1767", "FSpec-FR-725-AC-3")]
#[test]
fn timed_document_has_no_component_member() {
    let world = retx(GUARD, true, "", &every_deadlines());
    let mut doc = document(&settle(&world, "Deadline"));
    doc["components"] = json!([]);
    assert!(!accepted(&world, "Deadline", &doc));
    // Retx's random parameter is checked but not yet emitted by this S4
    // profile; use a deterministic timed package to test that the certificate
    // profile itself is not part of package source.
    let emission_world = timed_world::world(
        &timed_world::document(&["x"], &[("send", &["late", "x"])]),
        PROFILES,
        &format!(
            "{}\
             pre Send using v on M::Call::send {{ not self.late and self.x >= 1 ms }}\n\
             post Sent using v on M::Call::send {{ self.late = true and self.x = 0 ms }}\n\
             time invariant Deadline when {{ not self.late }} {{ self.x <= 2 ms }}\n",
            timed_world::DENSE,
        ),
    );
    let emission =
        fsl_package::emit_checked(&emission_world.package, fsl_package::EmitLimits::fixture())
            .expect("S4 emits the deterministic timed unit");
    let text = String::from_utf8_lossy(emission.package().bytes()).into_owned();
    assert!(!text.contains("timed-product/v1"));
}

// --- FR-921 (TC-2490) -------------------------------------------------------

/// FR-921-AC-1: an altered value rejects `InequalityFails`; another identity
/// refuses before the rebuild.
#[trace("TC-2490", "FR-921-AC-1")]
#[trace("FSpec-TC-1768", "FSpec-FR-751-AC-1")]
#[test]
fn altered_timed_value_is_rejected() {
    let world = retx(GUARD, true, "", &every_deadlines());
    let doc = document(&settle(&world, "Deadline"));
    assert_eq!(
        rejected_rule(&world, "Deadline", &first_value_altered(&doc)),
        ProbabilityCertificateRule::InequalityFails
    );
    assert!(matches!(
        check(&world, "Max", &doc),
        Err(ProbabilityDocumentError::IdentityMismatch)
    ));
}

/// FR-921-AC-2: the stated route and scale are compared, never used; a
/// wrong cap is malformed.
#[trace("TC-2490", "FR-921-AC-2")]
#[trace("FSpec-TC-1768", "FSpec-FR-751-AC-2")]
#[test]
fn stated_route_and_scale_are_compared() {
    let world = retx(GUARD, true, "", &every_deadlines());
    let doc = document(&settle(&world, "Deadline"));
    for (member, value) in [("route", "region"), ("scale", "2")] {
        let mut changed = doc.clone();
        changed["objective"]["timed"][member] = json!(value);
        assert!(
            matches!(
                check(&world, "Deadline", &changed),
                Err(ProbabilityDocumentError::ObjectiveMismatch)
            ),
            "{member}"
        );
    }
}

/// FR-921-AC-3: rows cover exactly the rebuilt product.
#[trace("TC-2490", "FR-921-AC-3")]
#[trace("FSpec-TC-1768", "FSpec-FR-751-AC-3")]
#[test]
fn timed_rows_cover_exactly_the_rebuilt_product() {
    let world = retx(GUARD, true, "", &every_deadlines());
    let doc = document(&settle(&world, "Deadline"));
    let mut missing = doc.clone();
    missing["values"].as_array_mut().expect("values").pop();
    assert_eq!(
        rejected_rule(&world, "Deadline", &missing),
        ProbabilityCertificateRule::Malformed
    );
    // A row from the strict-guard product names a key this product never reaches.
    let strict = retx(STRICT, true, "", &every_deadlines());
    let foreign = document(&settle(&strict, "Deadline"))["values"][0].clone();
    let mut extra = doc.clone();
    extra["values"]
        .as_array_mut()
        .expect("values")
        .push(foreign);
    assert!(!accepted(&world, "Deadline", &extra));
}

// --- FR-922 (TC-2491) -------------------------------------------------------

/// FR-922-AC-1: a strict guard is proved over regions at both extrema.
#[trace("TC-2491", "FR-922-AC-1")]
#[trace("FSpec-TC-1769", "FSpec-FR-752-AC-1")]
#[test]
fn strict_guard_is_certified_over_regions() {
    let world = retx(STRICT, true, "", &every_deadlines());
    for (claim, value) in [("Deadline", ratio(99, 100)), ("Max", ratio(999, 1000))] {
        let record = settle(&world, claim);
        assert_eq!(proved_value(&record), value, "{claim}");
        let doc = document(&record);
        assert_eq!(doc["objective"]["timed"]["route"], "region");
        assert!(accepted(&world, claim, &doc), "{claim}");
    }
}

/// FR-922-AC-2: a digital route is refused, and the closed guard's values
/// are rejected, for a strict guard.
#[trace("TC-2491", "FR-922-AC-2")]
#[trace("FSpec-TC-1769", "FSpec-FR-752-AC-2")]
#[test]
fn digital_values_for_a_strict_guard_are_rejected() {
    let world = retx(STRICT, true, "", &every_deadlines());
    let mut doc = document(&settle(&world, "Deadline"));
    doc["objective"]["timed"]["route"] = json!("digital");
    assert!(matches!(
        check(&world, "Deadline", &doc),
        Err(ProbabilityDocumentError::ObjectiveMismatch)
    ));
    // The closed guard's maximum 9999/10000 stated at the initial state.
    let record = settle_tampered(&world, "Max", |certificate| {
        for entry in &mut certificate.values {
            if entry.value == ratio(999, 1000) {
                entry.value = ratio(9999, 10000);
            }
        }
    });
    assert_eq!(
        rejected_item_rule(&record),
        CertificateRule::InequalityFails
    );
}

/// FR-922-AC-3: expected elapsed time over a strict guard is certified over
/// boundary regions.
#[trace("TC-2491", "FR-922-AC-3")]
#[test]
fn strict_mean_time_is_certified_over_boundary_regions() {
    let world = retx(
        STRICT,
        true,
        "",
        "probabilistic MeanTime using inf on M over (c: M::Call) under every scheduler on origin \
         { expected elapsed until holds(c.late) <= 3 ms }\n",
    );
    let record = settle(&world, "MeanTime");
    assert_eq!(proved_value(&record), ratio(20, 9));
    let doc = document(&record);
    assert_eq!(doc["objective"]["timed"]["form"], "expected-elapsed");
    let encoded = doc["values"][0]["state"]
        .as_str()
        .expect("encoded boundary-region key");
    let bytes = URL_SAFE_NO_PAD.decode(encoded).expect("base64 ProductKey");
    let key: Value = serde_json::from_slice(&bytes).expect("canonical ProductKey");
    assert_eq!(key[4][1][0], "boundary-region");
    assert!(accepted(&world, "MeanTime", &doc));
}

// --- FR-923 (TC-2492) -------------------------------------------------------

/// A `discrete` delay over every scheduler leaves no choice: minimum and
/// maximum are the workload's value. This is the proof half of FR-923-AC-1,
/// whose refutation half is the ignored test below, so the criterion is
/// tagged there.
#[trace("TC-2492")]
#[trace("FSpec-TC-1769", "FSpec-FR-752-AC-3")]
#[test]
fn every_scheduler_discrete_delay_is_certified() {
    let world = retx(GUARD, true, TWO_DELAYS, &every_deadlines());
    let record = settle(&world, "Deadline");
    assert_eq!(proved_value(&record), ratio(159_129, 160_000));
    let doc = document(&record);
    assert!(accepted(&world, "Deadline", &doc));
}

/// FR-923-AC-1: a bound above the discrete delay's exact value is refuted.
#[trace("TC-2492", "FR-923-AC-1")]
#[test]
fn every_scheduler_discrete_delay_refutes() {
    let world = retx(GUARD, true, TWO_DELAYS, &every_deadlines());
    let record = settle(&world, "Strict");
    assert!(is_refuted(&record), "{record:#?}");
    let mass = record
        .probabilistic_counterexample
        .as_ref()
        .expect("the refutation carries its witness")
        .mass()
        .clone();
    assert_eq!(mass, ratio(871, 160_000));
}

/// FR-923-AC-1: a refutation is evidence only when the checker's own replay of
/// the timed product reproduces it, so a witness whose path does not end in the
/// forbidden decided state does not settle `refuted`.
#[trace("TC-2492", "FR-923-AC-1")]
#[test]
fn altered_timed_witness_is_not_refuted() {
    let world = retx(GUARD, true, TWO_DELAYS, &every_deadlines());
    let record = settle_witness_tampered(&world, "Strict", |witness| {
        let initial = witness.initial_key.clone();
        for path in &mut witness.paths {
            path.steps.last_mut().expect("a step").post_key = initial.clone();
        }
    });
    assert!(!is_refuted(&record), "{record:#?}");
    assert!(record.probabilistic_counterexample.is_none());
}

/// FR-921: the clock valuation a digital key names is placed by clock identity
/// in the model's place order, never by the position of the clock in the key.
/// The same two-clock key with its clocks listed in the opposite order names
/// the same valuation, and each value is the one its clock holds.
#[trace("TC-2490", "FR-921-AC-2")]
#[test]
fn a_digital_key_valuation_does_not_depend_on_the_key_clock_order() {
    let world = retx_over(
        &["x", "a"],
        &[("send", &["late", "x", "a"])],
        " and self.x >= 1 ms and self.a >= 0 ms",
        true,
        TWO_DELAYS,
        &every_deadlines(),
    );
    let doc = document(&settle(&world, "Deadline"));
    assert_eq!(doc["objective"]["timed"]["route"], "digital");
    let claim = world
        .package
        .graph()
        .probabilistic_model()
        .claim("Deadline")
        .expect("the claim checks");
    let system = world.system();
    let valuation = |key: &Value| {
        fsl_replay::digital_key_clocks(
            &system,
            claim,
            &fsl_replay::ProbabilityProductKey::from_canonical_bytes(
                serde_json::to_vec(key).expect("serializes"),
            ),
        )
        .expect("a digital key")
        .values()
        .to_vec()
    };
    let timed = fsl_eval::simulation::model::TimedSystem::new(&system, &claim.subject.alias)
        .expect("a timed subject");
    let places: Vec<String> = timed
        .places()
        .iter()
        .map(|place| {
            timed
                .model()
                .clock(place.clock())
                .expect("a clock")
                .name()
                .to_string()
        })
        .collect();
    let mut distinct = false;
    for entry in doc["values"].as_array().expect("values") {
        let encoded = entry["state"].as_str().expect("encoded key");
        let bytes = URL_SAFE_NO_PAD.decode(encoded).expect("base64 key");
        let key: Value = serde_json::from_slice(&bytes).expect("canonical key");
        let clocks = key[4][1][4].as_array().expect("clocks").clone();
        let scale: i64 = key[4][1][3]
            .as_str()
            .expect("scale")
            .parse()
            .expect("number");
        let whole = |name: &str| -> i64 {
            clocks
                .iter()
                .find(|clock| clock[0][1] == name)
                .and_then(|clock| clock[1].as_str())
                .expect("the clock is keyed")
                .parse()
                .expect("whole ticks")
        };
        distinct |= whole("x") != whole("a");
        let mut reversed = key.clone();
        reversed[4][1][4] = Value::Array(clocks.iter().rev().cloned().collect());
        let straight = valuation(&key);
        assert_eq!(straight, valuation(&reversed));
        let held: Vec<_> = places
            .iter()
            .map(|name| ratio(whole(name), scale))
            .collect();
        assert_eq!(straight, held, "each place holds its own clock's ticks");
    }
    assert!(distinct, "some state must tell the two clocks apart");
}

fn settle_witness_tampered(
    world: &World,
    claim: &str,
    tamper: impl Fn(&mut fsl_replay::ProbabilisticCounterexample),
) -> AnalyzeRecord {
    let engines = Engines::fsl().expect("the linked engines are admitted");
    let item = fsl_analyze::analyze::exact_probabilistic_item_witness_tampered(
        RequestIndex::new(0),
        request(world, claim),
        tamper,
    );
    analyze_items(&engines, vec![item], &Cancel::new()).remove(0)
}

/// FR-923-AC-1: a forged path into a decided state the chosen choice has no
/// outcome to is not reported: the mass is the checker graph's own and the
/// evidence keeps only the paths the checker validated.
#[trace("TC-2492", "FR-923-AC-1")]
#[test]
fn a_forged_decided_target_is_neither_counted_nor_reported() {
    let world = retx(GUARD, true, TWO_DELAYS, &every_deadlines());
    let honest = settle(&world, "Strict");
    let honest_witness = honest
        .probabilistic_counterexample
        .as_ref()
        .expect("the refutation carries its witness");
    let paths = honest_witness.evidence().paths.len();
    assert!(paths >= 2, "the forgery needs a second honest path");
    // Replace the decided end of one honest path by the decided end of another.
    // Where the new end is no outcome of the chosen choice the path leaves the
    // evidence; wherever the witness still refutes, the mass is the honest one.
    let mut dropped_one = false;
    for victim in 0..paths {
        for source in 0..paths {
            let forged = settle_witness_tampered(&world, "Strict", |witness| {
                let other = witness.paths[source]
                    .steps
                    .last()
                    .expect("a step")
                    .post_key
                    .clone();
                witness.paths[victim]
                    .steps
                    .last_mut()
                    .expect("a step")
                    .post_key = other;
            });
            if let Some(reproduced) = forged.probabilistic_counterexample.as_ref() {
                assert_eq!(reproduced.mass(), honest_witness.mass());
                assert!(reproduced.evidence().paths.len() <= paths);
                dropped_one |= reproduced.evidence().paths.len() < paths;
            }
        }
    }
    assert!(dropped_one, "some forged end must leave the evidence");
}

/// FR-923-AC-3: a witness that takes the tie's NON-failing ordering does not
/// reach the bound, whatever claim it is replayed against.
#[trace("TC-2492", "FR-923-AC-3")]
#[test]
fn a_witness_of_the_non_failing_choice_is_not_refuted() {
    let claim = |name: &str, comparison: &str| {
        format!(
            "probabilistic {name} using ev on M over (c: M::Call) under every scheduler on origin \
             {{ probability {comparison} [ eventually[0 ms, 5 ms] holds(c.retried) ] }}\n"
        )
    };
    let world = retx_with(
        &[("send", &["late", "x"]), ("tick", &["retried"])],
        GUARD,
        true,
        "delay M::Call::send ~ discrete { 2 ms: 1 };\n\
         pre Tick using v on M::Call::tick { not self.late and not self.retried and self.x >= 2 ms }\n\
         post Ticked using v on M::Call::tick { self.retried = true }\n",
        &format!("{}{}", claim("Most", "<= 0.6"), claim("Least", ">= 0.9")),
    );
    let most = settle(&world, "Most");
    assert!(is_refuted(&most), "{most:#?}");
    let least = settle(&world, "Least");
    assert!(is_refuted(&least), "{least:#?}");
    // The `Least` witness runs the ordering that never retries.
    let other = least
        .probabilistic_counterexample
        .as_ref()
        .expect("the refutation carries its witness")
        .evidence()
        .clone();
    let transplanted = settle_witness_tampered(&world, "Most", |witness| {
        let item = witness.item.clone();
        *witness = other.clone();
        witness.item = item;
    });
    assert!(!is_refuted(&transplanted), "{transplanted:#?}");
    assert!(transplanted.probabilistic_counterexample.is_none());
}

/// FR-923-AC-2: values that read the drawn delay as a free choice are
/// rejected.
#[trace("TC-2492", "FR-923-AC-2")]
#[trace("FSpec-TC-1769", "FSpec-FR-752-AC-4")]
#[test]
fn distribution_free_values_are_rejected() {
    let world = retx(GUARD, true, TWO_DELAYS, &every_deadlines());
    let record = settle_tampered(&world, "Deadline", |certificate| {
        for entry in &mut certificate.values {
            if entry.value == ratio(159_129, 160_000) {
                entry.value = ratio(99, 100);
            }
        }
    });
    assert_eq!(
        rejected_item_rule(&record),
        CertificateRule::InequalityFails
    );
}

/// FR-923-AC-3: over every scheduler a tie at the least delay is the
/// scheduler's choice, so the maximum is 1 and a bound of `<= 0.6` is refuted.
#[trace("TC-2492", "FR-923-AC-3")]
#[test]
fn every_scheduler_tie_is_the_schedulers_choice() {
    // `send` is drawn at 2 ms; `tick` is free but can only fire at x = 2 ms.
    let claim = |name: &str, scheduler: &str| {
        format!(
            "probabilistic {name} using ev on M over (c: M::Call) {scheduler} on origin \
             {{ probability <= 0.6 [ eventually[0 ms, 5 ms] holds(c.retried) ] }}\n"
        )
    };
    let world = retx_with(
        &[("send", &["late", "x"]), ("tick", &["retried"])],
        GUARD,
        true,
        "delay M::Call::send ~ discrete { 2 ms: 1 };\n\
         pre Tick using v on M::Call::tick { not self.late and not self.retried and self.x >= 2 ms }\n\
         post Ticked using v on M::Call::tick { self.retried = true }\n\
         workload Both on M { weight Call::send = 1; weight Call::tick = 1; }\n",
        &format!(
            "{}{}",
            claim("Every", "under every scheduler"),
            claim("Weighted", "under Both")
        ),
    );
    let record = settle(&world, "Every");
    assert!(is_refuted(&record), "{record:#?}");
}

/// FR-923-AC-3: the same tie under the workload `Both` is resolved by its
/// weights and gives `21/40`.
#[trace("TC-2492", "FR-923-AC-3")]
#[test]
fn a_workload_resolves_a_timed_tie_by_its_weights() {
    let claim = |name: &str, scheduler: &str| {
        format!(
            "probabilistic {name} using ev on M over (c: M::Call) {scheduler} on origin \
             {{ probability <= 0.6 [ eventually[0 ms, 5 ms] holds(c.retried) ] }}\n"
        )
    };
    let world = retx_with(
        &[("send", &["late", "x"]), ("tick", &["retried"])],
        GUARD,
        true,
        "delay M::Call::send ~ discrete { 2 ms: 1 };\n\
         pre Tick using v on M::Call::tick { not self.late and not self.retried and self.x >= 2 ms }\n\
         post Ticked using v on M::Call::tick { self.retried = true }\n\
         workload Both on M { weight Call::send = 1; weight Call::tick = 1; }\n",
        &format!(
            "{}{}",
            claim("Every", "under every scheduler"),
            claim("Weighted", "under Both")
        ),
    );
    let record = settle(&world, "Weighted");
    assert_eq!(proved_value(&record), ratio(21, 40));
    // The independent checker accepts the document under the workload's own
    // spelling of the objective.
    let doc = document(&record);
    let optimization = doc["objective"]["optimization"].as_str().expect("text");
    // The tie leaves the scheduler a choice (free tick against send, then
    // unit against idle delay), so a `<=` bound is spelled by the maximum.
    assert_eq!(optimization, "workload-maximum");
    assert!(accepted(&world, "Weighted", &doc));
    for other in ["workload", "workload-minimum"] {
        assert!(
            objective_mismatch(&world, "Weighted", &relabelled(&doc, other)),
            "{other}"
        );
    }
}

/// The FR-923-AC-3 world with a workload claim of bound `bound`.
fn weighted_tie(bound: &str) -> World {
    retx_with(
        &[("send", &["late", "x"]), ("tick", &["retried"])],
        GUARD,
        true,
        "delay M::Call::send ~ discrete { 2 ms: 1 };\n\
         pre Tick using v on M::Call::tick { not self.late and not self.retried and self.x >= 2 ms }\n\
         post Ticked using v on M::Call::tick { self.retried = true }\n\
         workload Both on M { weight Call::send = 1; weight Call::tick = 1; }\n",
        &format!(
            "probabilistic Weighted using ev on M over (c: M::Call) under Both on origin \
             {{ probability {bound} [ eventually[0 ms, 5 ms] holds(c.retried) ] }}\n"
        ),
    )
}

/// FR-923-AC-3: a workload bound its value `21/40` misses is refuted with the
/// checker's own exact mass of the forbidden outcome.
#[trace("TC-2492", "FR-923-AC-3")]
#[test]
fn a_workload_refutes_a_timed_deadline_with_its_exact_mass() {
    let world = weighted_tie("<= 0.5");
    let record = settle(&world, "Weighted");
    assert!(is_refuted(&record), "{record:#?}");
    let mass = record
        .probabilistic_counterexample
        .as_ref()
        .expect("the refutation carries its witness")
        .mass()
        .clone();
    assert_eq!(mass, ratio(21, 40));
}

/// FR-923-AC-3: a workload value altered in the certificate is rejected.
#[trace("TC-2492", "FR-923-AC-3")]
#[test]
fn a_tampered_workload_certificate_is_rejected() {
    let world = weighted_tie("<= 0.6");
    let record = settle_tampered(&world, "Weighted", |certificate| {
        for entry in &mut certificate.values {
            if entry.value == ratio(21, 40) {
                entry.value = ratio(1, 2);
            }
        }
    });
    assert_eq!(
        rejected_item_rule(&record),
        CertificateRule::InequalityFails
    );
    let doc = document(&settle(&world, "Weighted"));
    assert!(!accepted(&world, "Weighted", &first_value_altered(&doc)));
}

/// FR-923-AC-3: a witness altered after production cannot change a workload
/// refutation: the mass is the checker graph's own, so a path that no longer
/// ends in the forbidden state is not reported.
#[trace("TC-2492", "FR-923-AC-3")]
#[test]
fn an_altered_workload_witness_is_not_refuted() {
    let world = weighted_tie("<= 0.5");
    let record = settle_witness_tampered(&world, "Weighted", |witness| {
        let initial = witness.initial_key.clone();
        for path in &mut witness.paths {
            path.steps.last_mut().expect("a step").post_key = initial.clone();
        }
    });
    assert!(!is_refuted(&record), "{record:#?}");
    assert!(record.probabilistic_counterexample.is_none());
}

/// `doc` with its optimization spelled `spelling`.
fn relabelled(doc: &Value, spelling: &str) -> Value {
    let mut altered = doc.clone();
    altered["objective"]["optimization"] = json!(spelling);
    altered
}

/// Whether the checker rejects `doc` for the spelling of its optimization.
fn objective_mismatch(world: &World, claim: &str, doc: &Value) -> bool {
    matches!(
        check(world, claim, doc),
        Err(ProbabilityDocumentError::OptimizationMismatch)
    )
}

/// The spelling a proof's document states.
fn spelling(record: &AnalyzeRecord) -> String {
    document(record)["objective"]["optimization"]
        .as_str()
        .expect("text")
        .to_owned()
}

/// The mass of a refutation's witness.
fn witness_mass(record: &AnalyzeRecord) -> Rational {
    record
        .probabilistic_counterexample
        .as_ref()
        .expect("the refutation carries its witness")
        .mass()
        .clone()
}

/// `tick` is free and guarded `self.x >= 1 ms` (window 1 to 2 ms) while `send`
/// is given `delays`, under `Both`: the scheduler picks tick at 1 or at 2, so
/// the minimum is below the maximum. `claims` are `(name, bound)` pairs over
/// `eventually[0 ms, 5 ms] holds(c.retried)`.
fn free_tick_world(delays: &str, claims: &[(&str, &str)]) -> World {
    let claims: String = claims
        .iter()
        .map(|(name, bound)| {
            format!(
                "probabilistic {name} using ev on M over (c: M::Call) under Both on origin \
                 {{ probability {bound} [ eventually[0 ms, 5 ms] holds(c.retried) ] }}\n"
            )
        })
        .collect();
    retx_with(
        &[("send", &["late", "x"]), ("tick", &["retried"])],
        GUARD,
        true,
        &format!(
            "delay M::Call::send ~ discrete {delays};\n\
             pre Tick using v on M::Call::tick {{ not self.late and not self.retried and self.x >= 1 ms }}\n\
             post Ticked using v on M::Call::tick {{ self.retried = true }}\n\
             workload Both on M {{ weight Call::send = 1; weight Call::tick = 1; }}\n"
        ),
        &claims,
    )
}

/// FR-920-AC-4: over a workload that leaves the scheduler a delay choice, a
/// `>=` bound is spelled and decided by the minimum and a `<=` bound by the
/// maximum, and the other spellings are not the checker's own. With `send`
/// given 1 or 2 ms the minimum is `8641/32000` and the maximum
/// `7876923/10240000`.
#[trace("TC-2489", "FR-920-AC-4")]
#[test]
fn a_workload_with_a_delay_choice_is_decided_on_its_extremum() {
    let world = free_tick_world(
        "{ 1 ms: 1, 2 ms: 1 }",
        &[
            ("AtLeastMin", ">= 0.27003125"),
            ("AtMostMax", "<= 0.76923076171875"),
        ],
    );
    let least = settle(&world, "AtLeastMin");
    assert_eq!(proved_value(&least), ratio(8641, 32000));
    assert_eq!(spelling(&least), "workload-minimum");
    assert!(accepted(&world, "AtLeastMin", &document(&least)));
    let most = settle(&world, "AtMostMax");
    assert_eq!(proved_value(&most), ratio(7_876_923, 10_240_000));
    assert_eq!(spelling(&most), "workload-maximum");
    assert!(accepted(&world, "AtMostMax", &document(&most)));
    for other in ["workload", "workload-maximum"] {
        assert!(
            objective_mismatch(&world, "AtLeastMin", &relabelled(&document(&least), other)),
            "{other}"
        );
    }
    for other in ["workload", "workload-minimum"] {
        assert!(
            objective_mismatch(&world, "AtMostMax", &relabelled(&document(&most), other)),
            "{other}"
        );
    }
}

/// The world where `send` is given 2 ms: the minimum is `21/40` (tick at 2,
/// the tie) and the maximum 1 (tick at 1).
fn between_world() -> World {
    free_tick_world(
        "{ 2 ms: 1 }",
        &[("AtLeastBetween", ">= 0.8"), ("AtMostBetween", "<= 0.8")],
    )
}

/// FR-920-AC-4: a bound strictly between the extrema is refuted with the mass
/// the checker recomputes for the extremum's scheduler: `>= 0.8` over the
/// minimum `21/40` fails with `19/40` (not retrying), `<= 0.8` under the
/// maximum 1 fails with 1.
#[trace("TC-2489", "FR-920-AC-4")]
#[test]
fn a_bound_between_the_extrema_is_refuted_with_the_recomputed_mass() {
    let world = between_world();
    let below = settle(&world, "AtLeastBetween");
    assert!(is_refuted(&below), "{below:#?}");
    // The witness scheduler's failing mass, recomputed by the checker, is
    // above the bound's `1 - 0.8` and never above the best a scheduler does.
    let mass = witness_mass(&below);
    assert!(mass > ratio(1, 5) && mass <= ratio(19, 40), "{mass:?}");
    let above = settle(&world, "AtMostBetween");
    assert!(is_refuted(&above), "{above:#?}");
    assert_eq!(witness_mass(&above), ratio(1, 1));
}

/// FR-920-AC-4: a witness that picks tick at 2 (the minimising choice)
/// cannot refute the bound the maximum fails.
#[trace("TC-2489", "FR-920-AC-4")]
#[test]
fn a_witness_of_the_other_extremum_settles_parity() {
    let world = between_world();
    let low = settle(&world, "AtLeastBetween")
        .probabilistic_counterexample
        .as_ref()
        .expect("the refutation carries its witness")
        .evidence()
        .clone();
    let transplanted = settle_witness_tampered(&world, "AtMostBetween", |witness| {
        let item = witness.item.clone();
        *witness = low.clone();
        witness.item = item;
    });
    assert!(transplanted.probabilistic_counterexample.is_none());
    assert!(
        matches!(
            transplanted.terminal().map(|terminal| terminal.value()),
            Some(TerminalValue::Inconclusive(
                InconclusiveCause::ReplayParity(_)
            ))
        ),
        "{transplanted:#?}"
    );
}

/// FR-920-AC-4: a workload whose product has one choice at every state is
/// spelled `workload`, and a residual spelling is not its own.
#[trace("TC-2489", "FR-920-AC-4")]
#[test]
fn a_workload_with_no_choice_is_spelled_workload() {
    let world = retx_with(
        &[("send", &["late", "x"])],
        GUARD,
        true,
        TWO_DELAYS,
        &format!(
            "workload Both on M {{ weight Call::send = 1; }}\n{}",
            deadline("Chain", "under Both", ">= 0.5")
        ),
    );
    let record = settle(&world, "Chain");
    let doc = document(&record);
    assert_eq!(spelling(&record), "workload");
    assert!(accepted(&world, "Chain", &doc));
    for other in ["workload-minimum", "workload-maximum"] {
        assert!(
            objective_mismatch(&world, "Chain", &relabelled(&doc, other)),
            "{other}"
        );
    }
}

/// The unsupported cause an item settled with.
fn unsupported_cause(record: &AnalyzeRecord) -> String {
    let Settlement::Disposed(disposed) = &record.settlement else {
        panic!("expected a disposition, got {record:#?}");
    };
    assert_eq!(disposed.kind, DispositionKind::Unsupported);
    disposed
        .diagnostic
        .as_ref()
        .expect("a cause")
        .code
        .cause()
        .to_owned()
}

/// FR-204 remaining scope: free `send` and `tick` are both enabled now and
/// `tick` disables `send`. FR-204 read literally (a unit delay and one
/// weighted resolution) proves `<= 0.6` with maximum 1/2; ADR-026 SD-3 (the
/// scheduler picks each free delay) refutes it with maximum 1. The engine
/// refuses, naming the cause, and the checker does the same on a document it
/// is given.
#[trace("TC-2492")]
#[test]
fn several_free_identities_enabled_now_are_refused_not_decided() {
    let world = |scheduler: &str| {
        retx_with(
            &[("send", &["late", "x"]), ("tick", &["retried"])],
            " and self.x >= 1 ms and not self.retried",
            true,
            "pre Tick using v on M::Call::tick { not self.late and not self.retried and self.x >= 1 ms }\n\
             post Ticked using v on M::Call::tick { self.retried = true }\n\
             workload Both on M { weight Call::send = 1; weight Call::tick = 1; }\n",
            &format!(
                "probabilistic W using ev on M over (c: M::Call) {scheduler} on origin \
                 {{ probability <= 0.9999 [ eventually[0 ms, 4 ms] holds(c.late) ] }}\n"
            ),
        )
    };
    let record = settle(&world("under Both"), "W");
    assert_eq!(unsupported_cause(&record), "workload-free-delays-race");
    assert!(record.terminal().is_none());
    // The checker, given a document of the same claim under the workload.
    let mut doc = document(&settle(&world("under every scheduler"), "W"));
    doc["objective"]["optimization"] = json!("workload-maximum");
    let workload = r#"{"ordinal":0,"package":"package","path":["probabilistic","workload",{"ordinal":0,"package":"test/timed","path":["model","object_type","ix://test/timed/Call"]},"Both","send*1,tick*1"]}"#;
    doc["identity"] = serde_json::from_str(&doc["identity"].to_string().replace(
        r#""scheduler","every""#,
        &format!(r#""scheduler",{workload}"#),
    ))
    .expect("identity JSON");
    let result = check(&world("under Both"), "W", &doc);
    assert!(
        matches!(
            result,
            Err(ProbabilityDocumentError::WorkloadFreeDelays(
                fsl_replay::WorkloadFreeDelayRefusal::ManyFreeEnabled
            ))
        ),
        "{result:?}"
    );
}

/// FR-204 remaining scope: the one free identity enabled now is disabled by
/// the unit delay, which FR-204's resolution and ADR-026 SD-3 read differently.
#[trace("TC-2492")]
#[test]
fn a_free_identity_the_unit_delay_disables_is_refused_not_decided() {
    let world = retx_with(
        &[("send", &["late", "x"]), ("tick", &["retried"])],
        GUARD,
        true,
        "pre Tick using v on M::Call::tick { not self.late and not self.retried and self.x <= 0 ms }\n\
         post Ticked using v on M::Call::tick { self.retried = true }\n\
         workload Both on M { weight Call::send = 1; weight Call::tick = 1; }\n",
        "probabilistic W using ev on M over (c: M::Call) under Both on origin \
         { probability <= 0.6 [ eventually[0 ms, 4 ms] holds(c.retried) ] }\n",
    );
    let record = settle(&world, "W");
    assert_eq!(
        unsupported_cause(&record),
        "workload-free-delay-window-closes"
    );
}

/// FR-204: one free identity that stays enabled through the unit delay is
/// still decided, by the minimum over its choices.
#[trace("TC-2492")]
#[test]
fn one_free_identity_that_stays_enabled_is_decided() {
    let world = retx_with(
        &[("send", &["late", "x"])],
        GUARD,
        true,
        "workload Both on M { weight Call::send = 1; }\n",
        &deadline("Free", "under Both", ">= 0.5"),
    );
    let record = settle(&world, "Free");
    assert!(proved_value_of(&record).is_some(), "{record:#?}");
    assert_eq!(spelling(&record), "workload-minimum");
    assert!(accepted(&world, "Free", &document(&record)));
}

// --- FR-924 (TC-2493) -------------------------------------------------------

fn toggling() -> World {
    retx_with(
        &[("send", &["late", "x"]), ("toggle", &["retried"])],
        GUARD,
        true,
        TOGGLE,
        &every_deadlines(),
    )
}

/// FR-924-AC-1: zero-time toggle components take their best exit.
#[trace("TC-2493", "FR-924-AC-1")]
#[trace("FSpec-TC-1820", "FSpec-FR-753-AC-1")]
#[test]
fn zero_time_components_take_their_best_exit() {
    let world = toggling();
    let record = settle(&world, "Deadline");
    assert_eq!(proved_value(&record), ratio(99, 100));
    assert!(accepted(&world, "Deadline", &document(&record)));
    assert_eq!(proved_value(&settle(&world, "Max")), ratio(9999, 10000));
}

/// FR-924-AC-2: the toggle-forever (Zeno) values are rejected.
#[trace("TC-2493", "FR-924-AC-2")]
#[trace("FSpec-TC-1820", "FSpec-FR-753-AC-2")]
#[test]
fn zeno_values_are_rejected() {
    let world = toggling();
    let record = settle_tampered(&world, "Deadline", set_interior_values(ratio(0, 1)));
    assert_eq!(
        rejected_item_rule(&record),
        CertificateRule::InequalityFails
    );
}

/// FR-924-AC-3: a retry cycle holds delays and is not collapsed.
#[trace("TC-2493", "FR-924-AC-3")]
#[trace("FSpec-TC-1820", "FSpec-FR-753-AC-3")]
#[test]
fn delay_cycles_are_not_collapsed() {
    let world = retx(
        GUARD,
        true,
        "",
        "probabilistic MeanLow using inf on M over (c: M::Call) under every scheduler on origin \
         { expected elapsed until holds(c.late) >= 1 ms }\n",
    );
    let record = settle(&world, "MeanLow");
    assert_eq!(proved_value(&record), ratio(10, 9));
    assert!(accepted(&world, "MeanLow", &document(&record)));
    let tampered = settle_tampered(&world, "MeanLow", |certificate| {
        for entry in &mut certificate.values {
            if entry.value == ratio(10, 9) {
                entry.value = ratio(1, 1);
            }
        }
    });
    assert_eq!(
        rejected_item_rule(&tampered),
        CertificateRule::InequalityFails
    );
}

/// FR-924-AC-4: a zero-time cycle every action leaves is no end component.
#[trace("TC-2493", "FR-924-AC-4")]
#[test]
fn zero_time_cycle_without_end_component_is_certified() {
    let world = retx("", false, "", &every_deadlines());
    let record = settle(&world, "Deadline");
    assert_eq!(proved_value(&record), ratio(1, 1));
    assert!(accepted(&world, "Deadline", &document(&record)));
}

/// FR-924-AC-5: a time-locked initial state is terminal.
#[trace("TC-2493", "FR-924-AC-5")]
#[trace("FSpec-TC-1820", "FSpec-FR-753-AC-6")]
#[test]
fn time_locked_state_is_certified_terminal() {
    let world = retx(
        GUARD,
        true,
        "delay M::Call::send ~ discrete { 5 ms: 1 };\nworkload One on M { weight Call::send = 1; }\n",
        &deadline("UnderMax", "under One", "<= 0.999"),
    );
    let record = settle(&world, "UnderMax");
    assert_eq!(proved_value(&record), ratio(0, 1));
    assert!(accepted(&world, "UnderMax", &document(&record)));
    let tampered = settle_tampered(&world, "UnderMax", |certificate| {
        for entry in &mut certificate.values {
            entry.value = ratio(1, 1);
        }
    });
    assert_eq!(
        rejected_item_rule(&tampered),
        CertificateRule::InequalityFails
    );
}

// --- FR-877 (TC-2494) -------------------------------------------------------

fn quantiles() -> World {
    retx(
        GUARD,
        true,
        "",
        &[
            quantile("AtMostTwo", "0.5", "<= 2 ms"),
            quantile("AtMostOne", "0.5", "<= 1 ms"),
            quantile("AtLeastOne", "0.5", ">= 1 ms"),
            quantile("Tight", "0.9", ">= 2 ms"),
        ]
        .concat(),
    )
}

/// FR-877-AC-1: `<= 2 ms` is proved with the transform's minimum `2/5`.
#[trace("TC-2494", "FR-877-AC-1")]
#[trace("FSpec-TC-1820", "FSpec-FR-753-AC-4")]
#[test]
fn elapsed_quantile_at_most_is_certified() {
    let world = quantiles();
    let record = settle(&world, "AtMostTwo");
    assert!(matches!(
        record.terminal().map(|terminal| terminal.value()),
        Some(TerminalValue::Proved { .. })
    ));
    let doc = document(&record);
    assert_eq!(doc["objective"]["timed"]["form"], "elapsed-quantile");
    assert_eq!(doc["objective"]["timed"]["route"], "digital");
    assert_eq!(proved_value(&record), ratio(2, 5));
    assert_eq!(doc["kind"], "exact");
    assert!(accepted(&world, "AtMostTwo", &doc));
}

/// FR-877-AC-2 (certificate half): AC-1's item with its initial value set to
/// `1/2` is rejected `InequalityFails` by the checker's own enumeration.
#[trace("TC-2494", "FR-877-AC-2")]
#[test]
fn elapsed_quantile_altered_transform_value_is_rejected() {
    let world = quantiles();
    let tampered = settle_tampered(&world, "AtMostTwo", |certificate| {
        for entry in &mut certificate.values {
            if entry.value == ratio(2, 5) {
                entry.value = ratio(1, 2);
            }
        }
    });
    assert_eq!(
        rejected_item_rule(&tampered),
        CertificateRule::InequalityFails
    );
}

/// FR-877-AC-2: `<= 1 ms` is refuted.
#[trace("TC-2494", "FR-877-AC-2")]
#[trace("FSpec-TC-1820", "FSpec-FR-753-AC-4")]
#[ignore = "QSL-2156: refutation needs the timed witness replay"]
#[test]
fn elapsed_quantile_at_most_refutes() {
    let world = quantiles();
    assert!(is_refuted(&settle(&world, "AtMostOne")));
}

/// FR-877-AC-3: `>= 1 ms` is proved by a region `upper` document of `-1/2`.
#[trace("TC-2494", "FR-877-AC-3")]
#[trace("FSpec-TC-1820", "FSpec-FR-753-AC-5")]
#[test]
fn elapsed_quantile_at_least_is_certified_over_regions() {
    let world = quantiles();
    let record = settle(&world, "AtLeastOne");
    assert!(matches!(
        record.terminal().map(|terminal| terminal.value()),
        Some(TerminalValue::Proved { .. })
    ));
    let doc = document(&record);
    assert_eq!(doc["objective"]["timed"]["route"], "region");
    assert_eq!(doc["kind"], "upper");
    assert_eq!(proved_value(&record), ratio(-1, 2));
    assert!(accepted(&world, "AtLeastOne", &doc));
}

/// FR-877-AC-3 (certificate half): an `upper` document whose values are
/// lowered below the checker's own enumeration is rejected `InequalityFails`,
/// and one raised to a non-negative initial value misses the strict threshold.
#[trace("TC-2494", "FR-201")]
#[test]
fn elapsed_quantile_altered_upper_value_is_rejected() {
    let world = quantiles();
    let lowered = settle_tampered(&world, "AtLeastOne", |certificate| {
        for entry in &mut certificate.values {
            if entry.value == ratio(-1, 2) {
                entry.value = ratio(-1, 1);
            }
        }
    });
    assert_eq!(
        rejected_item_rule(&lowered),
        CertificateRule::InequalityFails
    );
    let raised = settle_tampered(&world, "AtLeastOne", |certificate| {
        for entry in &mut certificate.values {
            if entry.value == ratio(-1, 2) {
                entry.value = ratio(1, 2);
            }
        }
    });
    assert_eq!(
        rejected_item_rule(&raised),
        CertificateRule::ThresholdMissed
    );
}

/// FR-877-AC-4: at a transform maximum of exactly 0 the tight search reaches
/// the activated initial state, so the claim is refuted, never proved.
#[trace("TC-2494", "FR-877-AC-4")]
#[ignore = "QSL-2156: a refutation needs the timed witness replay (draco/qsl-2156-cycles)"]
#[test]
fn elapsed_quantile_tight_search_refutes() {
    let world = quantiles();
    assert!(is_refuted(&settle(&world, "Tight")));
}

/// FR-921-AC-2: a key whose cap of `x` is changed is malformed.
#[trace("TC-2490", "FR-921-AC-2")]
#[test]
fn a_wrong_clock_cap_is_malformed() {
    let world = retx(GUARD, true, "", &every_deadlines());
    let doc = document(&settle(&world, "Deadline"));
    let mut changed = doc.clone();
    let mut lowered_caps = 0;
    for row in changed["values"].as_array_mut().expect("values") {
        let encoded = row["state"].as_str().expect("encoded ProductKey");
        let bytes = URL_SAFE_NO_PAD.decode(encoded).expect("base64 ProductKey");
        let mut key: Value = serde_json::from_slice(&bytes).expect("canonical ProductKey");
        let clocks = key[4][1][4].as_array_mut().expect("digital clocks");
        if let Some(clock) = clocks.iter_mut().find(|clock| clock[0][1] == "x") {
            // FR-196: the cap of `x` is 3; lower it to 2 in the encoded key.
            assert_eq!(clock[2], "3");
            clock[2] = json!("2");
            let bytes = serde_json::to_vec(&key).expect("canonical JSON bytes");
            row["state"] = json!(URL_SAFE_NO_PAD.encode(bytes));
            lowered_caps += 1;
        }
    }
    assert!(lowered_caps > 0, "the digital ProductKeys name clock x");
    assert_ne!(changed, doc, "the document names the cap of x");
    match check(&world, "Deadline", &changed) {
        Ok(ExactProbabilityCertificateCheck::Rejected(rejection)) => {
            assert_eq!(rejection.rule, ProbabilityCertificateRule::Malformed);
            assert_eq!(rejection.at, ProbabilityCertificateLocus::Member("values"));
        }
        other => panic!("expected Malformed at values, got {other:?}"),
    }
}

/// FR-924-AC-6 (FSpec FR-753-AC-7): on the region route the loop "reset `y`,
/// delay into `0 < frac(y) < frac(x)`, reset `y`" takes positive time but
/// `x` crosses no integer, so it is not progressive (Alur–Dill) and its Zeno
/// value 0 is rejected.
#[trace("TC-2493", "FR-924-AC-6")]
#[trace("FSpec-TC-1820", "FSpec-FR-753-AC-7")]
#[test]
fn reset_loop_zeno_values_are_rejected() {
    let world = retx_over(
        &["x", "y"],
        &[("send", &["late", "x"]), ("again", &["y"])],
        GUARD,
        true,
        "pre Again using v on M::Call::again { not self.late and self.y < 1 ms }\n\
         post Reset using v on M::Call::again { self.y = 0 ms }\n",
        &every_deadlines(),
    );
    let record = settle(&world, "Deadline");
    assert_eq!(proved_value(&record), ratio(99, 100));
    let doc = document(&record);
    assert_eq!(doc["objective"]["timed"]["route"], "region");
    assert!(
        accepted(&world, "Deadline", &doc),
        "{:?}",
        check(&world, "Deadline", &doc)
    );
    let tampered = settle_tampered(&world, "Deadline", set_interior_values(ratio(0, 1)));
    assert_eq!(
        rejected_item_rule(&tampered),
        CertificateRule::InequalityFails
    );
}

/// FR-924-AC-6 with no deadline clock in the key (the expected-elapsed route):
/// the reset loop is not progressive because `x` is never reset, crossed or
/// capped in it, so the value is still the best exit, 10/9, and a value read
/// off the Zeno loop is rejected.
#[trace("TC-2493", "FR-924-AC-6")]
#[test]
fn reset_loop_is_nonprogressive_without_a_deadline_clock() {
    let world = retx_over(
        &["x", "y"],
        &[("send", &["late", "x"]), ("again", &["y"])],
        GUARD,
        true,
        "pre Again using v on M::Call::again { not self.late and self.y < 1 ms }\n\
         post Reset using v on M::Call::again { self.y = 0 ms }\n",
        "probabilistic MeanLow using inf on M over (c: M::Call) under every scheduler on origin \
         { expected elapsed until holds(c.late) >= 1 ms }\n",
    );
    let record = settle(&world, "MeanLow");
    assert_eq!(proved_value(&record), ratio(10, 9));
    assert_eq!(document(&record)["objective"]["timed"]["route"], "region");
    assert!(accepted(&world, "MeanLow", &document(&record)));
    let tampered = settle_tampered(&world, "MeanLow", |certificate| {
        for entry in &mut certificate.values {
            entry.value = ratio(0, 1);
        }
    });
    assert!(proved_value_of(&tampered).is_none());
}

/// FR-923: a workload resolves a state by its weights, so an operation it
/// weighs at zero cannot be dropped from the checker's product. The engine
/// faults on a zero-weight race and retains no document for it, so the
/// document is the `every scheduler` one of the same model and claim name
/// with its optimization relabelled `workload`; the checker, given
/// the claim under the workload, must refuse the product as `Unsupported`
/// rather than check one without `tick`.
#[trace("TC-2492")]
#[test]
fn the_checker_refuses_a_workload_that_weighs_an_enabled_operation_at_zero() {
    let world = |scheduler: &str| {
        retx_with(
            &[("send", &["late", "x"]), ("tick", &["retried"])],
            GUARD,
            true,
            "delay M::Call::send ~ discrete { 2 ms: 1 };\n\
             pre Tick using v on M::Call::tick { not self.late and not self.retried and self.x >= 2 ms }\n\
             post Ticked using v on M::Call::tick { self.retried = true }\n\
             workload Both on M { weight Call::send = 1; weight Call::tick = 0; }\n",
            &format!(
                "probabilistic W using ev on M over (c: M::Call) {scheduler} on origin \
                 {{ probability <= 0.99 [ eventually[0 ms, 5 ms] holds(c.retried and c.late) ] }}\n"
            ),
        )
    };
    let mut doc = document(&settle(&world("under every scheduler"), "W"));
    doc["objective"]["optimization"] = json!("workload");
    // The claim identity names its scheduler, so it names the workload too.
    let workload = r#"{"ordinal":0,"package":"package","path":["probabilistic","workload",{"ordinal":0,"package":"test/timed","path":["model","object_type","ix://test/timed/Call"]},"Both","send*1,tick*0"]}"#;
    doc["identity"] = serde_json::from_str(&doc["identity"].to_string().replace(
        r#""scheduler","every""#,
        &format!(r#""scheduler",{workload}"#),
    ))
    .expect("identity JSON");
    let result = check(&world("under Both"), "W", &doc);
    assert!(
        matches!(result, Err(ProbabilityDocumentError::Unsupported)),
        "{result:?}"
    );
}

/// The exact value of a `proved` record, when it is one.
fn proved_value_of(record: &AnalyzeRecord) -> Option<Rational> {
    match record.terminal()?.value() {
        TerminalValue::Proved {
            basis: ProofBasis::ExactValue { value, .. },
            ..
        } => Some(value.value().clone()),
        _ => None,
    }
}

// --- QSL-2850: a limit or a cancel is never `not-yet-implemented` ----------

/// The record of `claim` with the exact-rational budget set to `bits`.
fn settle_with_rational_bits(world: &World, claim: &str, bits: u64) -> AnalyzeRecord {
    let engines = Engines::fsl().expect("the linked engines are admitted");
    let mut request = request(world, claim);
    request.limits.max_rational_bits = bits;
    let item = exact_probabilistic_item(RequestIndex::new(0), request);
    analyze_items(&engines, vec![item], &Cancel::new()).remove(0)
}

/// FR-281 "Engine refusals": a timed bounded claim whose values outgrow
/// `max_rational_bits` is `incomplete` naming that limit, not
/// `unsupported`/`not-yet-implemented`.
// QSL-2880 changes the expected outcome: a decided interval will then prove with ValueBounds.
#[trace("TC-2489", "FR-281-AC-6")]
#[test]
fn a_timed_deadline_past_the_rational_budget_settles_incomplete_naming_the_limit() {
    let world = retx(GUARD, true, "", &every_deadlines());
    let record = settle_with_rational_bits(&world, "Strict", 4);
    assert_eq!(
        record.terminal().expect("a terminal record").value(),
        &TerminalValue::Incomplete(fsl_replay::IncompleteCause::LimitReached {
            limit: fsl_replay::ModelCheckLimit::ExactProbMaxRationalBits,
            value: 4,
        }),
        "{record:#?}"
    );
}

/// FR-281 "Engine refusals": a timed bound that equals the exact value stays
/// inside the interval at every precision, which is `incomplete` with the
/// precision budget, not `unsupported`/`not-yet-implemented`.
// QSL-2880 may change the expected outcome once ValueBounds production exists.
#[trace("TC-2489", "FR-281-AC-6")]
#[test]
fn a_timed_deadline_at_its_exact_value_past_the_rational_budget_settles_precision_budget() {
    let world = retx(GUARD, true, "", &every_deadlines());
    let record = settle_with_rational_bits(&world, "Deadline", 4);
    assert!(
        matches!(
            record.terminal().expect("a terminal record").value(),
            TerminalValue::Incomplete(fsl_replay::IncompleteCause::PrecisionBudget { .. })
        ),
        "{record:#?}"
    );
}

/// FR-281 "Engine refusals": cancelling a timed refutation at any poll
/// settles `incomplete` with the cancel's source, or finishes `refuted`; no
/// poll turns it into `unsupported`/`not-yet-implemented`.
///
/// This guards the whole run; it also passes before the QSL-2850 change, so it
/// does not by itself reach the witness-replay expansion exit.
#[trace("TC-2492", "FR-281-AC-6")]
#[test]
fn cancelling_a_timed_refutation_at_any_poll_settles_cancelled_or_refuted() {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{Arc, OnceLock};

    let world = retx(GUARD, true, TWO_DELAYS, &every_deadlines());
    let engines = Engines::fsl().expect("the linked engines are admitted");
    let run = |cancel: &Cancel| {
        let item = exact_probabilistic_item(RequestIndex::new(0), request(&world, "Strict"));
        analyze_items(&engines, vec![item], cancel).remove(0)
    };
    let live = Cancel::new();
    let _counting = live.count_charges();
    assert!(is_refuted(&run(&live)), "the positive control refutes");
    let total = live.charges();
    assert!(total > 0);
    let cause = filament_exact::CancelCause::Requested;
    let mut cancelled = 0_u32;
    let stride = (total / 150).max(1);
    for at in (1..=total).step_by(stride as usize) {
        let slot = Arc::new(OnceLock::<Cancel>::new());
        let weak = Arc::downgrade(&slot);
        let polls = Arc::new(AtomicU32::new(0));
        let seen = Arc::clone(&polls);
        let cancel = Cancel::observing(move || {
            if seen.fetch_add(1, Ordering::Relaxed) + 1 == at {
                weak.upgrade()
                    .expect("the run owns the slot")
                    .get()
                    .expect("the slot is initialized")
                    .cancel(cause);
            }
        });
        slot.set(cancel.clone()).expect("initialized once");
        let record = run(&cancel);
        match record.terminal().expect("a terminal record").value() {
            TerminalValue::Refuted => {}
            TerminalValue::Incomplete(fsl_replay::IncompleteCause::Cancelled { source }) => {
                assert_eq!(*source, cause);
                cancelled += 1;
            }
            other => panic!("cancel at poll {at} of {total} settled {other:?}"),
        }
    }
    assert!(cancelled > 0, "some poll must land inside the run");
}

/// FR-281 "Engine refusals": a document-storage budget that stops a timed
/// refutation's replay is `incomplete`, at every size, never
/// `unsupported`/`not-yet-implemented` and never a fault.
#[trace("TC-2492", "FR-281-AC-6")]
#[test]
fn a_timed_refutation_settles_refuted_or_incomplete_at_every_storage_budget() {
    let world = retx(GUARD, true, TWO_DELAYS, &every_deadlines());
    let engines = Engines::fsl().expect("the linked engines are admitted");
    let mut stopped = 0_u32;
    for bytes in [0_u64, 1, 16, 64, 256, 1024, 4096, 16_384, 65_536, 1 << 20] {
        let mut request = request(&world, "Strict");
        request.limits.document.storage_bytes = bytes;
        let item = exact_probabilistic_item(RequestIndex::new(0), request);
        let record = analyze_items(&engines, vec![item], &Cancel::new()).remove(0);
        let value = record.terminal().expect("a terminal record").value();
        assert!(
            matches!(value, TerminalValue::Refuted | TerminalValue::Incomplete(_)),
            "storage {bytes}: {record:#?}"
        );
        stopped += u32::from(matches!(value, TerminalValue::Incomplete(_)));
    }
    assert!(stopped > 0, "some storage budget must stop the replay");
}
