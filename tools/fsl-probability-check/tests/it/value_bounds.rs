// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-3212 steps 1 and 2 (FR-1042): `lower` and `upper` documents over an
//! unbounded eventual base, checked in a separate `fsl-probability-check`
//! process as the claim side or as the counterpart, with policy hints and the
//! checker's own selection of an existential action.
//!
//! Each document wraps the base objective of an exact proof the analyzer
//! emits for the same model, with the values the criterion names.

use super::bounded::{
    document, mutate, replace_source_and_rebind, request_with, requests_offering,
    requests_with_mode, run,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use filament_exact::{Integer, Rational};
use fsl_analyze::analyze::Settlement;
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
#[trace("TC-3151")]
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
#[trace("TC-3151")]
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
