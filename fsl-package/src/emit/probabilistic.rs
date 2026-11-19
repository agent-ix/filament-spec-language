// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-640, FSL FR-740: the checked random parameters, workloads and delays
//! written as `probabilistic` nodes of the checked-package/v2 graph.
//!
//! A node's identity is `["probabilistic", form, operand...]` in the unit's
//! package: the model type's node, the operation and parameter (or the
//! workload's name), and the canonical spelling of the node's values and
//! weights, so a changed weight or support value changes the identity
//! (FSpec FR-640-AC-3). Each node holds the one `declaration` occurrence of its
//! declaration's label span, and the nodes are appended in node-id order, so
//! neither the order the unit declares them in nor the order a workload
//! writes its weights in reaches the bytes.

use filament_checked::key::DeclarationKey;
use filament_contract_model::{
    CheckedLiteralValue, CheckedNodeId, CheckedOccurrence, CheckedOccurrenceRole,
    CheckedSemanticNodeV2, CheckedSourceMapEntry, CheckedSourceRegion, CheckedTerm, LiteralKind,
};

use filament_exact::{NodeKey, PathSegment, Rational, UnitId, Value};
use fsl_semantics::check::wire_id::StructuralIds;
use fsl_semantics::check::{
    CheckedGraph, ClaimReadFault, Delay, DelayDistribution, ProbScheduler, ProbabilisticFeature,
    RandomParameter, Reward, SchedulerRead, Statement, Workload,
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

/// Write a `probabilistic` node for each checked random parameter, workload,
/// delay and reward of `graph`, with its source-map entry.
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
    for delay in model.delays() {
        pending.push(delay_node(ids, graph, nodes, source_map, delay)?);
    }
    for reward in model.rewards() {
        pending.push(reward_node(ids, graph, nodes, source_map, reward)?);
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

/// The key and node of the model object type `declaration` names, when the
/// graph holds its node.
fn model_node(
    ids: &StructuralIds,
    nodes: &[CheckedSemanticNodeV2],
    declaration: &DeclarationKey,
) -> Option<(NodeKey, CheckedNodeId)> {
    let key = fsl_semantics::model::key::object_type_node_key(declaration).ok()?;
    let id = node_id(ids, &key);
    nodes
        .iter()
        .any(|node| node.node_id == id)
        .then_some((key, id))
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
    ids: &StructuralIds,
    form: &str,
    key: &NodeKey,
    body: CheckedTerm,
) -> CheckedSemanticNodeV2 {
    let id = node_id(ids, key);
    CheckedSemanticNodeV2 {
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
    }
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
    let (_, declaring) = model_node(
        ids,
        nodes,
        operation.declaring_key().ok_or_else(unwritable)?,
    )
    .ok_or_else(unwritable)?;
    let key = random
        .node_key(graph.package_name())
        .ok_or_else(unwritable)?;
    let text = scalar_type(ids, graph, nodes, source_map, "text", &region)?;
    let rational = scalar_type(ids, graph, nodes, source_map, "rational", &region)?;
    let mut rows = Vec::with_capacity(random.support().len());
    for support in random.support() {
        let value = match support.value.0.clone() {
            // IR's reader admits no Boolean or enum support value yet
            // (probabilistic_join.rs): `not_member` refuses every literal
            // whose payload is not a string, and `member_literal_kind` maps
            // no model variant type to a literal kind. A value written here
            // would refuse the whole package, so emission refuses the
            // declaration with its locus instead.
            Value::Boolean(_) => return Err(unwritable()),
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
            Value::Enum(_) => return Err(unwritable()),
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
    Ok(Pending {
        node: node(ids, "random_parameter", &key, body),
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
    // A workload whose operations no single model type's lineage declares
    // has no model node to hang from; QSL-2444 tracks emitting it.
    let (_, model) = workload
        .model_key()
        .and_then(|declaration| model_node(ids, nodes, declaration))
        .ok_or_else(unwritable)?;
    let key = workload
        .node_key(graph.package_name())
        .ok_or_else(unwritable)?;
    let mut weights = Vec::with_capacity(workload.weights().len());
    for (operation, weight) in workload.weights() {
        let (_, declaring) = model_node(
            ids,
            nodes,
            operation.declaring_key().ok_or_else(unwritable)?,
        )
        .ok_or_else(unwritable)?;
        weights.push((declaring, operation.name().to_owned(), weight.clone()));
    }
    weights.sort_by(|left, right| (&left.0, &left.1).cmp(&(&right.0, &right.1)));
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
    Ok(Pending {
        node: node(ids, "workload", &key, body),
        region,
    })
}

/// The member `position` of an aggregate `term`, bound or not.
fn member(term: &CheckedTerm, position: usize) -> Option<&CheckedTerm> {
    match term {
        CheckedTerm::Aggregate { members } => match members.get(position)? {
            CheckedTerm::Binding { value, .. } => Some(value),
            _ => None,
        },
        _ => None,
    }
}

/// The unit node of the `dense` time source of the timed model rooted on
/// `declaring`, read from the timed nodes already written. IR's reader
/// types each delay time by this node (FR-640 Reader rule 5).
pub(super) fn dense_unit(
    nodes: &[CheckedSemanticNodeV2],
    declaring: &CheckedNodeId,
) -> Option<CheckedNodeId> {
    let reference_of = |term: Option<&CheckedTerm>| match term {
        Some(CheckedTerm::Reference { target }) => Some(target.clone()),
        _ => None,
    };
    let root = nodes.iter().find(|node| {
        CheckedNodeTag::from_wire(&node.node_tag) == Some(CheckedNodeTag::TimedModel)
            && &*node.semantic_form == "timed_model"
            && reference_of(member(&node.body, 0)).as_ref() == Some(declaring)
    })?;
    let source = reference_of(member(&root.body, 2))?;
    let source = nodes.iter().find(|node| node.node_id == source)?;
    let dense = member(&source.body, 0)?
        .literal_read()
        .is_some_and(|read| matches!(read.value, CheckedLiteralValue::Text("dense")));
    dense
        .then(|| reference_of(member(&source.body, 1)))
        .flatten()
}

fn delay_node(
    ids: &StructuralIds,
    graph: &CheckedGraph,
    nodes: &mut Vec<CheckedSemanticNodeV2>,
    source_map: &mut Vec<CheckedSourceMapEntry>,
    delay: &Delay,
) -> Result<Pending, EmitRefusal> {
    let operation = delay.operation();
    let location = delay.declaration_location();
    let unwritable = || {
        refusal(
            graph,
            ProbabilisticFeature::Delay,
            operation.name().to_owned(),
            &location,
        )
    };
    let region = region_of(graph, &location)?;
    let (_, declaring) = model_node(
        ids,
        nodes,
        operation.declaring_key().ok_or_else(unwritable)?,
    )
    .ok_or_else(unwritable)?;
    let key = delay
        .node_key(graph.package_name())
        .ok_or_else(unwritable)?;
    // IR's reader admits a delay only on a model with a timed root whose
    // source is `dense`, and types every time by that source's unit.
    let unit = dense_unit(nodes, &declaring).ok_or_else(unwritable)?;
    let text = scalar_type(ids, graph, nodes, source_map, "text", &region)?;
    let rational = scalar_type(ids, graph, nodes, source_map, "rational", &region)?;
    let time = |value: &Rational| literal(&unit, LiteralKind::Quantity, &spelling(value));
    let (family, parameters) = match delay.distribution() {
        DelayDistribution::Uniform => ("uniform", claim_aggregate(Vec::new())),
        DelayDistribution::UniformRange { a, b } => (
            "uniform_interval",
            claim_aggregate(vec![
                claim_binding("lower", time(a)?),
                claim_binding("upper", time(b)?),
            ]),
        ),
        DelayDistribution::Exponential { rate } => (
            "exponential",
            claim_aggregate(vec![claim_binding(
                "rate",
                weight_literal(&rational, rate)?,
            )]),
        ),
        DelayDistribution::Discrete(rows) => {
            let mut written = Vec::with_capacity(rows.len());
            for (delay, weight) in rows {
                written.push(claim_aggregate(vec![
                    claim_binding("delay", time(delay)?),
                    claim_binding("weight", weight_literal(&rational, weight)?),
                ]));
            }
            ("discrete", claim_aggregate(written))
        }
    };
    let body = claim_aggregate(vec![
        claim_binding("declaring", reference(declaring)),
        claim_binding("operation", text_literal(&text, operation.name())?),
        claim_binding("family", text_literal(&text, family)?),
        claim_binding("parameters", parameters),
    ]);
    Ok(Pending {
        node: node(ids, "delay_distribution", &key, body),
        region,
    })
}

/// The `(tag, form)` of the type nodes FSpec FR-640 admits as a reward's
/// `value_type`: an integer, a unit, a compound unit, a carried or bounded
/// quantity, or an integer range.
const REWARD_TYPE_FORMS: [(&str, &str); 6] = [
    ("scalar_type", "integer"),
    ("scalar_type", "unit"),
    ("scalar_type", "compound_unit"),
    ("scalar_type", "carried_quantity"),
    ("scalar_type", "bounded_quantity"),
    ("bounded_domain", "integer_range"),
];

/// FSpec FR-640 `reward`: `declaring`, `operation`, `name`, `value_type` and
/// `expression`, the reference to the clause function S3 lowered the
/// reward's expression to. The `value_type` is the function's own type, which
/// the reader requires to be the reward's.
fn reward_node(
    ids: &StructuralIds,
    graph: &CheckedGraph,
    nodes: &mut Vec<CheckedSemanticNodeV2>,
    source_map: &mut Vec<CheckedSourceMapEntry>,
    reward: &Reward,
) -> Result<Pending, EmitRefusal> {
    let operation = reward.operation();
    let location = reward.declaration_location();
    let unwritable = || {
        refusal(
            graph,
            ProbabilisticFeature::Reward,
            reward.name().to_owned(),
            &location,
        )
    };
    let region = region_of(graph, &location)?;
    let declaring_key = reward.declaring_node().ok_or_else(unwritable)?.clone();
    let declaring = node_id(ids, declaring_key.clone());
    if !nodes.iter().any(|node| node.node_id == declaring) {
        return Err(unwritable());
    }
    // The key carries the content operand: the key of the expression node the
    // function computes (keyed by that expression's content), or a literal
    // body's text, because the function's own key holds no content. Changing
    // the expression changes the identity (FSpec FR-640-AC-3).
    let key = reward
        .node_key(graph.package_name())
        .ok_or_else(unwritable)?;
    let function_key = reward.function().ok_or_else(unwritable)?;
    let function = node_id(ids, function_key);
    let function_node = nodes
        .iter()
        .find(|node| node.node_id == function)
        .ok_or_else(unwritable)?;
    let value_type = function_node.semantic_type.clone();
    let typed = nodes.iter().any(|node| {
        node.node_id == value_type
            && REWARD_TYPE_FORMS
                .iter()
                .any(|(tag, form)| node.node_tag.as_ref() == *tag && &*node.semantic_form == *form)
    });
    if !typed {
        return Err(unwritable());
    }
    let text = scalar_type(ids, graph, nodes, source_map, "text", &region)?;
    let body = claim_aggregate(vec![
        claim_binding("declaring", reference(declaring)),
        claim_binding("operation", text_literal(&text, operation.name())?),
        claim_binding("name", text_literal(&text, reward.name())?),
        claim_binding("value_type", reference(value_type)),
        claim_binding("expression", reference(function)),
    ]);
    Ok(Pending {
        node: node(ids, "reward", &key, body),
        region,
    })
}

/// The term of `statement` in a claim body: a text as a text literal, a
/// reference as a reference to its emitted node, an exact value as a literal of
/// its type node. `None` when a node it names is not emitted.
fn statement_term(
    ids: &StructuralIds,
    nodes: &[CheckedSemanticNodeV2],
    text: &CheckedNodeId,
    statement: &Statement,
) -> Option<Result<CheckedTerm, EmitRefusal>> {
    let emitted = |key: &NodeKey| {
        let id = node_id(ids, key);
        nodes.iter().any(|node| node.node_id == id).then_some(id)
    };
    Some(match statement {
        Statement::Text(value) => text_literal(text, value),
        Statement::Ref(key) => Ok(reference(emitted(key)?)),
        Statement::Exact {
            kind,
            ty,
            text: value,
        } => {
            let kind = match *kind {
                "integer" => LiteralKind::Integer,
                "quantity" => LiteralKind::Quantity,
                _ => LiteralKind::Rational,
            };
            literal(&emitted(ty)?, kind, value)
        }
        Statement::Aggregate(members) => {
            let mut written = Vec::with_capacity(members.len());
            for (name, value) in members {
                match statement_term(ids, nodes, text, value)? {
                    Ok(value) => written.push(claim_binding(name, value)),
                    Err(refusal) => return Some(Err(refusal)),
                }
            }
            Ok(claim_aggregate(written))
        }
    })
}

/// FSpec FR-640 "Probabilistic claim node": write the body of each claim node
/// S3 keyed, the five bindings `subject`, `scheduler`, `reward`,
/// `distributions` and `delays`, referencing the nodes the claim's path holds.
/// Neither a table nor a sidecar row carries what a claim reads.
///
/// # Errors
///
/// A claim that is not keyed, or that reads a workload, reward or other node
/// that has no emitted node.
pub(super) fn write_claim_bodies(
    ids: &StructuralIds,
    graph: &CheckedGraph,
    nodes: &mut Vec<CheckedSemanticNodeV2>,
    source_map: &mut Vec<CheckedSourceMapEntry>,
) -> Result<(), EmitRefusal> {
    let model = graph.probabilistic_model();
    let package = graph.package_name();
    let rewards: Vec<Option<NodeKey>> = model
        .rewards()
        .iter()
        .map(|reward| reward.node_key(package))
        .collect();
    for claim in model.claims() {
        let unkeyed = || EmitRefusal::UnkeyedProbabilisticClaim {
            claim: claim.name.clone(),
        };
        let claim_id = claim
            .occurrence()
            .map(|occurrence| node_id(ids, occurrence.node()))
            .ok_or_else(unkeyed)?;
        let reads = claim.reads(package, model, &rewards);
        match &reads.fault {
            Some(ClaimReadFault::Workload(workload)) => {
                return Err(EmitRefusal::UnresolvedClaimWorkload {
                    claim: claim.name.clone(),
                    workload: workload.clone(),
                });
            }
            Some(ClaimReadFault::Reward(reward)) => {
                return Err(EmitRefusal::UnresolvedClaimReward {
                    claim: claim.name.clone(),
                    reward: reward.clone(),
                });
            }
            Some(ClaimReadFault::Subject) => return Err(EmitRefusal::SourceMapInvariant),
            None => {}
        }
        let region = region_of(graph, &claim.declaration_location())?;
        let text = scalar_type(ids, graph, nodes, source_map, "text", &region)?;
        // Each node the claim reads is emitted before it.
        let emitted = |key: &NodeKey| {
            let id = node_id(ids, key);
            nodes.iter().any(|node| node.node_id == id).then_some(id)
        };
        let refs = |keys: &[NodeKey]| {
            keys.iter()
                .map(|key| emitted(key).map(reference))
                .collect::<Option<Vec<_>>>()
        };
        let missing = || EmitRefusal::UnresolvedNodeReference;
        let subject = reads
            .subject
            .as_ref()
            .and_then(&emitted)
            .ok_or_else(missing)?;
        let scheduler = match &reads.scheduler {
            Some(SchedulerRead::Workload(workload)) => {
                let workload_id =
                    emitted(workload).ok_or_else(|| EmitRefusal::UnresolvedClaimWorkload {
                        claim: claim.name.clone(),
                        workload: match &claim.scheduler {
                            ProbScheduler::Workload(name) => name.clone(),
                            ProbScheduler::Every => String::new(),
                        },
                    })?;
                reference(workload_id)
            }
            Some(SchedulerRead::Every(fairness)) => {
                let mut members = Vec::with_capacity(fairness.len());
                for constraint in fairness {
                    let declaring = emitted(&constraint.declaring).ok_or_else(missing)?;
                    members.push(claim_aggregate(vec![
                        claim_binding("kind", text_literal(&text, constraint.kind)?),
                        claim_binding("granularity", text_literal(&text, constraint.granularity)?),
                        claim_binding("declaring", reference(declaring)),
                        claim_binding("operation", text_literal(&text, &constraint.operation)?),
                    ]));
                }
                claim_aggregate(vec![claim_binding("fairness", claim_aggregate(members))])
            }
            None => return Err(missing()),
        };
        let read_rewards =
            refs(&reads.rewards).ok_or_else(|| EmitRefusal::UnresolvedClaimReward {
                claim: claim.name.clone(),
                reward: claim.reward_read().unwrap_or_default().to_owned(),
            })?;
        let distributions = refs(&reads.distributions).ok_or_else(missing)?;
        let delays = refs(&reads.delays).ok_or_else(missing)?;
        let statement = claim.statement().ok_or_else(unkeyed)?;
        let CheckedTerm::Aggregate { members: stated } =
            statement_term(ids, nodes, &text, statement).ok_or_else(missing)??
        else {
            return Err(EmitRefusal::SourceMapInvariant);
        };
        let mut members = vec![
            claim_binding("subject", reference(subject)),
            claim_binding("scheduler", scheduler),
            claim_binding("reward", claim_aggregate(read_rewards)),
            claim_binding("distributions", claim_aggregate(distributions)),
            claim_binding("delays", claim_aggregate(delays)),
        ];
        members.extend(stated);
        let body = CheckedTerm::Aggregate { members };
        let written = nodes
            .iter_mut()
            .find(|node| node.node_id == claim_id)
            .ok_or_else(unkeyed)?;
        written.dependencies = body_reference_targets(&body);
        written.body = body;
    }
    Ok(())
}
