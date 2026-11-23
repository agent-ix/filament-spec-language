// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940 / FSpec FR-825: carry the checker's scalar requirement facts in V2.

use std::collections::{BTreeMap, HashMap, HashSet};

use filament_contract_model::{
    CapabilityKind, CheckedNodeId, CheckedOccurrence, CheckedOccurrenceRole, CheckedSemanticNodeV2,
    CheckedSourceMapEntry, CheckedSourceRegion, CheckedTerm, DomainKey, DomainKind, LiteralKind,
    RequirementExtent, RequirementGuard, RequirementItemRow, ValueValidityRecord,
};
use filament_exact::{NodeKey, PathSegment};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_semantics::check::wire_id::StructuralIds;
use fsl_semantics::check::{Capability, CheckedGraph, NodeTag, RequirementRecord};
use fsl_semantics::family::ClaimExtent;

use super::{
    admitted_key, body_reference_targets, checked_literal_json, checked_occurrence,
    claim_aggregate, claim_binding, encoding, node_id, push_scalar_type, source_artifact,
    EmitRefusal, OmissionCause, OmittedNode, GRAPH_V2,
};

type Records = HashMap<OccurrenceKey, ValueValidityRecord>;

/// The least node an omitted record names, when the emitter omits one of
/// them (FSL FR-062 "Packaging is all-or-nothing"): its claim node and its
/// extent's domain nodes, and for a `value-validity` record the nodes its
/// `claim`/`value_validity` node names (FSpec FR-825): the owner function,
/// the result bound's type node, the narrow, every guard, every extent root
/// and every object type. Such a record has no row and no record node,
/// because each would name a node the package does not hold.
fn omitted_name(
    graph: &CheckedGraph,
    claim: &OccurrenceKey,
    source: &RequirementRecord,
    omitted: &HashSet<NodeKey>,
) -> Result<Option<NodeKey>, EmitRefusal> {
    if omitted.is_empty() {
        return Ok(None);
    }
    let mut names = vec![claim.node().clone()];
    if let ClaimExtent::Unbounded(domains) = source.requirements().extent() {
        names.extend(domains.iter().map(|(key, _)| match key {
            fsl_foundation::bound::DomainKey::Node { node, .. } => node.clone(),
            fsl_foundation::bound::DomainKey::Population { member_type, .. } => member_type.clone(),
        }));
    }
    if source.requirements().kind() == Capability::ValueValidity {
        let owner = source.owner().ok_or(EmitRefusal::SourceMapInvariant)?;
        names.push(
            graph
                .function_identity(owner)
                .ok_or(EmitRefusal::SourceMapInvariant)?,
        );
        names.push(source.result_bound().node());
        names.extend(source.conversion_context().map(|key| key.node().clone()));
        names.extend(
            source
                .path_condition()
                .iter()
                .map(|guard| guard.occurrence().node().clone()),
        );
        names.extend(source.extent_roots().iter().cloned());
        names.extend(
            graph
                .item_object_nodes(claim)
                .ok_or(EmitRefusal::SourceMapInvariant)?
                .map_err(|_| EmitRefusal::SourceMapInvariant)?,
        );
    }
    Ok(names
        .into_iter()
        .filter(|name| omitted.contains(name))
        .min())
}

fn reference(target: &CheckedNodeId) -> CheckedTerm {
    CheckedTerm::Reference {
        target: target.clone(),
    }
}

fn count(integer: &CheckedNodeId, value: u64) -> Result<CheckedTerm, EmitRefusal> {
    Ok(CheckedTerm::Literal {
        type_id: integer.clone(),
        value_kind: LiteralKind::Integer,
        value: checked_literal_json(&value)?,
    })
}

pub(super) fn append(
    ids: &StructuralIds,
    graph: &CheckedGraph,
    nodes: &mut Vec<CheckedSemanticNodeV2>,
    source_map: &mut Vec<CheckedSourceMapEntry>,
    regions: &impl Fn(&OccurrenceKey) -> Option<fsl_foundation::source::provenance::SourceRegion>,
    omitted: &HashSet<NodeKey>,
) -> Result<(Records, Vec<OmittedNode>), EmitRefusal> {
    let mut records = HashMap::new();
    let mut omissions = Vec::new();
    if !graph
        .requirements()
        .values()
        .any(|record| record.requirements().kind() == Capability::ValueValidity)
    {
        return Ok((records, omissions));
    }
    let integer = node_id(
        ids,
        fsl_semantics::check::builtin_integer_type_key(graph.package_name()).map_err(encoding)?,
    );
    let boolean = node_id(
        ids,
        fsl_semantics::check::builtin_boolean_type_key(graph.package_name()).map_err(encoding)?,
    );
    let mut types_written = false;
    let mut ordinals = BTreeMap::<String, u64>::new();
    let mut owners = HashMap::<String, (NodeKey, CheckedNodeId)>::new();
    for (claim, source) in graph.requirements().iter() {
        if source.requirements().kind() != Capability::ValueValidity {
            continue;
        }
        let owner_name = source.owner().ok_or(EmitRefusal::SourceMapInvariant)?;
        let (owner_key, owner) = match owners.entry(owner_name.to_owned()) {
            std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::hash_map::Entry::Vacant(entry) => {
                let key = graph
                    .function_identity(owner_name)
                    .ok_or(EmitRefusal::SourceMapInvariant)?;
                let id = node_id(ids, &key);
                if !omitted.contains(&key) && !nodes.iter().any(|node| node.node_id == id) {
                    return Err(EmitRefusal::SourceMapInvariant);
                }
                entry.insert((key, id))
            }
        };
        let omitted_name = omitted_name(graph, claim, source, omitted)?;
        let application = node_id(ids, claim.node());
        let result_bound = node_id(ids, source.result_bound().node());
        let conversion = source
            .conversion_context()
            .map(|key| (node_id(ids, key.node()), key.origin().ordinal()));
        let guards = source
            .path_condition()
            .iter()
            .map(|guard| RequirementGuard {
                guard: node_id(ids, guard.occurrence().node()),
                occurrence: guard.occurrence().origin().ordinal(),
                holds: guard.holds(),
            })
            .collect::<Vec<_>>();
        let roots = source
            .extent_roots()
            .iter()
            .map(|key| node_id(ids, key))
            .collect::<Vec<_>>();
        let mut objects = graph
            .item_object_nodes(claim)
            .ok_or(EmitRefusal::SourceMapInvariant)?
            .map_err(|_| EmitRefusal::SourceMapInvariant)?
            .iter()
            .map(|key| node_id(ids, key))
            .collect::<Vec<_>>();
        objects.sort();
        let mut operands = vec![
            PathSegment::Node(owner_key.clone()),
            PathSegment::Node(claim.node().clone()),
            PathSegment::Text(claim.origin().ordinal().to_string()),
            PathSegment::Text("bound".to_owned()),
            PathSegment::Node(source.result_bound().node()),
            PathSegment::Text("conversion".to_owned()),
        ];
        if let Some(context) = source.conversion_context() {
            operands.push(PathSegment::Node(context.node().clone()));
            operands.push(PathSegment::Text(context.origin().ordinal().to_string()));
        }
        operands.push(PathSegment::Text("guards".to_owned()));
        for guard in source.path_condition() {
            operands.extend([
                PathSegment::Node(guard.occurrence().node().clone()),
                PathSegment::Text(guard.occurrence().origin().ordinal().to_string()),
                PathSegment::Text(guard.holds().to_string()),
            ]);
        }
        operands.push(PathSegment::Text("roots".to_owned()));
        operands.extend(source.extent_roots().iter().cloned().map(PathSegment::Node));
        operands.push(PathSegment::Text("objects".to_owned()));
        for object in &objects {
            operands.push(PathSegment::Node(admitted_key(object)?));
        }
        let key = fsl_semantics::check::derived_node_key(
            graph.package_name(),
            NodeTag::Claim,
            "value_validity",
            operands,
            0,
        )
        .map_err(encoding)?;
        let id = node_id(ids, key);
        if let Some(name) = omitted_name {
            omissions.push(OmittedNode {
                node: id,
                cause: OmissionCause::NamesOmittedNode(node_id(ids, name)),
            });
            continue;
        }
        let region = regions(claim).ok_or(EmitRefusal::SourceMapInvariant)?;
        let region = CheckedSourceRegion {
            source: source_artifact(region.source()),
            start: region.start(),
            end: region.end(),
        };
        if !types_written {
            push_scalar_type(nodes, source_map, &integer, "integer", &region);
            push_scalar_type(nodes, source_map, &boolean, "boolean", &region);
            types_written = true;
        }
        let ordinal = ordinals.entry(owner_name.to_owned()).or_default();
        let occurrence = CheckedOccurrence {
            role: CheckedOccurrenceRole::Generated,
            ordinal: *ordinal,
        };
        *ordinal = ordinal
            .checked_add(1)
            .ok_or(EmitRefusal::SourceMapInvariant)?;
        let conversion_term = match &conversion {
            Some((narrow, ordinal)) => claim_aggregate(vec![
                claim_binding("narrow", reference(narrow)),
                claim_binding("occurrence", count(&integer, *ordinal)?),
            ]),
            None => claim_aggregate(Vec::new()),
        };
        let guard_terms = guards
            .iter()
            .map(|guard| {
                Ok(claim_aggregate(vec![
                    claim_binding("guard", reference(&guard.guard)),
                    claim_binding("occurrence", count(&integer, guard.occurrence)?),
                    claim_binding(
                        "holds",
                        CheckedTerm::Literal {
                            type_id: boolean.clone(),
                            value_kind: LiteralKind::Boolean,
                            value: checked_literal_json(&guard.holds)?,
                        },
                    ),
                ]))
            })
            .collect::<Result<Vec<_>, EmitRefusal>>()?;
        let body = claim_aggregate(vec![
            claim_binding("owner", reference(&owner)),
            claim_binding("application", reference(&application)),
            claim_binding("occurrence", count(&integer, claim.origin().ordinal())?),
            claim_binding("result_bound", reference(&result_bound)),
            claim_binding("conversion", conversion_term),
            claim_binding("path_condition", claim_aggregate(guard_terms)),
            claim_binding(
                "extent_roots",
                claim_aggregate(roots.iter().map(reference).collect()),
            ),
            claim_binding(
                "objects",
                claim_aggregate(objects.iter().map(reference).collect()),
            ),
        ]);
        nodes.push(CheckedSemanticNodeV2 {
            node_id: id.clone(),
            schema_version: GRAPH_V2.into(),
            node_tag: "claim".into(),
            semantic_form: "value_validity".into(),
            semantic_type: boolean.clone(),
            dependencies: body_reference_targets(&body),
            occurrences: vec![occurrence.clone()],
            recursion_group: None,
            declaration: None,
            // A generated claim has no wire owner; its function is the
            // explicit owner reference in the record body.
            owner: None,
            owner_model: None,
            check_mode: None,
            body,
        });
        source_map.push(CheckedSourceMapEntry {
            node_id: id.clone(),
            role: occurrence.role,
            ordinal: occurrence.ordinal,
            regions: vec![region],
        });
        if records
            .insert(
                claim.clone(),
                ValueValidityRecord {
                    node: id,
                    owner: owner.clone(),
                    application,
                    occurrence: claim.origin().ordinal(),
                    result_bound,
                    conversion,
                    path_condition: guards,
                    extent_roots: roots,
                    objects,
                },
            )
            .is_some()
        {
            return Err(EmitRefusal::SourceMapInvariant);
        }
    }
    Ok((records, omissions))
}

pub(super) fn rows(
    ids: &StructuralIds,
    graph: &CheckedGraph,
    emitted: &HashMap<NodeKey, CheckedNodeId>,
    records: &Records,
    existing: &[RequirementItemRow],
    omitted: &HashSet<NodeKey>,
) -> Result<Vec<RequirementItemRow>, EmitRefusal> {
    // Sidecar references remain document-held. Only the record body goes
    // through the containing node's operand-slot writer.
    let mut keys = ids.key_reader();
    let mut remap = |id: &CheckedNodeId| -> Result<CheckedNodeId, EmitRefusal> {
        emitted
            .get(&keys.key(id).map_err(|_| EmitRefusal::SourceMapInvariant)?)
            .cloned()
            .ok_or(EmitRefusal::SourceMapInvariant)
    };
    let existing: BTreeMap<_, _> = existing
        .iter()
        .map(|row| ((row.claim.clone(), row.occurrence.clone()), row))
        .collect();
    let mut rows = Vec::new();
    for (claim, source) in graph.requirements().iter() {
        if omitted_name(graph, claim, source, omitted)?.is_some() {
            continue;
        }
        let kind = match source.requirements().kind() {
            Capability::ValueValidity => CapabilityKind::ValueValidity,
            Capability::OperationContract => CapabilityKind::OperationContract,
            Capability::FiniteReplay => CapabilityKind::FiniteReplay,
            Capability::TemporalSatisfaction => CapabilityKind::TemporalSatisfaction,
            Capability::GlobalConformance => CapabilityKind::GlobalConformance,
            Capability::Monitorability => CapabilityKind::Monitorability,
            Capability::LocalProjection => CapabilityKind::LocalProjection,
            Capability::Refinement => CapabilityKind::Refinement,
            Capability::Realizability => CapabilityKind::Realizability,
            Capability::Composition => CapabilityKind::Composition,
            Capability::ProbabilisticSatisfaction => CapabilityKind::ProbabilisticSatisfaction,
            Capability::Schedulability => CapabilityKind::Schedulability,
        };
        let claim_id =
            emitted
                .get(claim.node())
                .cloned()
                .ok_or(EmitRefusal::RequirementNotEmitted {
                    kind: source.requirements().kind().to_wire(),
                    shape: "claim-node",
                })?;
        let occurrence =
            checked_occurrence(claim.origin()).ok_or(EmitRefusal::RequirementNotEmitted {
                kind: source.requirements().kind().to_wire(),
                shape: "claim-occurrence",
            })?;
        if let Some(row) = existing
            .get(&(claim_id.clone(), occurrence.clone()))
            .filter(|row| row.kind == kind)
        {
            rows.push((**row).clone());
            continue;
        }
        let extent = match source.requirements().extent() {
            ClaimExtent::Bounded => RequirementExtent::Bounded,
            ClaimExtent::Unbounded(domains) => RequirementExtent::Unbounded(
                domains
                    .iter()
                    .map(|(key, kind)| {
                        let key = match key {
                            fsl_foundation::bound::DomainKey::Node { node, path } => {
                                DomainKey::Node {
                                    node: emitted
                                        .get(node)
                                        .cloned()
                                        .ok_or(EmitRefusal::SourceMapInvariant)?,
                                    path: path.clone(),
                                }
                            }
                            fsl_foundation::bound::DomainKey::Population {
                                member_type,
                                ordinal,
                            } => DomainKey::Population {
                                member_type: emitted
                                    .get(member_type)
                                    .cloned()
                                    .ok_or(EmitRefusal::SourceMapInvariant)?,
                                ordinal: *ordinal,
                            },
                        };
                        let kind = match kind {
                            fsl_foundation::bound::DomainKind::Collection => DomainKind::Collection,
                            fsl_foundation::bound::DomainKind::Population => DomainKind::Population,
                            fsl_foundation::bound::DomainKind::Integer => DomainKind::Integer,
                            fsl_foundation::bound::DomainKind::Recursive => DomainKind::Recursive,
                            fsl_foundation::bound::DomainKind::Quantity => DomainKind::Quantity,
                            fsl_foundation::bound::DomainKind::Loop => DomainKind::Loop,
                            fsl_foundation::bound::DomainKind::InfiniteTrace => {
                                DomainKind::InfiniteTrace
                            }
                        };
                        Ok((key, kind))
                    })
                    .collect::<Result<_, EmitRefusal>>()?,
            ),
        };
        let record = if kind == CapabilityKind::ValueValidity {
            let record = records.get(claim).ok_or(EmitRefusal::SourceMapInvariant)?;
            Some(ValueValidityRecord {
                node: remap(&record.node)?,
                owner: remap(&record.owner)?,
                application: remap(&record.application)?,
                occurrence: record.occurrence,
                result_bound: remap(&record.result_bound)?,
                conversion: record
                    .conversion
                    .as_ref()
                    .map(|(node, ordinal)| -> Result<_, EmitRefusal> {
                        Ok((remap(node)?, *ordinal))
                    })
                    .transpose()?,
                path_condition: record
                    .path_condition
                    .iter()
                    .map(|guard| {
                        Ok(RequirementGuard {
                            guard: remap(&guard.guard)?,
                            occurrence: guard.occurrence,
                            holds: guard.holds,
                        })
                    })
                    .collect::<Result<_, EmitRefusal>>()?,
                extent_roots: record
                    .extent_roots
                    .iter()
                    .map(&mut remap)
                    .collect::<Result<_, _>>()?,
                objects: record
                    .objects
                    .iter()
                    .map(&mut remap)
                    .collect::<Result<_, _>>()?,
            })
        } else {
            None
        };
        rows.push(RequirementItemRow {
            claim: claim_id,
            occurrence,
            kind,
            extent,
            record,
        });
    }
    Ok(rows)
}

/// The base package's requirement rows, for a writer that re-encodes an
/// admitted package. FSpec FR-825: a `claim`/`value_validity` node is valid
/// only when a requirement row names it, so a writer that carries the
/// record nodes carries their rows too.
///
/// `nodes` is the writer's graph in index-segment form before operand slots
/// are written, holding the base graph's nodes at their base positions. Each
/// local id a row holds is re-pointed to the id of the node at its base
/// position, so a node the writer renamed is named by its new id; an id that
/// names no local node (a dependency node) is kept.
pub(super) fn carry_rows(
    base: &filament_contract_model::CheckedPackageV2,
    nodes: &[CheckedSemanticNodeV2],
) -> Result<Vec<RequirementItemRow>, EmitRefusal> {
    let rows = base.requirement_items();
    if rows.is_empty() {
        return Ok(Vec::new());
    }
    let base_nodes = &base.resolved_graph().nodes;
    let finals = nodes
        .get(..base_nodes.len())
        .ok_or(EmitRefusal::SourceMapInvariant)?;
    let renamed: BTreeMap<&CheckedNodeId, &CheckedNodeId> = base_nodes
        .iter()
        .zip(finals)
        .filter(|(old, new)| old.node_id != new.node_id)
        .map(|(old, new)| (&old.node_id, &new.node_id))
        .collect();
    let follow = |id: &mut CheckedNodeId| {
        if let Some(&new) = renamed.get(&*id) {
            *id = new.clone();
        }
    };
    let mut rows = rows.to_vec();
    for row in &mut rows {
        follow(&mut row.claim);
        if let RequirementExtent::Unbounded(domains) = &mut row.extent {
            for (domain, _) in domains {
                match domain {
                    DomainKey::Node { node, .. } => follow(node),
                    DomainKey::Population { member_type, .. } => follow(member_type),
                }
            }
        }
        if let Some(record) = &mut row.record {
            follow(&mut record.node);
            follow(&mut record.owner);
            follow(&mut record.application);
            follow(&mut record.result_bound);
            if let Some((narrow, _)) = &mut record.conversion {
                follow(narrow);
            }
            for guard in &mut record.path_condition {
                follow(&mut guard.guard);
            }
            record.extent_roots.iter_mut().for_each(follow);
            record.objects.iter_mut().for_each(follow);
        }
    }
    Ok(rows)
}

#[cfg(test)]
mod tests;
