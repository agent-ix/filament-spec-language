// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-2065 (FR-201-AC-51): `fsl-probability-check`, a separate process with
//! no `fsl-analyze` linkage, accepts every certificate kind EN-5 emits for
//! the exact classes FSpec FR-641 requires, and rejects each one altered.
//! The certificates are produced by the analyzer over the cyclic `link`
//! model, as the other suites of this binary produce theirs.

use super::bounded::{document, mutate, request_with, run};
use fsl_analyze::exact_probabilistic::ExactProbLimits;
use ix_trace_rs::trace;
use serde_json::{json, Value};

const SNAPSHOT: &[u8] = include_bytes!("../fixtures/link.snapshot.json");

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

fn likely(profile: &str, scheduler: &str, form: &str) -> String {
    format!(
        "{}\nprobabilistic Likely using {profile} on M over (m: M::Msg) under {scheduler} on origin {{ {form} }}\n",
        model()
    )
}

/// The analyzer's retained certificate request for `source`, which the
/// decided behaviour requires to exist for every exact class.
fn certified(source: &str, limits: ExactProbLimits) -> Value {
    let (request, settlement) = request_with(source, SNAPSHOT, limits, domain().as_bytes());
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

    let altered = mutate(request, |doc| {
        doc["values"][0]["value"] = json!({"numerator": "1", "denominator": "7"});
    });
    let (code, output) = run(&altered);
    assert_ne!(code, 0, "an altered certificate is rejected: {output}");
}

/// Unbounded fair eventual reachability over every scheduler with a
/// fairness set: an `exact` certificate the checker accepts.
#[trace("TC-2065", "FR-201-AC-51")]
#[test]
#[ignore = "QSL-2158: unbounded fair eventual certificates through the independent checker not implemented yet"]
fn an_unbounded_fair_eventual_certificate_is_accepted() {
    let source = likely(
        "inf",
        "every scheduler fair strong Msg::send_a",
        "probability >= rational(1,2) [ eventually holds(m.delivered) ]",
    );
    accepted_then_rejected(&certified(&source, ExactProbLimits::default()), &["exact"]);
}

/// A value-bounds proof under a tight rational budget: `lower` and `upper`
/// certificates the checker accepts.
#[trace("TC-2065", "FR-201-AC-51")]
#[test]
#[ignore = "QSL-2158: lower and upper value-bounds certificates through the independent checker not implemented yet"]
fn value_bounds_certificates_are_accepted() {
    let source = likely(
        "inf",
        "Even",
        "probability >= rational(1,2) [ eventually holds(m.delivered) ]",
    );
    let limits = ExactProbLimits {
        max_rational_bits: 2,
        ..ExactProbLimits::default()
    };
    accepted_then_rejected(&certified(&source, limits), &["lower", "upper"]);
}

/// A finite-horizon quantile over every scheduler: its certificate is
/// accepted by the checker.
#[trace("TC-2065", "FR-201-AC-51")]
#[test]
#[ignore = "QSL-2158: finite-horizon quantile certificates through the independent checker not implemented yet"]
fn a_finite_horizon_quantile_certificate_is_accepted() {
    let source = likely(
        "ev",
        "every scheduler",
        "quantile 0.5 of (steps from holds(true) until holds(m.delivered) within 4) <= 3",
    );
    accepted_then_rejected(
        &certified(&source, ExactProbLimits::default()),
        &["exact", "lower", "upper"],
    );
}

/// The long-run kind: a long-run fraction under a workload is accepted as a
/// `long-run` certificate.
#[trace("TC-2065", "FR-201-AC-51")]
#[test]
#[ignore = "QSL-2158: long-run certificates over the cyclic link model through the independent checker not implemented yet"]
fn a_long_run_certificate_is_accepted() {
    let source = likely(
        "inf",
        "Even",
        "long_run_fraction holds(m.delivered) >= rational(1,2)",
    );
    accepted_then_rejected(
        &certified(&source, ExactProbLimits::default()),
        &["long-run"],
    );
}
