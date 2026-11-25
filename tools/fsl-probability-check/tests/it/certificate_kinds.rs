// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-2065 (FR-201-AC-51): `fsl-probability-check`, a separate process with
//! no `fsl-analyze` linkage, accepts every certificate kind EN-5 emits for
//! the exact classes FSpec FR-641 requires, and rejects each one altered.
//! The certificates are produced by the analyzer over the cyclic `link`
//! model, as the other suites of this binary produce theirs.

use super::bounded::{document, key, mutate, replace_source_and_rebind, request_with, run};
use fsl_analyze::exact_probabilistic::ExactProbLimits;
use ix_trace_rs::trace;
use serde_json::{json, Value};

const SNAPSHOT: &[u8] = include_bytes!("../fixtures/link.snapshot.json");

/// The `link` domain as declared, with a Boolean `lost`: the domain of
/// [`fair_link_model`], whose draws are `true` and `false`.
const FAIR_DOMAIN: &str = include_str!("../fixtures/reward.semantic-ir.json");

/// [`model`]'s domain, with `lost` retyped to count attempts.
fn domain() -> String {
    include_str!("../fixtures/reward.semantic-ir.json").replace(
        "\"typeRef\":\"ix://filament/native/Boolean\"}],\"post\"",
        "\"typeRef\":\"ix://test/link/Attempts\"}],\"post\"",
    )
}

/// The cyclic `link` model: `send_a` retries until delivered or two losses,
/// `send_b` and `send_c` are never enabled.
fn model() -> String {
    let language = include_str!("../fixtures/link.native")
        .lines()
        .next()
        .unwrap();
    format!(
        "{language}\n\
        profile v = \"filament.model.complete/v1\";\n\
        profile inf = \"filament.temporal.infinite-trace/v1\";\n\
        profile ev = \"filament.temporal.event-position.false-extension/v1\";\n\
        model M = \"test/link\";\n\
        pre A using v on M::Msg::send_a {{ self.attempts < 2 or self.delivered }}\n\
        post A_done using v on M::Msg::send_a {{ self.attempts - (if lost = 0 then 0 else 2) = 0 and self.delivered = (lost = 1) }}\n\
        pre B using v on M::Msg::send_b {{ false }}\n\
        post B_done using v on M::Msg::send_b {{ self.attempts = 0 and not self.delivered }}\n\
        pre C using v on M::Msg::send_c {{ false }}\n\
        post C_done using v on M::Msg::send_c {{ self.attempts = 0 and not self.delivered }}\n\
        random Msg::send_a(lost) ~ {{ 0: 1, 1: 1, 2: 1 }};\n\
        random Msg::send_b(lost) ~ {{ 0: 1, 1: 1, 2: 1 }};\n\
        random Msg::send_c(lost) ~ {{ 0: 1, 1: 1, 2: 1 }};\n\
        workload Even on M {{ weight Msg::send_a = 1; weight Msg::send_b = 1; weight Msg::send_c = 1; }}\n\
        terminal Stop using v on M when {{ self.attempts = 2 and not self.delivered }};\n"
    )
}

/// FR-1041-AC-5's cyclic Link model: `send_b` is the live wait self-loop;
/// fair strong `send_a` is sampled once and reaches either delivered or the
/// stopped terminal with probability one half. `send_c` is never enabled, as
/// in [`model`]: left without clauses it can leave the initial state for good
/// without delivering, and the fair minimum is 0.
fn fair_link_model() -> String {
    format!(
        "{}\n\
        pre Ready_send_a using v on M::Msg::send_a {{ not self.delivered and self.attempts = 0 }}\n\
        post Sent_send_a using v on M::Msg::send_a {{ self.attempts = 1 and self.delivered = lost }}\n\
        pre Ready_send_b using v on M::Msg::send_b {{ not self.delivered and self.attempts = 0 }}\n\
        post Sent_send_b using v on M::Msg::send_b {{ self.attempts = 0 and not self.delivered }}\n\
        pre Never_send_c using v on M::Msg::send_c {{ false }}\n\
        post Sent_send_c using v on M::Msg::send_c {{ self.attempts = 0 and not self.delivered }}\n\
        random Msg::send_a(lost) ~ {{ true: 1, false: 1 }};\n\
        random Msg::send_b(lost) ~ {{ true: 1, false: 1 }};\n\
        random Msg::send_c(lost) ~ {{ true: 1, false: 1 }};\n\
        terminal Stop using v on M when {{ self.delivered or self.attempts = 1 }};\n",
        include_str!("../fixtures/link.native")
            .lines()
            .take(4)
            .collect::<Vec<_>>()
            .join("\n")
    )
}

pub(super) fn likely(profile: &str, scheduler: &str, form: &str) -> String {
    format!(
        "{}\nprobabilistic Likely using {profile} on M over (m: M::Msg) under {scheduler} on origin {{ {form} }}\n",
        model()
    )
}

/// The analyzer's retained certificate request for `source`, which the
/// decided behaviour requires to exist for every exact class.
fn certified(source: &str, limits: ExactProbLimits) -> Value {
    certified_over(source, limits, &domain())
}

/// [`certified`] for a source over `domain`.
fn certified_over(source: &str, limits: ExactProbLimits, domain: &str) -> Value {
    let (request, settlement) = request_with(source, SNAPSHOT, limits, domain.as_bytes());
    request.unwrap_or_else(|| {
        panic!("every exact proof retains an accepted certificate: {settlement:?}\n{source}")
    })
}

/// Accepted by the separate process with `kind`; with its first value
/// altered it is rejected.
fn accepted_then_rejected(request: &Value, kinds: &[&str]) {
    let (code, output) = run(request);
    assert_eq!(code, 0, "{output}");
    let doc = document(request);
    let kind = doc["kind"].as_str().expect("a certificate kind");
    assert!(kinds.contains(&kind), "{kind} not in {kinds:?}");
    // FR-196: every state is named by the complete eight-member key.
    for row in doc["values"].as_array().expect("values") {
        let members = key(&row["state"]);
        assert_eq!(members.as_array().map(Vec::len), Some(8), "{members}");
    }

    let altered = mutate(request, |doc| {
        doc["values"][0]["value"] = json!({"numerator": "1", "denominator": "7"});
    });
    let (code, output) = run(&altered);
    assert_ne!(code, 0, "an altered certificate is rejected: {output}");
}

/// Unbounded fair eventual reachability over every scheduler with a
/// fairness set: an `exact` certificate the checker accepts.
#[trace("TC-2065", "FR-201-AC-51", "FR-1041-AC-5")]
#[test]
fn an_unbounded_fair_eventual_certificate_is_accepted() {
    let source = format!(
        "{}\nprobabilistic Likely using inf on M over (m: M::Msg) under every scheduler fair strong Msg::send_a on origin {{ probability >= rational(1,2) [ eventually holds(m.delivered) ] }}\n",
        fair_link_model()
    );
    let (request, settlement) = request_with(
        &source,
        SNAPSHOT,
        ExactProbLimits::default(),
        FAIR_DOMAIN.as_bytes(),
    );
    let request = request.unwrap_or_else(|| panic!("expected fair certificate: {settlement:?}"));
    assert!(
        matches!(&settlement, fsl_analyze::analyze::Settlement::Terminal(record)
            if matches!(record.value(), fsl_replay::TerminalValue::Proved {
                basis: fsl_replay::ProofBasis::ExactValue { value, .. }, ..
            } if value.value().numerator() == &filament_exact::Integer::from(1_i64)
                && value.value().denominator() == &filament_exact::Integer::from(2_i64))),
        "AC-5 settles proved with exact value 1/2: {settlement:?}"
    );
    let doc = document(&request);
    assert_eq!(doc["kind"], "exact");
    let values = doc["values"].as_array().expect("certificate values");
    assert_eq!(values.len(), 3);
    for (delivered, attempts, value, state_name) in [
        (
            false,
            "0",
            json!({"numerator":"1","denominator":"2"}),
            "undelivered live state",
        ),
        (
            true,
            "1",
            json!({"numerator":"1","denominator":"1"}),
            "delivered terminal",
        ),
        (
            false,
            "1",
            json!({"numerator":"0","denominator":"1"}),
            "stopped terminal",
        ),
    ] {
        let row = values
            .iter()
            .find(|row| {
                let state = key(&row["state"]);
                let fields = &state[4][1]["semantic"]["entries"][0]["value"]["entries"][0]["value"]
                    ["fields"];
                fields[0]["value"]["value"] == json!(delivered)
                    && fields[1]["value"]["value"] == attempts
            })
            .unwrap_or_else(|| panic!("missing state delivered={delivered}, attempts={attempts}"));
        assert_eq!(row["value"], value, "AC-5 {state_name}");
    }
    let ranks = doc["ranking"].as_array().expect("fair progress ranks");
    assert_eq!(ranks.len(), 3);
    assert_eq!(ranks.iter().filter(|row| row["rank"] == 1).count(), 1);
    assert_eq!(ranks.iter().filter(|row| row["rank"] == 0).count(), 2);

    // The later checker request names the emitted package, then independently
    // rejects a certificate whose one literal value is changed.
    let later = package_request_over(&source, FAIR_DOMAIN);
    assert_eq!(later["certificate"], request["certificate"]);
    accepted_then_rejected(&later, &["exact"]);
}

/// The measure `Pr(activated and M <= 3)` over every scheduler has the
/// minimum `40/81`, so the full run proves `>= 0.3` and refutes `>= 0.5`. One
/// state of the product shows neither (FSpec FR-411 "Evidence on a partial
/// product"). A `lower` naming the initial state and its delivered successor
/// proves `>= 0.3`, each unnamed successor counting as 0 (FR-1042
/// "Document"), and cannot prove `>= 0.5`.
fn measure_bounds_are_checked_over_a_prefix(one_state: ExactProbLimits) {
    let claim = |bound: &str| {
        likely(
            "ev",
            "every scheduler",
            &format!(
                "probability >= {bound} [steps from holds(m.delivered) until holds(true) within 4 <= 3]"
            ),
        )
    };
    let settle = |bound: &str, limits: ExactProbLimits| {
        request_with(&claim(bound), SNAPSHOT, limits, domain().as_bytes())
    };

    let exact = certified(&claim("0.3"), ExactProbLimits::default());
    let (code, output) = run(&exact);
    assert_eq!(code, 0, "{output}");
    assert_eq!(
        output["value"],
        json!({"numerator": "40", "denominator": "81"})
    );
    let (_, full) = settle("0.5", ExactProbLimits::default());
    assert!(
        matches!(&full, fsl_analyze::analyze::Settlement::Terminal(record)
            if matches!(record.value(), fsl_replay::TerminalValue::Refuted)),
        "the full run refutes 0.5: {full:?}"
    );

    let (request, stopped) = settle("0.5", one_state);
    assert!(request.is_none(), "{request:?}");
    assert!(
        matches!(&stopped, fsl_analyze::analyze::Settlement::Terminal(record)
        if record.value() == &fsl_replay::TerminalValue::Incomplete(
            fsl_replay::IncompleteCause::LimitReached {
                limit: fsl_replay::ModelCheckLimit::ExactProbMaxStates,
                value: 1,
                required: None,
            }
        )),
        "one state settles nothing: {stopped:?}"
    );

    // The initial state and the delivered successor it steps to, with the
    // exact document's own ranks.
    let prefix = [
        json!(["measure", "4", "inactive"]),
        json!(["measure", "3", "complete"]),
    ];
    let lower = mutate(&exact, |doc| {
        let named = |rows: &Value| {
            let rows = rows.as_array().expect("certificate rows");
            let named: Vec<Value> = rows
                .iter()
                .filter(|row| prefix.contains(&key(&row["state"])[5]))
                .cloned()
                .collect();
            assert_eq!(named.len(), prefix.len(), "{named:?}");
            named
        };
        let mut values = named(&doc["values"]);
        for row in &mut values {
            if key(&row["state"])[5] == prefix[0] {
                row["value"] = json!({"numerator": "1", "denominator": "3"});
            }
        }
        *doc = json!({
            "identity": doc["identity"],
            "objective": {
                "profile": "filament.probability-objective/value-bounds/v1",
                "base": doc["objective"],
            },
            "kind": "lower",
            "values": values,
            "ranking": named(&doc["ranking"]),
        });
    });
    let (code, output) = run(&lower);
    assert_eq!(code, 0, "{output}");
    assert_eq!(
        output["value"],
        json!({"numerator": "1", "denominator": "3"})
    );

    let raised = mutate(&lower, |doc| {
        for row in doc["values"].as_array_mut().expect("certificate values") {
            if key(&row["state"])[5] == prefix[0] {
                row["value"] = json!({"numerator": "2", "denominator": "5"});
            }
        }
    });
    let (code, output) = run(&raised);
    assert_eq!(code, 1, "{output}");
    assert_eq!(output["rule"], "inequality-fails", "{output}");

    let mut higher = lower.clone();
    replace_source_and_rebind(&mut higher, &claim("0.5"));
    let (code, output) = run(&higher);
    assert_eq!(code, 1, "{output}");
    assert_eq!(output["rule"], "threshold-missed", "{output}");
}

/// A value-bounds proof under a tight rational budget: `lower` and `upper`
/// certificates the checker accepts.
#[trace("TC-2065", "FR-201-AC-51")]
#[test]
fn value_bounds_certificates_are_accepted() {
    let source = likely(
        "inf",
        "Even",
        "probability >= rational(2,5) [ eventually holds(m.delivered) ]",
    );
    let limits = ExactProbLimits {
        max_rational_bits: 1,
        ..ExactProbLimits::default()
    };
    accepted_then_rejected(&certified(&source, limits), &["lower", "upper"]);

    let partial_source = likely(
        "inf",
        "Even",
        "probability >= 0.5 [ eventually holds(true) ]",
    );
    let mut partial_limits = ExactProbLimits::default();
    partial_limits.model.limits.max_states = 1;
    let (partial, settlement) = request_with(
        &partial_source,
        SNAPSHOT,
        partial_limits,
        domain().as_bytes(),
    );
    let partial = partial.unwrap_or_else(|| {
        panic!("a lower proof on the explored prefix settles: {settlement:?}")
    });
    let fsl_analyze::analyze::Settlement::Terminal(record) = settlement else {
        panic!("partial evidence settles with a terminal record: {settlement:?}");
    };
    assert!(matches!(
        record.value(),
        fsl_replay::TerminalValue::Proved {
            basis: fsl_replay::ProofBasis::ValueBounds {
                upper: fsl_replay::ExactBound::Finite(value),
                exploration: fsl_replay::BoundsExploration::Partial,
                ..
            },
            ..
        } if value.value() == &filament_exact::Rational::from_integer(1_i64.into())
    ));
    let doc = document(&partial);
    assert_eq!(doc["kind"], "lower");
    let (code, output) = run(&partial);
    assert_eq!(code, 0, "{output}");

    measure_bounds_are_checked_over_a_prefix(partial_limits);

    let shifted_source = likely(
        "ev",
        "every scheduler",
        "quantile 0.4 of (steps from holds(true) until holds(m.delivered) within 4) <= 3",
    );
    let mut shifted_limits = ExactProbLimits::default();
    shifted_limits.model.limits.max_states = 10;
    let (shifted, shifted_settlement) = request_with(
        &shifted_source,
        SNAPSHOT,
        shifted_limits,
        domain().as_bytes(),
    );
    let shifted = shifted.unwrap_or_else(|| {
        panic!("the explored shifted prefix proves level 0.4: {shifted_settlement:?}")
    });
    assert_eq!(document(&shifted)["kind"], "lower");
    let (code, output) = run(&shifted);
    assert_eq!(code, 0, "{output}");
    let fsl_analyze::analyze::Settlement::Terminal(record) = shifted_settlement else {
        panic!("partial shifted proof settles: {shifted_settlement:?}");
    };
    assert!(matches!(
        record.value(),
        fsl_replay::TerminalValue::Proved {
            basis: fsl_replay::ProofBasis::ValueBounds {
                upper: fsl_replay::ExactBound::Finite(value),
                exploration: fsl_replay::BoundsExploration::Partial,
                ..
            },
            ..
        } if value.value() == &filament_exact::Rational::new(3_i64.into(), 5_i64.into()).unwrap()
    ));
}

/// A finite-horizon quantile over every scheduler: its certificate is
/// accepted by the checker.
#[trace("TC-2065", "FR-201-AC-51", "FR-1043-AC-6")]
#[test]
fn a_finite_horizon_quantile_certificate_is_accepted() {
    let source = likely(
        "ev",
        "every scheduler",
        "quantile 0.4 of (steps from holds(true) until holds(m.delivered) within 4) <= 3",
    );
    let request = package_request(&source);
    let doc = document(&request);
    assert_eq!(doc["objective"]["payoff"], "shifted");
    let initial = doc["ranking"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["rank"] == 4)
        .expect("the initial remaining-horizon row");
    let value = doc["values"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["state"] == initial["state"])
        .unwrap();
    assert_eq!(
        value["value"],
        json!({"numerator":"11","denominator":"135"})
    );
    accepted_then_rejected(&request, &["exact"]);
    let false_source = source.replace("quantile 0.4", "quantile 0.5");
    let (certificate, settlement) = request_with(
        &false_source,
        SNAPSHOT,
        ExactProbLimits::default(),
        domain().as_bytes(),
    );
    assert!(certificate.is_none());
    assert!(
        matches!(&settlement,fsl_analyze::analyze::Settlement::Terminal(record) if record.value()==&fsl_replay::TerminalValue::Refuted),
        "{settlement:?}"
    );
}

/// The long-run kind: a long-run fraction under a workload is accepted as a
/// `long-run` certificate. Every run of the cyclic model ends in the
/// terminal state where `delivered` is false, so the fraction is 0 and only
/// the `<=` side is provable.
#[trace("TC-2065", "FR-201-AC-51")]
#[test]
fn a_long_run_certificate_is_accepted() {
    let source = likely(
        "inf",
        "Even",
        "long_run_fraction holds(m.delivered) <= rational(1,2)",
    );
    accepted_then_rejected(
        &certified(&source, ExactProbLimits::default()),
        &["long-run"],
    );
}

/// The probabilistic unit's source with a reward, whose exact certificate
/// is checked over the package.
fn package_source() -> String {
    likely(
        "inf",
        "Even",
        "probability >= rational(1,2) [ eventually holds(m.delivered) ]",
    )
    .replace(
        "terminal Stop",
        "reward cost on Msg::send_a = 1;\nterminal Stop",
    )
}

/// The request that names the package S4 emits for `source` in place of the
/// source; its certificate is the document's bytes as the run wrote them.
fn package_request(source: &str) -> Value {
    package_request_over(source, &domain())
}

/// [`package_request`] for a source over `domain`.
fn package_request_over(source: &str, domain: &str) -> Value {
    let request = certified_over(source, ExactProbLimits::default(), domain);
    let (checked, _) = super::bounded::compile_source_with(source, domain.as_bytes());
    let emitted = fsl_replay::spine::package(
        &checked,
        fsl_replay::spine::PackageLimits::default(),
        &filament_exact::Cancel::new(),
    )
    .unwrap_or_else(|failure| panic!("S4 emits the probabilistic unit: {failure:?}"))
    .into_value();
    let mut later = request;
    later
        .as_object_mut()
        .unwrap()
        .remove("source")
        .expect("the source member");
    later["package"] = json!(emitted.package().bytes().to_vec());
    later
}

/// TC-2065 (FR-203-AC-8, FR-282-AC-20): the exact certificate `analyze`
/// writes for a claim over a unit with random parameters, a workload and a
/// reward is carried as bytes out of the run and accepted by the checker in
/// a separate, later process over the package S4 emits: the request names
/// the emitted `filament.checked-package/v2` bytes in place of the source,
/// and the same certificate with one value altered is rejected.
#[trace("TC-2065", "FR-203-AC-8")]
#[trace("TC-2065", "FR-282-AC-20")]
#[test]
fn an_analyze_certificate_is_accepted_later_over_the_emitted_package() {
    for source in [
        package_source(),
        likely(
            "ev",
            "Even",
            "probability >= rational(2,9) [ always[0,2] holds(not m.delivered) ]",
        ),
    ] {
        let original = certified(&source, ExactProbLimits::default());
        let later = package_request(&source);
        assert_eq!(later["certificate"], original["certificate"]);
        let (source_code, source_output) = run(&original);
        let (code, output) = run(&later);
        assert_eq!(source_code, 0, "{source_output}");
        assert_eq!(code, 0, "{output}");
        assert_eq!(output, source_output);
        let altered = mutate(&later, |doc| {
            doc["values"][0]["value"] = json!({"numerator": "1", "denominator": "7"});
        });
        let (code, output) = run(&altered);
        assert_ne!(code, 0, "an altered certificate is rejected: {output}");
        let mut absent_source = later;
        absent_source["byte_provision"] = json!([]);
        refused_package(
            &absent_source,
            "conflicting_identity: probability certificate source",
        );
    }
}

/// A request the package route must refuse, never accept.
fn refused_package(request: &Value, reason: &str) {
    let (code, output) = run(request);
    assert_eq!(code, 2, "{output}");
    assert!(
        output["reason"]
            .as_str()
            .is_some_and(|text| text.contains(reason)),
        "{reason}: {output}"
    );
}

/// FR-282-AC-20 (negative): the package the provision recompiles to must be
/// the package the request names; a source that recompiles to other bytes is
/// refused.
#[trace("TC-2065", "FR-282-AC-20")]
#[test]
fn a_package_the_provision_does_not_recompile_to_is_refused() {
    let mut request = package_request(&package_source());
    let other = package_source().replace(
        "terminal Stop",
        "reward extra on Msg::send_a = 1;\nterminal Stop",
    );
    request["byte_provision"][0][2] = json!(other.as_bytes());
    refused_package(
        &request,
        "conflicting_identity: probability certificate package",
    );
}

/// FR-282-AC-20 (negative): a package that names no source, or two, and bytes
/// that are no document, are refused with their own cause.
#[trace("TC-2065", "FR-282-AC-20")]
#[test]
fn a_package_without_exactly_one_source_or_without_a_document_is_refused() {
    let request = package_request(&package_source());
    let bytes: Vec<u8> = serde_json::from_value(request["package"].clone()).unwrap();
    let mut document: Value = serde_json::from_slice(&bytes).unwrap();
    document["lock"]["sources"] = json!([]);
    let mut none = request.clone();
    none["package"] = json!(serde_json::to_vec(&document).unwrap());
    refused_package(&none, "names 0 sources");
    let mut unreadable = request;
    unreadable["package"] = json!([255, 254]);
    refused_package(&unreadable, "package unreadable");
}

/// FR-282-AC-20 (negative): a stated package name other than the package's
/// own is refused, and a request names exactly one of source and package.
#[trace("TC-2065", "FR-282-AC-20")]
#[test]
fn a_package_request_names_its_own_package_and_exactly_one_subject() {
    let request = package_request(&package_source());
    let mut renamed = request.clone();
    renamed["package_name"] = json!("another-package");
    refused_package(&renamed, "package name");
    let mut neither = request.clone();
    neither.as_object_mut().unwrap().remove("package");
    refused_package(&neither, "exactly one of source and package");
    let mut both = request;
    both["source"] = json!({"authority": "test", "identity": "bounded-certificate-source", "content": package_source()});
    refused_package(&both, "exactly one of source and package");
}
