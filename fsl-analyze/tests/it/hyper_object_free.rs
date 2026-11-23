// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-488-AC-28 (TC-3010), FSpec FR-402-AC-11 (TC-2034): the body wrapper of a
//! hyper clause with no atom slot ranges over the `model`/`model_root` node of
//! its first subject, the strict reader admits it, and refuses a model-root
//! `over` everywhere else.

use filament_contract_model::{
    CheckedPackageEvidence, CheckedPackageReadLimits, CheckedPackageRefusalCause,
    CheckedPackageRefusalCode, CheckedPackageV2, CheckedPackageV2ReadResult, JsonPointer,
};
use fsl_semantics::model::intake::package_input;
use ix_trace_rs::trace;
use serde_json::Value;

use crate::hyper_world::{clause_documents, non_interference, package_with_clause, Vault};

const WHOLE: &str = "hyper Whole using inf over behaviours () \
                     forall trace a of V forall trace b of V { always holds(true) }";

/// The emitted bytes and their resolved wire, in which a node id names graph
/// positions.
fn emitted(package: &fsl_package::CheckedPackage) -> (Vec<u8>, Value) {
    let emission = fsl_package::emit_checked(package, fsl_package::EmitLimits::fixture())
        .unwrap_or_else(|refusal| panic!("S4 emits the hyper clause: {refusal:?}"));
    let bytes = emission.package().bytes().to_vec();
    let mut wire = serde_json::from_slice(&bytes).expect("wire JSON");
    fsl_semantics::check::wire_id::resolve_operand_slots(&mut wire)
        .expect("every operand slot names a row entry");
    (bytes, wire)
}

fn nodes(wire: &Value) -> &Vec<Value> {
    wire["semantic_graph"]["nodes"].as_array().expect("nodes")
}

fn node<'a>(wire: &'a Value, id: &Value) -> &'a Value {
    nodes(wire)
        .iter()
        .find(|node| &node["node_id"] == id)
        .unwrap_or_else(|| panic!("a node {id}"))
}

/// The node id the one hyperproperty node's `body` member references, in the
/// form of `wire`.
fn wrapper_id(wire: &Value) -> Value {
    let hyper = nodes(wire)
        .iter()
        .find(|node| node["semantic_form"] == "hyperproperty")
        .expect("a hyperproperty node");
    let body = hyper["body"]["members"]
        .as_array()
        .expect("members")
        .iter()
        .find(|member| member["name"] == "body")
        .expect("a body member");
    body["value"]["target"].clone()
}

/// The `over` target of the body wrapper of the one hyperproperty node, and
/// the wrapper's dependencies.
fn wrapper_over(wire: &Value) -> (Value, Value) {
    let wrapper = node(wire, &wrapper_id(wire));
    let over = wrapper["body"]["arguments"][0]["target"].clone();
    (over, wrapper["dependencies"].clone())
}

/// The strict read of `bytes`, with the domain `documents` the package was
/// checked against.
fn read(bytes: &[u8], documents: &[Vec<u8>]) -> CheckedPackageV2ReadResult {
    let wire: Value = serde_json::from_slice(bytes).expect("wire JSON");
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    for (identity, domain) in
        package_input(documents.iter().map(Vec::as_slice)).expect("the vault documents")
    {
        evidence
            .insert_domain_package_document(identity.as_str(), domain)
            .expect("a fresh evidence key");
    }
    CheckedPackageV2::read(bytes, CheckedPackageReadLimits::bounded(), &evidence)
}

/// Edit the wrapper `wrapper` (a node id of the raw wire in `bytes`) so its
/// `over` is the `model_root` of `root_package`, and assert the strict read
/// refuses it `invalid_model_binding`/`malformed-declaration` at the wrapper's
/// first argument.
fn assert_root_over_refused(
    bytes: &[u8],
    documents: &[Vec<u8>],
    wrapper: &Value,
    root_package: &str,
) {
    let mut raw: Value = serde_json::from_slice(bytes).expect("wire JSON");
    let nodes = raw["semantic_graph"]["nodes"]
        .as_array_mut()
        .expect("nodes");
    let root = nodes
        .iter()
        .find(|node| {
            node["semantic_form"] == "model_root" && node["node_id"]["package"] == root_package
        })
        .unwrap_or_else(|| panic!("a model root of {root_package}"))["node_id"]
        .clone();
    let at = nodes
        .iter()
        .position(|node| &node["node_id"] == wrapper)
        .expect("the wrapper");
    nodes[at]["body"]["arguments"][0]["target"] = root.clone();
    nodes[at]["dependencies"]
        .as_array_mut()
        .expect("dependencies")
        .push(root);
    let edited = serde_json::to_vec(&raw).expect("bytes");
    let CheckedPackageV2ReadResult::Refused(refusal) = read(&edited, documents) else {
        panic!("a model-root over on this wrapper is refused");
    };
    assert_eq!(refusal.code, CheckedPackageRefusalCode::InvalidModelBinding);
    assert_eq!(
        refusal.cause,
        Some(CheckedPackageRefusalCause::MalformedDeclaration)
    );
    assert_eq!(
        refusal.loci[0].path,
        JsonPointer::parse(&format!("/semantic_graph/nodes/{at}/body/arguments/0"))
    );
}

#[trace("TC-3010", "FR-488-AC-28")]
#[trace("FSpec-TC-2034", "FSpec-FR-402-AC-11")]
#[test]
fn an_object_free_hyper_wrapper_ranges_over_the_model_root() {
    let package = package_with_clause(Vault::Secure, false, WHOLE);
    let (bytes, wire) = emitted(&package);
    let (over, dependencies) = wrapper_over(&wire);
    let target = node(&wire, &over);
    assert_eq!(
        (
            target["node_tag"].as_str(),
            target["semantic_form"].as_str()
        ),
        (Some("model"), Some("model_root"))
    );
    assert!(dependencies
        .as_array()
        .is_some_and(|deps| deps.contains(&over)));
    assert!(
        matches!(
            read(&bytes, &clause_documents(Vault::Secure, false)),
            CheckedPackageV2ReadResult::Admitted(_)
        ),
        "the strict reader admits the package"
    );
}

#[trace("TC-3010", "FR-488-AC-28")]
#[trace("FSpec-TC-2034", "FSpec-FR-402-AC-11")]
#[test]
fn a_model_root_over_on_a_wrapper_that_reads_a_slot_is_refused() {
    let package = package_with_clause(Vault::Secure, false, &non_interference(""));
    let (bytes, wire) = emitted(&package);
    let (over, _) = wrapper_over(&wire);
    assert_eq!(
        node(&wire, &over)["body"]["members"][0]["value"]["value"],
        "slot_sv_sa"
    );
    let raw: Value = serde_json::from_slice(&bytes).expect("wire JSON");
    assert_root_over_refused(
        &bytes,
        &clause_documents(Vault::Secure, false),
        &wrapper_id(&raw),
        "test/vault",
    );
}

#[trace("TC-3010", "FR-488-AC-28")]
#[trace("FSpec-TC-2034", "FSpec-FR-402-AC-11")]
#[test]
fn a_wrapper_over_the_root_of_a_package_no_subject_names_is_refused() {
    let package = package_with_clause(Vault::Secure, true, WHOLE);
    let (bytes, _) = emitted(&package);
    let raw: Value = serde_json::from_slice(&bytes).expect("wire JSON");
    assert_root_over_refused(
        &bytes,
        &clause_documents(Vault::Secure, true),
        &wrapper_id(&raw),
        "test/vault-other",
    );
}

#[trace("TC-3010", "FR-488-AC-28")]
#[trace("FSpec-TC-2034", "FSpec-FR-402-AC-11")]
#[test]
fn a_single_trace_clause_that_is_no_body_wrapper_cannot_range_over_a_model_root() {
    let package = package_with_clause(
        Vault::Secure,
        false,
        "temporal Plain using inf over (v: V::Vault) clock \"model-steps\" on origin \
         { always holds(v.l >= 0) }",
    );
    let (bytes, _) = emitted(&package);
    let raw: Value = serde_json::from_slice(&bytes).expect("wire JSON");
    let [clause] = &nodes(&raw)
        .iter()
        .filter(|node| node["semantic_form"] == "temporal_clause")
        .collect::<Vec<_>>()[..]
    else {
        panic!("one temporal clause");
    };
    assert_root_over_refused(
        &bytes,
        &clause_documents(Vault::Secure, false),
        &clause["node_id"],
        "test/vault",
    );
}
