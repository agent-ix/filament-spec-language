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
    Engines,
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
#[ignore = "QSL-2156: timed witness replay of a refuted every-scheduler deadline claim is not implemented yet"]
#[test]
fn every_scheduler_discrete_delay_refutes() {
    let world = retx(GUARD, true, TWO_DELAYS, &every_deadlines());
    assert!(is_refuted(&settle(&world, "Strict")));
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
/// scheduler's choice, so the maximum is 1 where `Both` gives `21/40`.
#[trace("TC-2492", "FR-923-AC-3")]
#[ignore = "QSL-2156: timed witness replay of an every-scheduler tie refutation and a workload-scheduler timed deadline check are not implemented yet"]
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
    assert!(is_refuted(&settle(&world, "Every")));
    assert_eq!(proved_value(&settle(&world, "Weighted")), ratio(21, 40));
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
/// emits no workload document for a timed deadline (and faults on a zero-weight
/// race), so the document is the `every scheduler` one of the same model and
/// claim name with its optimization relabelled `workload`; the checker, given
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
