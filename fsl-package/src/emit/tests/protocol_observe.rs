// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-2991: the checked package records an observe-only boundary as the last
//! body argument `boundary` of the marked role and control nodes, and a
//! `recommend` as a control of operator `recommend` (FR-383-AC-69; FSpec
//! FR-322, FR-465, FR-466, FR-822). IR's strict reader admits the package.

use super::admission_corpus::config_version_unit;
use super::*;
use filament_semantic_value::checking::CheckMode;

fn domain_document() -> &'static [u8] {
    config_version_unit::DOMAIN_PACKAGE.as_bytes()
}

/// `declarations` over the ConfigVersion model, emitted and read back by IR's
/// strict reader; returns the wire.
fn admitted(declarations: &str) -> Value {
    let packages = fsl_semantics::model::intake::package_input([domain_document()]).unwrap();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         profile p = \"filament.protocol.finite-global/v1\";\n\
         model Config = \"example/config-version\";\n{declarations}\n"
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckMode::Linked, CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("{unit}\n{refusals:?}"));
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
        .insert_domain_package_document(
            package_identity(domain_document()),
            domain_document().to_vec(),
        )
        .expect("a fresh evidence key");
    crate::checked_v2::read_checked_package_v2(
        emission.package.bytes(),
        library(),
        V2ReadLimits::fixture(),
        &evidence,
    )
    .unwrap_or_else(|failure| panic!("the I2 read admits the package: {failure:?}"));
    wire
}

fn flow(observe: &str) -> String {
    format!(
        "protocol Flow using p over (input: Config::ConfigVersion) on origin {{\n\
         {observe}role Auditor on Config::ConfigVersion;\n\
         run {observe}sequence Review {{\n\
         check Gate using v {{ true }};\n\
         recommend Advise by Auditor on Config::ConfigVersion::attemptUpdate \
         contracts [] as (r: Boolean) {{ true }};\n\
         }}\n\
         finish End as (outcome: Boolean) {{ true }};\n}}"
    )
}

fn node<'w>(wire: &'w Value, path: &[&str]) -> &'w Value {
    let found: Vec<&Value> = nodes(wire)
        .iter()
        .filter(|node| node["declaration"]["qualified_name"] == json!(path))
        .collect();
    let [node] = found[..] else {
        panic!("one node {path:?}: {found:?}");
    };
    node
}

fn argument_names(node: &Value) -> Vec<&str> {
    node["body"]["arguments"]
        .as_array()
        .expect("arguments")
        .iter()
        .map(|argument| argument["name"].as_str().expect("a name"))
        .collect()
}

/// FR-383-AC-69: exactly the `Auditor` role and the `Review` control carry
/// the last argument `boundary` `observe-only`; `Advise` is a `recommend`
/// control with FSpec FR-466's layout; the unmarked clause has the same node
/// ids and no `boundary` argument.
#[trace("TC-2991", "FR-383-AC-69", "FSpec-TC-2018", "FSpec-FR-822-AC-4", "FSpec-FR-322-AC-462")]
#[ignore = "QSL-2211: S4 emission of observe-only boundaries and recommend is not implemented yet"]
#[test]
fn the_package_records_the_observe_boundary() {
    let marked = admitted(&flow("observe "));
    for path in [&["Flow", "role", "Auditor"][..], &["Flow", "run", "Review"][..]] {
        let marked_node = node(&marked, path);
        let arguments = marked_node["body"]["arguments"].as_array().unwrap();
        let last = arguments.last().expect("an argument");
        assert_eq!(last["name"], "boundary", "{path:?}");
        assert_eq!(last["value"]["value"], "observe-only", "{path:?}");
    }
    let marked_count = nodes(&marked)
        .iter()
        .filter(|node| argument_names(node).contains(&"boundary"))
        .count();
    assert_eq!(marked_count, 2, "only the two marked nodes carry a boundary");

    let advise = node(&marked, &["Flow", "run", "Review", "Advise"]);
    assert_eq!(advise["semantic_form"], "control");
    assert_eq!(
        advise["body"]["operation"]["member"]["operator"],
        "recommend"
    );
    assert_eq!(
        argument_names(advise),
        ["role", "operation", "contracts", "binder", "related", "constraint"]
    );

    let plain = admitted(&flow(""));
    assert!(nodes(&plain)
        .iter()
        .all(|node| !argument_names(node).contains(&"boundary")));
    let ids = |wire: &Value| -> Vec<Value> {
        nodes(wire).iter().map(|node| node["node_id"].clone()).collect()
    };
    assert_eq!(ids(&marked), ids(&plain));
}
