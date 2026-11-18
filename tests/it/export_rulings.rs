// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1015 AC-5 to AC-8, FR-1035 and FSpec FR-811-AC-6, FR-812-AC-6/AC-7,
//! FR-814-AC-6 and FR-817-AC-8 to AC-10: these cover the single
//! unrepresentable cause, obligation names, the stop order, the generator,
//! the `fsl:` comment spelling, population bounds and `Init`.

use crate::plan_export_surface::counter_export;
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
#[ignore = "QSL-1979: lowering of every representable construct and the located target-unrepresentable cause are not implemented yet"]
fn only_what_the_target_cannot_state_is_unrepresented() {
    let (exit, value) = counter_export(tla(json!(["Bounded"])));
    assert_eq!(exit, 0, "{value}");
    let record = &value["export"]["records"][0];
    assert_eq!(record["disposition"], "preserved", "{value}");
    assert_eq!(record["causes"], json!([]), "{value}");
    let mut member = tla(json!(["SlowEnough"]));
    member["minimum_fidelity"] = json!("any");
    let (exit, value) = export(BRAKING, false, member);
    assert_eq!(exit, 0, "{value}");
    let record = &value["export"]["records"][0];
    assert_eq!(record["disposition"], "unrepresented", "{value}");
    let causes = record["causes"].as_array().expect("causes");
    assert!(!causes.is_empty(), "{value}");
    for cause in causes {
        assert_eq!(cause["kind"], "target-unrepresentable", "{value}");
        assert_eq!(cause["target"], "tla", "{value}");
        assert!(
            cause["construct"].as_str().is_some_and(|c| c.contains('/')),
            "{value}"
        );
        assert_ne!(cause["node"], "SlowEnough", "{value}");
        assert_eq!(cause["locus"]["kind"], "region", "{value}");
    }
}

#[test]
#[trace("TC-3002", "FR-1015-AC-6")]
#[trace("FSpec-TC-2032", "FSpec-FR-817-AC-8")]
#[trace("FSpec-TC-2032", "FSpec-FR-817-AC-9")]
#[ignore = "QSL-1979: the nonempty obligation set and not-an-obligation records are not implemented yet"]
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
    let (exit, value) = counter_export(tla(json!(["CanInc", "Bounded", "Nope"])));
    let records = value["export"]["records"].as_array().expect("records");
    let dispositions: Vec<_> = records.iter().map(|r| r["disposition"].clone()).collect();
    assert_eq!(
        dispositions,
        [json!("refused"), json!("preserved"), json!("refused")],
        "{value}"
    );
    names(&records[0], "not-an-obligation");
    names(&records[2], "not-an-obligation");
    assert_eq!(exit, 21, "{value}");
}

#[test]
#[trace("TC-3002", "FR-1015-AC-7")]
#[trace("FSpec-TC-2032", "FSpec-FR-817-AC-10")]
#[ignore = "QSL-1979: the floor-before-size stop order is not implemented yet"]
fn the_floor_is_decided_before_the_text_is_charged() {
    let mut member = tla(json!(["SlowEnough"]));
    member["limits"] = json!({"export.output_bytes": 1});
    let (exit, value) = export(BRAKING, false, member);
    assert_eq!(exit, 21, "{value}");
    names(&value["export"], "below-fidelity-floor");
    let mut member = tla(json!(["Bounded"]));
    member["limits"] = json!({"export.output_bytes": 1});
    let (exit, value) = counter_export(member);
    assert_eq!(exit, 22, "{value}");
    names(&value["export"], "export.output_bytes");
    let mut member = tla(json!(["SlowEnough"]));
    member["limits"] = json!({"export.nodes": 1});
    let (exit, value) = export(BRAKING, false, member);
    assert_eq!(exit, 22, "{value}");
    names(&value["export"], "export.nodes");
}

#[test]
#[trace("TC-3002", "FR-1015-AC-8")]
#[ignore = "QSL-1979: the application generator identity is not implemented yet"]
fn the_generator_is_the_application() {
    let (_, value) = counter_export(tla(json!(["Bounded"])));
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
#[ignore = "QSL-1979: the canonical fsl: region comment is not implemented yet"]
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
#[ignore = "QSL-1979: population bounds from bound declarations are not exposed to the export yet"]
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
#[ignore = "QSL-1979: Init from declared initial constraints is not implemented yet"]
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
#[ignore = "QSL-1979: CheckedGraph::population_bound and the bound's [N] region are not implemented yet"]
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
    let conjunct = lines[type_ok..]
        .iter()
        .position(|line| line.contains("Cardinality(DOMAIN v_") && line.contains(") <= 8"))
        .map(|offset| type_ok + offset)
        .expect("the bound's conjunct inside TypeOK");
    let comment = lines[conjunct - 1].trim_start();
    assert!(
        comment.starts_with("\\* fsl: "),
        "the conjunct's comment: {comment}"
    );
    assert!(comment.contains(&region), "{comment} names {region}");
}
