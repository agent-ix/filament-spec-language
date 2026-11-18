// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-640, FSL FR-740: the checked random parameters and workloads
//! written as `probabilistic` nodes of the checked-package/v2 graph.
//!
//! A node's identity is `["probabilistic", form, operand...]` in the unit's
//! package. Each node holds the one `declaration` occurrence of its
//! declaration's label span, and the nodes are appended in node-id order, so
//! neither the order the unit declares them in nor the order a workload
//! writes its weights in reaches the bytes.

use filament_contract_model::{
    CheckedNodeId, CheckedOccurrence, CheckedOccurrenceRole, CheckedSemanticNodeV2,
    CheckedSourceMapEntry, CheckedSourceRegion, CheckedTerm, LiteralKind,
};
use filament_exact::{NodeKey, PathSegment, Rational, UnitId, Value};
use fsl_semantics::check::wire_id::StructuralIds;
use fsl_semantics::check::{
    CheckedGraph, ProbabilisticFeature, ProbabilisticOperation, RandomParameter, Workload,
};

use super::{
    body_reference_targets, checked_literal_json, claim_aggregate, claim_binding, encoding,
    node_id, push_scalar_type, source_artifact, CheckedNodeTag, EmitRefusal, Location, GRAPH_V2,
};

/// A written node with its declaration occurrence's `region`.
struct Pending {
    node: CheckedSemanticNodeV2,
    region: CheckedSourceRegion,
}

/// Write a `probabilistic` node for each checked random parameter and
/// workload of `graph`, with its source-map entry.
///
/// # Errors
///
/// A declaration that cannot be written refuses with its feature, its name
/// and the declaration's region.
pub(super) fn append_probabilistic_nodes(
    ids: &StructuralIds,
    graph: &CheckedGraph,
    nodes: &mut Vec<CheckedSemanticNodeV2>,
    source_map: &mut Vec<CheckedSourceMapEntry>,
) -> Result<(), EmitRefusal> {
    let model = graph.probabilistic_model();
    let mut pending = Vec::new();
    for random in model.random_parameters() {
        pending.push(random_parameter_node(
            ids, graph, nodes, source_map, random,
        )?);
    }
    for workload in model.workloads() {
        pending.push(workload_node(ids, graph, nodes, source_map, workload)?);
    }
    pending.sort_by(|left, right| left.node.node_id.cmp(&right.node.node_id));
    for Pending { node, region } in pending {
        source_map.push(CheckedSourceMapEntry {
            node_id: node.node_id.clone(),
            role: CheckedOccurrenceRole::Declaration,
            ordinal: 0,
            regions: vec![region],
        });
        nodes.push(node);
    }
    Ok(())
}

/// The refusal of a declaration of `feature` named `declaration`, located at
/// `region`.
fn refusal(
    graph: &CheckedGraph,
    feature: ProbabilisticFeature,
    declaration: String,
    location: &Location,
) -> EmitRefusal {
    match graph.region(location) {
        Some(region) => EmitRefusal::ProbabilisticModelNotEmitted {
            declaration,
            feature,
            profile: graph.header_layers().last().map(|layer| layer.identity()),
            region,
        },
        None => EmitRefusal::SourceMapInvariant,
    }
}

fn text_literal(type_id: &CheckedNodeId, value: &str) -> Result<CheckedTerm, EmitRefusal> {
    literal(type_id, LiteralKind::Text, &value)
}

fn literal(
    type_id: &CheckedNodeId,
    kind: LiteralKind,
    value: &impl serde::Serialize,
) -> Result<CheckedTerm, EmitRefusal> {
    Ok(CheckedTerm::Literal {
        type_id: type_id.clone(),
        value_kind: kind,
        value: checked_literal_json(value)?,
    })
}

/// An exact weight: the FR-205 canonical spelling `n` or `n/d` of a reduced
/// rational.
fn weight_literal(
    rational_type: &CheckedNodeId,
    weight: &Rational,
) -> Result<CheckedTerm, EmitRefusal> {
    literal(rational_type, LiteralKind::Rational, &spelling(weight))
}

fn spelling(value: &Rational) -> String {
    if value.is_integer() {
        value.numerator().to_string()
    } else {
        format!("{}/{}", value.numerator(), value.denominator())
    }
}

fn reference(target: CheckedNodeId) -> CheckedTerm {
    CheckedTerm::Reference { target }
}

/// The node of the model object type that declares `operation`.
fn declaring_node(
    ids: &StructuralIds,
    nodes: &[CheckedSemanticNodeV2],
    operation: &ProbabilisticOperation,
) -> Option<CheckedNodeId> {
    let key = fsl_semantics::model::key::object_type_node_key(operation.declaring_key()).ok()?;
    let id = node_id(ids, key);
    nodes.iter().any(|node| node.node_id == id).then_some(id)
}

/// The built-in scalar type node of `form`, written beside the declaration
/// when the graph holds none.
fn scalar_type(
    ids: &StructuralIds,
    graph: &CheckedGraph,
    nodes: &mut Vec<CheckedSemanticNodeV2>,
    source_map: &mut Vec<CheckedSourceMapEntry>,
    form: &str,
    region: &CheckedSourceRegion,
) -> Result<CheckedNodeId, EmitRefusal> {
    let key = NodeKey::new(
        graph.package_name(),
        vec![
            PathSegment::Text("scalar_type".into()),
            PathSegment::Text(form.into()),
        ],
        0,
    )
    .map_err(encoding)?;
    let id = node_id(ids, key);
    push_scalar_type(nodes, source_map, &id, form, region);
    Ok(id)
}

fn node(
    graph: &CheckedGraph,
    ids: &StructuralIds,
    form: &str,
    operand: Vec<PathSegment>,
    body: CheckedTerm,
) -> Result<CheckedSemanticNodeV2, EmitRefusal> {
    let mut path = vec![
        PathSegment::Text("probabilistic".into()),
        PathSegment::Text(form.into()),
    ];
    path.extend(operand);
    let id = node_id(
        ids,
        NodeKey::new(graph.package_name(), path, 0).map_err(encoding)?,
    );
    Ok(CheckedSemanticNodeV2 {
        owner_model: None,
        check_mode: None,
        node_id: id.clone(),
        schema_version: GRAPH_V2.into(),
        node_tag: CheckedNodeTag::Probabilistic.as_wire().into(),
        semantic_form: form.into(),
        semantic_type: id,
        dependencies: body_reference_targets(&body),
        occurrences: vec![CheckedOccurrence {
            role: CheckedOccurrenceRole::Declaration,
            ordinal: 0,
        }],
        recursion_group: None,
        declaration: None,
        owner: None,
        body,
    })
}

fn region_of(
    graph: &CheckedGraph,
    location: &Location,
) -> Result<CheckedSourceRegion, EmitRefusal> {
    let region = graph
        .region(location)
        .ok_or(EmitRefusal::SourceMapInvariant)?;
    Ok(CheckedSourceRegion {
        source: source_artifact(graph.source()),
        start: region.start(),
        end: region.end(),
    })
}

fn random_parameter_node(
    ids: &StructuralIds,
    graph: &CheckedGraph,
    nodes: &mut Vec<CheckedSemanticNodeV2>,
    source_map: &mut Vec<CheckedSourceMapEntry>,
    random: &RandomParameter,
) -> Result<Pending, EmitRefusal> {
    let operation = random.operation();
    let name = format!("{}({})", operation.name(), random.parameter());
    let location = random.declaration_location();
    let unwritable = || {
        refusal(
            graph,
            ProbabilisticFeature::RandomParameter,
            name.clone(),
            &location,
        )
    };
    let region = region_of(graph, &location)?;
    let declaring = declaring_node(ids, nodes, operation).ok_or_else(unwritable)?;
    let text = scalar_type(ids, graph, nodes, source_map, "text", &region)?;
    let rational = scalar_type(ids, graph, nodes, source_map, "rational", &region)?;
    let mut rows = Vec::with_capacity(random.support().len());
    for support in random.support() {
        let value = match support.value.0.clone() {
            Value::Boolean(value) => {
                let type_id = scalar_type(ids, graph, nodes, source_map, "boolean", &region)?;
                literal(&type_id, LiteralKind::Boolean, &value)?
            }
            Value::Integer(value) => {
                let type_id = scalar_type(ids, graph, nodes, source_map, "integer", &region)?;
                literal(&type_id, LiteralKind::Integer, &value.to_string())?
            }
            Value::Rational(value) => literal(&rational, LiteralKind::Rational, &spelling(&value))?,
            Value::Text(value) => text_literal(&text, value.retained())?,
            Value::Quantity(value) => {
                let unit = match value.unit() {
                    UnitId::Declared(key) => node_id(ids, key),
                    UnitId::Compound(_) => return Err(unwritable()),
                };
                if !nodes.iter().any(|node| node.node_id == unit) {
                    return Err(unwritable());
                }
                literal(&unit, LiteralKind::Quantity, &spelling(value.magnitude()))?
            }
            Value::Enum(member) => {
                let owner = node_id(ids, member.variant().owner());
                if !nodes.iter().any(|node| node.node_id == owner) {
                    return Err(unwritable());
                }
                literal(&owner, LiteralKind::Enum, &member.variant().case())?
            }
            _ => return Err(unwritable()),
        };
        rows.push(claim_aggregate(vec![
            claim_binding("value", value),
            claim_binding("weight", weight_literal(&rational, &support.weight)?),
        ]));
    }
    let body = claim_aggregate(vec![
        claim_binding("declaring", reference(declaring)),
        claim_binding("operation", text_literal(&text, operation.name())?),
        claim_binding("parameter", text_literal(&text, random.parameter())?),
        claim_binding("support", claim_aggregate(rows)),
    ]);
    let operand = vec![
        PathSegment::Text(fsl_semantics::model::key::declaration_identity_text(
            operation.declaring_key(),
        )),
        PathSegment::Text(operation.name().into()),
        PathSegment::Text(random.parameter().into()),
    ];
    Ok(Pending {
        node: node(graph, ids, "random_parameter", operand, body)?,
        region,
    })
}

fn workload_node(
    ids: &StructuralIds,
    graph: &CheckedGraph,
    nodes: &mut Vec<CheckedSemanticNodeV2>,
    source_map: &mut Vec<CheckedSourceMapEntry>,
    workload: &Workload,
) -> Result<Pending, EmitRefusal> {
    let location = workload.declaration_location();
    let unwritable = || {
        refusal(
            graph,
            ProbabilisticFeature::Workload,
            workload.name().to_owned(),
            &location,
        )
    };
    let region = region_of(graph, &location)?;
    let text = scalar_type(ids, graph, nodes, source_map, "text", &region)?;
    let rational = scalar_type(ids, graph, nodes, source_map, "rational", &region)?;
    let mut weights = Vec::with_capacity(workload.weights().len());
    for (operation, weight) in workload.weights() {
        let declaring = declaring_node(ids, nodes, operation).ok_or_else(unwritable)?;
        weights.push((declaring, operation.name().to_owned(), weight.clone()));
    }
    weights.sort_by(|left, right| (&left.0, &left.1).cmp(&(&right.0, &right.1)));
    // The workload's model node is the one object type every weight's
    // declaring type is, which holds for a model of one type.
    let mut declaring: Vec<&CheckedNodeId> = weights.iter().map(|weight| &weight.0).collect();
    declaring.dedup();
    let [model] = declaring[..] else {
        return Err(unwritable());
    };
    let model = model.clone();
    let rows = weights
        .iter()
        .map(|(declaring, operation, weight)| {
            Ok(claim_aggregate(vec![
                claim_binding("declaring", reference(declaring.clone())),
                claim_binding("operation", text_literal(&text, operation)?),
                claim_binding("weight", weight_literal(&rational, weight)?),
            ]))
        })
        .collect::<Result<Vec<_>, EmitRefusal>>()?;
    let body = claim_aggregate(vec![
        claim_binding("model", reference(model)),
        claim_binding("name", text_literal(&text, workload.name())?),
        claim_binding("weights", claim_aggregate(rows)),
    ]);
    let operand = vec![PathSegment::Text(workload.name().into())];
    Ok(Pending {
        node: node(graph, ids, "workload", operand, body)?,
        region,
    })
}
