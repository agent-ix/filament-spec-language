// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1015 AC-5 to AC-8, FR-1035 and FSpec FR-811-AC-6, FR-812-AC-6/AC-7,
//! FR-814-AC-6 and FR-817-AC-8 to AC-10: these cover the single
//! unrepresentable cause, obligation names, the stop order, the generator,
//! the `fsl:` comment spelling, population bounds and `Init`.

use crate::plan_export_surface::{braking_request, counter_export, outcome_json};
use crate::support::config_version_spine::{spine, Case};
use ix_trace_rs::trace;
use serde_json::{json, Value};

const SPINE_HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"filament.model.complete/v1\";\n\
     model Config = \"example/config-version\";\n";

/// FR-447's `Braking` without `t`, with `SlowEnough` over it.
const BRAKING: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile p = \"filament.temporal.infinite-trace/v1\";\n\
     hybrid Braking using p {\n\
       mode Moving invariant { v >= 0 and v <= 20 and a >= 1 and a <= 2 }\n\
         flow { v' = -a; a' = 0; }\n\
         transition to Stopped when { v = 0 } reset { v = 0 };\n\
       mode Stopped invariant { v = 0 }\n\
         flow { v' = 0; a' = 0; }\n\
     }\n\
     temporal SlowEnough using p over (h: Braking) clock \"model-time\" \
     on origin { always holds(h.v < 30) }\n";

fn source(file: &str) -> Value {
    json!({"file": file, "authority": "agent-ix", "identity": format!("test:w53:{file}"),
        "document": "W53", "formal_revision": 1})
}

/// Exports `unit` (over the spine model when `with_model`) with `member`.
fn export(unit: &str, with_model: bool, member: Value) -> (u8, Value) {
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    std::fs::write(directory.path().join("unit.native"), unit).unwrap();
    let models = if with_model {
        json!([{"format": "semantic-ir/2.0.0", "source": source("spine-model.semantic-ir.json")}])
    } else {
        json!([])
    };
    let request = json!({"format": "native-run/1", "request": {"models": models,
        "program": {"source": source("unit.native")}, "export": member}});
    let path = directory.path().join("request.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    match fsl_command::command::run(&path, &crate::negotiation::Fr290) {
        Ok(result) => (
            result.exit_code,
            serde_json::from_str(&result.value.to_string()).expect("a JSON document"),
        ),
        Err(error) => (error.category().exit_code(), error.value().unwrap()),
    }
}

fn tla(obligations: Value) -> Value {
    json!({"profile": "filament.output.tla2/v1", "obligations": obligations})
}

fn lean(obligations: Value) -> Value {
    json!({"profile": "filament.output.lean4/v1", "obligations": obligations})
}

fn names(value: &Value, text: &str) {
    assert!(value.to_string().contains(text), "names {text}: {value}");
}

fn text(value: &Value) -> &str {
    value["export"]["target_text"]
        .as_str()
        .expect("target text")
}

#[test]
#[trace("TC-3002", "FR-1015-AC-5")]
#[trace("FSpec-TC-2029", "FSpec-FR-811-AC-6")]
fn only_what_the_target_cannot_state_is_unrepresented() {
    let (exit, value) = counter_export(tla(json!(["Bounded"])));
    assert_eq!(exit, 0, "{value}");
    let record = &value["export"]["records"][0];
    assert_eq!(record["disposition"], "preserved", "{value}");
    assert_eq!(record["causes"], json!([]), "{value}");
}

/// FR-1015-AC-5 and FSpec FR-811-AC-6: `SlowEnough` over the hybrid `Braking`
/// is `unrepresented` with one `target-unrepresentable` cause per distinct
/// flow-equation node, each naming the family, the node's
/// `node_tag/semantic_form`, its node id and its source region.
#[test]
#[trace("TC-3002", "FR-1015-AC-5")]
#[trace("FSpec-TC-2029", "FSpec-FR-811-AC-6")]
fn a_hybrid_claim_is_unrepresented_by_its_flow_nodes() {
    for (profile, family) in [
        ("filament.output.tla2/v1", "tla"),
        ("filament.output.lean4/v1", "lean"),
    ] {
        let member = json!({"profile": profile, "obligations": ["SlowEnough"],
            "minimum_fidelity": "any"});
        let (exit, value) = export(BRAKING, false, member);
        assert_eq!(exit, 0, "{value}");
        let record = &value["export"]["records"][0];
        assert_eq!(record["disposition"], "unrepresented", "{value}");
        assert_eq!(record["regions"], json!([]), "{value}");
        let causes = record["causes"].as_array().expect("causes");
        let mut constructs: Vec<&str> = causes
            .iter()
            .map(|cause| cause["construct"].as_str().expect("a construct"))
            .collect();
        constructs.sort_unstable();
        assert_eq!(constructs, ["expression/unary", "value/literal"], "{value}");
        for cause in causes {
            assert_eq!(cause["kind"], "target-unrepresentable", "{value}");
            assert_eq!(cause["target"], family, "{value}");
            assert_ne!(cause["node"], "SlowEnough", "{value}");
            names(&cause["node"], "test:w53:unit.native");
            assert_eq!(cause["locus"]["kind"], "region", "{value}");
        }
    }
}

#[test]
#[trace("TC-3002", "FR-1015-AC-6")]
#[trace("FSpec-TC-2032", "FSpec-FR-817-AC-8")]
#[trace("FSpec-TC-2032", "FSpec-FR-817-AC-9")]
fn obligations_are_nonempty_declared_names() {
    let (exit, value) = counter_export(tla(json!([])));
    assert_eq!(exit, 20, "{value}");
    names(&value, "invalid-value");
    names(&value, "/request/export/obligations");
    let mut member = tla(json!([]));
    member.as_object_mut().unwrap().remove("obligations");
    let (exit, value) = counter_export(member);
    assert_eq!(exit, 20, "{value}");
    names(&value, "missing-member");
    let (_, value) = export(BRAKING, false, tla(json!(["SlowEnough"])));
    assert_eq!(
        value["export"]["records"][0]["obligation"], "SlowEnough",
        "{value}"
    );
    let (exit, value) = counter_export(tla(json!(["CanInc", "Nope"])));
    let records = value["export"]["records"].as_array().expect("records");
    let dispositions: Vec<_> = records.iter().map(|r| r["disposition"].clone()).collect();
    assert_eq!(
        dispositions,
        [json!("refused"), json!("refused")],
        "{value}"
    );
    names(&records[0], "not-an-obligation");
    names(&records[1], "not-an-obligation");
    assert_eq!(exit, 21, "{value}");
}

/// FR-1015-AC-6 with an invariant among the names: `Bounded` is an
/// obligation, the precondition and the unknown name are refused records.
#[test]
#[trace("TC-3002", "FR-1015-AC-6")]
fn an_invariant_among_refused_names_is_preserved() {
    let (exit, value) = counter_export(tla(json!(["CanInc", "Bounded", "Nope"])));
    let records = value["export"]["records"].as_array().expect("records");
    let dispositions: Vec<_> = records.iter().map(|r| r["disposition"].clone()).collect();
    assert_eq!(
        dispositions,
        [json!("refused"), json!("preserved"), json!("refused")],
        "{value}"
    );
    assert_eq!(exit, 21, "{value}");
}

#[test]
#[trace("TC-3002", "FR-1015-AC-7")]
#[trace("FSpec-TC-2032", "FSpec-FR-817-AC-10")]
fn the_floor_is_decided_before_the_text_is_charged() {
    let mut member = tla(json!(["SlowEnough"]));
    member["limits"] = json!({"export.output_bytes": 1});
    let (exit, value) = export(BRAKING, false, member);
    assert_eq!(exit, 21, "{value}");
    names(&value["export"], "below-fidelity-floor");
    let mut member = tla(json!(["SlowEnough"]));
    member["limits"] = json!({"export.nodes": 1});
    let (exit, value) = export(BRAKING, false, member);
    assert_eq!(exit, 22, "{value}");
    names(&value["export"], "export.nodes");
}

/// FR-1015-AC-2: a lowered export over `export.output_bytes` is `incomplete`.
#[test]
#[trace("TC-2865", "FR-1015-AC-2")]
fn a_lowered_text_over_the_byte_limit_is_incomplete() {
    let mut member = tla(json!(["Bounded"]));
    member["limits"] = json!({"export.output_bytes": 1});
    let (exit, value) = counter_export(member);
    assert_eq!(exit, 22, "{value}");
    names(&value["export"], "export.output_bytes");
}

#[test]
#[trace("TC-3002", "FR-1015-AC-8")]
fn the_generator_is_the_application() {
    let mut member = tla(json!(["SlowEnough"]));
    member["minimum_fidelity"] = json!("any");
    let (_, value) = export(BRAKING, false, member);
    assert_eq!(
        value["export"]["package"]["generator"],
        json!({"identity": "filament-spec-language", "version": filament_spec_language::VERSION}),
        "{value}"
    );
}

/// One `fsl:` comment line: marker, node id and region, as FSpec FR-814
/// spells them.
fn comment_is_canonical(line: &str, marker: &str) -> bool {
    let Some(rest) = line.strip_prefix(&format!("{marker} fsl: ")) else {
        return false;
    };
    let tokens: Vec<&str> = rest.split(' ').collect();
    let [node, region] = tokens.as_slice() else {
        return false;
    };
    if node.is_empty() {
        return false;
    }
    if *region == "none" {
        return true;
    }
    let decimal = |s: &str| {
        !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) && (s == "0" || !s.starts_with('0'))
    };
    let Some((source, offsets)) = region.rsplit_once(':') else {
        return false;
    };
    let Some((start, end)) = offsets.split_once('-') else {
        return false;
    };
    source.contains('/')
        && decimal(start)
        && decimal(end)
        && start.parse::<u64>().unwrap() < end.parse::<u64>().unwrap()
}

#[test]
#[trace("FSpec-TC-2030", "FSpec-FR-814-AC-6")]
fn every_region_comment_has_the_one_spelling() {
    for (profile, marker) in [
        ("filament.output.tla2/v1", "\\*"),
        ("filament.output.lean4/v1", "--"),
    ] {
        let (exit, value) = counter_export(json!({"profile": profile, "obligations": ["Bounded"]}));
        assert_eq!(exit, 0, "{value}");
        let comments: Vec<&str> = text(&value)
            .lines()
            .filter(|line| line.contains(" fsl: "))
            .collect();
        assert!(!comments.is_empty(), "{value}");
        for line in comments {
            assert!(comment_is_canonical(line, marker), "{line}");
            assert!(line.contains("agent-ix/test:"), "{line}");
        }
    }
}

#[test]
#[trace("TC-3003", "FR-1035-AC-1")]
#[trace("FSpec-TC-2031", "FSpec-FR-812-AC-6")]
#[ignore = "QSL-2873: population bounds from bound declarations are not exposed to the export yet"]
fn a_bound_declaration_bounds_the_exported_population() {
    let low =
        "invariant Low using v on Config::ConfigVersion at current { self.versionNumber >= 0 }\n";
    let bounded = format!("{SPINE_HEADER}bound HistoryBound on Config::config_history [8]\n{low}");
    let (exit, value) = export(&bounded, true, tla(json!(["Low"])));
    assert_eq!(exit, 0, "{value}");
    let module = text(&value);
    assert!(
        module
            .lines()
            .any(|line| line.contains("Cardinality(DOMAIN v_") && line.contains(") <= 8")),
        "{module}"
    );
    let (exit, value) = export(&format!("{SPINE_HEADER}{low}"), true, tla(json!(["Low"])));
    assert_eq!(exit, 0, "{value}");
    assert!(!text(&value).contains("Cardinality"), "{value}");
}

#[test]
#[trace("TC-3003", "FR-1035-AC-2")]
#[trace("FSpec-TC-2031", "FSpec-FR-812-AC-7")]
#[ignore = "QSL-2873: Init from declared initial constraints is not implemented yet"]
fn init_states_the_declared_initial_constraints() {
    let unit = format!(
        "{SPINE_HEADER}invariant Low using v on Config::ConfigVersion at current {{ self.versionNumber >= 0 }}\n\
         constraint Small using v on Config::ConfigVersion at current {{ self.versionNumber <= 3 }}\n"
    );
    let (exit, value) = export(&unit, true, tla(json!(["Low"])));
    assert_eq!(exit, 0, "{value}");
    let module = text(&value);
    assert!(module.contains("Init == TypeOK"), "{module}");
    assert!(!module.contains("<= 3"), "{module}");
    let mut member = tla(json!(["SlowEnough"]));
    member["minimum_fidelity"] = json!("any");
    let (exit, value) = export(BRAKING, false, member);
    assert_eq!(exit, 0, "{value}");
    let module = text(&value);
    let init = module
        .split("\n\n")
        .find(|definition| definition.contains("Init =="))
        .expect("an Init definition");
    assert!(init.contains("Moving"), "{init}");
}

#[test]
#[trace("TC-3003", "FR-1035-AC-3")]
#[trace("TC-3003", "FR-1035-AC-4")]
#[ignore = "QSL-2873: CheckedGraph::population_bound and the bound's [N] region are not implemented yet"]
fn the_type_ok_comment_names_the_bound_region() {
    let low =
        "invariant Low using v on Config::ConfigVersion at current { self.versionNumber >= 0 }\n";
    let unbounded = format!("{SPINE_HEADER}{low}");
    let (_, value) = export(&unbounded, true, tla(json!(["Low"])));
    assert!(!text(&value).contains("Cardinality"), "{value}");
    let bounded = format!("{SPINE_HEADER}bound HistoryBound on Config::config_history [8]\n{low}");
    let start = bounded.find("[8]").expect("the bound's [N]");
    let region = format!("agent-ix/test:w53:unit.native:{start}-{}", start + 3);
    let (exit, value) = export(&bounded, true, tla(json!(["Low"])));
    assert_eq!(exit, 0, "{value}");
    let lines: Vec<&str> = text(&value).lines().collect();
    let type_ok = lines
        .iter()
        .position(|line| line.starts_with("TypeOK =="))
        .expect("a TypeOK definition");
    let own = lines[type_ok - 1];
    assert!(own.starts_with("\\* fsl: "), "the TypeOK comment: {own}");
    assert!(!own.contains(&region), "{own} names only TypeOK's region");
    let bound = lines
        .iter()
        .position(|line| {
            line.starts_with("b_HistoryBound == Cardinality(DOMAIN v_") && line.ends_with(") <= 8")
        })
        .expect("the bound's own b_HistoryBound definition");
    assert!(
        lines[type_ok..]
            .iter()
            .take_while(|line| !line.is_empty())
            .any(|line| line.contains("b_HistoryBound")),
        "TypeOK conjoins b_HistoryBound"
    );
    let comment = lines[bound - 1];
    assert!(
        comment.starts_with("\\* fsl: "),
        "the bound's unindented comment: {comment}"
    );
    assert!(
        comment.ends_with(&format!(" {region}")),
        "{comment} names {region}"
    );
}

/// The `test/float-export` package: object type `Gauge` (population
/// `gauges`) with the `Float64` field `a` and the operation `flip()`
/// (`modifies a`).
pub(crate) fn float_model() -> Value {
    let id = |suffix: &str| format!("ix://test/float-export/{suffix}");
    let origin = |identity: &str| {
        json!({"generated": {"generatorIdentity": identity, "generatorVersion": "1.0.0",
            "inputIdentities": [identity]}})
    };
    let construct = |name: &str, meaning: &str| {
        json!({"kind": {"module": "test/float-export", "name": name}, "moduleVersion": "1.0.0",
            "construct": {"identity": "none", "shape": "record", "members": {},
                "meaning": meaning}})
    };
    let gauge = id("Gauge");
    let field = id("Gauge/a");
    let flip = id("Gauge/flip");
    let population = id("gauges");
    json!({
        "contractVersion": "2.0.0",
        "canonicalization": {"algorithm": "rfc8785-v1"},
        "source": {"identity": id("spec"), "version": "1.0.0", "dialect": "spec-bundle"},
        "package": {"identity": "test/float-export", "version": "1.0.0",
            "mappingVersions": [], "profileVersions": []},
        "occurrences": [], "extensions": [],
        "constructs": [
            construct("object_type", "filament.meaning.model.object-type/v1"),
            construct("population", "filament.meaning.model.population/v1"),
        ],
        "types": [{
            "identity": gauge, "displayName": "Gauge",
            "kind": {"module": "test/float-export", "name": "object_type"},
            "roles": [], "origin": origin(&gauge), "constraints": [], "extensions": [],
            "unknownPolicy": "reject", "supertypes": [], "relationships": [],
            "fields": [{
                "identity": field, "name": "a", "typeRef": "ix://filament/native/Float64",
                "presence": "required", "nullable": false, "defaultKind": "none",
                "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
                "origin": origin(&field),
            }],
            "operations": [{
                "identity": flip, "name": "flip", "params": [], "pre": [], "post": [],
                "origin": {"source": {"sourceIdentity": id("spec"), "path": "spec.fspec",
                    "startLine": 1, "startColumn": 1}},
                "frame": {"modifies": [field], "creates": [], "deletes": []},
            }],
        }],
        "populations": [{
            "identity": population, "displayName": population,
            "kind": {"module": "test/float-export", "name": "population"},
            "members": [gauge], "extent": "closed", "origin": origin(&population),
        }],
    })
}

/// FR-056-AC-62 and FSpec FR-812-AC-8: a `Float64` model field exports
/// through the tagged binary floating point mapping, its `-` through the
/// generated negation, and a NaN constant as the `nan` tag carrying its exact
/// bits, so no rational or decimal value stands for the field.
#[test]
#[trace("TC-3006", "FR-056-AC-62")]
#[trace("FSpec-TC-1971", "FSpec-FR-812-AC-8")]
fn a_float_model_field_exports_its_exact_bits() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("float-model.semantic-ir.json"),
        serde_json::to_vec(&float_model()).unwrap(),
    )
    .unwrap();
    let unit = "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         model Config = \"test/float-export\";\n\
         post Flip using v on Config::Gauge::flip { self.a = -pre(self.a) }\n\
         invariant NotPayload using v on Config::Gauge at current \
         { self.a != float64(bits: 0x7ff8000000000001) }\n";
    std::fs::write(directory.path().join("unit.native"), unit).unwrap();
    let request = json!({"format": "native-run/1", "request": {
        "models": [{"format": "semantic-ir/2.0.0",
            "source": source("float-model.semantic-ir.json")}],
        "program": {"source": source("unit.native")},
        "export": tla(json!(["NotPayload"]))}});
    let path = directory.path().join("request.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    let (exit, value) = match fsl_command::command::run(&path, &crate::negotiation::Fr290) {
        Ok(result) => (
            result.exit_code,
            serde_json::from_str::<Value>(&result.value.to_string()).expect("a JSON document"),
        ),
        Err(error) => (error.category().exit_code(), error.value().unwrap()),
    };
    assert_eq!(exit, 0, "{value}");
    assert_eq!(
        value["export"]["records"][0]["disposition"], "preserved",
        "{value}"
    );
    let target = text(&value);
    assert!(
        target.contains("[tag |-> \"nan\", bits |-> \"7ff8000000000001\"]"),
        "the NaN constant keeps its payload bits: {target}"
    );
    assert!(
        target.contains("\"finite\""),
        "a finite float tag: {target}"
    );
    assert!(target.contains("4503599627370496..9007199254740991"));
    assert!(target.contains("1..4503599627370495, exponent : {-1074}"));
    assert!(target.contains("F_float64_nan_bits(bits)"));
    assert!(target.contains("Len(bits) = 16"));
    assert!(target.contains("SubSeq(bits, 1, 1) \\in {\"7\", \"f\"}"));
    assert!(target.contains("SubSeq(bits, 2, 3) = \"ff\""));
    assert!(target.contains("\\E i \\in 4..16 : SubSeq(bits, i, i) # \"0\""));
    assert!(!target.contains("[tag : {\"nan\"}, bits : STRING]"));
    assert!(
        !target.contains("[num |->"),
        "no rational stands for the float field: {target}"
    );

    let request = json!({"format": "native-run/1", "request": {
        "models": [{"format": "semantic-ir/2.0.0",
            "source": source("float-model.semantic-ir.json")}],
        "program": {"source": source("unit.native")},
        "export": lean(json!(["NotPayload"]))}});
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    let result = fsl_command::command::run(&path, &crate::negotiation::Fr290)
        .expect("Lean Float64 model export is admitted");
    let value: Value = serde_json::from_str(&result.value.to_string()).expect("JSON document");
    assert_eq!(result.exit_code, 0, "{value}");
    assert_eq!(
        value["export"]["records"][0]["disposition"], "preserved",
        "{value}"
    );
    let target = text(&value);
    assert!(
        target.contains("inductive F where"),
        "Float64 type: {target}"
    );
    assert!(
        target.contains("def F_float64_values"),
        "Float64 domain: {target}"
    );
    assert!(
        target.contains("def F_float64_nan_bits"),
        "canonical NaN bit-pattern predicate: {target}"
    );
    assert!(target.contains("sign ∈ \"7f\".toList"));
    assert!(
        target.contains("F_float64_hex_lower f12"),
        "all payload characters are lowercase hex: {target}"
    );
    assert!(target.contains("F_float64_hex_lower e3"));
    assert!(target.contains("e3 ≠ '0'"));
    assert!(
        target.contains("def F_float64_eq"),
        "Float64 equality: {target}"
    );
    assert!(
        target.contains("def F_float64_neg"),
        "Float64 negation: {target}"
    );
    assert!(
        target.contains("F.nan \"7ff8000000000001\""),
        "exact AC-8 NaN bit-pattern string: {target}"
    );
    assert!(target.contains("F_float64_neg_nan bits"));
}

#[test]
#[trace("TC-3020", "FR-1015-AC-9")]
fn a_malformed_export_member_refuses_with_its_code_and_cause() {
    let mut absent = tla(json!([]));
    absent.as_object_mut().unwrap().remove("obligations");
    let mut kind = tla(json!(["Bounded"]));
    kind["minimum_fidelity"] = json!(7);
    for (member, cause, path) in [
        (
            tla(json!([])),
            "invalid-value",
            "/request/export/obligations",
        ),
        (absent, "missing-member", "/request/export/obligations"),
        (kind, "wrong-value-kind", "/request/export/minimum_fidelity"),
    ] {
        let (exit, value) = counter_export(member);
        assert_eq!(exit, 20, "{value}");
        assert_eq!(value["code"], "invalid_runtime_input", "{value}");
        assert_eq!(value["cause"], cause, "{value}");
        assert_eq!(value["details"]["path"], path, "{value}");
    }
}

#[test]
#[trace("TC-3020", "FR-1015-AC-10")]
#[trace("FSpec-TC-2040", "FSpec-FR-817-AC-13")]
fn an_observer_the_build_cannot_run_is_unsupported() {
    let mut member = tla(json!(["SlowEnough"]));
    member["minimum_fidelity"] = json!("any");
    let (exit, value) = export(BRAKING, false, member.clone());
    assert_eq!(exit, 0, "{value}");
    member["observer"] = json!("tlc");
    member["output"] = json!({"file": "m.tla"});
    let directory = tempfile::tempdir().unwrap();
    let path = braking_request(directory.path(), member);
    let result = fsl_command::command::run(&path, &crate::negotiation::Fr290).expect("a result");
    assert_eq!(result.exit_code, 21);
    let value: Value = serde_json::from_str(&result.value.to_string()).unwrap();
    names(&value, "unsupported_projection");
    names(&value, "unsupported-requested-capability");
    names(&value, "tlc");
    assert!(!value.to_string().contains("invalid-request"), "{value}");
    assert!(value["export"].get("target_text").is_none(), "{value}");
    assert!(value["export"].get("package").is_none(), "{value}");
    assert!(!directory.path().join("m.tla").exists());
    let refusal = &value["export"]["refusal"];
    let reason = refusal["reason"].as_str().expect("a string reason");
    assert!(!reason.is_empty(), "{refusal}");
    assert_eq!(
        refusal,
        &json!({"code": "unsupported_projection", "cause": "unsupported-requested-capability",
            "capability": "observer", "observer": "tlc", "reason": reason,
            "details": {"path": "/request/export/observer"}}),
        "{refusal}"
    );
    let document = outcome_json(&result.outcome.expect("an outcome"));
    assert_eq!(document["last_stage"], Value::Null, "{document}");
    assert_eq!(
        document["diagnostics"][0]["details"]["path"], "/request/export/observer",
        "{document}"
    );
    let mut limited = member_with_observer();
    limited["limits"] = json!({"export.nodes": 1});
    let (exit, value) = export(BRAKING, false, limited);
    assert_eq!(exit, 21, "{value}");
    names(&value, "unsupported-requested-capability");
    let mut unknown = member_with_observer();
    unknown["observer"] = json!("coq");
    let (exit, value) = export(BRAKING, false, unknown);
    assert_eq!(exit, 20, "{value}");
    assert_eq!(value["code"], "invalid_runtime_input", "{value}");
    assert_eq!(value["cause"], "invalid-value", "{value}");
    assert_eq!(
        value["details"]["path"], "/request/export/observer",
        "{value}"
    );
}

fn member_with_observer() -> Value {
    let mut member = tla(json!(["SlowEnough"]));
    member["minimum_fidelity"] = json!("any");
    member["observer"] = json!("tlc");
    member
}

#[test]
#[trace("TC-3020", "FR-1015-AC-11")]
#[trace("FSpec-TC-2040", "FSpec-FR-817-AC-11")]
fn an_incomplete_export_names_its_limit_reached() {
    let mut member = tla(json!(["Bounded"]));
    member["limits"] = json!({"export.work_units": 1});
    let (exit, value) = counter_export(member.clone());
    assert_eq!(exit, 22, "{value}");
    assert_eq!(value["export"]["status"], "incomplete", "{value}");
    assert_eq!(value["export"]["records"], json!([]), "{value}");
    assert_eq!(
        value["export"]["cause"],
        json!({"type": "limit-reached", "limit": "work_units", "value": 1,
            "setting": "export.work_units"}),
        "{value}"
    );
    member["limits"] = json!({"export.nodes": 1, "export.work_units": 1});
    let (exit, value) = counter_export(member);
    assert_eq!(exit, 22, "{value}");
    assert_eq!(
        value["export"]["cause"],
        json!({"type": "limit-reached", "limit": "nodes", "value": 1,
            "setting": "export.nodes"}),
        "{value}"
    );
}

#[test]
#[trace("TC-3020", "FR-1015-AC-12")]
#[trace("FSpec-TC-2040", "FSpec-FR-817-AC-12")]
fn a_shared_closure_node_is_charged_once() {
    let unit = format!(
        "{SPINE_HEADER}invariant Low using v on Config::ConfigVersion at current {{ self.versionNumber >= 0 }}\n\
         invariant High using v on Config::ConfigVersion at current {{ self.versionNumber <= 100 }}\n"
    );
    let least = |obligations: Value| -> u64 {
        (1..=4096)
            .find(|nodes| {
                let mut member = tla(obligations.clone());
                member["limits"] = json!({"export.nodes": nodes});
                let (exit, value) = export(&unit, true, member);
                assert!(exit == 22 || exit == 21, "{exit}: {value}");
                exit != 22
            })
            .expect("the export assembles under some export.nodes")
    };
    let low = least(json!(["Low"]));
    let high = least(json!(["High"]));
    let both = least(json!(["Low", "High"]));
    assert!(both < low + high, "{both} < {low} + {high}");
    assert!(both >= low.max(high), "{both} >= max({low}, {high})");
}

#[test]
#[trace("TC-3020", "FR-1015-AC-13")]
#[trace("FSpec-TC-2040", "FSpec-FR-811-AC-8")]
fn a_cause_names_its_node_by_canonical_text() {
    let mut member = tla(json!(["SlowEnough"]));
    member["minimum_fidelity"] = json!("any");
    let (exit, value) = export(BRAKING, false, member);
    assert_eq!(exit, 0, "{value}");
    let causes = value["export"]["records"][0]["causes"]
        .as_array()
        .expect("causes");
    assert!(!causes.is_empty(), "{value}");
    for cause in causes {
        let node = cause["node"].as_str().expect("a string node");
        fsl_replay::protocol_analysis::read_node_text(node).expect("canonical node text");
        let locus = &cause["locus"];
        assert!(
            locus.is_null()
                || (locus["kind"] == "region" && locus["start"].as_u64() < locus["end"].as_u64()),
            "{cause}"
        );
        for entry in value["export"]["name_map"].as_array().into_iter().flatten() {
            if let Some(text) = entry["node"].as_str() {
                if fsl_replay::protocol_analysis::read_node_text(text).ok()
                    == fsl_replay::protocol_analysis::read_node_text(node).ok()
                {
                    assert_eq!(text, node, "{entry}");
                }
            }
        }
    }
}

#[test]
#[trace("FSpec-TC-2040", "FSpec-FR-811-AC-7")]
#[ignore = "QSL-2873: the mode and jump skeleton of an unrepresented hybrid claim is not lowered yet"]
fn an_unrepresented_claim_still_exports_its_model() {
    let mut member = tla(json!(["SlowEnough"]));
    member["minimum_fidelity"] = json!("any");
    let (exit, value) = export(BRAKING, false, member);
    assert_eq!(exit, 0, "{value}");
    assert_eq!(
        value["export"]["records"][0]["regions"],
        json!([]),
        "{value}"
    );
    let module = text(&value);
    let init = module
        .split("\n\n")
        .find(|definition| definition.contains("Init =="))
        .expect("an Init definition");
    assert!(init.contains("Moving"), "{init}");
    assert!(module.contains("Stopped"), "{module}");
    let entries = value["export"]["name_map"].as_array().expect("a name map");
    assert!(entries.len() >= 2, "{value}");
}
