// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2471: `fsl run <request>` at the command surface the driver calls
//! (`run_with_limits_and_overrides`): a completed call,
//! an indeterminate finite-window clause, refusals, an exhausted budget, the
//! `--clocks`, `--limit`, `--request-bytes` and `--dependent-bytes` inputs,
//! and odd request and source input.

use fsl_command::command::{RequestOverrides, RunLimits};
use ix_trace_rs::trace;
use serde_json::value::RawValue;
use serde_json::{json, Value};

use super::{run, Case, Outcome, INVALID_UTF8};

const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.value.complete/v1\";\n\
    function Remaining using v(total: Integer, held: Integer): Integer pure { total - held }\n\
    function IsPacked using v(n: Int[0, 10]): Boolean pure { n >= 6 }\n";

const STAMPED: &str = "filament.temporal.timestamped-event.finite-window/v1";

#[path = "../../../../examples/config-version/spine_unit.rs"]
#[allow(
    dead_code,
    reason = "the command regression uses the genuine domain package"
)]
mod config_domain;

fn request(file: &str, function: &str, arguments: &[(&str, Value)]) -> Value {
    let arguments: Vec<Value> = arguments
        .iter()
        .map(|(parameter, value)| json!({"parameter": parameter, "value": value}))
        .collect();
    json!({"format": "native-run/1", "request": {"models": [], "program": {"source": {
        "file": file, "authority": "run-contracts", "identity": "contracts:packing",
        "document": "Packing", "formal_revision": 1}},
        "call": {"function": function, "arguments": arguments}}})
}

fn packing(function: &str, arguments: &[(&str, Value)]) -> (Case, std::path::PathBuf) {
    let case = Case::new();
    case.write("packing.fsl", SOURCE.as_bytes());
    let path = case.json("run.json", &request("packing.fsl", function, arguments));
    (case, path)
}

fn plain(path: &std::path::Path) -> Outcome {
    run(path, RunLimits::default(), &RequestOverrides::default())
}

fn remaining() -> (Case, std::path::PathBuf) {
    packing(
        "Remaining",
        &[
            ("total", json!({"type":"integer", "value": "17"})),
            ("held", json!({"type":"integer", "value": "5"})),
        ],
    )
}

/// The command admits canonical values through the declared kernel type and
/// writes canonical results, preserving exact values and normalizing sets
/// and rational fractions.
#[test]
#[trace("FSpec-FR-550-AC-1", "FR-650-AC-18")]
fn canonical_run_values_preserve_exact_values_and_normalize_admitted_inputs() {
    let case = Case::new();
    case.write(
        "identity.fsl",
        br#"language "ix:native" edition "1-draft";
profile v = "filament.value.complete/v1";
function Number using v(x: Integer): Integer pure { x }
function Flag using v(x: Boolean): Boolean pure { x }
function Fraction using v(x: Rational[-10, 10; 1, 20]): Rational[-10, 10; 1, 20] pure { x }
function Words using v(x: Text[0, 40; unicode-scalars]): Text[0, 40; unicode-scalars] pure { x }
function List using v(x: Sequence<Integer>[0, 3]): Sequence<Integer>[0, 3] pure { x }
function Members using v(x: Set<Integer>[0, 3]): Set<Integer>[0, 3] pure { x }
function Constant using v(): Integer pure { 5 }
"#,
    );
    let integer = |value: &str| json!({"type":"integer", "value":value});
    let inputs = [
        (
            "Number",
            integer("12345678901234567890123"),
            integer("12345678901234567890123"),
        ),
        ("Number", integer("-5"), integer("-5")),
        ("Number", integer("0"), integer("0")),
        (
            "Flag",
            json!({"type":"boolean","value":true}),
            json!({"type":"boolean","value":true}),
        ),
        (
            "Fraction",
            json!({"type":"rational","numerator":"4","denominator":"6"}),
            json!({"type":"rational","numerator":"2","denominator":"3"}),
        ),
        (
            "Words",
            json!({"type":"text","value":"hello"}),
            json!({"type":"text","value":"hello"}),
        ),
        (
            "List",
            json!({"type":"sequence","elements":[]}),
            json!({"type":"sequence","elements":[]}),
        ),
        (
            "List",
            json!({"type":"sequence","elements":[integer("3"),integer("1"),integer("2")]}),
            json!({"type":"sequence","elements":[integer("3"),integer("1"),integer("2")]}),
        ),
        (
            "Members",
            json!({"type":"set","elements":[integer("3"),integer("1")]}),
            json!({"type":"set","elements":[integer("1"),integer("3")]}),
        ),
    ];
    for (function, input, expected) in inputs {
        let path = case.json(
            "run.json",
            &request("identity.fsl", function, &[("x", input)]),
        );
        let outcome = plain(&path);
        let document = outcome.expect("execute", "success", 0);
        assert_eq!(
            document["result"]["value"], expected,
            "{function}: {document}"
        );
    }
    let path = case.json("constant.json", &request("identity.fsl", "Constant", &[]));
    assert_eq!(
        plain(&path).expect("execute", "success", 0)["result"]["value"],
        integer("5")
    );
}

/// Legacy shapes and malformed canonical inputs refuse at the command boundary.
#[test]
#[trace("FSpec-FR-550-AC-1", "FR-650-AC-19")]
fn canonical_run_values_refuse_legacy_and_malformed_inputs() {
    for value in [
        json!({"integer":"7"}),
        json!(7),
        json!({"type":"integer","value":"07"}),
        json!({"type":"integer","value":"7","extra":true}),
    ] {
        let (_case, path) = packing("IsPacked", &[("n", value)]);
        let outcome = plain(&path);
        assert_eq!(outcome.exit, 20, "{}", outcome.document);
    }
    let (_case, path) = packing("IsPacked", &[("n", json!({"type":"integer","value":"11"}))]);
    assert_eq!(plain(&path).exit, 20);
}

/// A value-only call has no model work to charge, even when its result is
/// emitted through the borrowed canonical writer.
#[test]
#[trace("FR-650-AC-23")]
fn canonical_run_without_models_preserves_zero_model_work_limit() {
    let (_case, path) = remaining();
    let mut limits = fsl_replay::spine::SpineLimits::default();
    limits.model.work_units = 0;
    let outcome = run(
        &path,
        RunLimits::default(),
        &RequestOverrides {
            limits: Some(&limits),
            ..RequestOverrides::default()
        },
    );
    let document = outcome.expect("execute", "success", 0);
    assert_eq!(
        document["result"]["value"],
        json!({"type":"integer","value":"12"})
    );
}

/// Native selections retain their index and order through the owned runner.
#[test]
#[trace("FSpec-FR-550-AC-3", "FR-651-AC-1")]
fn canonical_run_values_items_keep_selection_order_and_individual_results() {
    let case = Case::new();
    case.write("packing.fsl", SOURCE.as_bytes());
    let integer = |value: &str| json!({"type":"integer", "value":value});
    let mut job = request("packing.fsl", "Remaining", &[]);
    let selections: Vec<Value> = ["3", "1", "2"]
        .into_iter()
        .map(|value| {
            let call = request(
                "packing.fsl",
                "Remaining",
                &[("total", integer(value)), ("held", integer("0"))],
            );
            json!({"call": call["request"]["call"]})
        })
        .collect();
    let body = job["request"]
        .as_object_mut()
        .expect("the authored request is an object");
    body.remove("call");
    body.insert("items".to_owned(), json!({"selections": selections}));
    let outcome = plain(&case.json("items.json", &job));
    assert_eq!(outcome.exit, 0, "{}", outcome.document);
    let document = &outcome.document;
    assert_eq!(document["category"], "success", "{document}");
    assert_eq!(
        document["selections"],
        json!([{"index":0,"expanded":1},{"index":1,"expanded":1},{"index":2,"expanded":1}]),
        "{document}"
    );
    let items = document["items"]
        .as_array()
        .expect("one result per selection");
    assert_eq!(items.len(), 3, "{document}");
    for (index, expected) in ["3", "1", "2"].into_iter().enumerate() {
        assert_eq!(items[index]["selection"], index, "{document}");
        assert_eq!(
            items[index]["result"]["outcome"]["value"],
            integer(expected),
            "{document}"
        );
    }
}

fn reference_call_world() -> (Case, Value, &'static str) {
    let case = Case::new();
    case.write("domain.json", config_domain::DOMAIN_PACKAGE.as_bytes());
    let source = format!("language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nmodel Config = {:?};\nfunction Keep using v(x: Config::ConfigVersion): Config::ConfigVersion pure {{ x }}\nfunction Find using v(p: Population<Config::ConfigVersion>[4], r: Config::ConfigVersion): Config::ConfigVersion pure {{ lookup<Config::ConfigVersion>(p, r) absent refused }}\n", config_domain::PACKAGE_IDENTITY);
    case.write("world.native", source.as_bytes());
    let population = "ix://example/config-version/config_history";
    let objects: Vec<Value> = [("root", "1"), ("child", "2")]
        .into_iter()
        .map(|(key, number)| {
            json!({
                "key":key,"type":"ix://example/config-version/ConfigVersion",
                "fields":{"versionNumber":{"integer":number},"parent":{"absent":{}}}
            })
        })
        .collect();
    case.json("current.json", &json!({
        "format":"filament.state.snapshot/v1", "identity":{"authority":"test","identity":"current"},
        "observation":"current","anchor":{"kind":"handler","name":"read"},
        "model":{"identity":config_domain::PACKAGE_IDENTITY},
        "populations":[{"population":population,"complete":true,"objects":objects}]
    }));
    let mut job = request("world.native", "Keep", &[]);
    job["request"]["models"] = json!([{"format":"semantic-ir/2.0.0", "source":{
        "file":"domain.json","authority":"test","identity":"domain",
        "document":"Domain","formal_revision":1
    }}]);
    (case, job, population)
}

/// Population materialization keeps document order internally, while native
/// population arguments retain their declared form and references can be reused.
#[test]
#[trace("FR-650-AC-15", "FR-651-AC-2")]
fn canonical_run_population_materialization_and_reference_roundtrip() {
    let (case, mut job, population) = reference_call_world();
    let body = job["request"].as_object_mut().unwrap();
    body.remove("call");
    body.insert(
        "items".into(),
        json!({"observations":{"current":"current.json"},
            "selections":[{"call":{"function":"Keep","snapshot":"current",
                "arguments":[{"parameter":"x","value":{"each":{"population":population}}}]}}]
        }),
    );
    let expanded = plain(&case.json("each.json", &job));
    assert_eq!(expanded.exit, 0, "{}", expanded.document);
    assert_eq!(
        expanded.document["selections"],
        json!([{"index":0,"expanded":2}])
    );
    let items = expanded.document["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    for (item, key) in items.iter().zip(["root", "child"]) {
        assert_eq!(item["object"], json!({"population":population,"key":key}));
        assert_eq!(item["result"]["outcome"]["value"]["type"], "reference");
    }
    let root = items[0]["result"]["outcome"]["value"].clone();
    let child = items[1]["result"]["outcome"]["value"].clone();
    assert_eq!(root["identity"], "726f6f74");
    assert_eq!(child["identity"], "6368696c64");
    let body = job["request"].as_object_mut().unwrap();
    body.remove("items");
    body.insert(
        "call".into(),
        json!({"function":"Keep","snapshot":"current.json",
        "arguments":[{"parameter":"x","value":root}]}),
    );
    let roundtrip = plain(&case.json("reference.json", &job));
    assert_eq!(roundtrip.exit, 0, "{}", roundtrip.document);
    assert_eq!(roundtrip.document["result"]["value"], root);
    job["request"]["call"] = json!({"function":"Find","snapshot":"current.json",
        "arguments":[{"parameter":"p","value":{"population":population}},
            {"parameter":"r","value":root}]});
    let members = plain(&case.json("population.json", &job));
    assert_eq!(members.exit, 0, "{}", members.document);
    assert_eq!(members.document["result"]["value"], root);
    job["request"]["call"] = json!({"function":"Keep","snapshot":"current.json",
        "arguments":[{"parameter":"x","value":{"reference":{"population":population,"key":"root"}}}]});
    assert_eq!(plain(&case.json("legacy-reference.json", &job)).exit, 20);
}

/// The package route retains the genuine snapshot normalization owners and
/// strict artifact identities, then reads its own canonical reference result.
#[test]
#[trace("FR-945-AC-1", "FR-650-AC-22")]
fn canonical_run_package_reference_roundtrip_uses_retained_snapshot_owners() {
    let (case, mut job, population) = reference_call_world();
    let mut compile = job["request"].clone();
    compile.as_object_mut().unwrap().remove("call");
    compile["output"] = json!({"package":"package.json"});
    compile["handoff"] = json!({"directory":"handoff","package_name":"canonical-world",
        "native_join":{"work_units":1_000_000,"storage":16_777_216}});
    fsl_command::command::compile(&case.json(
        "compile.json",
        &json!({
            "format":"native-compile/1","request":compile
        }),
    ))
    .expect("the genuine checked package and its identity witnesses are emitted");
    let body = job["request"].as_object_mut().unwrap();
    body.remove("call");
    body.insert(
        "items".into(),
        json!({"observations":{"current":"current.json"},
            "selections":[{"call":{"function":"Keep","snapshot":"current",
                "arguments":[{"parameter":"x","value":{"each":{"population":population}}}]}}]
        }),
    );
    let source = plain(&case.json("source-reference.json", &job));
    assert_eq!(source.exit, 0, "{}", source.document);
    let reference = source.document["items"][0]["result"]["outcome"]["value"].clone();
    assert_eq!(reference["type"], "reference");
    std::fs::remove_file(case.path("world.native")).unwrap();
    let body = job["request"].as_object_mut().unwrap();
    body.remove("program");
    body.remove("items");
    body.insert("package".into(), json!({"directory":"handoff"}));
    body.insert(
        "call".into(),
        json!({"function":"Keep","snapshot":"current.json",
        "arguments":[{"parameter":"x","value":reference}]}),
    );
    let first = plain(&case.json("package-reference.json", &job));
    assert_eq!(first.exit, 0, "{}", first.document);
    assert_eq!(first.document["result"]["value"], reference);
    job["request"]["call"]["arguments"][0]["value"] = first.document["result"]["value"].clone();
    let second = plain(&case.json("package-reference-again.json", &job));
    assert_eq!(second.exit, 0, "{}", second.document);
    assert_eq!(second.document["result"]["value"], reference);
}

/// Review FND-002: valid deep results exceed both former JSON and codec caps.
#[test]
#[trace("FR-650")]
fn review_fix_deep_completed_values_use_actual_depth_policy() {
    fn deep_result(path: &std::path::Path) -> Value {
        let result = fsl_command::command::run_with_limits_and_overrides(
            path,
            RunLimits::default(),
            &RequestOverrides::default(),
            &crate::negotiation::Fr290,
        )
        .expect("deep call completes");
        assert_eq!(result.exit_code, 0);
        let bytes = result.outcome.expect("call outcome").to_bytes().unwrap();
        // This regression deliberately exceeds serde's default test-reader cap.
        let mut reader = serde_json::Deserializer::from_slice(&bytes);
        reader.disable_recursion_limit();
        let document: Value = serde::Deserialize::deserialize(&mut reader).unwrap();
        reader.end().unwrap();
        assert_eq!(document["operation"], "execute");
        assert_eq!(document["category"], "success");
        document
    }

    fn source(reference: bool) -> String {
        let profile = if reference { "model" } else { "value" };
        let mut source = format!("language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.{profile}.complete/v1\";\n");
        if reference {
            source.push_str(&format!(
                "model Config = {:?};\n",
                config_domain::PACKAGE_IDENTITY
            ));
        }
        let arguments = if reference {
            "x: Config::ConfigVersion"
        } else {
            ""
        };
        let actual = if reference { "x" } else { "" };
        for index in 0..100 {
            let ty = if index == 0 {
                if reference {
                    "Config::ConfigVersion".to_owned()
                } else {
                    "Integer".to_owned()
                }
            } else {
                format!("D{}", index - 1)
            };
            source.push_str(&format!("record D{index} {{ inner: {ty}; }}\n"));
            let inner = if index == 0 {
                if reference {
                    "x".to_owned()
                } else {
                    "5".to_owned()
                }
            } else {
                format!("Make{}({actual})", index - 1)
            };
            source.push_str(&format!("function Make{index} using v({arguments}): D{index} pure {{ D{index}{{inner: {inner}}} }}\n"));
        }
        source.push_str(&format!(
            "function Deep using v({arguments}): D99 pure {{ Make99({actual}) }}\n"
        ));
        source
    }
    let case = Case::new();
    case.write("deep.native", source(false).as_bytes());
    let document = deep_result(&case.json("deep.json", &request("deep.native", "Deep", &[])));
    let mut leaf = &document["result"]["value"];
    for _ in 0..100 {
        assert_eq!(leaf["type"], "record");
        leaf = &leaf["fields"][0]["value"];
    }
    assert_eq!(leaf, &json!({"type":"integer","value":"5"}));

    let (case, mut job, population) = reference_call_world();
    job["request"].as_object_mut().unwrap().remove("call");
    job["request"]["items"] = json!({"observations":{"current":"current.json"},"selections":[{"call":{
        "function":"Keep","snapshot":"current","arguments":[{"parameter":"x","value":{"each":{"population":population}}}]
    }}]});
    let reference = plain(&case.json("input-reference.json", &job));
    assert_eq!(reference.exit, 0, "{}", reference.document);
    let input = reference.document["items"][0]["result"]["outcome"]["value"].clone();
    case.write("world.native", source(true).as_bytes());
    job["request"].as_object_mut().unwrap().remove("items");
    job["request"]["call"] = json!({"function":"Deep","snapshot":"current.json","arguments":[{"parameter":"x","value":input}]});
    let document = deep_result(&case.json("deep-reference.json", &job));
    let mut leaf = &document["result"]["value"];
    for _ in 0..100 {
        assert_eq!(leaf["type"], "record");
        leaf = &leaf["fields"][0]["value"];
    }
    assert_eq!(leaf, &input);
}

/// Run the owner-maintained corpus in place; the source checkout is never
/// copied into this repository and its refusal snapshots are not the oracle.
#[test]
#[ignore = "external acceptance lane: FSL_RUN_INPUTS_FIXTURES names the authoritative examples/run-inputs directory"]
#[trace("FSpec-FR-550-AC-1", "FR-650-AC-18")]
fn canonical_run_acceptance_values_from_authoritative_examples() {
    let directory = std::path::PathBuf::from(
        std::env::var_os("FSL_RUN_INPUTS_FIXTURES")
            .expect("the acceptance lane supplies its read-only fixture directory"),
    );
    let integer = |value: &str| json!({"type":"integer", "value":value});
    for (case, expected) in [
        ("call-two-arguments", integer("5")),
        ("call-arguments-reordered", integer("5")),
        ("call-big-integer", integer("99999999999999999999999")),
        ("call-negative-integer", integer("-5")),
        ("call-zero-integer", integer("0")),
        ("call-boolean", json!({"type":"boolean","value":true})),
        (
            "call-sequence-at-bound",
            json!({"type":"sequence","elements":[integer("1"),integer("2"),integer("3")]}),
        ),
        (
            "call-sequence-empty",
            json!({"type":"sequence","elements":[]}),
        ),
        ("call-digit-at-max", integer("9")),
        ("call-digit-at-min", integer("0")),
        ("call-tagged-integer", integer("5")),
        (
            "call-tagged-rational",
            json!({"type":"rational","numerator":"2","denominator":"3"}),
        ),
        ("call-tagged-text", json!({"type":"text","value":"hi"})),
        (
            "call-tagged-set-order",
            json!({"type":"set","elements":[integer("1"),integer("3")]}),
        ),
        ("option-engine-interpreter", integer("5")),
        ("option-request-bytes-large", integer("5")),
    ] {
        let path = directory.join("requests").join(format!("{case}.json"));
        let limits = if case == "option-request-bytes-large" {
            RunLimits {
                request_bytes: 100_000,
                ..RunLimits::default()
            }
        } else {
            RunLimits::default()
        };
        let outcome = run(&path, limits, &RequestOverrides::default());
        let document = outcome.expect("execute", "success", 0);
        assert_eq!(document["result"]["value"], expected, "{case}: {document}");
    }
}

/// The public command entry uses the existing plan coordinator and retains
/// distinct passed, refuted and blocked outcomes.
#[test]
#[ignore = "external acceptance lane: FSL_RUN_INPUTS_FIXTURES names the authoritative examples/run-inputs directory"]
#[trace("FSpec-FR-824-AC-1", "FSpec-FR-824-AC-6", "FR-1038-AC-1")]
fn canonical_run_acceptance_plans_retain_dispositions_and_severity() {
    let directory = std::path::PathBuf::from(
        std::env::var_os("FSL_RUN_INPUTS_FIXTURES")
            .expect("the acceptance lane supplies its read-only fixture directory"),
    );
    let temporary = Case::new();
    for (case, status, exit) in [
        ("plan-passes", "passed", 0),
        ("plan-fails", "failed", 10),
        ("plan-blocks", "failed", 22),
        ("plan-null-members-as-absent", "passed", 0),
    ] {
        let originals = directory.join("requests");
        let mut job: Value = serde_json::from_slice(
            &std::fs::read(originals.join(format!("{case}.json")))
                .expect("the authoritative request exists"),
        )
        .expect("the authored request is JSON");
        // Only the private request selects a backend. Its source/model/snapshot
        // files remain in the authoritative read-only checkout.
        let body = &mut job["request"];
        body["requirements"] = json!({"backend":"en-1"});
        for model in body["models"].as_array_mut().unwrap() {
            model["source"]["file"] =
                json!(originals.join(model["source"]["file"].as_str().unwrap()));
        }
        body["program"]["source"]["file"] =
            json!(originals.join(body["program"]["source"]["file"].as_str().unwrap()));
        body["verification_plan"]["initial"] =
            json!(originals.join(body["verification_plan"]["initial"].as_str().unwrap()));
        let outcome = plain(&temporary.json(&format!("{case}.json"), &job));
        let document = &outcome.document["verification_plan"];
        assert_eq!(outcome.exit, exit, "{case}: {document}");
        assert_eq!(document["status"], status, "{case}: {document}");
        let dispositions = document["dispositions"]
            .as_object()
            .expect("one named disposition per plan step");
        assert_eq!(dispositions.len(), 2, "{case}: {document}");
        if case == "plan-passes" || case == "plan-null-members-as-absent" {
            assert!(
                dispositions.values().all(|step| step["status"] == "proved"),
                "{case}: {document}"
            );
        }
        if case == "plan-fails" || case == "plan-blocks" {
            assert_eq!(document["failed_steps"], json!(["low"]), "{document}");
            assert_eq!(dispositions["low"]["status"], "refuted", "{document}");
        }
        if case == "plan-blocks" {
            assert_eq!(dispositions["safe"]["blocked_by"], "low", "{document}");
            assert!(
                dispositions
                    .values()
                    .any(|step| step["status"] == "blocked" && step["result"].is_null()),
                "{document}"
            );
        }
        if case == "plan-fails" {
            assert_eq!(dispositions["safe"]["status"], "proved", "{document}");
        }
        if case == "plan-passes" {
            job["request"]["requirements"]["backend"] = json!("fsl.absent");
            let unknown = plain(&temporary.json("unknown-plan-backend.json", &job));
            assert_eq!(unknown.exit, 20, "{}", unknown.document);
            assert_eq!(
                unknown.document["verification_plan"]["dispositions"]["safe"]["result"]
                    ["diagnostic"]["cause"],
                "unknown-backend",
                "{}",
                unknown.document
            );
        }
    }
}

/// FR-100, FR-286-AC-5: a completed call writes one `execute` outcome with
/// `result` `completed` and its value, exit 0.
#[test]
#[trace("FR-100", "FR-286-AC-5", "FR-285-AC-1")]
fn a_completed_call_writes_its_value() {
    let (_case, path) = remaining();
    let outcome = plain(&path);
    let document = outcome.expect("execute", "success", 0);
    assert_eq!(
        document["result"],
        json!({"kind": "completed", "value": {"type":"integer", "value": "12"}}),
        "{document}"
    );
    let (_case, path) = packing(
        "IsPacked",
        &[("n", json!({"type":"integer", "value": "7"}))],
    );
    let document = plain(&path);
    assert_eq!(
        document.expect("execute", "success", 0)["result"]["value"],
        json!({"type":"boolean", "value": true})
    );
}

/// A finite-window timestamped clause over stamps 0, 2 and 5 whose
/// `level > 0` atom holds as `levels` says; `missing` leaves the valuation of
/// the atom absent at position 1 (FR-501-AC-4).
fn finite_window(case: &Case, levels: [i64; 3], missing: bool) -> std::path::PathBuf {
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile t = \"{STAMPED}\";\n\
         record Reading {{ level: Int[0, 9]; }}\n\
         temporal Seen using t over (r: Reading) clock \"plant-ms\" on origin {{\n\
           eventually[0,5] holds(r.level > 0)\n\
         }}\n"
    );
    case.write("program.native", source.as_bytes());
    let clocks = json!({"plant-ms": {"kind": "timestamped_event", "timestamp_unit": "ms"}});
    let observations: Vec<Value> = [0_u64, 2, 5]
        .into_iter()
        .zip(levels)
        .map(|(stamp, level)| {
            json!({"position": {"sample": {"level": {"integer": level.to_string()}}},
                "stamp": stamp})
        })
        .collect();
    let missing: Vec<Value> = if missing {
        vec![json!({"position": 1, "atom": 1})]
    } else {
        vec![]
    };
    let request = json!({"format": "native-run/1", "request": {"models": [],
        "program": {"source": {"file": "program.native", "authority": "test",
            "identity": "test:finite-window", "document": "FiniteWindow", "formal_revision": 1}},
        "clocks": clocks,
        "clause": {"selection": {"temporal": {"name": "Seen", "trace": {"timestamped": {
            "observations": observations,
            "progress": {"watermark": 5, "completeness": {"complete": {"through": 5}},
                "scope": "closed", "execution": "closed"},
            "lower_boundary": "authoritative-origin",
            "binding": {"profile": STAMPED, "clock": "plant-ms",
                "parameters": clocks["plant-ms"]},
            "missing": missing}}}},
            "observations": {}}}});
    case.json("run.json", &request)
}

/// FR-285-AC-7, AC-12, FR-501-AC-4: a finite-window clause whose valuation is
/// absent is `indeterminate`, exit 11; complete and satisfied it succeeds,
/// exit 0; complete and never satisfied it is a violation, exit 10.
#[test]
#[trace("FR-285-AC-7", "FR-285-AC-12", "FR-501-AC-4")]
fn finite_window_clause_reaches_indeterminate_and_violation() {
    let case = Case::new();
    let open = plain(&finite_window(&case, [0, 0, 0], true));
    let document = open.expect("monitor", "indeterminate", 11);
    let item = &document["items"][0];
    assert_eq!(item["result"], "indeterminate", "{document}");
    assert_eq!(item["cause"]["kind"], "missing-valuation", "{document}");
    let held = plain(&finite_window(&case, [0, 0, 1], false));
    assert_eq!(held.exit, 0, "{}", held.document);
    let failed = plain(&finite_window(&case, [0, 0, 0], false));
    assert_eq!(failed.exit, 10, "{}", failed.document);
}

/// FR-100-AC-5: an unknown function refuses `missing_declaration` at stage
/// `call`, exit 20, naming the function in `details`.
#[test]
#[trace("FR-100-AC-5")]
fn an_unknown_function_refuses_missing_declaration() {
    let (_case, path) = packing("nope", &[]);
    let diagnostic = plain(&path)
        .refused("execute", 20, "missing_declaration")
        .clone();
    assert_eq!(
        diagnostic["details"],
        json!({"function": "nope"}),
        "{diagnostic}"
    );
}

/// FR-100-AC-4: an out-of-range argument (`invalid-value`) and a wrong value
/// kind (`wrong-value-kind`) each refuse `invalid_runtime_input`, exit 20.
#[test]
#[trace("FR-100-AC-4")]
fn call_defects_refuse_invalid_runtime_input() {
    for (function, arguments, cause) in [
        (
            "IsPacked",
            vec![("n", json!({"type":"integer", "value": "11"}))],
            "invalid-value",
        ),
        (
            "IsPacked",
            vec![("n", json!({"type":"boolean", "value": true}))],
            "wrong-value-kind",
        ),
    ] {
        let (_case, path) = packing(function, &arguments);
        let outcome = plain(&path);
        let diagnostic = outcome.refused("execute", 20, "invalid_runtime_input");
        assert_eq!(diagnostic["cause"], cause, "{diagnostic}");
    }
}

/// FR-100-AC-6: a call budget of zero work units ends `incomplete`, exit 22.
#[test]
#[trace("FR-100-AC-6", "FR-285-AC-6")]
fn an_exhausted_call_budget_exits_22() {
    let case = Case::new();
    case.write("packing.fsl", SOURCE.as_bytes());
    let mut exhausted = request(
        "packing.fsl",
        "Remaining",
        &[
            ("total", json!({"type":"integer", "value": "17"})),
            ("held", json!({"type":"integer", "value": "5"})),
        ],
    );
    exhausted["request"]["call"]["accounting"] = json!({"work_units": 0});
    let outcome = plain(&case.json("run.json", &exhausted));
    outcome.expect("execute", "incomplete", 22);
}

/// `--request-bytes` and `--dependent-bytes` (FR-026-AC-4): one byte under the
/// request or source size stops at intake with exit 22 naming the ceiling;
/// the exact size runs.
#[test]
#[trace("FR-026-AC-4")]
fn request_and_dependent_byte_ceilings_are_reached_and_raised() {
    let (_case, path) = remaining();
    let request_size = std::fs::metadata(&path).unwrap().len() as usize;
    let over = run(
        &path,
        RunLimits {
            request_bytes: request_size - 1,
            ..RunLimits::default()
        },
        &RequestOverrides::default(),
    );
    over.expect("execute", "incomplete", 22);
    assert!(
        over.document.to_string().contains("request_bytes"),
        "{}",
        over.document
    );

    let over = run(
        &path,
        RunLimits {
            dependent_bytes: SOURCE.len() - 1,
            ..RunLimits::default()
        },
        &RequestOverrides::default(),
    );
    over.expect("execute", "incomplete", 22);
    assert!(
        over.document.to_string().contains("dependent_bytes"),
        "{}",
        over.document
    );

    let fits = run(
        &path,
        RunLimits {
            request_bytes: request_size,
            dependent_bytes: SOURCE.len(),
            ..RunLimits::default()
        },
        &RequestOverrides::default(),
    );
    fits.expect("execute", "success", 0);
}

/// FR-001-AC-4 and FR-285-AC-5 (`--limit s1.input_bytes=N`): a run whose
/// source is over the S1 byte ceiling refuses `stage_limit_exceeded`
/// naming the setting, exit 22.
#[test]
#[trace("FR-001-AC-4", "FR-285-AC-5")]
fn source_byte_stage_limit_is_reached() {
    let (_case, path) = remaining();
    let mut limits = fsl_replay::spine::SpineLimits::default();
    limits.source.source_bytes = 1;
    let overrides = RequestOverrides {
        limits: Some(&limits),
        ..RequestOverrides::default()
    };
    let outcome = run(&path, RunLimits::default(), &overrides);
    let diagnostic = outcome.refused("execute", 22, "stage_limit_exceeded");
    assert!(
        diagnostic.to_string().contains("s1.input_bytes"),
        "{diagnostic}"
    );
}

/// FR-640-AC-14 (`--clocks`): a request that carries `clocks` and is also
/// handed the override refuses `invalid-request` at stage `request`, exit 20.
#[test]
#[trace("FR-640-AC-14")]
fn clocks_given_twice_refuse() {
    let case = Case::new();
    let path = finite_window(&case, [0, 0, 1], false);
    let supplied = RawValue::from_string(
        json!({"plant-ms": {"kind": "timestamped_event",
            "timestamp_unit": "ms"}})
        .to_string(),
    )
    .unwrap();
    let overrides = RequestOverrides {
        clocks: Some(&supplied),
        ..RequestOverrides::default()
    };
    let outcome = run(&path, RunLimits::default(), &overrides);
    outcome.refused("execute", 20, "invalid-request");
    assert!(
        outcome.document["last_stage"].is_null(),
        "{}",
        outcome.document
    );
}

/// FR-001-AC-3, AC-8: invalid UTF-8 and NUL sources refuse, exit 20.
#[test]
#[trace("FR-001-AC-3", "FR-001-AC-8")]
fn invalid_utf8_and_nul_sources_refuse() {
    let mut nul = SOURCE.as_bytes().to_vec();
    nul.extend_from_slice(b"\0\n");
    for source in [INVALID_UTF8.to_vec(), nul] {
        let case = Case::new();
        case.write("packing.fsl", &source);
        let path = case.json("run.json", &request("packing.fsl", "Remaining", &[]));
        let outcome = plain(&path);
        outcome.expect("execute", "refusal", 20);
        assert!(
            !outcome.document["diagnostics"]
                .as_array()
                .unwrap()
                .is_empty(),
            "{}",
            outcome.document
        );
    }
}

/// FR-001 file errors and malformed requests: a missing, a directory and an
/// empty request operand, a missing source, malformed JSON and an unknown
/// member each refuse, exit 20, with one diagnostic.
#[test]
#[trace("FR-001", "FR-286")]
fn odd_operands_and_malformed_requests_refuse() {
    let case = Case::new();
    case.write("packing.fsl", SOURCE.as_bytes());
    std::fs::create_dir_all(case.path("folder")).unwrap();
    let mut unknown = request("packing.fsl", "Remaining", &[]);
    unknown["request"]["extra"] = json!(1);
    for path in [
        case.path("absent.json"),
        case.path("folder"),
        case.write("empty.json", b""),
        case.json("source.json", &request("absent.fsl", "Remaining", &[])),
        case.write("malformed.json", b"[1,"),
        case.json("unknown.json", &unknown),
    ] {
        let outcome = plain(&path);
        outcome.expect("execute", "refusal", 20);
        assert_eq!(
            outcome.document["diagnostics"].as_array().map(Vec::len),
            Some(1),
            "{}: {}",
            path.display(),
            outcome.document
        );
    }
}
