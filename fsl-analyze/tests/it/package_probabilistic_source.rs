// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSL FR-740 "Source objects", FR-940-AC-6: a `package` request that names a
//! probabilistic claim of a package with source objects settles a located
//! refusal at the claim, through the production route (compile handoff,
//! strict read, `settle_package_run`). The package's rows rebuild the
//! probabilistic model but carry no effects, fields or populations, so no
//! engine steps it; a stepping item over the same package keeps its
//! source-object refusal.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{analyze, package_selection, Run, Workspace};
use crate::prob_world;

const CLAIM: &str = "probabilistic Z using ev on M over (c: M::Coin) under Even on origin \
                     { probability >= 0.5 [ always[0,3] holds(c.v = 0) ] }\n";
const SOURCE_OBJECT: &str =
    "object O { n: Integer; operation step(k: Int[1, 3]) modifies self.n; }\n\
                             random O::step(k) ~ { 1: 1, 3: 3 };\n\
                             workload W on O { weight O::step = 1; }\n";

/// The package of the coin model with `extra` after its declarations,
/// analyzed over `items`.
fn run(name: &str, extra: &str, items: Value) -> Run {
    let ws = Workspace::new(name);
    let model = prob_world::coin(extra);
    let (unit, document) = model.unit_and_document();
    ws.write("program.fsl", unit.as_bytes());
    ws.write("model.json", &document);
    let request = json!({
        "program": {"source": {"file": "program.fsl", "authority": "test",
            "identity": "analyze-unit", "document": "AnalyzeUnit", "formal_revision": 1}},
        "models": [{"format": "semantic-ir/2.0.0", "source": {"file": "model.json",
            "authority": "test", "identity": "coin-model", "document": "Coin",
            "formal_revision": 1}}],
        "items": items,
    });
    let (members, _) = package_selection(&ws, &request);
    let document = json!({"format": "native-analyze/1", "request": members});
    analyze(&ws.write("analyze-package.json", document.to_string().as_bytes()))
}

fn diagnostic(run: &Run) -> Value {
    run.document()["diagnostics"][0].clone()
}

fn assert_in_words(diagnostic: &Value) -> String {
    let message = diagnostic["message"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    for debug in ["CheckedNodeId", "Span {", "RawSourceRef", "{ "] {
        assert!(!message.contains(debug), "{message}");
    }
    assert_eq!(diagnostic["locus"]["kind"], "occurrence", "{diagnostic}");
    assert!(
        diagnostic["locus"]["occurrence"]
            .as_str()
            .is_some_and(|text| !text.is_empty()),
        "{diagnostic}"
    );
    message
}

/// FR-740-AC-6, FR-940-AC-6: a claim whose probabilistic model rebuilds from
/// the rows is refused at the claim's node, naming the claim and the missing
/// effects, fields and populations.
#[trace("TC-3012", "FR-740-AC-6")]
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn a_probabilistic_claim_over_a_source_object_package_is_refused_at_the_claim() {
    let run = run(
        "prob-source-resolved",
        &format!("{SOURCE_OBJECT}{CLAIM}"),
        json!([{"claim": "Z"}]),
    );
    assert_ne!(run.exit, 0);
    let diagnostic = diagnostic(&run);
    assert_eq!(diagnostic["code"], "unsupported_construct", "{diagnostic}");
    let message = assert_in_words(&diagnostic);
    assert!(message.contains("probabilistic claim `Z`"), "{message}");
    assert!(
        message.contains("no effects, fields or populations"),
        "{message}"
    );
}

/// FR-740-AC-6: a claim of a package whose source-object rows do not let the
/// probabilistic model rebuild is refused at the claim, naming it and why.
#[trace("TC-3012", "FR-740-AC-6")]
#[test]
fn a_probabilistic_claim_whose_model_does_not_rebuild_is_refused_at_the_claim() {
    let unread = "object O { n: Integer; operation odd(r: Rational[0, 4; 1, 4]) modifies self.n; \
                  operation plain(k: Int[1, 3]) modifies self.n; }\n\
                  random O::plain(k) ~ { 1: 1, 3: 3 };\n\
                  workload W on O { weight O::odd = 1; weight O::plain = 1; }\n";
    let run = run(
        "prob-source-unresolved",
        &format!("{unread}{CLAIM}"),
        json!([{"claim": "Z"}]),
    );
    let diagnostic = diagnostic(&run);
    let message = assert_in_words(&diagnostic);
    assert!(message.contains("probabilistic claim `Z`"), "{message}");
    assert!(message.contains("does not resolve"), "{message}");
    assert!(message.contains("parameter `r`"), "{message}");
}

/// FR-940-AC-6: an item that names no probabilistic claim of a source-object
/// package settles as before, the source-object refusal of the model-check
/// input, which has no claim to locate.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn an_item_that_names_no_probabilistic_claim_keeps_its_source_object_refusal() {
    let run = run(
        "prob-source-other",
        &format!("{SOURCE_OBJECT}{CLAIM}"),
        json!([{"claim": "NoSuchClaim"}]),
    );
    let message = diagnostic(&run)["message"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    assert!(!message.contains("probabilistic claim"), "{message}");
}

/// The package without a source object has no such refusal: its claim
/// settles as every other probabilistic claim of the package route does.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn a_package_without_source_objects_is_not_refused_by_the_probabilistic_wiring() {
    let run = run("prob-no-source", CLAIM, json!([{"claim": "Z"}]));
    let message = diagnostic(&run)["message"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    assert!(!message.contains("source objects"), "{message}");
}

/// FR-740-AC-6: a package with two source object types resolves its claim
/// over both types' rows. The compile request names one program, so the
/// two-unit identities are read at the reader by `source_object_probabilistic`
/// in fsl-eval's `it` target.
#[trace("TC-3012", "FR-740-AC-6")]
#[test]
fn a_package_with_two_source_object_types_resolves_its_claim() {
    let run = run(
        "prob-source-two-objects",
        &format!(
            "{SOURCE_OBJECT}object P {{ m: Integer; operation go(j: Int[1, 2]) modifies self.m; }}\n\
             random P::go(j) ~ {{ 1: 1, 2: 1 }};\n{CLAIM}"
        ),
        json!([{"claim": "Z"}]),
    );
    let message = assert_in_words(&diagnostic(&run));
    assert!(message.contains("probabilistic claim `Z`"), "{message}");
}
