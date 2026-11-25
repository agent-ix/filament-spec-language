// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-3212 steps 1 and 2 (FR-1042): `lower` and `upper` documents over an
//! unbounded eventual base, checked in a separate `fsl-probability-check`
//! process as the claim side or as the counterpart, with policy hints and the
//! checker's own selection of an existential action.
//!
//! Each document wraps the base objective of an exact proof the analyzer
//! emits for the same model, with the values the criterion names.

use super::bounded::{
    document, mutate, replace_source_and_rebind, request_with, requests_with_mode, run,
};
use fsl_analyze::exact_probabilistic::ExactProbLimits;
use ix_trace_rs::trace;
use serde_json::{Value, json};

const SNAPSHOT: &[u8] = include_bytes!("../fixtures/link.snapshot.json");
const VALUE_BOUNDS: &str = "filament.probability-objective/value-bounds/v1";

fn domain() -> String {
    include_str!("../fixtures/reward.semantic-ir.json").replace(
        "\"typeRef\":\"ix://filament/native/Boolean\"}],\"post\"",
        "\"typeRef\":\"ix://test/link/Attempts\"}],\"post\"",
    )
}

fn header() -> String {
    let language = include_str!("../fixtures/link.native")
        .lines()
        .next()
        .unwrap();
    format!(
        "{language}\n\
        profile v = \"filament.model.complete/v1\";\n\
        profile inf = \"filament.temporal.infinite-trace/v1\";\n\
        model M = \"test/link\";\n"
    )
}

fn claim(op: &str, threshold: &str) -> String {
    format!(
        "probabilistic Likely using inf on M over (m: M::Msg) under every scheduler on origin {{ probability {op} {threshold} [ eventually holds(m.delivered) ] }}\n"
    )
}

/// AC-1's cyclic `link` chain: from the undelivered `s`, `send_a` moves
/// `1/3` each to `s`, to delivered `g` and to the stopped terminal `f`.
fn link(op: &str, threshold: &str) -> String {
    format!(
        "{}\
        pre A using v on M::Msg::send_a {{ self.attempts < 2 or self.delivered }}\n\
        post A_done using v on M::Msg::send_a {{ self.attempts - (if lost = 0 then 0 else 2) = 0 and self.delivered = (lost = 1) }}\n\
        pre B using v on M::Msg::send_b {{ false }}\n\
        post B_done using v on M::Msg::send_b {{ self.attempts = 0 and not self.delivered }}\n\
        pre C using v on M::Msg::send_c {{ false }}\n\
        post C_done using v on M::Msg::send_c {{ self.attempts = 0 and not self.delivered }}\n\
        random Msg::send_a(lost) ~ {{ 0: 1, 1: 1, 2: 1 }};\n\
        random Msg::send_b(lost) ~ {{ 0: 1, 1: 1, 2: 1 }};\n\
        random Msg::send_c(lost) ~ {{ 0: 1, 1: 1, 2: 1 }};\n\
        terminal Stop using v on M when {{ self.attempts = 2 and not self.delivered }};\n{}",
        header(),
        claim(op, threshold)
    )
}

/// FR-1041-AC-1's `W` with no fairness set: the undelivered `s` has `wait`
/// (`send_a`, back to `s`) and `go` (`send_b`, `1/2` each to delivered `g`
/// and to the undelivered terminal `f`). Maximum `1/2`, minimum 0.
fn w(op: &str, threshold: &str) -> String {
    format!(
        "{}\
        pre A using v on M::Msg::send_a {{ self.attempts = 0 }}\n\
        post AD using v on M::Msg::send_a {{ self.attempts = 0 and not self.delivered }}\n\
        pre B using v on M::Msg::send_b {{ self.attempts = 0 }}\n\
        post BD using v on M::Msg::send_b {{ self.attempts - (if lost = 0 then 0 else if lost = 1 then 1 else 2) = 0 and self.delivered = (lost = 1) }}\n\
        pre C using v on M::Msg::send_c {{ false }}\n\
        post CD using v on M::Msg::send_c {{ self.attempts = 0 and not self.delivered }}\n\
        random Msg::send_a(lost) ~ {{ 0: 1, 1: 1, 2: 1 }};\n\
        random Msg::send_b(lost) ~ {{ 1: 1, 2: 1 }};\n\
        random Msg::send_c(lost) ~ {{ 0: 1, 1: 1, 2: 1 }};\n\
        terminal Stop using v on M when {{ self.attempts = 2 and not self.delivered }};\n{}",
        header(),
        claim(op, threshold)
    )
}

/// The analyzer's exact proof request for `source`.
fn proof(source: &str) -> Value {
    let (request, settlement) = request_with(
        source,
        SNAPSHOT,
        ExactProbLimits::default(),
        domain().as_bytes(),
    );
    request.unwrap_or_else(|| panic!("an exact proof: {settlement:?}\n{source}"))
}

/// The three keys of a one-loop product: the ranked initial `s`, the target
/// `g` and the other decided state `f`.
struct Keys {
    s: Value,
    g: Value,
    f: Value,
}

fn keys(exact: &Value) -> Keys {
    let doc = document(exact);
    let g = doc["objective"]["targets"][0].clone();
    let rows = doc["ranking"].as_array().unwrap();
    let s = rows
        .iter()
        .find(|row| row["rank"] != 0)
        .expect("one ranked undecided state")["state"]
        .clone();
    let f = rows
        .iter()
        .map(|row| row["state"].clone())
        .find(|state| *state != s && *state != g)
        .expect("a non-target decided state");
    Keys { s, g, f }
}

fn rational(n: i64, d: i64) -> Value {
    json!({"numerator": n.to_string(), "denominator": d.to_string()})
}

/// A `kind` document over `exact`'s base objective, with `values` in place of
/// the exact values, the exact ranking for a `lower` document, the policy
/// `hints` when given, offered in the role `claim_side` names.
fn bound(
    exact: &Value,
    kind: &str,
    values: &[(&Value, Value)],
    hints: Option<Value>,
    claim_side: bool,
) -> Value {
    let base = document(exact);
    let mut doc = json!({
        "identity": base["identity"],
        "objective": {"profile": VALUE_BOUNDS, "base": base["objective"]},
        "kind": kind,
        "values": base["values"],
    });
    if kind == "lower" {
        doc["ranking"] = base["ranking"].clone();
    }
    for row in doc["values"].as_array_mut().unwrap() {
        if let Some((_, value)) = values.iter().find(|(state, _)| **state == row["state"]) {
            row["value"] = value.clone();
        }
    }
    if let Some(hints) = hints {
        doc["policy"] = hints;
    }
    let mut request = exact.clone();
    request["certificate"] = json!(serde_json::to_string(&doc).unwrap());
    request["claim_side"] = json!(claim_side);
    request
}

/// Edit the offered document of `request`.
fn edit(request: &Value, change: impl FnOnce(&mut Value)) -> Value {
    let mut changed = request.clone();
    let mut doc = document(request);
    change(&mut doc);
    changed["certificate"] = json!(serde_json::to_string(&doc).unwrap());
    changed
}

fn accepted(request: &Value, value: Value) -> Value {
    let (code, output) = run(request);
    assert_eq!(code, 0, "{output}");
    assert_eq!(output["value"], value, "{output}");
    output
}

fn rejected(request: &Value, rule: &str, at: Value) {
    let (code, output) = run(request);
    assert_eq!(code, 1, "{output}");
    assert_eq!(output["rule"], rule, "{output}");
    assert_eq!(output["at"], at, "{output}");
}

fn at_key(key: &Value) -> Value {
    json!({"product_key": key})
}

fn at_member(member: &str) -> Value {
    json!({"member": member})
}

/// The `{state, choice}` hint naming the action the checker reported as
/// `action` at `state`.
fn hint(state: &Value, action: &Value) -> Value {
    let body = serde_json::to_vec(action).unwrap();
    let fsl_replay::ProductAction::Identity(schedule) =
        fsl_replay::ProductAction::parse(&body).unwrap()
    else {
        panic!("an identity action: {action}");
    };
    let limits = quire_canonical::Limits::new(1_000_000);
    let transition = schedule.transition_identity(limits).unwrap();
    let choice: Value =
        serde_json::from_slice(&transition.canonical_bytes(limits).unwrap()).unwrap();
    json!({"state": state, "choice": choice})
}

/// The single policy row a checker output reports at `state`.
fn selected(output: &Value, state: &Value) -> Value {
    let rows = output["policy"].as_array().expect("a selected policy");
    assert_eq!(rows.len(), 1, "{output}");
    assert_eq!(rows[0]["state"], *state, "{output}");
    rows[0]["action"].clone()
}

fn names(action: &Value, operation: &str) -> bool {
    action.to_string().contains(operation)
}

/// AC-1, AC-2 and AC-16 over the cyclic `link` chain (value `1/2`) for
/// `probability >= rational(2,5)`: the `lower` `s` `7/16`, rank 1, is
/// accepted as the claim side and its `upper` `9/16` with the claim-side flag
/// unset; the threshold, the kind, the ranking and every value are checked.
#[trace("TC-3212", "FR-1042-AC-1")]
#[trace("TC-3212", "FR-1042-AC-2")]
#[trace("TC-3212", "FR-1042-AC-16")]
#[test]
fn cyclic_chain_bounds_are_checked_by_their_claim_side_role() {
    let exact = proof(&link(">=", "rational(2,5)"));
    assert_eq!(
        document(&exact)["objective"]["profile"],
        "filament.probability-objective/cyclic-dtmc-eventual/v1"
    );
    let Keys { s, g, f } = keys(&exact);
    let lower = bound(&exact, "lower", &[(&s, rational(7, 16))], None, true);
    let upper = bound(&exact, "upper", &[(&s, rational(9, 16))], None, false);
    accepted(&lower, rational(7, 16));
    accepted(&upper, rational(9, 16));

    // The claim side meets the threshold; the counterpart role tests none.
    let stricter_exact = {
        let mut request = exact.clone();
        replace_source_and_rebind(&mut request, &link(">=", "rational(1,2)"));
        request
    };
    let stricter = bound(
        &stricter_exact,
        "lower",
        &[(&s, rational(7, 16))],
        None,
        true,
    );
    rejected(&stricter, "threshold-missed", at_key(&s));
    let mut counterpart = stricter.clone();
    counterpart["claim_side"] = json!(false);
    accepted(&counterpart, rational(7, 16));

    // An `upper` document cannot be the claim side of an `AtLeast` claim.
    let mut misplaced = upper.clone();
    misplaced["claim_side"] = json!(true);
    rejected(&misplaced, "malformed", at_member("kind"));
    // An exact document is never a counterpart.
    let mut exact_counterpart = exact.clone();
    exact_counterpart["claim_side"] = json!(false);
    rejected(&exact_counterpart, "malformed", at_member("kind"));

    // One value altered: each side's inequality rejects at that state.
    rejected(
        &bound(&exact, "lower", &[(&s, rational(5, 8))], None, true),
        "inequality-fails",
        at_key(&s),
    );
    rejected(
        &bound(&exact, "upper", &[(&s, rational(7, 16))], None, false),
        "inequality-fails",
        at_key(&s),
    );
    rejected(
        &bound(
            &exact,
            "upper",
            &[(&s, rational(9, 16)), (&f, rational(1, 16))],
            None,
            false,
        ),
        "inequality-fails",
        at_key(&f),
    );
    rejected(
        &bound(
            &exact,
            "lower",
            &[(&s, rational(7, 16)), (&g, rational(15, 16))],
            None,
            true,
        ),
        "inequality-fails",
        at_key(&g),
    );
    let unranked = edit(&lower, |doc| {
        doc.as_object_mut().unwrap().remove("ranking");
    });
    rejected(&unranked, "malformed", at_member("ranking"));
    // A chain has no existential choice to hint.
    let hinted = edit(&lower, |doc| {
        doc["policy"] = json!([]);
    });
    rejected(&hinted, "malformed", at_member("policy"));
    let null_policy = edit(&lower, |doc| {
        doc["policy"] = Value::Null;
    });
    rejected(&null_policy, "malformed", at_member("policy"));
}

/// FR-1042-AC-21: a one-action cyclic MDP exceeds the rational budget but its
/// interval documents still pass the independent checker, including mutation.
#[trace("TC-3212", "FR-1042-AC-21")]
#[test]
fn cyclic_mdp_interval_documents_are_accepted_by_the_separate_checker() {
    let source = link(">=", "rational(2,5)");
    let limits = ExactProbLimits {
        max_rational_bits: 1,
        precision_bits: 4,
        max_precision_bits: 16,
        ..ExactProbLimits::default()
    };
    let (requests, settlement) =
        requests_with_mode(&source, SNAPSHOT, limits, domain().as_bytes(), false);
    let [claim, counterpart] = requests.as_slice() else {
        panic!("interval proof carries both documents: {settlement:?}");
    };
    let fsl_analyze::analyze::Settlement::Terminal(record) = settlement else {
        panic!("interval proof settles terminal: {settlement:?}");
    };
    assert!(matches!(
        record.value(),
        fsl_replay::TerminalValue::Proved {
            basis: fsl_replay::ProofBasis::ValueBounds {
                method: fsl_replay::BoundsMethod::IntervalIteration { .. },
                ..
            },
            certification: fsl_replay::Certification::Certified,
        }
    ));
    assert_eq!(document(claim)["kind"], "lower");
    assert_eq!(claim["claim_side"], true);
    assert_eq!(document(counterpart)["kind"], "upper");
    assert_eq!(counterpart["claim_side"], false);
    for request in [claim, counterpart] {
        let (code, output) = run(request);
        assert_eq!(code, 0, "{output}");
        let altered = mutate(request, |doc| {
            doc["values"][0]["value"] = rational(1, 7);
        });
        let (code, output) = run(&altered);
        assert_ne!(code, 0, "a changed interval row is rejected: {output}");
    }
}

/// The `<= rational(3,5)` proof over `W` and the same proof rebound to the
/// minimum claim `>= rational(1,4)`, with its objective's optimization made
/// the minimum's.
fn w_proofs() -> (Value, Value, Keys) {
    let maximum = proof(&w("<=", "rational(3,5)"));
    assert_eq!(
        document(&maximum)["objective"]["profile"],
        "filament.probability-objective/cyclic-mdp-eventual/v1"
    );
    let keys = keys(&maximum);
    let mut minimum = maximum.clone();
    replace_source_and_rebind(&mut minimum, &w(">=", "rational(1,4)"));
    let minimum = edit(&minimum, |doc| {
        doc["objective"]["optimization"] = json!("minimum");
    });
    (maximum, minimum, keys)
}

/// AC-3, AC-14 and AC-17 over `W`: the universal side of each extremum
/// checks every action, and the existential side checks the hinted action or
/// selects the first satisfying one in canonical full-product-action order.
#[trace("TC-3212", "FR-1042-AC-3")]
#[trace("TC-3212", "FR-1042-AC-14")]
#[trace("TC-3212", "FR-1042-AC-17")]
#[test]
fn cyclic_mdp_bounds_check_hinted_and_selected_policies() {
    let (maximum, minimum, Keys { s, .. }) = w_proofs();

    // Maximum (1/2), `probability <= rational(3,5)`: the claim-side `upper`
    // holds under both actions.
    accepted(
        &bound(&maximum, "upper", &[(&s, rational(3, 5))], None, true),
        rational(3, 5),
    );
    accepted(
        &bound(&maximum, "upper", &[(&s, rational(1, 2))], None, true),
        rational(1, 2),
    );
    rejected(
        &bound(&maximum, "upper", &[(&s, rational(2, 5))], None, true),
        "inequality-fails",
        at_key(&s),
    );
    // Its `lower` counterpart without a hint selects `go`: `wait` meets the
    // inequality but makes no progress.
    let output = accepted(
        &bound(&maximum, "lower", &[(&s, rational(7, 16))], None, false),
        rational(7, 16),
    );
    let go = selected(&output, &s);
    assert!(names(&go, "send_b"), "{go}");

    // A minimum objective cannot back an every-scheduler `<=` claim: that
    // claim requires the maximum objective. Keep the mismatch as a refusal.
    let output = accepted(
        &bound(&minimum, "upper", &[(&s, rational(1, 4))], None, false),
        rational(1, 4),
    );
    let wait = selected(&output, &s);
    let minimum_at_most = maximum.clone();
    let minimum_at_most = edit(&minimum_at_most, |doc| {
        doc["objective"]["optimization"] = json!("minimum");
    });
    let mismatched = bound(
        &minimum_at_most,
        "upper",
        &[(&s, rational(1, 4))],
        Some(json!([hint(&s, &wait)])),
        true,
    );
    let (code, output) = run(&mismatched);
    assert_eq!(code, 2, "{output}");
    assert_eq!(output["status"], "refused", "{output}");
    assert_eq!(
        output["reason"],
        "invalid_runtime_input: probability certificate objective mismatch",
        "{output}"
    );

    // A minimum `upper` is still accepted as a counterpart, even when its
    // value is below the actual minimum claim's `>= 1/4` threshold.
    let output = accepted(
        &bound(
            &minimum,
            "upper",
            &[(&s, rational(0, 1))],
            Some(json!([hint(&s, &wait)])),
            false,
        ),
        rational(0, 1),
    );
    assert_eq!(selected(&output, &s), wait);

    // The same minimum `upper` is valid as a counterpart to its actual
    // minimum claim. The counterpart role checks its conditions, not a
    // threshold.
    let output = accepted(
        &bound(
            &minimum,
            "upper",
            &[(&s, rational(1, 4))],
            Some(json!([hint(&s, &wait)])),
            false,
        ),
        rational(1, 4),
    );
    assert_eq!(selected(&output, &s), wait);

    // AC-3, AC-17: hinted `go` is checked and accepted for the maximum
    // `lower`; hinted `wait` is rank-broken there, and with the value 1 no
    // action satisfies both inequality and progress.
    let output = accepted(
        &bound(
            &maximum,
            "lower",
            &[(&s, rational(7, 16))],
            Some(json!([hint(&s, &go)])),
            false,
        ),
        rational(7, 16),
    );
    assert_eq!(selected(&output, &s), go);
    rejected(
        &bound(
            &maximum,
            "lower",
            &[(&s, rational(7, 16))],
            Some(json!([hint(&s, &wait)])),
            false,
        ),
        "rank-broken",
        at_key(&s),
    );
    rejected(
        &bound(
            &maximum,
            "lower",
            &[(&s, rational(1, 1))],
            Some(json!([hint(&s, &wait)])),
            false,
        ),
        "rank-broken",
        at_key(&s),
    );
    rejected(
        &bound(&maximum, "lower", &[(&s, rational(1, 1))], None, false),
        "rank-broken",
        at_key(&s),
    );

    // AC-3, AC-14, AC-17: the minimum `upper` `1/4` with hint `wait` is
    // accepted; with hint `go` it is rejected although `wait` satisfies.
    let output = accepted(
        &bound(
            &minimum,
            "upper",
            &[(&s, rational(1, 4))],
            Some(json!([hint(&s, &wait)])),
            false,
        ),
        rational(1, 4),
    );
    assert_eq!(selected(&output, &s), wait);
    rejected(
        &bound(
            &minimum,
            "upper",
            &[(&s, rational(1, 4))],
            Some(json!([hint(&s, &go)])),
            false,
        ),
        "inequality-fails",
        at_key(&s),
    );
    // AC-14: a hint naming an action not enabled at `s` rejects even though
    // `wait` satisfies.
    let mut disabled = hint(&s, &go);
    let operation = disabled["choice"]["operation"].as_str().unwrap().to_owned();
    disabled["choice"]["operation"] = json!(operation.replace("send_b", "send_c"));
    rejected(
        &bound(
            &minimum,
            "upper",
            &[(&s, rational(1, 4))],
            Some(json!([disabled])),
            false,
        ),
        "inequality-fails",
        at_key(&s),
    );
    // AC-14: with `1/2` both actions satisfy; the literal canonical first
    // action is `send_a`.
    let output = accepted(
        &bound(&minimum, "upper", &[(&s, rational(1, 2))], None, false),
        rational(1, 2),
    );
    let first = selected(&output, &s);
    assert!(names(&first, "ix://test/link/Msg/send_a"), "{first}");

    // A hint has no place on a universal side.
    rejected(
        &bound(
            &maximum,
            "upper",
            &[(&s, rational(3, 5))],
            Some(json!([hint(&s, &go)])),
            true,
        ),
        "malformed",
        at_member("policy"),
    );
}
