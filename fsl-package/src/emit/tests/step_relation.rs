// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-180 / FSpec FR-477 (TC-1143): an HP-1 relation is emitted as a typed
//! step-relation node and admitted by the complete FSL I2 reader.

use super::admission_corpus::config_version_unit;
use super::*;
use filament_semantic_value::checking::CheckMode;

pub(super) fn document() -> &'static [u8] {
    config_version_unit::DOMAIN_PACKAGE.as_bytes()
}

fn relation_unit() -> String {
    String::from("language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         profile t = \"filament.temporal.infinite-trace/v1\";\n\
         model Config = \"example/config-version\";\n\
         relation Det using t over (x: Config::ConfigVersion::attemptUpdate, y: Config::ConfigVersion::attemptUpdate) \
         { x.result = y.result and pre(x.self.versionNumber) = pre(y.self.versionNumber) implies x.self.versionNumber = y.self.versionNumber }\n")
}

pub(super) fn checked_graph() -> fsl_semantics::check::CheckedGraph {
    let packages = fsl_semantics::model::intake::package_input([document()]).unwrap();
    assemble_with_models(relation_unit().as_bytes(), &packages)
        .expect("the unit assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("the unit checks: {refusals:?}"))
}

fn checked_bound_graph() -> fsl_semantics::check::CheckedGraph {
    let packages = fsl_semantics::model::intake::package_input([document()]).unwrap();
    let unit = format!(
        "{}\
         abstraction A using v {{\n\
         population Config::config_history to \"store::history\";\n\
         operation Config::ConfigVersion::attemptUpdate to \"store::update\" receiver \"self\" ();\n\
         }}\n",
        relation_unit()
    );
    assemble_with_models(unit.as_bytes(), &packages)
        .expect("the bound unit assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("the bound unit checks: {refusals:?}"))
}

fn checked_bound_graph_with_state_clauses() -> fsl_semantics::check::CheckedGraph {
    let packages = fsl_semantics::model::intake::package_input([document()]).unwrap();
    let unit = format!(
        "{}\
         invariant ConfigInvariant using v on Config::ConfigVersion at current {{ true }}\n\
         pre UpdatePre using v on Config::ConfigVersion::attemptUpdate {{ true }}\n\
         post UpdatePost using v on Config::ConfigVersion::attemptUpdate {{ true }}\n\
         abstraction A using v {{\n\
         population Config::config_history to \"store::history\";\n\
         operation Config::ConfigVersion::attemptUpdate to \"store::update\" receiver \"self\" ();\n\
         }}\n",
        relation_unit()
    );
    assemble_with_models(unit.as_bytes(), &packages)
        .expect("the bound unit assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("the bound unit checks: {refusals:?}"))
}

/// FR-180/FR-477: both execution binders remain distinct, their body is
/// represented in the checked wire, and I2 admits the emitted package.
#[trace(
    "TC-1143",
    "FR-180-AC-2",
    "FR-180-AC-3",
    "FR-940-AC-23"
)]
#[test]
fn a_step_relation_emits_and_reads_back_its_execution_body() {
    let graph = checked_graph();
    assert_eq!(graph.step_relations().len(), 1);
    assert_eq!(graph.requirements().len(), 1);
    assert_eq!(
        graph
            .requirements()
            .values()
            .next()
            .expect("the model claim record")
            .requirements()
            .kind(),
        fsl_semantics::check::Capability::TemporalSatisfaction
    );
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    let relation = nodes(&wire)
        .iter()
        .find(|node| node["semantic_form"] == "step_relation")
        .expect("the step-relation node is present");
    assert_eq!(relation["node_tag"], "claim");
    let body_members = relation["body"]["members"].as_array().unwrap();
    assert_eq!(
        body_members[..4]
            .iter()
            .map(|member| member["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["subjects", "executions", "invariants", "body"]
    );
    let executions = relation["body"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|member| member["name"] == "executions")
        .unwrap();
    let executions = executions["value"]["members"].as_array().unwrap();
    assert_eq!(executions.len(), 2);
    let parameter_references: Vec<_> = executions
        .iter()
        .map(|execution| {
            let parameters = execution["value"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .find(|member| member["name"] == "parameters")
                .unwrap()["value"]["members"]
                .as_array()
                .unwrap();
            assert_eq!(parameters.len(), 2);
            for member in execution["value"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|member| member["name"] == "implementation")
            {
                assert!(member["value"]["members"].as_array().unwrap().is_empty());
            }
            parameters
                .iter()
                .map(|parameter| parameter["target"].clone())
                .collect::<Vec<_>>()
        })
        .collect();
    assert_ne!(parameter_references[0][0], parameter_references[1][0]);
    assert_ne!(parameter_references[0][1], parameter_references[1][1]);
    let level_for = |target: &serde_json::Value| {
        let parameter = nodes(&wire)
            .iter()
            .find(|node| node["node_id"] == *target)
            .expect("the execution parameter node is emitted");
        parameter["body"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|member| member["name"] == "level")
            .unwrap()["value"]["value"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    assert_eq!(
        parameter_references[0]
            .iter()
            .map(level_for)
            .collect::<Vec<_>>(),
        ["0", "1"]
    );
    assert_eq!(
        parameter_references[1]
            .iter()
            .map(level_for)
            .collect::<Vec<_>>(),
        ["2", "3"]
    );
    let relation_body = body_members
        .iter()
        .find(|member| member["name"] == "body")
        .unwrap();
    fn contains(
        value: &serde_json::Value,
        target: &serde_json::Value,
        wire: &serde_json::Value,
        visited: &mut std::collections::BTreeSet<String>,
    ) -> bool {
        if value == target {
            return true;
        }
        if value["term"] == "reference" {
            let reference = &value["target"];
            if reference == target {
                return true;
            }
            if let Some(node) = nodes(wire).iter().find(|node| node["node_id"] == *reference) {
                let identity = serde_json::to_string(reference).expect("node id encodes");
                if visited.insert(identity) && contains(&node["body"], target, wire, visited) {
                    return true;
                }
            }
        }
        value.as_array().is_some_and(|values| {
            values
                .iter()
                .any(|value| contains(value, target, wire, visited))
        }) || value.as_object().is_some_and(|values| {
            values
                .values()
                .any(|value| contains(value, target, wire, visited))
        })
    }
    for execution in &parameter_references {
        for parameter in execution {
            assert!(
                contains(
                    &relation_body["value"],
                    parameter,
                    &wire,
                    &mut std::collections::BTreeSet::new(),
                ),
                "the body reads its own execution binder {parameter}; body={}",
                relation_body["value"]
            );
        }
    }
    let parameter_levels: Vec<_> = nodes(&wire)
        .iter()
        .filter(|node| node["semantic_form"] == "parameter")
        .filter_map(|node| node["body"]["members"].as_array())
        .filter_map(|members| {
            members
                .iter()
                .find(|member| member["name"] == "level")
                .and_then(|member| member["value"]["value"].as_str())
        })
        .collect();
    assert_eq!(
        parameter_levels
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>(),
        ["0", "1", "2", "3"].into_iter().collect()
    );
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
        .insert_domain_package_document(package_identity(document()), document().to_vec())
        .expect("the fixture evidence is unique");
    let admitted = crate::checked_v2::read_checked_package_v2(
        emission.package.bytes(),
        library(),
        V2ReadLimits::fixture(),
        &evidence,
    )
    .unwrap_or_else(|failure| panic!("I2 admits the step relation: {failure:?}"))
    .into_value()
    .admitted;
    let typed = admitted
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| node.semantic_form.as_ref() == "step_relation")
        .expect("the reader retains the relation node");
    let typed_body = typed.step_relation_body().expect("the typed relation body");
    assert_eq!(typed_body.executions.len(), 2);
    assert_eq!(typed_body.executions[0].variable.as_ref(), "x");
    assert_eq!(typed_body.executions[1].variable.as_ref(), "y");
    use filament_contract_model::{
        CapabilityKind, DomainKey, DomainKind, RequirementExtent, RequirementItemRow,
    };
    assert_eq!(
        admitted.requirement_items(),
        &[RequirementItemRow {
            claim: typed.node_id.clone(),
            occurrence: CheckedOccurrence {
                role: CheckedOccurrenceRole::Claim,
                ordinal: 0
            },
            kind: CapabilityKind::TemporalSatisfaction,
            extent: RequirementExtent::Unbounded(vec![(
                DomainKey::Node {
                    node: typed.node_id.clone(),
                    path: Vec::new()
                },
                DomainKind::InfiniteTrace
            )]),
            record: None,
        }]
    );
}

/// FSL FR-507-AC-7, FSpec FR-477/FR-485: a bound relation has one claim
/// occurrence shared by its model and code capability records, plus one
/// declaration occurrence; Contract IR admits the emitted package.
#[trace("FR-507-AC-7", "FSpec-FR-477", "FSpec-FR-485")]
#[test]
fn one_step_relation_claim_occurrence_is_admitted_by_ir() {
    use fsl_semantics::check::Capability;

    let graph = checked_bound_graph();
    assert_eq!(graph.requirements().len(), 2);
    let [relation] = graph.step_relations() else {
        panic!("one step relation");
    };
    let claim = relation.occurrence().expect("the model claim occurrence");
    assert_eq!(
        graph
            .requirements()
            .get_kind(claim, Capability::TemporalSatisfaction)
            .expect("the model claim record")
            .requirements()
            .kind(),
        Capability::TemporalSatisfaction
    );
    let model_identity = graph
        .requirements()
        .get_kind(claim, Capability::TemporalSatisfaction)
        .expect("the model obligation")
        .requirements();
    let code_identity = graph
        .requirements()
        .get_kind(claim, Capability::OperationContract)
        .expect("the code obligation")
        .requirements();
    assert_ne!(model_identity, code_identity);
    assert_eq!(
        graph
            .requirements()
            .get_kind(claim, Capability::OperationContract)
            .expect("the code claim record")
            .requirements()
            .kind(),
        Capability::OperationContract
    );
    let claim_origins: Vec<_> = graph
        .occurrences()
        .filter(|(node, origin, _)| node == claim.node() && origin.role().as_str() == "claim")
        .collect();
    let declaration_origins: Vec<_> = graph
        .occurrences()
        .filter(|(node, origin, _)| node == claim.node() && origin.role().as_str() == "declaration")
        .collect();
    assert_eq!(claim_origins.len(), 1);
    assert_eq!(declaration_origins.len(), 1);

    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    let wire = wire(&emission);
    let relation_node = nodes(&wire)
        .iter()
        .find(|node| node["semantic_form"] == "step_relation")
        .expect("the relation node is present");
    let execution_members = relation_node["body"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|member| member["name"] == "executions")
        .unwrap()["value"]["members"]
        .as_array()
        .unwrap();
    assert_eq!(execution_members.len(), 2);
    let implementation_references: Vec<_> = execution_members
        .iter()
        .map(|execution| {
            execution["value"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .find(|member| member["name"] == "implementation")
                .unwrap()["value"]["target"]
                .clone()
        })
        .collect();
    assert_eq!(implementation_references[0], implementation_references[1]);
    assert!(!implementation_references[0].is_null());
    let roles: Vec<_> = relation_node["occurrences"]
        .as_array()
        .expect("occurrences")
        .iter()
        .map(|occurrence| occurrence["role"].as_str().expect("role"))
        .collect();
    assert_eq!(roles.len(), 2);
    assert!(roles.contains(&"claim"));
    assert!(roles.contains(&"declaration"));
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
        .insert_domain_package_document(package_identity(document()), document().to_vec())
        .expect("the fixture evidence is unique");
    let admitted = crate::checked_v2::read_checked_package_v2(
        emission.package.bytes(),
        library(),
        V2ReadLimits::fixture(),
        &evidence,
    )
    .unwrap_or_else(|failure| panic!("Contract IR admits the relation package: {failure:?}"))
    .into_value()
    .admitted;
    let admitted_relation = admitted
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| node.semantic_form.as_ref() == "step_relation")
        .expect("the reader retains the relation node");
    assert_eq!(admitted_relation.occurrences.len(), 2);
    assert!(admitted_relation.occurrences.iter().any(|occurrence| {
        occurrence.role == filament_contract_model::CheckedOccurrenceRole::Claim
            && occurrence.ordinal == 0
    }));
    assert!(admitted_relation.occurrences.iter().any(|occurrence| {
        occurrence.role == filament_contract_model::CheckedOccurrenceRole::Declaration
            && occurrence.ordinal == 0
    }));
}

/// FR-180-AC-4 (TC-1143): bound execution entries retain their operation
/// anchor, anchor-specific pre/postcondition nodes and the same abstraction
/// relation node, independently for each execution.
#[trace("TC-1143")]
#[test]
fn a_bound_step_relation_emits_its_execution_clause_and_abstraction_joins() {
    let graph = checked_bound_graph_with_state_clauses();
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    let relation = nodes(&wire)
        .iter()
        .find(|node| node["semantic_form"] == "step_relation")
        .expect("the relation node is present");
    let invariants = relation["body"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|member| member["name"] == "invariants")
        .expect("the model invariant member");
    assert_eq!(invariants["value"]["members"].as_array().unwrap().len(), 1);
    assert_eq!(
        invariants["value"]["members"][0]["value"]["members"]
            .as_array()
            .unwrap()
            .len(),
        1,
        "the selected model's declared invariant is referenced"
    );
    let abstractions: Vec<_> = nodes(&wire)
        .iter()
        .filter(|node| node["semantic_form"] == "abstraction_relation")
        .collect();
    let [abstraction] = abstractions.as_slice() else {
        panic!("one abstraction node: {abstractions:?}");
    };
    let executions = relation["body"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|member| member["name"] == "executions")
        .unwrap()["value"]["members"]
        .as_array()
        .unwrap();
    assert_eq!(executions.len(), 2);
    let operation_targets: Vec<_> = executions
        .iter()
        .map(|execution| {
            let members = execution["value"]["members"].as_array().unwrap();
            for clause in ["preconditions", "postconditions"] {
                let row = members.iter().find(|member| member["name"] == clause).unwrap();
                assert_eq!(row["value"]["members"].as_array().unwrap().len(), 1);
            }
            let implementation = members
                .iter()
                .find(|member| member["name"] == "implementation")
                .unwrap();
            assert_eq!(implementation["value"]["target"], abstraction["node_id"]);
            members
                .iter()
                .find(|member| member["name"] == "operation")
                .unwrap()["value"]["target"]
                .clone()
        })
        .collect();
    assert_eq!(operation_targets[0], operation_targets[1]);
}

/// FR-180-AC-4 / TC-1143: the I2 reader refuses a bound relation whose first
/// execution no longer references its effective precondition, at that
/// execution's `preconditions` aggregate.
fn a_step_relation_reader_rejects_a_missing_precondition() {
    let graph = checked_bound_graph_with_state_clauses();
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    let mut invalid_wire = wire(&emission);
    let relation_index = nodes(&invalid_wire)
        .iter()
        .position(|node| node["semantic_form"] == "step_relation")
        .expect("the relation node is present");
    invalid_wire["semantic_graph"]["nodes"][relation_index]["body"]["members"][1]["value"]
        ["members"][0]["value"]["members"][3]["value"]["members"] = serde_json::json!([]);
    let invalid_bytes = wire_bytes(&invalid_wire).expect("wire bytes");
    let mut evidence = CheckedPackageEvidence::new();
    for feature in invalid_wire["lock"]["required_features"]
        .as_array()
        .expect("required features")
    {
        evidence.support_feature(feature.as_str().expect("feature name"));
    }
    evidence
        .insert_domain_package_document(package_identity(document()), document().to_vec())
        .expect("the fixture evidence is unique");
    let refusal = crate::checked_v2::read_checked_package_v2(
        &invalid_bytes,
        library(),
        V2ReadLimits::fixture(),
        &evidence,
    );
    let Err(fsl_foundation::diagnostic::StageFailure::Refused(refusal)) = refusal else {
        panic!("the I2 reader refuses a missing precondition");
    };
    let crate::checked_v2::V2ReadRefusal::Envelope { refusal, .. } = &refusal else {
        panic!("IR refuses at the model binding: {refusal:?}");
    };
    assert_eq!(
        refusal.code,
        filament_contract_model::CheckedPackageRefusalCode::InvalidModelBinding
    );
    assert_eq!(
        refusal.cause,
        Some(filament_contract_model::CheckedPackageRefusalCause::MalformedDeclaration)
    );
    let path = refusal.loci[0]
        .path
        .as_ref()
        .expect("the missing-precondition aggregate locus")
        .as_str();
    assert!(
        path.ends_with("/body/members/1/value/members/0/value/members/3/value"),
        "refusal path names the preconditions aggregate: {path}"
    );
}

/// The complete FR-180-AC-4 hand-over check combines the wire's member order,
/// its distinct execution binders and body reads, the bound operation
/// premises, and I2's refusal of a removed precondition.
#[trace("TC-1143", "FR-180-AC-4")]
#[test]
fn a_step_relation_satisfies_fr180_ac4_at_i2() {
    a_step_relation_emits_and_reads_back_its_execution_body();
    a_bound_step_relation_emits_its_execution_clause_and_abstraction_joins();
    a_step_relation_reader_rejects_a_missing_precondition();
}

/// Complete FR-180-AC-1 evidence: unbound model-only claim, distinct bound
/// claim identities, and the bound relation premises in the admitted package.
#[trace("TC-605", "FR-180-AC-1")]
#[test]
fn a_step_relation_satisfies_fr180_ac1() {
    a_step_relation_emits_and_reads_back_its_execution_body();
    one_step_relation_claim_occurrence_is_admitted_by_ir();
    a_bound_step_relation_emits_its_execution_clause_and_abstraction_joins();
}
