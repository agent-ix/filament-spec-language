// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940 / FSpec FR-825: carry the checker's scalar requirement facts in V2.

use std::collections::{BTreeMap, HashMap};

use filament_contract_model::{
    CapabilityKind, CheckedNodeId, CheckedNodeOwner, CheckedOccurrence, CheckedOccurrenceRole,
    CheckedSemanticNodeV2, CheckedSourceMapEntry, CheckedSourceRegion, CheckedTerm, DomainKey,
    DomainKind, LiteralKind, RequirementExtent, RequirementGuard, RequirementItemRow,
    ValueValidityRecord,
};
use filament_exact::{NodeKey, PathSegment};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_semantics::check::wire_id::StructuralIds;
use fsl_semantics::check::{Capability, CheckedGraph, NodeTag};
use fsl_semantics::family::ClaimExtent;

use super::{
    admitted_key, body_reference_targets, checked_literal_json, checked_occurrence,
    claim_aggregate, claim_binding, encoding, node_id, push_scalar_type, source_artifact,
    EmitRefusal, GRAPH_V2,
};

type Records = HashMap<OccurrenceKey, ValueValidityRecord>;

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
) -> Result<Records, EmitRefusal> {
    let mut records = HashMap::new();
    if !graph
        .requirements()
        .values()
        .any(|record| record.requirements().kind() == Capability::ValueValidity)
    {
        return Ok(records);
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
    let mut owners = HashMap::<String, (NodeKey, CheckedNodeId, Option<CheckedNodeOwner>)>::new();
    for (claim, source) in graph.requirements().iter() {
        if source.requirements().kind() != Capability::ValueValidity {
            continue;
        }
        let owner_name = source.owner().ok_or(EmitRefusal::SourceMapInvariant)?;
        let (owner_key, owner, owner_metadata) = match owners.entry(owner_name.to_owned()) {
            std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::hash_map::Entry::Vacant(entry) => {
                let key = graph
                    .function_identity(owner_name)
                    .ok_or(EmitRefusal::SourceMapInvariant)?;
                let id = node_id(ids, &key);
                let metadata = nodes
                    .iter()
                    .find(|node| node.node_id == id)
                    .ok_or(EmitRefusal::SourceMapInvariant)?
                    .owner
                    .clone();
                entry.insert((key, id, metadata))
            }
        };
        let application = node_id(ids, claim.node());
        let result_bound = node_id(ids, source.result_bound().node());
        let location = graph
            .occurrence(claim.node().clone(), claim.origin())
            .ok_or(EmitRefusal::SourceMapInvariant)?;
        let region = graph
            .region(location)
            .ok_or(EmitRefusal::SourceMapInvariant)?;
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
            owner: owner_metadata.clone(),
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
    Ok(records)
}

pub(super) fn rows(
    ids: &StructuralIds,
    graph: &CheckedGraph,
    emitted: &HashMap<NodeKey, CheckedNodeId>,
    records: &Records,
    existing: &[RequirementItemRow],
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
