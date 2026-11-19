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
/// strict reader; returns both the wire and the admitted package.
fn admitted_with_package(
    declarations: &str,
) -> (
    Value,
    std::sync::Arc<filament_contract_model::CheckedPackageV2>,
) {
    let packages = fsl_semantics::model::intake::package_input([domain_document()]).unwrap();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         profile p = \"filament.protocol.finite-global/v1\";\n\
         model Config = \"example/config-version\";\n{declarations}\n"
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
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
    let checked = crate::checked_v2::read_checked_package_v2(
        emission.package.bytes(),
        library(),
        V2ReadLimits::fixture(),
        &evidence,
    )
    .unwrap_or_else(|failure| panic!("the I2 read admits the package: {failure:?}"))
    .into_value()
    .admitted;
    (wire, checked)
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
        .into_iter()
        .flatten()
        .filter_map(|argument| argument["name"].as_str())
        .collect()
}

/// FR-383-AC-69: exactly the `Auditor` role and the `Review` control carry
/// the last argument `boundary` `observe-only`; `Advise` is a `recommend`
/// control with FSpec FR-466's layout; the unmarked clause has the same node
/// ids and no `boundary` argument.
#[trace(
    "TC-2991",
    "FR-383-AC-69",
    "FSpec-TC-2018",
    "FSpec-FR-822-AC-4",
    "FSpec-FR-322-AC-462"
)]
#[test]
fn the_package_records_the_observe_boundary() {
    let (marked, checked_marked) = admitted_with_package(&flow("observe "));
    let role_id: filament_contract_model::CheckedNodeId =
        serde_json::from_value(node(&marked, &["Flow", "role", "Auditor"])["node_id"].clone())
            .expect("the emitted role node has a checked identity");
    assert_eq!(
        checked_marked.protocol_role_boundary(&role_id),
        Some(filament_contract_model::CheckedProtocolBoundary::ObserveOnly)
    );
    for path in [
        &["Flow", "role", "Auditor"][..],
        &["Flow", "run", "Review"][..],
    ] {
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
    assert_eq!(
        marked_count, 2,
        "only the two marked nodes carry a boundary"
    );

    let advise = node(&marked, &["Flow", "run", "Review", "Advise"]);
    assert_eq!(advise["semantic_form"], "control");
    assert_eq!(
        advise["body"]["operation"]["member"]["operator"],
        "recommend"
    );
    assert_eq!(
        argument_names(advise),
        [
            "role",
            "operation",
            "contracts",
            "binder",
            "related",
            "constraint"
        ]
    );

    let (plain, checked_plain) = admitted_with_package(&flow(""));
    let plain_role_id: filament_contract_model::CheckedNodeId =
        serde_json::from_value(node(&plain, &["Flow", "role", "Auditor"])["node_id"].clone())
            .expect("the unmarked role node has a checked identity");
    assert_eq!(checked_plain.protocol_role_boundary(&plain_role_id), None);
    assert!(
        nodes(&plain)
            .iter()
            .all(|node| !argument_names(node).contains(&"boundary"))
    );
    let ids = |wire: &Value| -> Vec<Value> {
        nodes(wire)
            .iter()
            .map(|node| node["node_id"].clone())
            .collect()
    };
    assert_eq!(ids(&marked), ids(&plain));

    let changed_ids = [
        node(&marked, &["Flow", "role", "Auditor"])["node_id"].to_string(),
        node(&marked, &["Flow", "run", "Review"])["node_id"].to_string(),
    ];
    for (marked_node, plain_node) in nodes(&marked).iter().zip(nodes(&plain)) {
        assert_eq!(marked_node["node_id"], plain_node["node_id"]);
        let id = marked_node["node_id"].to_string();
        if changed_ids.contains(&id) {
            let mut body = marked_node["body"].clone();
            let arguments = body["arguments"]
                .as_array_mut()
                .expect("the marked body has arguments");
            let boundary = arguments.pop().expect("the boundary is the last argument");
            assert_eq!(boundary["name"], "boundary");
            assert_eq!(boundary["value"]["value"], "observe-only");
            assert_eq!(
                body, plain_node["body"],
                "only the boundary differs at {id}"
            );
        } else {
            assert_eq!(
                marked_node["body"], plain_node["body"],
                "unmarked node body is unchanged for {id}"
            );
        }
    }
}
