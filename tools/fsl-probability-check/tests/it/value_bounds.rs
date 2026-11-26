// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-3212 steps 1 and 2 (FR-1042): `lower` and `upper` documents over an
//! unbounded eventual base, checked in a separate `fsl-probability-check`
//! process as the claim side or as the counterpart, with policy hints and the
//! checker's own selection of an existential action.
//!
//! Each document wraps the base objective of an exact proof the analyzer
//! emits for the same model, with the values the criterion names.

use super::bounded::{
    DOMAIN, document, model_source, mutate, replace_source_and_rebind, request_with,
    requests_offering, requests_with_mode, run,
};
use super::certificate_kinds::likely;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use filament_exact::{Integer, Rational};
use fsl_analyze::analyze::Settlement;
use fsl_analyze::exact_probabilistic::ExactProbLimits;
use fsl_replay::{
    BoundsExploration, BoundsMethod, Certification, ExactBound, IncompleteCause, ProofBasis,
    TerminalValue,
};
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

/// AC-1's chain with its claim and its draws under the workload `Even`.
fn link_under_even(op: &str, threshold: &str) -> String {
    let chain = link(op, threshold).replace("under every scheduler", "under Even");
    format!(
        "{chain}workload Even on M {{ weight Msg::send_a = 1; weight Msg::send_b = 1; weight Msg::send_c = 1; }}\n"
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

/// AC-1's chain stopped at `max_states` 2 omits `f` before the edges of `g`.
/// Its explored prefix bounds the value by `1/2`, which proves nothing for
/// `>= rational(9,10)`, so the item stops naming `max_states` (FR-203): the
/// completion's rows are ordered, and the solve reads them.
#[trace("TC-638", "FR-203-AC-14")]
#[test]
fn a_partial_chain_that_settles_nothing_stops_at_max_states() {
    let source = link_under_even(">=", "rational(9,10)");
    let mut limits = ExactProbLimits::default();
    limits.model.limits.max_states = 2;
    let (requests, settlement) =
        requests_with_mode(&source, SNAPSHOT, limits, domain().as_bytes(), false);
    assert!(requests.is_empty(), "{requests:?}");
    let fsl_analyze::analyze::Settlement::Terminal(record) = settlement else {
        panic!("a stopped item settles terminal: {settlement:?}");
    };
    assert_eq!(
        record.value(),
        &fsl_replay::TerminalValue::Incomplete(fsl_replay::IncompleteCause::LimitReached {
            limit: fsl_replay::ModelCheckLimit::ExactProbMaxStates,
            value: 2,
            required: None,
        }),
        "{record:?}"
    );
}

/// FR-1042-AC-18: AC-1's chain with exploration stopped at `max_states` 2,
/// which admits `s` and `g` and omits `f`. The `lower` document offering
/// `7/16` at the ranked `s` proves `>= rational(2,5)` with no counterpart and
/// the upper bound 1, and a separate checker accepts it, whether or not its
/// own `max_states` admits the whole graph. Below the threshold, or under
/// another identity, it proves nothing.
#[trace("TC-3212", "FR-1042-AC-18")]
#[test]
fn a_partial_lower_over_the_cyclic_chain_proves_without_a_counterpart() {
    let ratio = |n: i64, d: i64| Rational::new(Integer::from(n), Integer::from(d)).unwrap();
    let (requests, settlement) = partial_chain(ratio(7, 16));
    let [claim] = requests.as_slice() else {
        panic!("one claim-side document and no counterpart: {settlement:?}");
    };
    let Settlement::Terminal(record) = settlement else {
        panic!("the partial proof settles terminal: {settlement:?}");
    };
    let fsl_replay::TerminalValue::Proved {
        basis:
            fsl_replay::ProofBasis::ValueBounds {
                lower,
                upper: fsl_replay::ExactBound::Finite(upper),
                exploration: fsl_replay::BoundsExploration::Partial,
                ..
            },
        certification: fsl_replay::Certification::Certified,
    } = record.value()
    else {
        panic!("a certified partial value-bounds proof: {record:?}");
    };
    assert_eq!(
        (lower.value(), upper.value()),
        (&ratio(7, 16), &ratio(1, 1))
    );

    let emitted = document(claim);
    assert_eq!(emitted["kind"], "lower");
    assert_eq!(claim["claim_side"], true);
    let ranked: Vec<&Value> = emitted["ranking"]
        .as_array()
        .expect("the lower document's ranking")
        .iter()
        .filter(|row| row["rank"] != 0)
        .collect();
    let [row] = ranked.as_slice() else {
        panic!("one ranked state: {emitted}");
    };
    assert_eq!(row["rank"], 1);
    let s = row["state"].clone();
    let initial = emitted["values"]
        .as_array()
        .unwrap()
        .iter()
        .position(|row| row["state"] == s)
        .expect("a value at the initial key");
    assert_eq!(emitted["values"][initial]["value"], rational(7, 16));

    // The separate checker, first with room for the whole graph and then
    // stopped where the analyzer was.
    for request in [claim.clone(), within(claim, 2)] {
        accepted(&request, rational(7, 16));

        let lowered = mutate(&request, |doc| {
            doc["values"][initial]["value"] = rational(1, 4);
        });
        rejected(&lowered, "threshold-missed", at_key(&s));

        let other_identity = mutate(&request, |doc| {
            let ordinal = doc["identity"]["ordinal"].as_u64().expect("an ordinal");
            doc["identity"]["ordinal"] = json!(ordinal + 1);
        });
        refused(&other_identity, "conflicting_identity");
    }

    // Lowered below the threshold, the settlement refuses the proof too.
    let (_, settlement) = partial_chain(ratio(1, 4));
    let Settlement::Terminal(record) = settlement else {
        panic!("a rejected document settles terminal: {settlement:?}");
    };
    assert!(
        matches!(
            record.value(),
            fsl_replay::TerminalValue::Inconclusive(
                fsl_replay::InconclusiveCause::CertificateRejected {
                    rule: fsl_replay::CertificateRule::ThresholdMissed,
                    ..
                }
            )
        ),
        "{record:?}"
    );
}

/// One edit of a document, the rule that refuses it and the key it names.
type FalseRow<'a> = (&'a dyn Fn(&mut Value), &'a str, &'a Value);

/// FR-1042-AC-18, FSpec FR-412-AC-5: a document checked over its named states
/// and their successors is refused wherever the whole-graph check refuses it,
/// and reads nothing it does not name.
#[trace("TC-3212", "FR-1042-AC-18", "FSpec-FR-412-AC-5")]
#[test]
fn a_partial_lower_checked_over_its_named_states_refuses_a_false_document() {
    let (requests, settlement) =
        partial_chain(Rational::new(Integer::from(7_i64), Integer::from(16_i64)).unwrap());
    let [claim] = requests.as_slice() else {
        panic!("one claim-side document: {settlement:?}");
    };
    let emitted = document(claim);
    let g = emitted["objective"]["base"]["targets"][0].clone();
    let s = emitted["values"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["state"].clone())
        .find(|state| *state != g)
        .expect("the initial key");
    let exact = proof(&link_under_even(">=", "rational(2,5)"));
    let f = document(&exact)["ranking"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["state"].clone())
        .find(|state| *state != s && *state != g)
        .expect("the rejecting terminal the partial document omits");
    let narrow = within(claim, 2);
    let wide = claim.clone();

    // A value above the one-step value, a successor left out, no decreasing
    // successor, and a rejecting state named to raise `s`.
    let inflated = |doc: &mut Value| set(doc, "values", &s, "value", rational(5, 8));
    let without_target = |doc: &mut Value| {
        drop_row(doc, "values", &g);
        drop_row(doc, "ranking", &g);
    };
    let unranked_progress = |doc: &mut Value| set(doc, "ranking", &s, "rank", json!(0));
    let raised_failure = |doc: &mut Value| {
        drop_row(doc, "values", &g);
        drop_row(doc, "ranking", &g);
        insert_row(doc, "values", json!({"state": f, "value": rational(1, 1)}));
        insert_row(doc, "ranking", json!({"state": f, "rank": 0}));
    };
    let changes: [FalseRow; 4] = [
        (&inflated, "inequality-fails", &s),
        (&without_target, "rank-broken", &s),
        (&unranked_progress, "rank-broken", &s),
        (&raised_failure, "inequality-fails", &f),
    ];
    for (change, rule, at) in changes {
        for request in [&narrow, &wide] {
            rejected(&mutate(request, change), rule, at_key(at));
        }
    }

    // Over the named states, a ranking row for an unnamed state is malformed,
    // and more rows than `max_states` admits stop the check.
    let ranked_unnamed = mutate(&narrow, |doc| {
        drop_row(doc, "ranking", &g);
        insert_row(doc, "ranking", json!({"state": f, "rank": 0}));
    });
    rejected(&ranked_unnamed, "malformed", at_member("ranking"));
    let overfull = mutate(&narrow, |doc| {
        insert_row(doc, "ranking", json!({"state": f, "rank": 0}));
    });
    stopped_at_max_states(&overfull, 2);

    // Targets that are not the explored part's.
    for targets in [json!([]), json!([s])] {
        for request in [&narrow, &wide] {
            let changed = mutate(request, |doc| {
                doc["objective"]["base"]["targets"] = targets.clone();
            });
            refused(&changed, "objective mismatch");
        }
    }

    // Without the initial key the named states reach nothing.
    let without_initial = mutate(&narrow, |doc| {
        drop_row(doc, "values", &s);
        drop_row(doc, "ranking", &s);
    });
    stopped_at_max_states(&without_initial, 2);

    // More names than `max_states` admits, and a document that is not the
    // claim side, are not read over the named states.
    stopped_at_max_states(&within(claim, 1), 1);
    let mut counterpart = narrow.clone();
    counterpart["claim_side"] = json!(false);
    stopped_at_max_states(&counterpart, 2);
}

/// AC-1's chain under `Even` explored to `max_states` 2, offering `value` at
/// the one undecided state `s`; `g` holds exactly 1.
fn partial_chain(value: Rational) -> (Vec<Value>, Settlement) {
    let mut limits = ExactProbLimits::default();
    limits.model.limits.max_states = 2;
    requests_offering(
        &link_under_even(">=", "rational(2,5)"),
        SNAPSHOT,
        limits,
        domain().as_bytes(),
        &|certificate| {
            assert_eq!(certificate.values.len(), 2, "the explored prefix");
            let s = certificate
                .values
                .iter_mut()
                .find(|entry| entry.value != Rational::from_integer(Integer::from(1_i64)))
                .expect("the undecided initial state");
            s.value = value.clone();
        },
    )
}

/// `request` checked under `max_states`.
fn within(request: &Value, max_states: u64) -> Value {
    let mut changed = request.clone();
    changed["limits"]["states"] = json!(max_states);
    changed
}

/// Set `member` of the `list` row at `state`.
fn set(doc: &mut Value, list: &str, state: &Value, member: &str, value: Value) {
    let row = doc[list]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["state"] == *state)
        .expect("a row at the state");
    row[member] = value;
}

fn drop_row(doc: &mut Value, list: &str, state: &Value) {
    doc[list]
        .as_array_mut()
        .unwrap()
        .retain(|row| row["state"] != *state);
}

/// Add `row` to `list`, keeping complete-key order.
fn insert_row(doc: &mut Value, list: &str, row: Value) {
    let rows = doc[list].as_array_mut().unwrap();
    rows.push(row);
    rows.sort_by_key(|row| {
        URL_SAFE_NO_PAD
            .decode(row["state"].as_str().expect("an encoded key"))
            .expect("base64url")
    });
}

fn refused(request: &Value, reason: &str) {
    let (code, output) = run(request);
    assert_eq!(code, 2, "{output}");
    assert_eq!(output["status"], "refused", "{output}");
    assert!(
        output["reason"].as_str().unwrap().contains(reason),
        "{output}"
    );
}

fn unsupported(request: &Value) {
    let (code, output) = run(request);
    assert_eq!(code, 2, "{output}");
    assert_eq!(output["status"], "unsupported", "{output}");
}

fn stopped_at_max_states(request: &Value, bound: u64) {
    let (code, output) = run(request);
    assert_eq!(code, 2, "{output}");
    assert_eq!(output["status"], "stopped", "{output}");
    assert_eq!(
        output["limit"]["setting"], "exact_prob.max_states",
        "{output}"
    );
    assert_eq!(output["limit"]["bound"], json!(bound), "{output}");
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

/// An `eventually` chain with no cycle before its target, under `Even`:
/// `send_b` takes `s` to the delivered `g` or the stopped terminal `f` with
/// `1/2` each (value `1/2`). With `continues`, the model runs one `send_a`
/// step past `g` to a terminal state; otherwise it stops there.
fn acyclic(threshold: &str, continues: bool) -> String {
    let (after, stop) = if continues {
        ("self.delivered and self.attempts = 1", "self.attempts = 2")
    } else {
        ("false", "self.delivered or self.attempts = 2")
    };
    let chain = format!(
        "{}\
        pre A using v on M::Msg::send_a {{ {after} }}\n\
        post A_done using v on M::Msg::send_a {{ self.attempts = 2 and self.delivered }}\n\
        pre B using v on M::Msg::send_b {{ self.attempts = 0 and not self.delivered }}\n\
        post BD using v on M::Msg::send_b {{ self.attempts - (if lost = 1 then 1 else 2) = 0 and self.delivered = (lost = 1) }}\n\
        pre C using v on M::Msg::send_c {{ false }}\n\
        post CD using v on M::Msg::send_c {{ self.attempts = 0 and not self.delivered }}\n\
        random Msg::send_a(lost) ~ {{ 0: 1, 1: 1, 2: 1 }};\n\
        random Msg::send_b(lost) ~ {{ 1: 1, 2: 1 }};\n\
        random Msg::send_c(lost) ~ {{ 0: 1, 1: 1, 2: 1 }};\n\
        terminal Stop using v on M when {{ {stop} }};\n{}",
        header(),
        claim(">=", threshold)
    )
    .replace("under every scheduler", "under Even");
    format!(
        "{chain}workload Even on M {{ weight Msg::send_a = 1; weight Msg::send_b = 1; weight Msg::send_c = 1; }}\n"
    )
}

/// FR-1042 "Bases" and "Conditions": the acyclic `reachability/v1` base of an
/// unbounded `eventually` under a workload takes `lower` and `upper`
/// documents, checked like a chain's: every value, the threshold and the
/// `lower` document's progress ranking.
#[trace("TC-3151", "FR-1042-AC-25")]
#[test]
fn an_acyclic_eventual_base_takes_checked_bounds() {
    let exact = proof(&acyclic("rational(2,5)", false));
    assert_eq!(
        document(&exact)["objective"]["profile"],
        "filament.probability-objective/reachability/v1"
    );
    let keys = keys(&exact);
    let s = &keys.s;

    let lower = bound(&exact, "lower", &[(s, rational(7, 16))], None, true);
    accepted(&lower, rational(7, 16));
    let upper = bound(&exact, "upper", &[(s, rational(9, 16))], None, false);
    accepted(&upper, rational(9, 16));

    rejected(
        &bound(&exact, "lower", &[(s, rational(5, 8))], None, true),
        "inequality-fails",
        at_key(s),
    );
    rejected(
        &bound(&exact, "upper", &[(s, rational(7, 16))], None, false),
        "inequality-fails",
        at_key(s),
    );
    rejected(
        &bound(&exact, "lower", &[(&keys.f, rational(1, 16))], None, true),
        "inequality-fails",
        at_key(&keys.f),
    );
    rejected(
        &bound(&exact, "lower", &[(s, rational(3, 8))], None, true),
        "threshold-missed",
        at_key(s),
    );
    let unranked = edit(&lower, |doc| {
        doc.as_object_mut().unwrap().remove("ranking");
    });
    rejected(&unranked, "malformed", at_member("ranking"));
    let flat = edit(&lower, |doc| set(doc, "ranking", s, "rank", json!(0)));
    rejected(&flat, "rank-broken", at_key(s));
    // An unnamed successor counts as 0, so without `g` no successor of `s`
    // shows progress.
    let unnamed = edit(&lower, |doc| {
        drop_row(doc, "values", &keys.g);
        drop_row(doc, "ranking", &keys.g);
    });
    rejected(&unnamed, "rank-broken", at_key(s));
}

/// FR-1042 "Document", FSpec FR-411 "Evidence on a partial product": where
/// `max_states` stops the explored model past a target but admits the graph
/// that stops there, the `lower` document keeps the acyclic base, names that
/// graph's states alone and proves without a counterpart.
#[trace("TC-3151", "FR-1042-AC-25")]
#[test]
fn a_partial_lower_over_an_acyclic_eventual_names_only_the_stopped_graph() {
    // `max_states` 3 admits `s`, `g` and `f` and omits the state past `g`.
    // `max_states` 2 admits the initial state and one past it, where the
    // target already holds initially and the graph is that one state.
    let initially =
        link_under_even(">=", "rational(2,5)").replace("holds(m.delivered)", "holds(true)");
    for (source, max_states, rows, value) in [
        (acyclic("rational(2,5)", true), 3, 3, rational(1, 2)),
        (initially, 2, 1, rational(1, 1)),
    ] {
        let mut limits = ExactProbLimits::default();
        limits.model.limits.max_states = max_states;
        let (requests, settlement) =
            requests_with_mode(&source, SNAPSHOT, limits, domain().as_bytes(), false);
        let [claim] = requests.as_slice() else {
            panic!("one claim-side document and no counterpart: {settlement:?}");
        };
        assert!(
            matches!(&settlement, Settlement::Terminal(record) if matches!(
                record.value(),
                fsl_replay::TerminalValue::Proved {
                    basis: fsl_replay::ProofBasis::ValueBounds {
                        exploration: fsl_replay::BoundsExploration::Partial,
                        ..
                    },
                    ..
                }
            )),
            "{settlement:?}"
        );
        let emitted = document(claim);
        assert_eq!(emitted["kind"], "lower");
        assert_eq!(
            emitted["objective"]["base"]["profile"],
            "filament.probability-objective/reachability/v1"
        );
        assert_eq!(emitted["values"].as_array().map(Vec::len), Some(rows));
        assert_eq!(emitted["ranking"].as_array().map(Vec::len), Some(rows));
        accepted(claim, value);
    }
}

/// The key of the one row of `exact`'s ranking that is not 0: the initial
/// state of a product with one undecided state.
fn ranked_key(exact: &Value) -> Value {
    document(exact)["ranking"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["rank"] != 0)
        .expect("one ranked undecided state")["state"]
        .clone()
}

/// A key of value 0 in `exact`: a state a `lower` document may leave unnamed.
fn zero_key(exact: &Value) -> Value {
    document(exact)["values"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["value"] == rational(0, 1))
        .expect("a state of value 0")["state"]
        .clone()
}

/// [`link_under_even`] over the bounded event `eventually[0,1]`, whose exact
/// value is `1/3`.
fn one_step_under_even() -> String {
    link_under_even(">=", "rational(1,4)")
        .replace(
            "filament.temporal.infinite-trace/v1",
            "filament.temporal.event-position.false-extension/v1",
        )
        .replace("[ eventually ", "[ eventually[0,1] ")
}

/// FR-1042 "Document": over a graph that fits `max_states`, a `lower` naming a
/// subset of its states ranks exactly those, over a cyclic, an acyclic and a
/// bounded base alike.
#[trace("TC-3151", "FR-1042-AC-26")]
#[test]
fn a_subset_lower_over_a_whole_graph_ranks_exactly_the_states_it_names() {
    for (source, value) in [
        (link_under_even(">=", "rational(2,5)"), rational(7, 16)),
        (acyclic("rational(2,5)", false), rational(7, 16)),
        (one_step_under_even(), rational(1, 3)),
    ] {
        let exact = proof(&source);
        let s = ranked_key(&exact);
        let unnamed = zero_key(&exact);
        let full = bound(&exact, "lower", &[(&s, value.clone())], None, true);
        accepted(&full, value.clone());

        let ranked_unnamed = edit(&full, |doc| drop_row(doc, "values", &unnamed));
        rejected(&ranked_unnamed, "malformed", at_member("ranking"));
        let named_only = edit(&ranked_unnamed, |doc| drop_row(doc, "ranking", &unnamed));
        accepted(&named_only, value);
    }
}

/// Pins a known gap, not required behaviour: FR-1042 "Bases" admits
/// `reachability/v1` under every scheduler as well as under a workload, and
/// neither side offered for the claim under every scheduler is checked yet.
///
/// Remaining work: QSL-3405
#[test]
fn an_acyclic_eventual_base_takes_no_bounds_under_every_scheduler() {
    let source = acyclic("rational(2,5)", false)
        .replace("under Even on origin", "under every scheduler on origin");
    let exact = proof(&source);
    assert_eq!(
        document(&exact)["objective"]["profile"],
        "filament.probability-objective/reachability/v1"
    );
    assert_eq!(document(&exact)["objective"]["optimization"], "minimum");
    for (kind, claim_side) in [("lower", true), ("upper", false)] {
        unsupported(&bound(&exact, kind, &[], None, claim_side));
    }
}

/// A fair claim over the acyclic chain has its own base, so the
/// `reachability/v1` bounds of the same model without fairness are refused as
/// a changed base.
#[trace("TC-3151", "FR-1042-AC-8")]
#[test]
fn an_acyclic_eventual_base_takes_no_bounds_for_a_fair_claim() {
    let unfair = acyclic("rational(2,5)", false)
        .replace("under Even on origin", "under every scheduler on origin");
    let fair = unfair.replace(
        "under every scheduler on origin",
        "under every scheduler fair strong M::Msg::send_b on origin",
    );
    let base = proof(&unfair);
    let fair_exact = proof(&fair);
    assert_eq!(
        document(&fair_exact)["objective"]["profile"],
        "filament.probability-objective/cyclic-mdp-fair-eventual/v1"
    );
    for (kind, claim_side) in [("lower", true), ("upper", false)] {
        let mut request = fair_exact.clone();
        let offered = edit(&bound(&base, kind, &[], None, claim_side), |doc| {
            doc["identity"] = document(&fair_exact)["identity"].clone();
        });
        request["certificate"] = offered["certificate"].clone();
        request["claim_side"] = json!(claim_side);
        refused(&request, "objective mismatch");
    }
}

/// FR-1042 "Document": where `max_states` stops the explored model but admits
/// the checker's graph, and the offered certificate leaves a state of that
/// graph out, the analyzer's `lower` ranks the states it names alone.
#[trace("TC-3151", "FR-1042-AC-26")]
#[test]
fn a_partial_lower_the_analyzer_writes_over_a_whole_graph_ranks_its_named_states() {
    let mut limits = ExactProbLimits::default();
    limits.model.limits.max_states = 3;
    let (requests, settlement) = requests_offering(
        &acyclic("rational(2,5)", true),
        SNAPSHOT,
        limits,
        domain().as_bytes(),
        &|certificate| {
            let unnamed = certificate
                .values
                .iter()
                .position(|row| row.value == Rational::from_integer(Integer::from(0_i64)))
                .expect("a state of value 0");
            certificate.values.remove(unnamed);
        },
    );
    let [claim] = requests.as_slice() else {
        panic!("one claim-side document and no counterpart: {settlement:?}");
    };
    let emitted = document(claim);
    assert_eq!(emitted["values"].as_array().map(Vec::len), Some(2));
    assert_eq!(emitted["ranking"].as_array().map(Vec::len), Some(2));
    accepted(claim, rational(1, 2));
}

/// FR-1042 "Conditions": over a bounded base a subset `lower` carries each
/// named state's remaining horizon, and no other rank.
#[trace("TC-3151", "FR-1042-AC-26")]
#[test]
fn a_subset_bounded_lower_with_a_changed_rank_is_refused() {
    let exact = proof(&one_step_under_even());
    let s = ranked_key(&exact);
    let unnamed = zero_key(&exact);
    let full = bound(&exact, "lower", &[(&s, rational(1, 3))], None, true);
    let subset = edit(&full, |doc| {
        drop_row(doc, "values", &unnamed);
        drop_row(doc, "ranking", &unnamed);
    });
    accepted(&subset, rational(1, 3));
    for rank in [0, 2] {
        let changed = edit(&subset, |doc| set(doc, "ranking", &s, "rank", json!(rank)));
        rejected(&changed, "rank-broken", at_key(&s));
    }
}

/// FR-1043-AC-2's claim at `level` and `comparison` over the cyclic `link`
/// model under every scheduler.
fn quantile(level: &str, comparison: &str) -> String {
    likely(
        "ev",
        "every scheduler",
        &format!(
            "quantile {level} of (steps from holds(true) until holds(m.delivered) within 4) {comparison}"
        ),
    )
}

/// A measure probability claim `comparison theta` over the same measure.
fn measured(comparison: &str, theta: &str) -> String {
    likely(
        "ev",
        "every scheduler",
        &format!(
            "probability {comparison} {theta} [steps from holds(true) until holds(m.delivered) within 4 <= 3]"
        ),
    )
}

/// The analyzer's settlement of `source` explored to `max_states`.
fn explored(source: &str, max_states: usize) -> (Vec<Value>, Settlement) {
    let mut limits = ExactProbLimits::default();
    limits.model.limits.max_states = max_states;
    requests_with_mode(source, SNAPSHOT, limits, domain().as_bytes(), false)
}

/// The one claim-side document of a partial proof of `source`, whose upper
/// end is `upper`.
fn partial_proof(source: &str, max_states: usize, upper: (i64, i64)) -> Value {
    let (requests, settlement) = explored(source, max_states);
    let Settlement::Terminal(record) = &settlement else {
        panic!("a partial proof settles: {settlement:?}");
    };
    let TerminalValue::Proved {
        basis:
            ProofBasis::ValueBounds {
                upper: ExactBound::Finite(end),
                method: BoundsMethod::BackwardInduction { precision_bits },
                exploration: BoundsExploration::Partial,
                ..
            },
        certification: Certification::Certified,
    } = record.value()
    else {
        panic!(
            "a certified partial ValueBounds proof: {:?}",
            record.value()
        );
    };
    assert_eq!(
        end.value(),
        &Rational::new(Integer::from(upper.0), Integer::from(upper.1)).unwrap()
    );
    assert!(*precision_bits > 0, "FSpec's PositiveCount");
    let [claim] = requests.as_slice() else {
        panic!("one claim-side document and no counterpart: {settlement:?}");
    };
    assert_eq!(document(claim)["kind"], "lower");
    assert_eq!(claim["claim_side"], true);
    claim.clone()
}

/// The state and value of each row of `doc` ranked `rank`.
fn ranked(doc: &Value, rank: u64) -> Vec<(Value, Value)> {
    doc["ranking"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["rank"] == rank)
        .map(|row| {
            let value = doc["values"]
                .as_array()
                .unwrap()
                .iter()
                .find(|entry| entry["state"] == row["state"])
                .expect("a value at each ranked state")["value"]
                .clone();
            (row["state"].clone(), value)
        })
        .collect()
}

/// Keep the rows of `doc` whose state `keep` admits.
fn keep_rows(doc: &mut Value, keep: &[&Value]) {
    for list in ["values", "ranking"] {
        doc[list]
            .as_array_mut()
            .unwrap()
            .retain(|row| keep.contains(&&row["state"]));
    }
}

fn inequality_fails(request: &Value) {
    let (code, output) = run(request);
    assert_eq!(code, 1, "{output}");
    assert_eq!(output["rule"], "inequality-fails", "{output}");
}

/// FSpec FR-411-AC-8, FR-1042 "Document": a level-0.4 `shifted` `lower` over
/// the explored prefix proves the claim with upper `3/5`, is checked over its
/// named states and their successors under the same `max_states`, and refuses
/// each false document there and over the whole graph.
#[trace("TC-3151", "FSpec-FR-411-AC-8", "FSpec-FR-412-AC-5")]
#[test]
fn a_partial_shifted_lower_over_the_named_states_refuses_a_false_document() {
    let claim = partial_proof(&quantile("0.4", "<= 3"), 10, (3, 5));
    let emitted = document(&claim);
    let [(initial, value)]: [(Value, Value); 1] =
        ranked(&emitted, 4).try_into().expect("one initial key");
    let narrow = within(&claim, 10);
    accepted(&narrow, value.clone());
    accepted(&claim, value.clone());

    let satisfied: Vec<Value> = ranked(&emitted, 0)
        .into_iter()
        .filter(|(_, value)| *value == rational(3, 5))
        .map(|(state, _)| state)
        .collect();
    assert!(!satisfied.is_empty(), "the prefix reaches delivery");
    for request in [&narrow, &claim] {
        // A value above the one-step value, and a payoff above the terminal's.
        let raised = mutate(request, |doc| {
            set(doc, "values", &initial, "value", rational(1, 2));
        });
        rejected(&raised, "inequality-fails", at_key(&initial));
        let overpaid = mutate(request, |doc| {
            set(doc, "values", &satisfied[0], "value", rational(1, 1));
        });
        rejected(&overpaid, "inequality-fails", at_key(&satisfied[0]));
        let misranked = mutate(request, |doc| {
            set(doc, "ranking", &initial, "rank", json!(3));
        });
        rejected(&misranked, "rank-broken", at_key(&initial));
        let unranked = mutate(request, |doc| drop_row(doc, "ranking", &initial));
        rejected(&unranked, "malformed", at_member("ranking"));
        // A successor left out counts as `-2/5`, below the value it held.
        for state in &satisfied {
            inequality_fails(&mutate(request, |doc| {
                drop_row(doc, "values", state);
                drop_row(doc, "ranking", state);
            }));
        }
    }

    // A state named without the states that reach it is outside the part read,
    // and neither is a part read with more rows than `max_states` or for a
    // document that is not the claim side.
    let (deep, _) = ranked(&emitted, 2)
        .first()
        .cloned()
        .expect("an undecided state two steps in");
    let detached = mutate(&narrow, |doc| {
        keep_rows(doc, &[&initial, &deep]);
        set(doc, "values", &initial, "value", rational(-2, 5));
    });
    stopped_at_max_states(&detached, 10);
    let rows = emitted["values"].as_array().unwrap().len();
    let rows = u64::try_from(rows).unwrap();
    stopped_at_max_states(&within(&claim, rows - 1), rows - 1);
    let mut counterpart = narrow.clone();
    counterpart["claim_side"] = json!(false);
    stopped_at_max_states(&counterpart, 10);
}

/// FR-1043-AC-2's claim at `level` over its `fast` (`1/2`) and `slow` (`1/4`)
/// model.
fn retry(level: &str) -> String {
    let source = model_source()
        .replace("{ true: 1, false: 9 }", "{ true: 1, false: 1 }")
        .replace("{ true: 1, false: 4 }", "{ true: 3, false: 1 }");
    format!(
        "{source}\nprobabilistic Likely using inf on M over (m: M::Msg) under every scheduler on origin {{ quantile {level} of (steps from holds(true) until holds(m.delivered) within 4) <= 2 }}\n"
    )
}

/// FR-1042-AC-10, FSpec FR-412-AC-16: at level 0.5 a `lower` naming only the
/// initial key at 0 fails there, over the named states as over the whole
/// graph, since each unnamed successor counts as `-1/2`. At `-1/2` it meets
/// its conditions and misses the threshold.
#[trace("TC-3151", "FR-1042-AC-10", "FSpec-FR-412-AC-16")]
#[test]
fn a_partial_shifted_lower_naming_the_initial_key_counts_the_rest_at_minus_the_level() {
    let (exact, settlement) =
        request_with(&retry("0.4"), SNAPSHOT, ExactProbLimits::default(), DOMAIN);
    let exact = exact.unwrap_or_else(|| panic!("an exact proof: {settlement:?}"));
    let whole = bound(&exact, "lower", &[], None, true);
    let (initial, value) = ranked(&document(&whole), 4).remove(0);
    assert_eq!(value, rational(3, 80), "FR-1043-AC-2's initial value");
    let mut request = edit(&whole, |doc| {
        keep_rows(doc, &[&initial]);
        set(doc, "values", &initial, "value", rational(0, 1));
        doc["objective"]["base"]["measure"]["level"] = rational(1, 2);
    });
    replace_source_and_rebind(&mut request, &retry("0.5"));
    let floor = edit(&request, |doc| {
        set(doc, "values", &initial, "value", rational(-1, 2));
    });
    for states in [1, 100] {
        rejected(
            &within(&request, states),
            "inequality-fails",
            at_key(&initial),
        );
        rejected(
            &within(&floor, states),
            "threshold-missed",
            at_key(&initial),
        );
    }
}

/// FSpec FR-411-AC-8: a measure probability `>= theta` is proved on a partial
/// product with upper 1, each unnamed successor counting as 0.
#[trace("TC-3151", "FSpec-FR-411-AC-8", "FSpec-FR-412-AC-5")]
#[test]
fn a_measure_probability_is_proved_on_a_partial_product() {
    let claim = partial_proof(&measured(">=", "0.3"), 4, (1, 1));
    let emitted = document(&claim);
    let (initial, value) = ranked(&emitted, 4).remove(0);
    let narrow = within(&claim, 4);
    accepted(&narrow, value.clone());
    accepted(&claim, value);
    for request in [&narrow, &claim] {
        let raised = mutate(request, |doc| {
            set(doc, "values", &initial, "value", rational(1, 2));
        });
        rejected(&raised, "inequality-fails", at_key(&initial));
        let alone = mutate(request, |doc| keep_rows(doc, &[&initial]));
        rejected(&alone, "inequality-fails", at_key(&initial));
    }
}

/// FSpec FR-411-AC-8: a prefix that shows no claim-side bound, and a claim a
/// `lower` cannot prove, stop at `max_states` with no document.
#[trace("TC-3151", "FSpec-FR-411-AC-8")]
#[test]
fn a_measure_prefix_that_settles_nothing_stops_at_max_states() {
    for (source, max_states) in [
        (quantile("0.4", "<= 3"), 2),
        (measured("<=", "0.9"), 4),
        (quantile("0.9", ">= 2"), 4),
    ] {
        let (requests, settlement) = explored(&source, max_states);
        assert!(requests.is_empty(), "no document: {settlement:?}");
        let Settlement::Terminal(record) = &settlement else {
            panic!("a typed stop: {settlement:?}");
        };
        assert!(
            matches!(
                record.value(),
                TerminalValue::Incomplete(IncompleteCause::LimitReached { value, .. })
                    if *value == u64::try_from(max_states).unwrap()
            ),
            "{source}: {:?}",
            record.value()
        );
    }
}

/// The checker request offering `kind` bounds of the analyzer's exact proof of
/// `source` as the claim side.
fn measure_bound(source: &str, kind: &str) -> Value {
    bound(&proof(source), kind, &[], None, true)
}

/// FR-1042-AC-27: over a measure base a claim-side document of the wrong kind
/// is rejected, over the whole graph and naming the initial key alone, whose
/// bound meets the claim's number from the wrong side.
#[trace("TC-3151", "FR-1042-AC-27")]
#[test]
fn a_claim_side_measure_bound_of_the_wrong_kind_is_rejected() {
    for (source, wrong, right) in [
        (quantile("0.4", "<= 3"), "upper", "lower"),
        (measured(">=", "0.3"), "upper", "lower"),
        (measured("<=", "0.9"), "lower", "upper"),
        (quantile("0.9", ">= 2"), "lower", "upper"),
    ] {
        let (code, output) = run(&measure_bound(&source, right));
        assert_eq!(code, 0, "{source}: {output}");
        rejected(
            &measure_bound(&source, wrong),
            "malformed",
            at_member("kind"),
        );
    }

    // `Pr <= 0.1` is false: a `lower` of 0 at the initial key alone meets
    // every `lower` condition and says nothing about the claim.
    let whole = measure_bound(&measured("<=", "0.9"), "lower");
    let (initial, _) = ranked(&document(&whole), 4).remove(0);
    let mut least = edit(&whole, |doc| {
        keep_rows(doc, &[&initial]);
        set(doc, "values", &initial, "value", rational(0, 1));
    });
    replace_source_and_rebind(&mut least, &measured("<=", "0.1"));
    // So is `Pr(M < 2) < 0.2`, whose `lower` of `-1/5` says as little.
    let whole = measure_bound(&quantile("0.9", ">= 2"), "lower");
    let (strict, _) = ranked(&document(&whole), 4).remove(0);
    let mut below = edit(&whole, |doc| {
        keep_rows(doc, &[&strict]);
        set(doc, "values", &strict, "value", rational(-1, 5));
        doc["objective"]["base"]["measure"]["level"] = rational(1, 5);
    });
    replace_source_and_rebind(&mut below, &quantile("0.2", ">= 2"));
    for request in [least, below] {
        for states in [1, 100] {
            rejected(&within(&request, states), "malformed", at_member("kind"));
        }
    }
}

/// Only a `shifted` payoff and a comparison event, with no fairness set, have
/// a part a `lower` shows: a `fraction` `lower`, and a fair claim's, that
/// names the initial key alone stays stopped at `max_states`.
#[trace("TC-3151")]
#[test]
fn a_fraction_or_fair_lower_over_a_stopped_graph_stays_stopped() {
    let fraction = likely(
        "ev",
        "every scheduler",
        "mean of (fraction holds(m.delivered) over [0,2]) >= rational(1,10)",
    );
    let fair = likely(
        "ev",
        "every scheduler fair weak Msg::send_a",
        "probability >= 0.3 [steps from holds(true) until holds(m.delivered) within 4 <= 3]",
    );
    for source in [fraction, fair] {
        let whole = measure_bound(&source, "lower");
        let initial = document(&whole)["ranking"]
            .as_array()
            .unwrap()
            .iter()
            .max_by_key(|row| row["rank"].as_u64())
            .expect("a ranked initial key")["state"]
            .clone();
        let alone = edit(&whole, |doc| {
            keep_rows(doc, &[&initial]);
            set(doc, "values", &initial, "value", rational(0, 1));
        });
        // Over the whole graph the document is read and misses the threshold.
        rejected(&within(&alone, 100), "threshold-missed", at_key(&initial));
        stopped_at_max_states(&within(&alone, 1), 1);
    }
}

/// FR-1042 "Pairing" over FR-1043-AC-3's `>= c` quantile with the maximum
/// shifted value exactly 0: the claim-side `upper` is refused where its tight
/// actions reach an activated state, as the exact document is.
#[trace("TC-3151", "FR-1042-AC-28")]
#[test]
fn a_strict_quantile_upper_of_zero_with_tight_activation_is_rejected() {
    let shift = Rational::new(Integer::from(17_i64), Integer::from(30_i64)).unwrap();
    let mut request = measure_bound(&quantile("0.9", ">= 2"), "upper");
    request = edit(&request, |doc| {
        for row in doc["values"].as_array_mut().unwrap() {
            let part = |name: &str| row["value"][name].as_str().unwrap().parse::<i64>().unwrap();
            let value = Rational::new(
                Integer::from(part("numerator")),
                Integer::from(part("denominator")),
            )
            .unwrap()
            .add(&shift);
            row["value"] = json!({"numerator": value.numerator().to_string(), "denominator": value.denominator().to_string()});
        }
        doc["objective"]["base"]["measure"]["level"] = rational(1, 3);
    });
    replace_source_and_rebind(&mut request, &quantile("rational(1,3)", ">= 2"));
    let (code, output) = run(&request);
    assert_eq!(code, 1, "{output}");
    assert_eq!(output["rule"], "inequality-fails", "{output}");
}

/// FR-1043 "Forms and documents", FR-1043-AC-10: where activation is
/// unreachable the checker refuses an offered numeric bound document, in
/// either role, at the initial key.
#[trace("TC-3151", "FR-1043-AC-10")]
#[test]
fn a_numeric_quantile_bound_without_activation_is_rejected() {
    let never = |text: String| text.replace("from holds(true)", "from holds(false)");
    for scheduler in ["every scheduler", "Even"] {
        for (comparison, claim_kind) in [("<= 2", "lower"), (">= 2", "upper")] {
            let source =
                |text: String| text.replace("under every scheduler", &format!("under {scheduler}"));
            let reachable = source(quantile("0.4", comparison));
            let unreachable = never(reachable.clone());
            // The never-activating product's keys and ranks, from the event
            // proof over the same measure.
            let event = proof(&never(source(measured("<=", "0.5"))));
            let keys = document(&event);
            let shifted = if scheduler == "Even" {
                // A workload's exact proof is a ratio; its bound takes the
                // `shifted` base the every-scheduler proof writes.
                let mut base = document(&proof(&quantile("0.4", comparison)))["objective"].clone();
                base["optimization"] = json!("workload");
                base
            } else {
                document(&proof(&reachable))["objective"].clone()
            };
            for (kind, claim_side) in [
                (claim_kind, true),
                (
                    if claim_kind == "lower" {
                        "upper"
                    } else {
                        "lower"
                    },
                    false,
                ),
            ] {
                let mut request = bound(&event, kind, &[], None, claim_side);
                request = edit(&request, |doc| {
                    doc["objective"]["base"] = shifted.clone();
                    doc["values"] = keys["values"].clone();
                    for row in doc["values"].as_array_mut().unwrap() {
                        row["value"] = rational(0, 1);
                    }
                    if kind == "lower" {
                        doc["ranking"] = keys["ranking"].clone();
                    } else {
                        doc.as_object_mut().unwrap().remove("ranking");
                    }
                });
                replace_source_and_rebind(&mut request, &unreachable);
                let initial = ranked(&document(&event), 4).remove(0).0;
                let (code, output) = run(&request);
                assert_eq!(code, 1, "{scheduler} {comparison} {kind}: {output}");
                assert_eq!(output["rule"], "inequality-fails", "{output}");
                assert_eq!(output["at"], at_key(&initial), "{output}");
            }
        }
    }
}

/// The analyzer's settlement of `source` from the `link` snapshot beside a
/// second initial one, with `m` delivered and `attempts` as given.
fn from_two_initials(source: &str, second: (u64, bool), exact: ExactProbLimits) -> Settlement {
    use fsl_eval::simulation::model::{
        ExpansionLimits, ModelSimulationRequest, PopulationUniverse,
    };
    use fsl_semantics::model::observation::{DocumentRef, ObservationLimits};
    let (checked, packages) = super::bounded::compile_source_with(source, domain().as_bytes());
    let package = checked.package();
    let claim = package
        .graph()
        .probabilistic_model()
        .claim("Likely")
        .unwrap();
    let limits = fsl_replay::spine::SpineLimits::default();
    let mut other: Value = serde_json::from_slice(SNAPSHOT).unwrap();
    other["identity"]["identity"] = json!("s1");
    let fields = &mut other["populations"][0]["objects"][0]["fields"];
    fields["attempts"] = json!({"integer": second.0.to_string()});
    fields["delivered"] = json!({"boolean": second.1});
    let reference = |identity: &str| DocumentRef {
        authority: "test".to_owned(),
        identity: identity.to_owned(),
    };
    let snapshots = [
        (reference("s0"), SNAPSHOT.to_vec()),
        (reference("s1"), serde_json::to_vec(&other).unwrap()),
    ]
    .into_iter()
    .collect();
    let simulation = ModelSimulationRequest {
        input: fsl_eval::simulation::model::ModelCheckInput::source(package),
        packages: &packages,
        normalization: limits.model,
        snapshots: &snapshots,
        initial: vec![reference("s0"), reference("s1")],
        universes: vec![PopulationUniverse {
            population: filament_checked::key::DeclarationKey {
                package: other["model"]["identity"].as_str().unwrap().to_owned(),
                node: other["populations"][0]["population"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
            },
            keys: vec!["m".to_owned()],
        }],
        observation: ObservationLimits::default(),
        expansion: ExpansionLimits {
            max_candidates: 100_000,
        },
        meter: fsl_replay::AccountingLimits::default().0,
        emit: limits.package.emit(),
    };
    let item = fsl_analyze::analyze::exact_probabilistic_item(
        fsl_foundation::RequestIndex::new(0),
        fsl_analyze::exact_probabilistic::ProductRequest {
            subject: fsl_eval::model_subject::ModelSubject {
                simulation,
                proof_bounds: Vec::new(),
            },
            claim,
            limits: exact,
        },
    );
    let engines = fsl_analyze::analyze::Engines::fsl().unwrap();
    fsl_analyze::analyze::analyze_items(&engines, vec![item], &filament_exact::Cancel::new())
        .remove(0)
        .settlement
}

/// FR-1042 "Pairing": a bound checks its threshold at one initial key, so over
/// two initial states it is refused as the exact document is, and EN-5 proves
/// nothing from the interval route. Each claim below is false at one of the
/// two initial states.
#[trace("TC-3151")]
#[test]
fn measure_bounds_over_two_initial_states_prove_nothing() {
    let interval = ExactProbLimits {
        max_rational_bits: 1,
        precision_bits: 4,
        max_precision_bits: 16,
        ..ExactProbLimits::default()
    };
    // `(2, false)` never delivers; `(0, true)` is already delivered.
    let mut proved = Vec::new();
    for (comparison, second) in [
        ("<= 3", (2, false)),
        ("<= 0", (0, true)),
        (">= 1", (0, true)),
        (">= 2", (2, false)),
    ] {
        for limits in [interval, ExactProbLimits::default()] {
            let settlement = from_two_initials(&quantile("0.4", comparison), second, limits);
            // A `<=` claim reaches the checker on both routes. (The interval
            // route offers a `>=` quantile the wrong kind first.)
            if comparison.starts_with("<=") {
                assert!(
                    matches!(&settlement, Settlement::Disposed(disposed)
                        if disposed.kind == fsl_analyze::analyze::DispositionKind::Unsupported),
                    "{comparison} {second:?}: {settlement:?}"
                );
            }
            if matches!(&settlement, Settlement::Terminal(record)
                if matches!(record.value(), TerminalValue::Proved { .. }))
            {
                proved.push((comparison, second));
            }
        }
    }
    assert!(
        proved.is_empty(),
        "proved at one of two initial states: {proved:?}"
    );
}

/// FR-1042 "Pairing", FR-1043-AC-3's maximum of 0: where only a path that
/// never activates attains 0, the claim-side `upper` is accepted.
#[trace("TC-3151", "FR-1042-AC-28")]
#[test]
fn a_strict_quantile_upper_of_zero_without_tight_activation_is_accepted() {
    let source = w(">=", "rational(1,4)")
        .replace(
            "profile inf = \"filament.temporal.infinite-trace/v1\";",
            "profile inf = \"filament.temporal.infinite-trace/v1\";\nprofile ev = \"filament.temporal.event-position.false-extension/v1\";",
        )
        .replace("using inf on M", "using ev on M")
        .replace(
            "probability >= rational(1,4) [ eventually holds(m.delivered) ]",
            "quantile 0.4 of (steps from holds(m.delivered) until holds(false) within 2) >= 1",
        );
    let exact = proof(&source);
    assert_eq!(run(&exact).0, 0, "{}", run(&exact).1);
    accepted(&bound(&exact, "upper", &[], None, true), rational(0, 1));
}
