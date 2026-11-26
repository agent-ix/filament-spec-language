// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-180 / FSpec FR-477 (TC-1143): an HP-1 relation is emitted as a typed
//! step-relation node and admitted by the complete FSL I2 reader.

use super::admission_corpus::config_version_unit;
use super::*;
use filament_checked::key::DeclarationKey;
use filament_exact::Identifier;
use filament_semantic_value::checking::CheckMode;
use fsl_semantics::check::{
    AbstractionNodeRefusal, AbstractionRefusal, AuthoredBinding, CheckCause, FrameBinding,
    OperationKey, PopulationBinding, RustPath, RustReceiver,
};
use std::collections::BTreeMap;

pub(super) fn document() -> &'static [u8] {
    config_version_unit::DOMAIN_PACKAGE.as_bytes()
}

fn relation_unit() -> String {
    String::from(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         profile t = \"filament.temporal.infinite-trace/v1\";\n\
         model Config = \"example/config-version\";\n\
         relation Det using t over (x: Config::ConfigVersion::attemptUpdate, y: Config::ConfigVersion::attemptUpdate) \
         { x.result = y.result and pre(x.self.versionNumber) = pre(y.self.versionNumber) implies x.self.versionNumber = y.self.versionNumber }\n",
    )
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
#[trace("TC-1143", "FR-180-AC-2", "FR-180-AC-3", "FR-940-AC-23")]
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
            if let Some(node) = nodes(wire)
                .iter()
                .find(|node| node["node_id"] == *reference)
            {
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
                let row = members
                    .iter()
                    .find(|member| member["name"] == clause)
                    .unwrap();
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
    let refusal = refuse_relation_mutation(&wire(&emission), |wire, relation| {
        wire["semantic_graph"]["nodes"][relation]["body"]["members"][1]["value"]["members"][0]
            ["value"]["members"][3]["value"]["members"] = serde_json::json!([]);
        vec![relation]
    });
    assert_tc962_refusal(
        &refusal,
        filament_contract_model::CheckedPackageRefusalCode::InvalidModelBinding,
        Some(filament_contract_model::CheckedPackageRefusalCause::MalformedDeclaration),
        "/body/members/1/value/members/0/value/members/3/value",
    );
}

/// All distinct reference targets of a term, in traversal order.
fn reference_targets(term: &serde_json::Value, targets: &mut Vec<serde_json::Value>) {
    if term["term"] == "reference" {
        targets.push(term["target"].clone());
    }
    match term {
        serde_json::Value::Array(values) => {
            for value in values {
                reference_targets(value, targets);
            }
        }
        serde_json::Value::Object(values) => {
            for value in values.values() {
                reference_targets(value, targets);
            }
        }
        _ => {}
    }
}

/// Replaces one edited node's dependencies with the distinct references its
/// body now holds, in structural-key order.
fn set_body_dependencies(wire: &mut serde_json::Value, position: usize) {
    let graph = fsl_semantics::check::wire_id::WireGraph::of_package_value(wire)
        .expect("the edited wire has a structural graph");
    let node = &mut wire["semantic_graph"]["nodes"][position];
    let mut targets = Vec::new();
    reference_targets(&node["body"], &mut targets);
    targets.sort_by_key(|target| graph.key_of_value(target).expect("a referenced node key"));
    targets.dedup();
    node["dependencies"] = serde_json::Value::Array(targets);
}

/// Recomputes the direct target dependencies of an FR-451 abstraction body.
fn set_abstraction_dependencies(wire: &mut serde_json::Value, position: usize) {
    let graph = fsl_semantics::check::wire_id::WireGraph::of_package_value(wire)
        .expect("the edited wire has a structural graph");
    let body = &wire["semantic_graph"]["nodes"][position]["body"];
    let mut targets = Vec::new();
    for (collection, field) in [("objects", "type"), ("populations", "population")] {
        targets.extend(
            body[collection]
                .as_array()
                .expect("abstraction entries")
                .iter()
                .map(|entry| entry[field].clone()),
        );
    }
    targets.extend(
        body["frames"]
            .as_array()
            .expect("abstraction frames")
            .iter()
            .map(|frame| frame["context"].clone()),
    );
    targets.sort_by_key(|target| graph.key_of_value(target).expect("a referenced node key"));
    targets.dedup();
    wire["semantic_graph"]["nodes"][position]["dependencies"] = serde_json::Value::Array(targets);
}

/// Applies one TC-962 mutation to the emitted relation and feeds the edited
/// package through FSL's full I2 reader.
fn refuse_relation_mutation(
    original: &serde_json::Value,
    mutate: impl FnOnce(&mut serde_json::Value, usize) -> Vec<usize>,
) -> filament_contract_model::CheckedPackageRefusal {
    let mut wire = original.clone();
    let position = nodes(&wire)
        .iter()
        .position(|node| node["semantic_form"] == "step_relation")
        .expect("the step-relation node is present");
    let changed_nodes = mutate(&mut wire, position);
    for position in changed_nodes {
        set_body_dependencies(&mut wire, position);
    }
    let bytes = wire_bytes(&wire).expect("the mutated wire serializes");
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"]
        .as_array()
        .expect("required features")
    {
        evidence.support_feature(feature.as_str().expect("feature name"));
    }
    evidence
        .insert_domain_package_document(package_identity(document()), document().to_vec())
        .expect("the fixture evidence is unique");
    let read = crate::checked_v2::read_checked_package_v2(
        &bytes,
        library(),
        V2ReadLimits::fixture(),
        &evidence,
    );
    let Err(fsl_foundation::diagnostic::StageFailure::Refused(refusal)) = read else {
        panic!("the I2 reader refuses the TC-962 mutation");
    };
    let crate::checked_v2::V2ReadRefusal::Envelope { refusal, .. } = refusal else {
        panic!("the IR reader refuses the TC-962 mutation: {refusal:?}");
    };
    *refusal
}

/// Checks one TC-962 result by stable code, cause and path suffix.
fn assert_tc962_refusal(
    refusal: &filament_contract_model::CheckedPackageRefusal,
    code: filament_contract_model::CheckedPackageRefusalCode,
    cause: Option<filament_contract_model::CheckedPackageRefusalCause>,
    path_suffix: &str,
) {
    assert_eq!(refusal.code, code, "refusal: {refusal:?}");
    assert_eq!(refusal.cause, cause, "refusal: {refusal:?}");
    let path = refusal.loci[0]
        .path
        .as_ref()
        .expect("the mutation refusal has a package locus")
        .as_str();
    assert!(
        path.ends_with(path_suffix),
        "expected locus suffix {path_suffix}, got {path}"
    );
}

/// TC-962 RN-07 through RN-12 and RN-14, applied to an emitted bound
/// step-relation package. Each edit keeps dependencies in sync and checks the
/// reader's first refusal.
fn tc962_step_relation_mutations_refuse(wire: &serde_json::Value) {
    use filament_contract_model::{
        CheckedPackageRefusalCause as Cause, CheckedPackageRefusalCode as Code,
    };

    // RN-07a: put the operation before the alias in x's execution Group.
    let refusal = refuse_relation_mutation(wire, |wire, relation| {
        wire["semantic_graph"]["nodes"][relation]["body"]["members"][1]["value"]["members"][0]
            ["value"]["members"]
            .as_array_mut()
            .expect("x's execution Group")
            .swap(0, 1);
        vec![relation]
    });
    assert_tc962_refusal(
        &refusal,
        Code::InvalidSemanticGraph,
        None,
        "/body/members/1/value/members/0/value/members/0",
    );

    // RN-07b: add the old top-level parameters binding with only x's Group.
    let refusal = refuse_relation_mutation(wire, |wire, relation| {
        let body = &mut wire["semantic_graph"]["nodes"][relation]["body"]["members"];
        let x_parameters = body[1]["value"]["members"][0]["value"]["members"]
            .as_array()
            .expect("x's execution members")
            .iter()
            .find(|member| member["name"] == "parameters")
            .expect("x's parameter Group")["value"]
            .clone();
        body[2] = serde_json::json!({"name": "parameters", "value": x_parameters});
        body.as_array_mut()
            .expect("step-relation body members")
            .truncate(4);
        vec![relation]
    });
    assert_tc962_refusal(
        &refusal,
        Code::InvalidSemanticGraph,
        None,
        "/body/members/2",
    );

    // RN-08: x names an unselected model alias.
    let refusal = refuse_relation_mutation(wire, |wire, relation| {
        wire["semantic_graph"]["nodes"][relation]["body"]["members"][1]["value"]["members"][0]
            ["value"]["members"][0]["value"]["value"] = "W".into();
        vec![relation]
    });
    assert_tc962_refusal(
        &refusal,
        Code::MissingDeclaration,
        Some(Cause::MissingName),
        "/body/members/1/value/members/0/value/members/0",
    );

    let object_type = nodes(wire)
        .iter()
        .find(|node| node["node_tag"] == "model" && node["semantic_form"] == "object_type")
        .expect("the model object type node")["node_id"]
        .clone();

    // RN-09a: x's operation reference names an object type.
    let refusal = refuse_relation_mutation(wire, |wire, relation| {
        wire["semantic_graph"]["nodes"][relation]["body"]["members"][1]["value"]["members"][0]
            ["value"]["members"][1]["value"]["target"] = object_type.clone();
        vec![relation]
    });
    assert_tc962_refusal(
        &refusal,
        Code::InvalidModelBinding,
        Some(Cause::MalformedDeclaration),
        "/body/members/1/value/members/0/value/members/1/value/target",
    );

    // RN-09b: the V subject reference names an object type.
    let refusal = refuse_relation_mutation(wire, |wire, relation| {
        wire["semantic_graph"]["nodes"][relation]["body"]["members"][0]["value"]["members"][0]
            ["value"]["target"] = object_type.clone();
        vec![relation]
    });
    assert_tc962_refusal(
        &refusal,
        Code::InvalidModelBinding,
        Some(Cause::MalformedDeclaration),
        "/body/members/0/value/members/0/value/target",
    );

    // RN-10: retain an abstraction relation node but remove its step frame.
    let refusal = refuse_relation_mutation(wire, |wire, relation| {
        let abstraction = nodes(wire)
            .iter()
            .position(|node| node["semantic_form"] == "abstraction_relation")
            .expect("the abstraction relation node");
        wire["semantic_graph"]["nodes"][abstraction]["body"]["frames"] = serde_json::json!([]);
        // The abstraction body stores target IDs directly rather than as
        // `reference` terms, so recompute its target dependencies explicitly.
        set_abstraction_dependencies(wire, abstraction);
        vec![relation]
    });
    assert_tc962_refusal(
        &refusal,
        Code::InvalidModelBinding,
        Some(Cause::MalformedDeclaration),
        "/body/members/1/value/members/0/value/members/5/value/target",
    );

    // RN-11a: remove x's effective precondition.
    let refusal = refuse_relation_mutation(wire, |wire, relation| {
        wire["semantic_graph"]["nodes"][relation]["body"]["members"][1]["value"]["members"][0]
            ["value"]["members"][3]["value"]["members"] = serde_json::json!([]);
        vec![relation]
    });
    assert_tc962_refusal(
        &refusal,
        Code::InvalidModelBinding,
        Some(Cause::MalformedDeclaration),
        "/body/members/1/value/members/0/value/members/3/value",
    );

    // RN-11b: remove the model's declared invariant.
    let refusal = refuse_relation_mutation(wire, |wire, relation| {
        wire["semantic_graph"]["nodes"][relation]["body"]["members"][2]["value"]["members"][0]
            ["value"]["members"] = serde_json::json!([]);
        vec![relation]
    });
    assert_tc962_refusal(
        &refusal,
        Code::InvalidModelBinding,
        Some(Cause::MalformedDeclaration),
        "/body/members/2/value/members/0/value",
    );

    // RN-12: replace the relation's Boolean root with a non-Boolean expression.
    let scalar_expression = nodes(wire)
        .iter()
        .find(|node| {
            node["node_tag"] == "expression"
                && node["semantic_type"]
                    != nodes(wire)
                        .iter()
                        .find(|candidate| candidate["semantic_form"] == "step_relation")
                        .expect("the relation node")["semantic_type"]
        })
        .expect("a non-Boolean expression node")["node_id"]
        .clone();
    let refusal = refuse_relation_mutation(wire, |wire, relation| {
        wire["semantic_graph"]["nodes"][relation]["body"]["members"][3]["value"]["target"] =
            scalar_expression.clone();
        vec![relation]
    });
    assert_tc962_refusal(
        &refusal,
        Code::IllTyped,
        Some(Cause::NonBooleanRoot),
        "/body/members/3/value/target",
    );

    // RN-14: y reuses x's parameter nodes instead of its own levels 2 and 3.
    let refusal = refuse_relation_mutation(wire, |wire, relation| {
        let executions = &mut wire["semantic_graph"]["nodes"][relation]["body"]["members"][1]
            ["value"]["members"];
        let x_parameters = executions[0]["value"]["members"]
            .as_array()
            .expect("x's execution members")
            .iter()
            .find(|member| member["name"] == "parameters")
            .expect("x's parameters")
            .clone();
        let y_parameters = executions[1]["value"]["members"]
            .as_array_mut()
            .expect("y's execution members")
            .iter_mut()
            .find(|member| member["name"] == "parameters")
            .expect("y's parameters");
        *y_parameters = x_parameters;
        vec![relation]
    });
    assert_tc962_refusal(
        &refusal,
        Code::InvalidModelBinding,
        Some(Cause::MalformedDeclaration),
        "/body/members/1/value/members/1/value/members/2/value",
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
    let graph = checked_bound_graph_with_state_clauses();
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    tc962_step_relation_mutations_refuse(&wire(&emission));
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

/// The relation unit with `bindings` declared as its abstraction relation.
fn checked_graph_declaring(bindings: &str) -> fsl_semantics::check::CheckedGraph {
    check_declaring(bindings).unwrap_or_else(|refusals| panic!("the unit checks: {refusals:?}"))
}

/// The check of the unit declaring `bindings`.
fn check_declaring(
    bindings: &str,
) -> Result<fsl_semantics::check::CheckedGraph, Vec<fsl_semantics::check::CheckRefusal>> {
    let packages = fsl_semantics::model::intake::package_input([document()]).unwrap();
    let unit = format!(
        "{}abstraction A using v {{\n{bindings}}}\n",
        relation_unit()
    );
    assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
}

const HISTORY: &str = "population Config::config_history to \"store::history\";\n";
const UPDATE: &str =
    "operation Config::ConfigVersion::attemptUpdate to \"store::update\" receiver \"self\" ();\n";

/// The binding of [`HISTORY`].
fn history_binding() -> AuthoredBinding {
    AuthoredBinding::Population {
        key: DeclarationKey {
            package: "example/config-version".to_owned(),
            node: "ix://example/config-version/config_history".to_owned(),
        },
        value: PopulationBinding {
            collection: RustPath::new("store::history"),
        },
    }
}

/// The binding of [`UPDATE`].
fn update_binding() -> AuthoredBinding {
    AuthoredBinding::Frame {
        key: OperationKey {
            declaring: DeclarationKey {
                package: "example/config-version".to_owned(),
                node: "ix://example/config-version/ConfigVersion".to_owned(),
            },
            operation: Identifier::new("attemptUpdate").expect("an identifier"),
        },
        value: FrameBinding {
            function: RustPath::new("store::update"),
            receiver: RustReceiver::SelfValue,
            parameters: BTreeMap::new(),
        },
    }
}

/// The step relation's claim key and whether it has the code claim's record.
fn step_claim(graph: &fsl_semantics::check::CheckedGraph) -> (filament_exact::NodeKey, bool) {
    use fsl_semantics::check::Capability;

    let [relation] = graph.step_relations() else {
        panic!("one step relation");
    };
    let claim = relation.occurrence().expect("the claim occurrence");
    assert!(graph
        .requirements()
        .get_kind(claim, Capability::TemporalSatisfaction)
        .is_some());
    (
        relation.identity().expect("the claim node"),
        graph
            .requirements()
            .get_kind(claim, Capability::OperationContract)
            .is_some(),
    )
}

/// The emitted step-relation node, and the `implementation` member's value of
/// each of its two executions.
fn emitted_step_relation(package: &CheckedPackage) -> (Value, [Value; 2]) {
    let emission = emit_checked(package).expect("the package emits");
    let wire = wire(&emission);
    let node = nodes(&wire)
        .iter()
        .find(|node| node["semantic_form"] == "step_relation")
        .expect("the step-relation node")
        .clone();
    let member = |members: &Value, name: &str| {
        members
            .as_array()
            .expect("members")
            .iter()
            .find(|member| member["name"] == name)
            .unwrap_or_else(|| panic!("the member {name}"))["value"]
            .clone()
    };
    let executions = member(&node["body"]["members"], "executions");
    let [x, y] = executions["members"]
        .as_array()
        .expect("executions")
        .as_slice()
    else {
        panic!("two executions: {executions}");
    };
    let implementations =
        [x, y].map(|execution| member(&execution["value"]["members"], "implementation"));
    (
        json!({"node_id": node["node_id"], "body": node["body"]}),
        implementations,
    )
}

/// The emitted `abstraction_relation` node.
fn emitted_abstraction(package: &CheckedPackage) -> Value {
    let emission = emit_checked(package).expect("the package emits");
    let wire = wire(&emission);
    nodes(&wire)
        .iter()
        .find(|node| node["semantic_form"] == "abstraction_relation")
        .expect("the abstraction node")
        .clone()
}

/// Rebinds a package declaring `HISTORY` and `UPDATE`, at the sites it
/// already has, so it still emits.
fn rebound(binding: AuthoredBinding) -> (CheckedPackage, CheckedPackage) {
    let declared = format!("{HISTORY}{UPDATE}");
    let original = CheckedPackage::link(checked_graph_declaring(&declared));
    let mut package = CheckedPackage::link(checked_graph_declaring(&declared));
    let sites = package.abstraction().expect("a relation").sites().to_vec();
    package
        .bind_abstraction_at(&[binding], sites)
        .expect("the relation binds");
    (original, package)
}

/// FR-306-AC-5: a rebind that drops the frame leaves the step relation with no
/// implementation and no code claim, as a unit declaring that relation does,
/// and does not change a graph already shared.
#[trace("TC-804", "FR-306-AC-5")]
#[test]
fn a_rebind_that_drops_a_frame_derives_the_step_relation_again() {
    let mut package = CheckedPackage::link(checked_graph_declaring(&format!("{HISTORY}{UPDATE}")));
    let shared = package.shared_graph();
    assert!(step_claim(&shared).1);
    let sites = package.abstraction().expect("a relation").sites().to_vec();
    package
        .bind_abstraction_at(&[history_binding()], sites)
        .expect("the relation binds");

    let (key, code) = step_claim(package.graph());
    assert!(!code);
    assert_eq!(package.graph().requirements().len(), 1);
    let fresh = CheckedPackage::link(checked_graph_declaring(HISTORY));
    assert_eq!(key, step_claim(fresh.graph()).0);
    assert_eq!(package.graph().requirements(), fresh.graph().requirements());
    let (node, implementations) = emitted_step_relation(&package);
    for implementation in implementations {
        assert_eq!(implementation["members"], json!([]));
    }
    assert_eq!(node, emitted_step_relation(&fresh).0);
    // The graph shared before the rebind keeps its code claim.
    assert!(step_claim(&shared).1);
    assert_eq!(shared.requirements().len(), 2);
}

/// FR-306-AC-5: a rebind that changes the target set gives the abstraction
/// relation a new node key, and each execution's implementation references it.
/// The claim's own key is its declared name's, so it does not change.
#[trace("TC-804", "FR-306-AC-5")]
#[test]
fn a_rebind_that_changes_the_targets_rekeys_the_implementation() {
    let (original, package) = rebound(update_binding());
    let abstraction = emitted_abstraction(&package);
    assert_ne!(
        abstraction["node_id"],
        emitted_abstraction(&original)["node_id"]
    );
    let (node, implementations) = emitted_step_relation(&package);
    for implementation in implementations {
        assert_eq!(implementation["target"], abstraction["node_id"]);
    }
    let fresh = CheckedPackage::link(checked_graph_declaring(UPDATE));
    assert_eq!(node, emitted_step_relation(&fresh).0);
    let (key, code) = step_claim(package.graph());
    assert!(code);
    assert_eq!(key, step_claim(fresh.graph()).0);
    assert_eq!(key, step_claim(original.graph()).0);
    assert_ne!(node, emitted_step_relation(&original).0);
    assert_eq!(package.graph().requirements(), fresh.graph().requirements());
}

/// FR-306-AC-5: binding a frame to a package whose unit declares no relation
/// adds the code claim a unit declaring the frame has.
#[trace("TC-804", "FR-306-AC-5")]
#[test]
fn a_first_binding_derives_the_step_relations_code_claim() {
    let mut package = CheckedPackage::link(checked_graph());
    let (unbound, code) = step_claim(package.graph());
    assert!(!code);
    package
        .bind_abstraction(&[update_binding()])
        .expect("the relation binds");
    let (key, code) = step_claim(package.graph());
    assert!(code);
    assert_eq!(key, unbound);
    assert_eq!(package.graph().requirements().len(), 2);
    let fresh = CheckedPackage::link(checked_graph_declaring(UPDATE));
    assert_eq!(key, step_claim(fresh.graph()).0);
    assert_eq!(package.graph().requirements(), fresh.graph().requirements());
    assert_eq!(
        package.graph().semantic_graph().node(key.clone()),
        fresh.graph().semantic_graph().node(key)
    );
}

/// [`update_binding`] bound to a Rust path with an empty segment.
fn unsegmented_update_binding() -> AuthoredBinding {
    let AuthoredBinding::Frame { key, mut value } = update_binding() else {
        panic!("a frame binding");
    };
    value.function = RustPath::new("store::");
    AuthoredBinding::Frame { key, value }
}

/// The refusal both routes give for [`unsegmented_update_binding`].
fn assert_empty_segment(refusal: &AbstractionRefusal) {
    assert_eq!(
        refusal,
        &AbstractionRefusal::RelationNodes(Box::new(AbstractionNodeRefusal::EmptyRustSegment {
            spelling: "store::".to_owned(),
        }))
    );
    assert_eq!(refusal.code(), fsl_foundation::Code::InvalidModelBinding);
    assert_eq!(refusal.cause(), "malformed-declaration");
}

/// FR-306-AC-6: a bound Rust path the step relation cannot be derived with
/// refuses the binding with that cause, not as an internal fault, and leaves
/// the package unchanged.
#[trace("TC-804", "FR-306-AC-6")]
#[test]
fn a_binding_a_step_relation_cannot_read_refuses_with_its_cause() {
    let mut package = CheckedPackage::link(checked_graph_declaring(&format!("{HISTORY}{UPDATE}")));
    let requirements = package.graph().requirements().clone();
    let refusals = package
        .bind_abstraction(&[unsegmented_update_binding()])
        .expect_err("the binding refuses");
    let [refusal] = &refusals[..] else {
        panic!("one refusal: {refusals:?}");
    };
    assert_empty_segment(refusal);
    assert_eq!(package.graph().requirements(), &requirements);
    assert!(step_claim(package.graph()).1);
}

/// FR-306-AC-6: checking a unit that declares that path refuses with the same
/// cause as the binding does.
#[trace("TC-804", "FR-306-AC-6")]
#[test]
fn checking_a_path_a_step_relation_cannot_read_refuses_with_its_cause() {
    let refusals =
        check_declaring(&UPDATE.replace("store::update", "store::")).expect_err("the unit refuses");
    let [refusal] = &refusals[..] else {
        panic!("one refusal: {refusals:?}");
    };
    let CheckCause::Abstraction(cause) = refusal.cause.as_ref() else {
        panic!("an abstraction refusal: {refusal:?}");
    };
    assert_empty_segment(cause);
}

/// The claim node's occurrences: role, ordinal and location.
fn claim_occurrences(graph: &fsl_semantics::check::CheckedGraph) -> Vec<(String, u64, String)> {
    let (claim, _) = step_claim(graph);
    graph
        .occurrences()
        .filter(|(node, _, _)| *node == claim)
        .map(|(_, origin, location)| {
            (
                origin.role().to_string(),
                origin.ordinal(),
                format!("{location:?}"),
            )
        })
        .collect()
}

/// FR-306-AC-5: a rebind leaves the claim node with the occurrences a fresh
/// check records, one `claim` and one `declaration`.
#[trace("TC-804", "FR-306-AC-5")]
#[test]
fn a_rebind_records_the_claim_occurrences_of_a_fresh_check() {
    let (_, package) = rebound(update_binding());
    let fresh = CheckedPackage::link(checked_graph_declaring(UPDATE));
    let held = claim_occurrences(package.graph());
    assert_eq!(
        held.iter()
            .map(|(role, ordinal, _)| (role.as_str(), *ordinal))
            .collect::<Vec<_>>(),
        [("claim", 0), ("declaration", 0)]
    );
    assert_eq!(held, claim_occurrences(fresh.graph()));
    let emitted = |package: &CheckedPackage| {
        let wire = wire(&emit_checked(package).expect("the package emits"));
        nodes(&wire)
            .iter()
            .find(|node| node["semantic_form"] == "step_relation")
            .expect("the step-relation node")["occurrences"]
            .clone()
    };
    assert_eq!(emitted(&package), emitted(&fresh));
    assert_eq!(
        emitted(&package),
        json!([
            {"role": "claim", "ordinal": 0},
            {"role": "declaration", "ordinal": 0},
        ])
    );
}
