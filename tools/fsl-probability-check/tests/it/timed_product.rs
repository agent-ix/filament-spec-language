// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-2495 (FR-921-AC-4), FSpec TC-1768 (FSpec FR-751-AC-4):
//! `fsl-probability-check`, a separate process with no `fsl-analyze`
//! linkage, accepts EN-5's `timed-product/v1` documents for the strict-clock,
//! every-scheduler delay, zero-time cycle and elapsed-quantile cases, and
//! rejects each with one value altered. The timed subject is ADR-028 §15.5's
//! `Retx` over the `link` package: `Msg` gains the clock `x` and the Boolean
//! `spun`, `send_a` is `Retx`'s `send` and `send_b` a zero-time toggle.

use super::bounded::{document, mutate, request_with, run};
use fsl_analyze::exact_probabilistic::ExactProbLimits;
use ix_trace_rs::trace;
use serde_json::{json, Value};

const SNAPSHOT: &[u8] = include_bytes!("../fixtures/link.snapshot.json");

/// The `link` domain with `Msg.x: Clock` and `Msg.spun: Boolean`; `send_a`
/// modifies `delivered` and `x`, `send_b` modifies `spun`.
fn domain() -> Vec<u8> {
    let mut domain: Value =
        serde_json::from_str(include_str!("../fixtures/link.semantic-ir.json")).unwrap();
    let message = &mut domain["types"][1];
    let template = message["fields"][0].clone();
    for (name, type_ref) in [
        ("x", "ix://filament/native/Clock"),
        ("spun", "ix://filament/native/Boolean"),
    ] {
        let mut field = template.clone();
        field["identity"] = json!(format!("ix://test/link/Msg/{name}"));
        field["name"] = json!(name);
        field["typeRef"] = json!(type_ref);
        message["fields"].as_array_mut().unwrap().push(field);
    }
    message["operations"][0]["frame"]["modifies"] =
        json!(["ix://test/link/Msg/delivered", "ix://test/link/Msg/x"]);
    message["operations"][1]["frame"]["modifies"] = json!(["ix://test/link/Msg/spun"]);
    serde_json::to_vec(&domain).unwrap()
}

/// `Retx` with `send_a` guarded by `guard`, `declarations` and the claim
/// `Likely` under every scheduler with `form`.
fn retx(guard: &str, declarations: &str, form: &str) -> String {
    let language = include_str!("../fixtures/link.native")
        .lines()
        .next()
        .unwrap();
    format!(
        "{language}\n\
        profile v = \"filament.model.complete/v1\";\n\
        profile ev = \"filament.temporal.timed/v1\";\n\
        model M = \"test/link\";\n\
        dimension Time;\nunit s : Time = rational(1, 1);\nunit ms : Time = rational(1, 1000) * s;\n\
        time M dense unit ms;\n\
        pre A using v on M::Msg::send_a {{ not self.delivered and self.x {guard} }}\n\
        post A_done using v on M::Msg::send_a {{ self.delivered = not lost and self.x = 0 ms }}\n\
        random M::Msg::send_a(lost) ~ {{ true: 1, false: 9 }};\n\
        time invariant Deadline when {{ not self.delivered }} {{ self.x <= 2 ms }}\n\
        {declarations}\
        probabilistic Likely using ev on M over (m: M::Msg) under every scheduler on origin {{ {form} }}\n"
    )
}

const DEADLINE: &str = "probability >= 0.99 [ eventually[0 ms, 4 ms] holds(m.delivered) ]";
const NO_TOGGLE: &str = "pre B using v on M::Msg::send_b { false }\n\
     post B_done using v on M::Msg::send_b { self.spun = self.spun }\n";
const TOGGLE: &str = "pre B using v on M::Msg::send_b { true }\n\
     post B_done using v on M::Msg::send_b { self.spun = not self.spun }\n";

fn certified(source: &str) -> Value {
    let (request, settlement) =
        request_with(source, SNAPSHOT, ExactProbLimits::default(), &domain());
    request.unwrap_or_else(|| {
        panic!("every timed proof retains an accepted certificate: {settlement:?}\n{source}")
    })
}

#[trace("TC-2495", "FR-921-AC-4")]
#[trace("FSpec-TC-1768", "FSpec-FR-751-AC-4")]
#[test]
#[ignore = "QSL-2156: timed-product certificates through the independent checker not implemented yet"]
fn timed_product_certificates_pass_the_separate_checker() {
    let cases = [
        (
            "region",
            retx(">= 1 ms", NO_TOGGLE, DEADLINE).replace(">= 1 ms }", "> 1 ms }"),
        ),
        (
            "digital",
            retx(
                ">= 1 ms",
                &format!("{NO_TOGGLE}delay M::Msg::send_a ~ discrete {{ 1 ms: 1, 2 ms: 1 }};\n"),
                DEADLINE,
            ),
        ),
        ("digital", retx(">= 1 ms", TOGGLE, DEADLINE)),
        (
            "digital",
            retx(
                ">= 1 ms",
                NO_TOGGLE,
                "quantile 0.5 of (elapsed from holds(not m.delivered) until holds(m.delivered) \
                 within 4 ms) <= 2 ms",
            ),
        ),
    ];
    for (route, source) in cases {
        let request = certified(&source);
        let (code, output) = run(&request);
        assert_eq!(code, 0, "{output}\n{source}");
        let doc = document(&request);
        assert_eq!(
            doc["objective"]["profile"],
            "filament.probability-objective/timed-product/v1"
        );
        assert_eq!(doc["objective"]["timed"]["route"], route);
        let altered = mutate(&request, |doc| {
            doc["values"][0]["value"] = json!({"numerator": "1", "denominator": "7"});
        });
        let (code, output) = run(&altered);
        assert_ne!(code, 0, "an altered certificate is rejected: {output}");
    }
}
