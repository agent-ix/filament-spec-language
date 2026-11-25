// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1041-AC-1: the analyzer's fair-MDP W document is accepted by the
//! independent `fsl-probability-check` process.

use super::bounded::{document, mutate, request_with, requests_with_mode, run};
use fsl_analyze::analyze::Settlement;
use fsl_analyze::exact_probabilistic::ExactProbLimits;
use fsl_replay::{ProofBasis, TerminalValue};
use ix_trace_rs::trace;
use serde_json::{json, Value};

const DOMAIN: &[u8] = include_bytes!("../fixtures/link.semantic-ir.json");
const SNAPSHOT: &[u8] = include_bytes!("../fixtures/link.snapshot.json");
const LINK_SOURCE: &str = include_str!("../fixtures/link.native");

fn fair_w_source() -> String {
    let header = LINK_SOURCE.lines().take(4).collect::<Vec<_>>().join("\n");
    format!(
        "{header}\n\
        pre Wait using v on M::Msg::send_a {{ self.attempts = 0 }}\n\
        post Waited using v on M::Msg::send_a {{ self.attempts = 0 and not self.delivered }}\n\
        pre Go using v on M::Msg::send_b {{ self.attempts = 0 }}\n\
        post Gone using v on M::Msg::send_b {{ self.attempts - (if lost then 1 else 2) = 0 and self.delivered = lost }}\n\
        random Msg::send_a(lost) ~ {{ true: 1, false: 1 }};\n\
        random Msg::send_b(lost) ~ {{ true: 1, false: 1 }};\n\
        terminal Stop using v on M when {{ self.attempts = 2 and not self.delivered }};\n\
        probabilistic Likely using inf on M over (m: M::Msg) under every scheduler fair strong M::Msg::send_b on origin {{ probability >= rational(1,2) [ eventually holds(m.delivered) ] }}\n"
    )
}

fn rational_rows(document: &Value) -> Vec<String> {
    let mut rows = document["values"]
        .as_array()
        .expect("certificate value rows")
        .iter()
        .map(|row| {
            format!(
                "{}/{}",
                row["value"]["numerator"].as_str().expect("numerator"),
                row["value"]["denominator"].as_str().expect("denominator")
            )
        })
        .collect::<Vec<_>>();
    rows.sort();
    rows
}

/// The literal fair-minimum W document is admitted in a fresh checker process.
#[trace("TC-3150", "FR-1041-AC-1")]
#[test]
fn the_emitted_fair_w_document_is_accepted_in_a_separate_process() {
    let source = fair_w_source();
    let (request, settlement) = request_with(&source, SNAPSHOT, ExactProbLimits::default(), DOMAIN);
    let request =
        request.unwrap_or_else(|| panic!("the analyzer emits AC-1 evidence: {settlement:?}"));
    let document: Value =
        serde_json::from_str(request["certificate"].as_str().expect("certificate wire"))
            .expect("emitted fair-MDP document is JSON");

    assert_eq!(
        document["objective"]["profile"],
        "filament.probability-objective/cyclic-mdp-fair-eventual/v1"
    );
    let fairness = document["objective"]["fairness"]
        .as_array()
        .expect("the certificate retains its fairness set");
    assert_eq!(fairness.len(), 1);
    assert_eq!(fairness[0]["fairness_kind"], "strong");
    assert_eq!(fairness[0]["granularity"], "whole");
    assert_eq!(fairness[0]["target"]["name"], "send_b");
    assert_eq!(rational_rows(&document), ["0/1", "1/1", "1/2"]);
    let mut ranks = document["ranking"]
        .as_array()
        .expect("literal progress ranks")
        .iter()
        .map(|row| row["rank"].as_u64().expect("rank"))
        .collect::<Vec<_>>();
    ranks.sort();
    assert_eq!(ranks, [0, 0, 1]);

    // `run` starts CARGO_BIN_EXE_fsl-probability-check as a child process.
    let (status, checked) = run(&request);
    assert_eq!(status, 0, "{checked}");
    assert_eq!(
        checked["value"],
        json!({"numerator": "1", "denominator": "2"})
    );
}

/// FR-1041-AC-1's `W` at one rational bit: both interval documents over the
/// fair base are accepted by fresh checker processes, and changing the initial
/// value in either document is rejected.
#[trace("TC-3212", "FR-1042-AC-22")]
#[test]
fn fair_eventual_interval_documents_are_accepted_by_separate_checker_processes() {
    let source = fair_w_source().replace("rational(1,2)", "rational(2,5)");
    let limits = ExactProbLimits {
        max_rational_bits: 1,
        max_precision_bits: 16,
        ..ExactProbLimits::default()
    };
    let (requests, settlement) = requests_with_mode(&source, SNAPSHOT, limits, DOMAIN, false);
    let [lower, upper] = requests.as_slice() else {
        panic!("the interval proof carries both documents: {settlement:?}");
    };
    let Settlement::Terminal(record) = &settlement else {
        panic!("the interval proof settles terminal: {settlement:?}");
    };
    assert!(
        matches!(
            record.value(),
            TerminalValue::Proved {
                basis: ProofBasis::ValueBounds { .. },
                ..
            }
        ),
        "{record:?}"
    );
    // The one row of positive rank in the upper document is the initial state.
    let initial = document(upper)["ranking"]
        .as_array()
        .expect("upper progress ranks")
        .iter()
        .find(|row| row["rank"] == 1)
        .expect("the initial state has rank 1")["state"]
        .clone();
    for (request, kind, replacement) in [
        (
            lower,
            "lower",
            json!({"numerator": "3", "denominator": "4"}),
        ),
        (
            upper,
            "upper",
            json!({"numerator": "1", "denominator": "4"}),
        ),
    ] {
        let doc = document(request);
        assert_eq!(doc["kind"], kind);
        assert_eq!(
            doc["objective"]["profile"],
            "filament.probability-objective/value-bounds/v1"
        );
        assert_eq!(
            doc["objective"]["base"]["profile"],
            "filament.probability-objective/cyclic-mdp-fair-eventual/v1"
        );
        let (status, output) = run(request);
        assert_eq!(status, 0, "{kind}: {output}");
        let altered = mutate(request, |doc| {
            let row = doc["values"]
                .as_array_mut()
                .expect("value rows")
                .iter_mut()
                .find(|row| row["state"] == initial)
                .expect("the initial state is valued");
            assert_ne!(row["value"], replacement);
            row["value"] = replacement.clone();
        });
        let (status, output) = run(&altered);
        assert_eq!(status, 1, "{kind}: {output}");
        assert_eq!(output["status"], "rejected", "{kind}: {output}");
        assert_eq!(output["at"]["product_key"], initial, "{kind}: {output}");
    }
}
