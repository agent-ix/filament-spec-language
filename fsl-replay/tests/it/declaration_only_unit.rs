// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-2570 to TC-2573 (FR-910, FSpec FR-746): a unit that declares only
//! aliases or dimensions gets one verdict from check and from compile. A
//! node-bearing declaration is written when nothing names it; a package with
//! no node refuses at check as `missing_declaration`/`empty-package`.

use std::collections::BTreeMap;

use filament_exact::Cancel;
use fsl_replay::spine::{self, CompileRefusal, ParseRequest, SpineLimits};
use fsl_replay::{Code, DependencyInput, SourceIdentity};
use ix_trace_rs::trace;
use serde_json::Value;

const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n";

fn refusal(failure: spine::FrontEndFailure) -> Box<CompileRefusal> {
    spine::refusal_or_fault(failure).expect("the front end does not fault on this source")
}

/// The unit list `units` through S1 and S3 check only.
fn check(units: &[&str]) -> Result<spine::CheckedUnit, Box<CompileRefusal>> {
    let source = SourceIdentity::new("a", "u");
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let text = units.concat();
    let parsed = spine::parse(
        &ParseRequest {
            source: &source,
            path: "unit.native",
            bytes: text.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .map_err(refusal)?
    .into_value();
    let models = spine::select(&parsed, &BTreeMap::new(), limits.model, &cancel)
        .map_err(refusal)?
        .into_value();
    Ok(spine::check(
        &parsed,
        "fixture".to_owned(),
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .map_err(refusal)?
    .into_value())
}

/// The unit through check and the emitter: its package bytes.
fn compile(units: &[&str]) -> Result<Vec<u8>, Box<CompileRefusal>> {
    let checked = check(units)?;
    let emitted = spine::package(&checked, spine::PackageLimits::default(), &Cancel::new())
        .map_err(refusal)?
        .into_value();
    Ok(emitted.package().bytes().to_vec())
}

fn strict_reader_error(bytes: &[u8]) -> Option<String> {
    let mut evidence = filament_contract_model::CheckedPackageEvidence::new();
    let wire: Value = serde_json::from_slice(bytes).expect("the package is JSON");
    for feature in wire["lock"]["required_features"]
        .as_array()
        .expect("required features array")
    {
        evidence.support_feature(feature.as_str().expect("feature identity string"));
    }
    match filament_contract_model::read_checked_package(
        bytes,
        filament_contract_model::CheckedPackageReadLimits::bounded(),
        &evidence,
    ) {
        filament_contract_model::CheckedPackageDispatchResult::AdmittedV2(_) => None,
        other => Some(format!("{other:?}")),
    }
}

fn nodes(bytes: &[u8]) -> Vec<Value> {
    let wire: Value = serde_json::from_slice(bytes).expect("the package is JSON");
    wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("a nodes array")
        .clone()
}

fn declares(node: &Value, name: &str) -> bool {
    let qualified = &node["declaration"]["qualified_name"];
    qualified
        .as_str()
        .is_some_and(|qualified| qualified == name || qualified.ends_with(&format!("::{name}")))
        || qualified
            .as_array()
            .and_then(|segments| segments.last())
            .and_then(Value::as_str)
            == Some(name)
}

fn assert_empty_package(refused: &CompileRefusal) {
    assert_eq!(
        refused.code(),
        Code::MissingDeclaration,
        "{refused:?}: {refused}"
    );
    assert_eq!(refused.cause(), Some("empty-package"), "{refused}");
    assert_ne!(refused.code(), Code::UnsupportedProjection);
    assert!(
        !refused.to_string().contains("no node of the package"),
        "the refusal names the rule, not the wire: {refused}"
    );
    // Located at the unit's `language` header declaration: byte 0 to its `;`.
    let header_end = HEADER.find(';').expect("the language declaration ends") as u64 + 1;
    let region = refused.region().expect("the refusal is located");
    assert_eq!((region.start(), region.end()), (0, header_end), "{refused}");
    // It retains the package name and the unit's source identity.
    let retained = format!("{refused:?}");
    assert!(retained.contains("fixture"), "no package name: {retained}");
    assert!(
        retained.contains("authority: \"a\"") && retained.contains("identity: \"u\""),
        "no source identity: {retained}"
    );
}

#[test]
#[trace("TC-2570", "FR-910-AC-1", "FSpec-TC-1810", "FSpec-FR-746-AC-1")]
fn dimension_only_unit_check_and_run_agree() {
    let is_form = |node: &Value, form: &str| {
        node["node_tag"] == "scalar_type" && node["semantic_form"] == form
    };
    let only = [HEADER, "dimension Length;\n"];
    check(&only).expect("a dimension-only unit checks");
    let bytes = compile(&only).expect("a dimension-only unit compiles");
    assert_eq!(strict_reader_error(&bytes), None, "strict-read failed");
    let dimensions: Vec<Value> = nodes(&bytes)
        .into_iter()
        .filter(|node| is_form(node, "dimension"))
        .collect();
    assert_eq!(dimensions.len(), 1);
    assert!(declares(&dimensions[0], "Length"));
    assert_eq!(
        dimensions[0]["body"]["base"], true,
        "Length is a base dimension"
    );

    let with_unit = [
        HEADER,
        "dimension Length;\nunit m : Length = rational(1, 1);\n",
    ];
    check(&with_unit).expect("a dimension-and-unit unit checks");
    let bytes = compile(&with_unit).expect("a dimension-and-unit unit compiles");
    assert_eq!(strict_reader_error(&bytes), None, "strict-read failed");
    let graph = nodes(&bytes);
    let length = graph
        .iter()
        .find(|node| is_form(node, "dimension") && declares(node, "Length"))
        .expect("the Length dimension node");
    let units: Vec<&Value> = graph.iter().filter(|node| is_form(node, "unit")).collect();
    assert_eq!(units.len(), 1);
    assert!(declares(units[0], "m"));
    assert_eq!(units[0]["body"]["dimension_node_id"], length["node_id"]);
}

#[test]
#[trace("TC-2571", "FR-910-AC-2", "FSpec-TC-1811", "FSpec-FR-746-AC-2")]
fn alias_only_unit_check_and_run_agree() {
    for units in [
        [HEADER, "type I = Integer;\n"],
        [HEADER, "type I = Integer;\ntype J = Boolean;\n"],
        [HEADER, ""],
    ] {
        let checked = check(&units).expect_err("a package with no node refuses at check");
        assert_empty_package(&checked);
        let compiled = compile(&units).expect_err("compile refuses as check does");
        assert_empty_package(&compiled);
        assert_eq!(compiled.code(), checked.code());
        assert_eq!(compiled.cause(), checked.cause());
    }
}

#[test]
#[trace("TC-2572", "FR-910-AC-3", "FSpec-TC-1812", "FSpec-FR-746-AC-3")]
fn mixed_alias_and_dimension_unit_agrees() {
    let units = [HEADER, "type I = Integer;\ndimension Length;\n"];
    check(&units).expect("an alias and a dimension check");
    let bytes = compile(&units).expect("an alias and a dimension compile");
    assert!(
        nodes(&bytes).iter().any(|node| declares(node, "Length")),
        "emitted nodes: {:?}",
        nodes(&bytes)
    );
    assert!(!nodes(&bytes).iter().any(|node| declares(node, "I")));
}

#[test]
#[trace("TC-2572", "FR-910-AC-3", "FSpec-TC-1812", "FSpec-FR-746-AC-3")]
fn alias_plus_function_still_compiles() {
    let bytes = compile(&[
        HEADER,
        "type I = Integer;\nfunction f(x: I): I pure { x }\n",
    ])
    .expect("an alias beside a function compiles");
    assert!(
        nodes(&bytes).iter().any(|node| declares(node, "f")),
        "emitted nodes: {:?}",
        nodes(&bytes)
    );
    assert!(!nodes(&bytes).iter().any(|node| declares(node, "I")));
}

#[test]
#[trace("TC-2573", "FR-910-AC-4", "FSpec-TC-1812", "FSpec-FR-746-AC-4")]
fn alias_only_unit_resolves_in_another_unit_and_refuses_alone() {
    let alias = "type Port = Integer;\n";
    let function = "function f(p: Port): Port pure { p }\n";
    let parse_unit = |identity: &str, body: &str| {
        let text = format!("{HEADER}{body}");
        spine::parse(
            &ParseRequest {
                source: &SourceIdentity::new("a", identity),
                path: "unit.native",
                bytes: text.as_bytes(),
            },
            SpineLimits::default().source,
            &Cancel::new(),
        )
        .expect("the unit parses")
        .into_value()
    };
    for reverse in [false, true] {
        let mut list = vec![
            parse_unit("aliases", alias),
            parse_unit("functions", function),
        ];
        if reverse {
            list.reverse();
        }
        let dependencies = DependencyInput::default();
        let admitted =
            spine::admit_parsed_unit_list(list, &dependencies).expect("the unit list is admitted");
        let limits = SpineLimits::default();
        let models = spine::select_units(
            &admitted,
            &BTreeMap::new(),
            limits.model,
            limits.source,
            &Cancel::new(),
        )
        .expect("no model is selected")
        .into_value();
        spine::check_units(
            &admitted,
            "fixture".to_owned(),
            &models,
            &dependencies,
            limits,
            &Cancel::new(),
        )
        .expect("the alias resolves in the other unit, in either order");
    }
    assert_empty_package(&check(&[HEADER, alias]).expect_err("the alias unit alone refuses"));
}
