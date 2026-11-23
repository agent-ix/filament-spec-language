// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-875, FR-741-AC-5, FR-201 "Conditional certificates": the `conditional`
//! certificate of an `on each` claim, produced by the analyzer over the
//! acyclic `link` model, is accepted by the separate `fsl-probability-check`
//! process, and each altered row, part, mode or threshold is rejected with its
//! FR-201 rule and locus.

use super::bounded::{document, key, mutate, replace_source_and_rebind, request_with, run};
use fsl_analyze::exact_probabilistic::ExactProbLimits;
use ix_trace_rs::trace;
use serde_json::{json, Value};

const DOMAIN: &[u8] = include_bytes!("../fixtures/link.semantic-ir.json");
const SNAPSHOT: &[u8] = include_bytes!("../fixtures/link.snapshot.json");
const SOURCE: &str = include_str!("../fixtures/link.native");

fn claim(threshold: &str) -> String {
    format!(
        "{SOURCE}\nprobabilistic Likely using inf on M under Even \
         on each (k: M::Msg) when (not k.delivered) \
         {{ probability >= rational({threshold}) [ eventually holds(k.delivered) ] }}\n"
    )
}

/// The `Link` start snapshot holding one message per `(key, delivered)`.
fn snapshot(messages: &[(&str, bool)]) -> Vec<u8> {
    let mut snapshot: Value = serde_json::from_slice(SNAPSHOT).expect("the Link snapshot");
    snapshot["populations"][0]["objects"] = Value::Array(
        messages
            .iter()
            .map(|(key, delivered)| {
                json!({"fields": {"attempts": {"integer": if *delivered { "1" } else { "0" }},
                                  "delivered": {"boolean": delivered}},
                       "key": key, "type": "ix://test/link/Msg"})
            })
            .collect(),
    );
    serde_json::to_vec(&snapshot).expect("snapshot bytes")
}

fn certified(messages: &[(&str, bool)]) -> Value {
    let (request, settlement) = request_with(
        &claim("9, 10"),
        &snapshot(messages),
        ExactProbLimits::default(),
        DOMAIN,
    );
    request.unwrap_or_else(|| panic!("an on-each proof retains its certificate: {settlement:?}"))
}

fn instance(row: &Value) -> Value {
    key(&row["state"])[2]["identity"].clone()
}

fn rows<'a>(doc: &'a Value, part: &str) -> &'a Vec<Value> {
    doc["conditional"][part]["values"].as_array().expect("rows")
}

fn value(row: &Value) -> Value {
    row["value"].clone()
}

fn ratio(num: i64, den: i64) -> Value {
    json!({"numerator": num.to_string(), "denominator": den.to_string()})
}

/// Whether the row's model state holds exactly the objects of `start`, each at
/// its start values (a delivered message has made one attempt).
fn at_start(row: &Value, start: &[(&str, bool)]) -> bool {
    let state = key(&row["state"]);
    let objects = state[4][1]["semantic"]["entries"][0]["value"]["entries"]
        .as_array()
        .expect("objects");
    objects.len() == start.len()
        && start.iter().all(|(name, delivered)| {
            objects.iter().any(|object| {
                let fields = &object["value"]["fields"];
                object["key"]["identity"] == *name
                    && fields[0]["value"]["value"] == *delivered
                    && fields[1]["value"]["value"] == if *delivered { "1" } else { "0" }
            })
        })
}

/// The text of instance `name`'s initial complete key: its numerator row at
/// the start state of `start`.
fn initial(doc: &Value, name: &str, start: &[(&str, bool)]) -> String {
    rows(doc, "numerator")
        .iter()
        .find(|row| instance(row) == name && at_start(row, start))
        .and_then(|row| row["state"].as_str())
        .expect("the instance's initial key")
        .to_string()
}

fn position(doc: &Value, part: &str, state: &str) -> usize {
    rows(doc, part)
        .iter()
        .position(|row| row["state"] == state)
        .expect("the row")
}

fn assert_rejected(request: &Value, rule: &str) -> Value {
    let (code, output) = run(request);
    assert_eq!(code, 1, "{output}");
    assert_eq!(output["rule"], rule, "{output}");
    output
}

/// FR-741-AC-5, FR-201-AC-76: the `Ratio` document has full `exact` parts, a
/// row and a rank per product key; `391/400` over `1` at `m`'s initial key,
/// `0` and `0` at `n`'s. It is accepted at `9/10` and rejected
/// `ThresholdMissed` at `99/100` at `m`'s initial key with no `part`.
#[trace("TC-3208", "FR-201-AC-76")]
#[trace("TC-3208", "FR-741-AC-5")]
#[test]
fn an_on_each_ratio_document_is_accepted_and_misses_a_stricter_threshold_at_the_first_key() {
    let request = certified(&[("m", false), ("n", true)]);
    let (code, output) = run(&request);
    assert_eq!(code, 0, "{output}");
    let doc = document(&request);
    assert_eq!(doc["kind"], "conditional", "{doc}");
    assert_eq!(doc["conditional"]["mode"], "ratio", "{doc}");
    for part in ["numerator", "denominator"] {
        let part_doc = &doc["conditional"][part];
        assert_eq!(part_doc["kind"], "exact", "{doc}");
        assert_eq!(part_doc["identity"], doc["identity"], "{doc}");
        assert_eq!(part_doc["objective"], doc["objective"], "{doc}");
        let states: Vec<_> = rows(&doc, part).iter().map(|row| &row["state"]).collect();
        let ranked: Vec<_> = part_doc["ranking"]
            .as_array()
            .expect("ranking")
            .iter()
            .map(|row| &row["state"])
            .collect();
        assert_eq!(states, ranked, "one rank per row: {doc}");
        for row in rows(&doc, part) {
            // FR-196: the complete eight-member key, instance and phase set.
            let members = key(&row["state"]);
            assert_eq!(members.as_array().map(Vec::len), Some(8), "{members}");
            assert!(members[2].is_object(), "{members}");
            assert!(
                matches!(members[3].as_str(), Some("waiting" | "active")),
                "{members}"
            );
        }
    }
    let start = [("m", false), ("n", true)];
    let initial = initial(&doc, "m", &start);
    let value_at = |part: &str, state: &str| value(&rows(&doc, part)[position(&doc, part, state)]);
    // FR-201-AC-76: `m` activates at position 0 with `391/400` over `1`.
    assert_eq!(value_at("numerator", &initial), ratio(391, 400), "{doc}");
    assert_eq!(value_at("denominator", &initial), ratio(1, 1), "{doc}");
    // `n` never activates: `0` and `0` at its initial key.
    let at_n = self::initial(&doc, "n", &start);
    assert_eq!(value_at("numerator", &at_n), ratio(0, 1), "{doc}");
    assert_eq!(value_at("denominator", &at_n), ratio(0, 1), "{doc}");
    for row in rows(&doc, "denominator") {
        if instance(row) == "n" {
            assert_eq!(value(row), ratio(0, 1), "n never activates: {doc}");
        }
    }
    let mut stricter = request.clone();
    replace_source_and_rebind(&mut stricter, &claim("99, 100"));
    let output = assert_rejected(&stricter, "threshold-missed");
    assert!(output["at"].get("part").is_none(), "{output}");
    assert!(output["at"].to_string().contains(&initial), "{output}");
}

/// The conditional of `conditional` with only the rows of instance `name`.
fn only(conditional: &Value, name: &str) -> Value {
    let mut kept = conditional.clone();
    for part in ["numerator", "denominator"] {
        for member in ["values", "ranking"] {
            let rows = kept[part][member].as_array_mut().expect("rows");
            // Only the rows where the other objects are still at their start.
            rows.retain(|row| {
                instance(row) == name
                    && key(&row["state"])[4][1]["semantic"]["entries"][0]["value"]["entries"]
                        .as_array()
                        .expect("objects")
                        .iter()
                        .filter(|object| object["key"]["identity"] != name)
                        .all(|object| {
                            let fields = &object["value"]["fields"];
                            fields[0]["value"]["value"] == false
                                && fields[1]["value"]["value"] == "0"
                        })
            });
            // The model state names every object: drop the other one so the
            // key is the one of a snapshot holding `name` alone.
            for row in rows.iter_mut() {
                let mut members = key(&row["state"]);
                members[4][1]["semantic"]["entries"][0]["value"]["entries"]
                    .as_array_mut()
                    .expect("objects")
                    .retain(|object| object["key"]["identity"] == name);
                row["state"] = json!(base64::Engine::encode(
                    &base64::engine::general_purpose::URL_SAFE_NO_PAD,
                    serde_json::to_vec(&members).unwrap()
                ));
            }
        }
    }
    kept
}

/// FR-201-AC-77: over a snapshot holding only `n`, a `Ratio` document is
/// rejected `InequalityFails` at `n`'s initial key with no `part`, and the
/// `Vacuous` document is accepted; over `m`'s snapshot the `Vacuous` document
/// is rejected at `m`'s initial key.
#[trace("TC-3208", "FR-201-AC-77")]
#[test]
fn only_a_vacuous_document_certifies_a_reading_no_initial_key_activates() {
    let vacuous = certified(&[("n", true)]);
    let doc = document(&vacuous);
    assert_eq!(doc["conditional"], json!({"mode": "vacuous"}), "{doc}");
    assert_eq!(doc["kind"], "conditional", "{doc}");
    assert_eq!(doc["values"], json!([]), "{doc}");
    assert!(doc["objective"].is_object(), "{doc}");
    let (code, output) = run(&vacuous);
    assert_eq!(code, 0, "{output}");

    let active = certified(&[("m", false), ("n", true)]);
    let rows_of_n = only(&document(&active)["conditional"], "n");
    let numeric = mutate(&vacuous, |doc| {
        doc["conditional"] = rows_of_n.clone();
        doc.as_object_mut().unwrap().remove("values");
    });
    let output = assert_rejected(&numeric, "inequality-fails");
    assert!(output["at"].get("part").is_none(), "{output}");

    let only_m = certified(&[("m", false)]);
    let initial = initial(&document(&only_m), "m", &[("m", false)]);
    let offered = mutate(&only_m, |doc| {
        doc["conditional"] = json!({"mode": "vacuous"});
        doc["values"] = json!([]);
    });
    let output = assert_rejected(&offered, "inequality-fails");
    assert!(output["at"].get("part").is_none(), "{output}");
    assert!(output["at"].to_string().contains(&initial), "{output}");
}

/// FR-201-AC-78: an altered numerator or denominator at `m`'s initial key is
/// rejected `InequalityFails` with its `part`, and a rank of 0 there is
/// rejected `RankBroken` with `part` `numerator`.
#[trace("TC-3209", "FR-201-AC-78")]
#[test]
fn an_altered_part_row_or_rank_is_rejected_at_its_key_and_part() {
    let request = certified(&[("m", false)]);
    let doc = document(&request);
    let initial = initial(&doc, "m", &[("m", false)]);
    for part in ["numerator", "denominator"] {
        let at = position(&doc, part, &initial);
        let altered = mutate(&request, |doc| {
            doc["conditional"][part]["values"][at]["value"] = ratio(1, 2);
        });
        let output = assert_rejected(&altered, "inequality-fails");
        assert_eq!(output["at"]["part"], part, "{output}");
        assert!(output["at"].to_string().contains(&initial), "{output}");
    }
    let rank_at = doc["conditional"]["numerator"]["ranking"]
        .as_array()
        .expect("ranking")
        .iter()
        .position(|row| row["state"] == initial.as_str())
        .expect("rank row");
    let unranked = mutate(&request, |doc| {
        doc["conditional"]["numerator"]["ranking"][rank_at]["rank"] = json!(0);
    });
    let output = assert_rejected(&unranked, "rank-broken");
    assert_eq!(output["at"]["part"], "numerator", "{output}");
    assert!(output["at"].to_string().contains(&initial), "{output}");
}

/// FR-201-AC-79: a part that omits, repeats, misorders or adds a row is
/// rejected `Malformed` at `CertificateMember` naming the part.
#[trace("TC-3209", "FR-201-AC-79")]
#[test]
fn a_part_with_a_missing_repeated_misordered_or_extra_row_is_malformed_at_its_member() {
    let request = certified(&[("m", false)]);
    let doc = document(&request);
    let extra = |member: &str| {
        let mut row = doc["conditional"]["numerator"][member][0].clone();
        let mut members = key(&row["state"]);
        members[3] = json!(if members[3] == "active" {
            "waiting"
        } else {
            "active"
        });
        row["state"] = json!(base64::Engine::encode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            serde_json::to_vec(&members).unwrap()
        ));
        row
    };
    for part in ["numerator", "denominator"] {
        let faults: [(&str, Box<dyn Fn(&str, &mut Vec<Value>)>); 4] = [
            (
                "missing",
                Box::new(|_, rows| {
                    rows.remove(0);
                }),
            ),
            (
                "repeated",
                Box::new(|_, rows| {
                    let row = rows[0].clone();
                    rows.insert(1, row);
                }),
            ),
            ("misordered", Box::new(|_, rows| rows.swap(0, 1))),
            ("extra", Box::new(|member, rows| rows.push(extra(member)))),
        ];
        for (name, fault) in faults {
            for member in ["values", "ranking"] {
                let altered = mutate(&request, |doc| {
                    let rows = doc["conditional"][part][member].as_array_mut().unwrap();
                    fault(member, rows);
                });
                let output = assert_rejected(&altered, "malformed");
                assert_eq!(
                    output["at"],
                    json!({"member": part}),
                    "{name} {member}: {output}"
                );
            }
        }
    }
}

/// FR-201-AC-80: a `Ratio` document offered under every scheduler is rejected
/// `Malformed` at `mode`.
#[trace("TC-3209", "FR-201-AC-80")]
#[test]
fn a_ratio_document_under_a_scheduler_choice_is_malformed_at_mode() {
    let request = certified(&[("m", false)]);
    for optimization in ["minimum", "maximum", "workload-minimum", "workload-maximum"] {
        let altered = mutate(&request, |doc| {
            doc["objective"]["optimization"] = json!(optimization);
        });
        let output = assert_rejected(&altered, "malformed");
        assert_eq!(output["at"], json!({"member": "mode"}), "{output}");
    }
}

/// The on-each claim `Likely` under `scheduler`, over `source`.
fn claim_under(source: &str, scheduler: &str) -> String {
    format!(
        "{source}\nprobabilistic Likely using inf on M under {scheduler} \
         on each (k: M::Msg) when (not k.delivered) \
         {{ probability >= rational(9, 10) [ eventually holds(k.delivered) ] }}\n"
    )
}

/// FR-201-AC-80: a `Ratio` document offered for FR-875's claim under every
/// scheduler is rejected `Malformed` at `mode`.
#[trace("TC-3209", "FR-201-AC-80")]
#[test]
fn a_ratio_document_offered_for_an_every_scheduler_claim_is_malformed_at_mode() {
    let request = certified(&[("m", false)]);
    let mut every = request.clone();
    replace_source_and_rebind(&mut every, &claim_under(SOURCE, "every scheduler"));
    let output = assert_rejected(&every, "malformed");
    assert_eq!(output["at"], json!({"member": "mode"}), "{output}");
}

/// FR-201-AC-80: a claim of the on-each form over a cyclic continuation graph,
/// with authored fairness or over a timed subject is not admitted: no
/// `on-each-reachability/v1` document is written for it, and the document of
/// an in-profile claim offered for the cyclic or fair claim is not accepted.
#[trace("TC-3209", "FR-201-AC-80")]
#[test]
fn an_on_each_claim_outside_the_closed_profile_has_no_document_written_or_accepted() {
    // A cyclic continuation graph: `send_a` leaves `attempts` alone, so an
    // undelivered message loops on itself.
    let cyclic = SOURCE.replace(
        "post Sent_send_a using v on M::Msg::send_a { self.attempts - pre(self.attempts) = 1 and",
        "post Sent_send_a using v on M::Msg::send_a { self.attempts = pre(self.attempts) and",
    );
    assert_ne!(cyclic, SOURCE, "the cyclic variant edits send_a");
    let fair = claim_under(SOURCE, "every scheduler fair weak Msg::send_a");
    let cyclic_claim = claim_under(&cyclic, "Even");
    for (name, source) in [("cyclic", &cyclic_claim), ("fair", &fair)] {
        let (written, settlement) = request_with(
            source,
            &snapshot(&[("m", false)]),
            ExactProbLimits::default(),
            DOMAIN,
        );
        assert!(written.is_none(), "{name}: {settlement:?}");
        let mut offered = certified(&[("m", false)]);
        replace_source_and_rebind(&mut offered, source);
        let (code, output) = run(&offered);
        assert_ne!(code, 0, "{name}: {output}");
        assert_ne!(output["status"], "accepted", "{name}: {output}");
    }
    // A timed subject.
    let timed = super::timed_product::on_each_timed_source();
    let (written, settlement) = request_with(
        &timed,
        &super::timed_product::snapshot(),
        ExactProbLimits::default(),
        &super::timed_product::domain(),
    );
    assert!(written.is_none(), "timed: {settlement:?}");
}

/// FR-201 "Closed on-each reachability conditional profile": a `Ratio`
/// document has exactly `identity`, `objective`, `kind` and `conditional`, a
/// `Vacuous` one exactly `identity`, `objective`, `kind`, an empty `values`
/// and `conditional`. A document written exactly so is accepted and an extra
/// top-level member is refused.
#[trace("TC-3208", "FR-201-AC-76")]
#[trace("TC-3208", "FR-201-AC-77")]
#[test]
fn a_document_written_exactly_to_the_profile_is_accepted_and_an_extra_member_is_refused() {
    for (messages, members) in [
        (
            &[("m", false)][..],
            vec!["conditional", "identity", "kind", "objective"],
        ),
        (
            &[("n", true)][..],
            vec!["conditional", "identity", "kind", "objective", "values"],
        ),
    ] {
        let request = certified(messages);
        let produced = document(&request);
        let mut names: Vec<_> = produced.as_object().unwrap().keys().cloned().collect();
        names.sort();
        assert_eq!(names, members, "{produced}");
        // Written by hand from the profile's members.
        let mut by_hand = json!({
            "identity": produced["identity"].clone(),
            "objective": produced["objective"].clone(),
            "kind": "conditional",
            "conditional": produced["conditional"].clone(),
        });
        if messages.len() == 1 && messages[0].1 {
            by_hand["values"] = json!([]);
        }
        let handwritten = mutate(&request, |doc| *doc = by_hand.clone());
        let (code, output) = run(&handwritten);
        assert_eq!(code, 0, "{output}");
        for extra in ["values", "ranking"] {
            if produced.get(extra).is_some() {
                continue;
            }
            let extended = mutate(&request, |doc| doc[extra] = json!([]));
            let (code, output) = run(&extended);
            assert_ne!(code, 0, "{extra}: {output}");
            assert_ne!(output["status"], "accepted", "{extra}: {output}");
        }
    }
}

/// FR-201-AC-80's timed clause, accepted half: an on-each document offered
/// against a timed domain is refused, never accepted.
#[trace("TC-3209", "FR-201-AC-80")]
#[test]
fn an_on_each_document_offered_against_a_timed_domain_is_refused() {
    let mut offered = super::timed_product::timed_request();
    offered["certificate"] = certified(&[("m", false)])["certificate"].clone();
    replace_source_and_rebind(&mut offered, &super::timed_product::on_each_timed_source());
    let (code, output) = run(&offered);
    assert_ne!(code, 0, "{output}");
    assert_ne!(output["status"], "accepted", "{output}");
}
