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

/// The text of `m`'s initial complete key: the activated numerator row that
/// reads `391/400`.
fn m_initial(doc: &Value) -> String {
    rows(doc, "numerator")
        .iter()
        .find(|row| {
            instance(row) == "m" && value(row) == json!({"numerator": "391", "denominator": "400"})
        })
        .and_then(|row| row["state"].as_str())
        .expect("m's initial key")
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
    let initial = m_initial(&doc);
    let at = position(&doc, "denominator", &initial);
    assert_eq!(value(&rows(&doc, "denominator")[at]), ratio(1, 1), "{doc}");
    let n_rows: Vec<_> = rows(&doc, "numerator")
        .iter()
        .filter(|row| instance(row) == "n")
        .collect();
    assert!(!n_rows.is_empty(), "{doc}");
    for row in n_rows {
        assert_eq!(value(row), ratio(0, 1), "n never activates: {doc}");
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
    let numeric = mutate(&vacuous, |doc| doc["conditional"] = rows_of_n.clone());
    let output = assert_rejected(&numeric, "inequality-fails");
    assert!(output["at"].get("part").is_none(), "{output}");

    let only_m = certified(&[("m", false)]);
    let initial = m_initial(&document(&only_m));
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
    let initial = m_initial(&doc);
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
