// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940 / FSpec FR-825: carry the checker's scalar requirement facts in V2.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use filament_contract_model::{
    CapabilityKind, CheckedNodeId, CheckedOccurrence, CheckedOccurrenceRole, CheckedSemanticNodeV2,
    CheckedSourceMapEntry, CheckedSourceRegion, CheckedTerm, DomainKey, DomainKind, LiteralKind,
    OperationContractRecord, OperationPopulation, PathConditionFrame, PathConditionFrameId,
    RequirementExtent, RequirementItemRow, RequirementRecord as WireRecord, ValueValidityRecord,
};
use filament_exact::{NodeKey, PathSegment};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_semantics::check::wire_id::StructuralIds;
use fsl_semantics::check::{Capability, CheckedGraph, NodeTag, RequirementRecord};
use fsl_semantics::family::ClaimExtent;

use super::{
    EmitRefusal, GRAPH_V2, OmissionCause, OmittedNode, admitted_key, body_reference_targets,
    checked_literal_json, checked_occurrence, claim_aggregate, claim_binding, claim_literal,
    encoding, node_id, push_scalar_type, source_artifact,
};

mod frames;
pub(in crate::emit) mod ordinals;

type Records = HashMap<OccurrenceKey, ValueValidityRecord>;
type OperationRecords = HashMap<OccurrenceKey, OperationContractRecord>;

/// The omitted graph nodes, each with its omission cause.
type Omitted = HashMap<NodeKey, OmissionCause>;

/// The record nodes [`append`] writes, by claim, and the record nodes it
/// omits because their owner function is omitted.
pub(super) struct Appended {
    pub(super) records: Records,
    /// Operation-contract record nodes and their exact typed references.
    pub(super) operation_records: OperationRecords,
    /// The shared path-condition frames the records cite (FSpec FR-825),
    /// each guard in the id space of the emitted nodes; empty when no record
    /// is guarded.
    pub(super) frames: Vec<PathConditionFrame>,
    /// Each with cause `NamesOmittedNode` naming its omitted owner.
    pub(super) omitted: Vec<OmittedNode>,
    /// The expression-occurrence renumbering the records and rows follow
    /// (FSpec FR-322).
    pub(super) renumbering: ordinals::Renumbering,
}

/// What a requirement record names (FSpec FR-825), grouped by what its
/// omission takes with it in the omission closure (FR-062 "Packaging is
/// all-or-nothing").
pub(super) struct RecordNames {
    /// The record's claim node; for a `value-validity` record, its
    /// application, which other owners' records may share.
    pub(super) claim: NodeKey,
    /// The owner function of a `value-validity` record, whose body holds the
    /// record's occurrence; `None` for every other kind.
    pub(super) owner: Option<NodeKey>,
    /// Every other node the record names: its extent's domain nodes, and for
    /// a `value-validity` record the result bound's type node, the narrow,
    /// every extent root and every object type. Its guards are named through
    /// `guards`, one shared chain, never one vector per record.
    pub(super) names: Vec<NodeKey>,
    /// The terminal row, in [`GuardRows`], of the record's guard chain.
    pub(super) guards: Option<usize>,
}

/// The guard chains of every record, interned as parent-linked rows holding
/// the guard node each names: one row per distinct guard edge.
#[derive(Default)]
pub(super) struct GuardRows {
    /// Each row's enclosing row and guard node.
    pub(super) rows: Vec<(Option<usize>, NodeKey)>,
    lookup: HashMap<(Option<usize>, NodeKey), usize>,
}

impl GuardRows {
    fn intern(&mut self, parent: Option<usize>, guard: &NodeKey) -> usize {
        if let Some(&row) = self.lookup.get(&(parent, guard.clone())) {
            return row;
        }
        self.rows.push((parent, guard.clone()));
        self.lookup
            .insert((parent, guard.clone()), self.rows.len() - 1);
        self.rows.len() - 1
    }
}

/// The names of one record, its guards as plain names when `guards` is `None`
/// and as a row of the shared `guards` otherwise.
fn record_names_of(
    graph: &CheckedGraph,
    claim: &OccurrenceKey,
    source: &RequirementRecord,
    mut guards: Option<&mut GuardRows>,
) -> Result<RecordNames, EmitRefusal> {
    let mut terminal = None;
    let mut names = Vec::new();
    if let ClaimExtent::Unbounded(domains) = source.requirements().extent() {
        names.extend(domains.iter().map(|(key, _)| match key {
            fsl_foundation::bound::DomainKey::Node { node, .. } => node.clone(),
            fsl_foundation::bound::DomainKey::Population { member_type, .. } => member_type.clone(),
        }));
    }
    let mut owner = None;
    if source.requirements().kind() == Capability::ValueValidity {
        let name = source.owner().ok_or(EmitRefusal::SourceMapInvariant)?;
        owner = Some(
            graph
                .function_identity(name)
                .ok_or(EmitRefusal::SourceMapInvariant)?,
        );
        names.push(source.result_bound().node());
        names.extend(source.conversion_context().map(|key| key.node().clone()));
        match guards {
            Some(rows) => {
                for guard in source.path_condition() {
                    terminal = Some(rows.intern(terminal, guard.occurrence().node()));
                }
            }
            None => names.extend(
                source
                    .path_condition()
                    .iter()
                    .map(|guard| guard.occurrence().node().clone()),
            ),
        }
        names.extend(source.extent_roots().iter().cloned());
        names.extend(
            graph
                .item_object_nodes(claim)
                .ok_or(EmitRefusal::SourceMapInvariant)?
                .map_err(|_| EmitRefusal::SourceMapInvariant)?,
        );
    } else if source.requirements().kind() == Capability::OperationContract {
        names.push(source.result_bound().node());
        let references = graph
            .item_operation_contract_references(claim)
            .ok_or(EmitRefusal::SourceMapInvariant)?
            .map_err(|_| EmitRefusal::SourceMapInvariant)?;
        names.extend(references.operation);
        names.extend(references.model_objects);
        names.extend(
            references
                .populations
                .into_iter()
                .map(|population| population.member_type),
        );
    }
    Ok(RecordNames {
        claim: claim.node().clone(),
        owner,
        names,
        guards: terminal,
    })
}

/// What each requirement record of `graph` names, in requirement-map order,
/// with the guard chains the records share.
pub(super) fn record_names(
    graph: &CheckedGraph,
) -> Result<(Vec<RecordNames>, GuardRows), EmitRefusal> {
    let mut guards = GuardRows::default();
    // A relation-owned `operation-contract` record is never written (FSpec
    // FR-825, FR-290), so it names nothing the omission closure could follow.
    let records = graph
        .requirements()
        .iter()
        .filter(|(claim, source)| {
            !(source.requirements().kind() == Capability::OperationContract
                && relation_owned(graph, claim))
        })
        .map(|(claim, source)| record_names_of(graph, claim, source, Some(&mut guards)))
        .collect::<Result<_, _>>()?;
    Ok((records, guards))
}

/// Whether a record the emitter writes names an omitted node. The omission
/// closure omits the owner of a `value-validity` record, or the claim node of
/// any other record, that names an omitted node, so this is an invariant
/// check: a record is never dropped while its owner or claim is written.
fn names_an_omitted_node(
    graph: &CheckedGraph,
    claim: &OccurrenceKey,
    source: &RequirementRecord,
    omitted: &Omitted,
) -> Result<bool, EmitRefusal> {
    if omitted.is_empty() {
        return Ok(false);
    }
    let names = record_names_of(graph, claim, source, None)?;
    Ok(std::iter::once(&names.claim)
        .chain(&names.owner)
        .chain(&names.names)
        .any(|name| omitted.contains_key(name)))
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

fn occurrence_role_wire(role: &CheckedOccurrenceRole) -> &'static str {
    match role {
        CheckedOccurrenceRole::Declaration => "declaration",
        CheckedOccurrenceRole::Type => "type",
        CheckedOccurrenceRole::Expression => "expression",
        CheckedOccurrenceRole::Anchor => "anchor",
        CheckedOccurrenceRole::Claim => "claim",
        CheckedOccurrenceRole::Generated => "generated",
    }
}

struct OperationContractKey<'a> {
    claim: &'a OccurrenceKey,
    occurrence: &'a CheckedOccurrence,
    result_bound: &'a NodeKey,
    invariant: bool,
    operation: Option<&'a NodeKey>,
    model_objects: &'a [NodeKey],
    populations: &'a [fsl_semantics::check::CheckedOperationPopulation],
    extent: &'a ClaimExtent,
}

fn operation_contract_key(input: OperationContractKey<'_>) -> Result<NodeKey, EmitRefusal> {
    let OperationContractKey {
        claim,
        occurrence,
        result_bound,
        invariant,
        operation,
        model_objects,
        populations,
        extent,
    } = input;
    // `derived_node_key` writes the `claim`/`operation_contract` prefix.
    let mut operands = vec![
        PathSegment::Node(claim.node().clone()),
        PathSegment::Text(occurrence_role_wire(&occurrence.role).to_owned()),
        PathSegment::Text(occurrence.ordinal.to_string()),
        PathSegment::Text("bound".to_owned()),
        PathSegment::Node(result_bound.clone()),
        PathSegment::Text("invariant".to_owned()),
        PathSegment::Text(if invariant { "true" } else { "false" }.to_owned()),
        PathSegment::Text("operation".to_owned()),
    ];
    match operation {
        Some(operation) => operands.push(PathSegment::Node(operation.clone())),
        None => operands.push(PathSegment::Text("none".to_owned())),
    }
    operands.push(PathSegment::Text("objects".to_owned()));
    operands.push(PathSegment::Text(model_objects.len().to_string()));
    operands.extend(model_objects.iter().cloned().map(PathSegment::Node));
    operands.push(PathSegment::Text("populations".to_owned()));
    operands.push(PathSegment::Text(populations.len().to_string()));
    for population in populations {
        operands.push(PathSegment::Text("population".to_owned()));
        operands.push(PathSegment::Node(population.member_type.clone()));
        operands.push(PathSegment::Text(population.ordinal.to_string()));
        operands.push(PathSegment::Text(
            population
                .maximum
                .map_or_else(|| "none".to_owned(), |maximum| maximum.to_string()),
        ));
    }
    operands.push(PathSegment::Text("extent".to_owned()));
    match extent {
        ClaimExtent::Bounded => operands.push(PathSegment::Text("bounded".to_owned())),
        ClaimExtent::Unbounded(domains) => {
            operands.push(PathSegment::Text("unbounded".to_owned()));
            operands.push(PathSegment::Text(domains.len().to_string()));
            for (domain, kind) in domains.iter() {
                match domain {
                    fsl_foundation::bound::DomainKey::Node { node, path } => {
                        operands.push(PathSegment::Text("node".to_owned()));
                        operands.push(PathSegment::Node(node.clone()));
                        operands.push(PathSegment::Text(path.len().to_string()));
                        operands.extend(
                            path.iter()
                                .map(|index| PathSegment::Text(index.to_string())),
                        );
                    }
                    fsl_foundation::bound::DomainKey::Population {
                        member_type,
                        ordinal,
                    } => {
                        operands.push(PathSegment::Text("population".to_owned()));
                        operands.push(PathSegment::Node(member_type.clone()));
                        operands.push(PathSegment::Text(ordinal.to_string()));
                    }
                }
                operands.push(PathSegment::Text(kind.to_wire().to_owned()));
            }
        }
    }
    fsl_semantics::check::derived_node_key(
        claim.node().package(),
        NodeTag::Claim,
        "operation_contract",
        operands,
        0,
    )
    .map_err(encoding)
}

struct OperationContractAppend<'a> {
    ids: &'a StructuralIds,
    graph: &'a CheckedGraph,
    claim: &'a OccurrenceKey,
    source: &'a RequirementRecord,
    nodes: &'a mut Vec<CheckedSemanticNodeV2>,
    source_map: &'a mut Vec<CheckedSourceMapEntry>,
    region: &'a dyn Fn(&OccurrenceKey) -> Option<fsl_foundation::source::provenance::SourceRegion>,
    omitted: &'a Omitted,
    written: &'a BTreeSet<CheckedNodeId>,
    omissions: &'a mut Vec<OmittedNode>,
    records: &'a mut OperationRecords,
    ordinal: &'a mut u64,
    types_written: &'a mut bool,
    text_type: &'a CheckedNodeId,
    integer: &'a CheckedNodeId,
    boolean: &'a CheckedNodeId,
}

fn append_operation_contract(input: OperationContractAppend<'_>) -> Result<(), EmitRefusal> {
    let OperationContractAppend {
        ids,
        graph,
        claim,
        source,
        nodes,
        source_map,
        region,
        omitted,
        written,
        omissions,
        records,
        ordinal,
        types_written,
        text_type,
        integer,
        boolean,
    } = input;
    let checked_occurrence =
        checked_occurrence(claim.origin()).ok_or(EmitRefusal::RequirementNotEmitted {
            kind: source.requirements().kind().to_wire(),
            shape: "claim-occurrence",
        })?;
    let references = graph
        .item_operation_contract_references(claim)
        .ok_or(EmitRefusal::SourceMapInvariant)?
        .map_err(|_| EmitRefusal::SourceMapInvariant)?;
    let operation_key = references.operation;
    let model_object_keys = references.model_objects;
    let population_facts = references.populations;
    let key = operation_contract_key(OperationContractKey {
        claim,
        occurrence: &checked_occurrence,
        result_bound: &source.result_bound().node(),
        invariant: source.requirements().invariant_clause(),
        operation: operation_key.as_ref(),
        model_objects: &model_object_keys,
        populations: &population_facts,
        extent: source.requirements().extent(),
    })?;
    let id = node_id(ids, key);
    let claim_key = claim.node();
    let claim_id = node_id(ids, claim_key);
    // FSpec FR-825: the record references its state-clause claim or operation
    // frame node, and every reference SHALL resolve in the package. A claim the
    // emitter writes no node for (a timed presence clause, FR-062 "Packaging
    // is all-or-nothing") leaves no node for the record to name, so the record
    // goes with it, as with an omitted claim.
    if omitted.contains_key(claim_key) {
        omissions.push(OmittedNode {
            node: id,
            cause: OmissionCause::NamesOmittedNode(claim_id),
        });
        return Ok(());
    }
    if !written.contains(&claim_id) {
        // The emitter decides to write no node for this claim, so the claim is
        // itself an omitted node (once, however many records name it) and the
        // record goes with it: an omission names a node that is omitted.
        if !omissions.iter().any(|omission| omission.node == claim_id) {
            omissions.push(OmittedNode {
                node: claim_id.clone(),
                cause: OmissionCause::NoClauseNode,
            });
        }
        omissions.push(OmittedNode {
            node: id,
            cause: OmissionCause::NamesOmittedNode(claim_id),
        });
        return Ok(());
    }
    if names_an_omitted_node(graph, claim, source, omitted)? {
        return Err(EmitRefusal::SourceMapInvariant);
    }

    let region = region(claim).ok_or(EmitRefusal::SourceMapInvariant)?;
    let region = CheckedSourceRegion {
        source: source_artifact(region.source()),
        start: region.start(),
        end: region.end(),
    };
    if !*types_written {
        push_scalar_type(nodes, source_map, text_type, "string", &region);
        push_scalar_type(nodes, source_map, integer, "integer", &region);
        push_scalar_type(nodes, source_map, boolean, "boolean", &region);
        *types_written = true;
    }

    let result_bound = node_id(ids, source.result_bound().node());
    let operation = operation_key.as_ref().map(|key| node_id(ids, key));
    let model_objects = model_object_keys
        .iter()
        .map(|key| node_id(ids, key))
        .collect::<Vec<_>>();
    let populations = population_facts
        .iter()
        .map(|population| {
            Ok(OperationPopulation {
                member_type: node_id(ids, &population.member_type),
                ordinal: u64::try_from(population.ordinal)
                    .map_err(|_| EmitRefusal::SourceMapInvariant)?,
                maximum: population.maximum,
            })
        })
        .collect::<Result<Vec<_>, EmitRefusal>>()?;

    let operation_term = operation
        .as_ref()
        .map(reference)
        .unwrap_or_else(|| claim_aggregate(Vec::new()));
    let population_terms = populations
        .iter()
        .map(|population| {
            Ok(claim_aggregate(vec![
                claim_binding("member_type", reference(&population.member_type)),
                claim_binding("ordinal", count(integer, population.ordinal)?),
                claim_binding(
                    "maximum",
                    population.maximum.map_or_else(
                        || Ok(claim_aggregate(Vec::new())),
                        |maximum| count(integer, maximum),
                    )?,
                ),
            ]))
        })
        .collect::<Result<Vec<_>, EmitRefusal>>()?;
    let body = claim_aggregate(vec![
        claim_binding("claim", reference(&claim_id)),
        claim_binding(
            "occurrence",
            claim_aggregate(vec![
                claim_binding(
                    "role",
                    claim_literal(
                        text_type,
                        LiteralKind::String,
                        occurrence_role_wire(&checked_occurrence.role).to_owned(),
                    )?,
                ),
                claim_binding("ordinal", count(integer, checked_occurrence.ordinal)?),
            ]),
        ),
        claim_binding("result_bound", reference(&result_bound)),
        claim_binding(
            "invariant",
            CheckedTerm::Literal {
                type_id: boolean.clone(),
                value_kind: LiteralKind::Boolean,
                value: checked_literal_json(&source.requirements().invariant_clause())?,
            },
        ),
        claim_binding("operation", operation_term),
        claim_binding(
            "model_objects",
            claim_aggregate(model_objects.iter().map(reference).collect()),
        ),
        claim_binding("populations", claim_aggregate(population_terms)),
    ]);
    let occurrence = CheckedOccurrence {
        role: CheckedOccurrenceRole::Generated,
        ordinal: *ordinal,
    };
    let node = CheckedSemanticNodeV2 {
        node_id: id.clone(),
        schema_version: GRAPH_V2.into(),
        node_tag: "claim".into(),
        semantic_form: "operation_contract".into(),
        semantic_type: boolean.clone(),
        dependencies: body_reference_targets(&body),
        occurrences: vec![occurrence.clone()],
        recursion_group: None,
        declaration: None,
        owner: None,
        owner_model: None,
        check_mode: None,
        body,
    };
    nodes.push(node);
    source_map.push(CheckedSourceMapEntry {
        node_id: id.clone(),
        role: occurrence.role,
        ordinal: occurrence.ordinal,
        regions: vec![region],
    });
    if records
        .insert(
            claim.clone(),
            OperationContractRecord {
                node: id,
                result_bound,
                invariant: source.requirements().invariant_clause(),
                operation,
                model_objects,
                populations,
            },
        )
        .is_some()
    {
        return Err(EmitRefusal::SourceMapInvariant);
    }
    *ordinal = ordinal
        .checked_add(1)
        .ok_or(EmitRefusal::SourceMapInvariant)?;
    Ok(())
}

/// Whether `claim` is owned by a model refinement or a step relation. Those
/// carry their own capability rows (`refinement`, `temporal-satisfaction`); an
/// `operation-contract` record is written only for a state clause or an
/// operation frame (FSpec FR-825, FR-290).
fn relation_owned(graph: &CheckedGraph, claim: &OccurrenceKey) -> bool {
    graph.refinements().iter().any(|refinement| {
        refinement
            .requirement_occurrence()
            .is_some_and(|key| key.node() == claim.node())
    }) || graph.step_relations().iter().any(|relation| {
        relation
            .occurrence()
            .is_some_and(|key| key.node() == claim.node())
    })
}

pub(super) fn append(
    ids: &StructuralIds,
    graph: &CheckedGraph,
    nodes: &mut Vec<CheckedSemanticNodeV2>,
    source_map: &mut Vec<CheckedSourceMapEntry>,
    regions: &impl Fn(&OccurrenceKey) -> Option<fsl_foundation::source::provenance::SourceRegion>,
    omitted: &Omitted,
    meter: &mut super::OutputMeter,
) -> Result<Appended, EmitRefusal> {
    let mut records = HashMap::new();
    let mut operation_records = HashMap::new();
    let mut omissions = Vec::new();
    let has_value_validity = graph
        .requirements()
        .values()
        .any(|record| record.requirements().kind() == Capability::ValueValidity);
    let has_operation_contract = graph
        .requirements()
        .values()
        .any(|record| record.requirements().kind() == Capability::OperationContract);
    if !has_value_validity && !has_operation_contract {
        return Ok(Appended {
            records,
            operation_records,
            frames: Vec::new(),
            omitted: omissions,
            renumbering: ordinals::Renumbering::default(),
        });
    }
    let renumbering = if has_value_validity {
        ordinals::renumber(ids, graph, nodes, source_map, omitted, meter)?
    } else {
        ordinals::Renumbering::default()
    };
    let frames = if has_value_validity {
        frames::derive(ids, graph, nodes, source_map, omitted, &renumbering, meter)?
    } else {
        frames::Frames::default()
    };
    let integer = node_id(
        ids,
        fsl_semantics::check::builtin_integer_type_key(graph.package_name()).map_err(encoding)?,
    );
    let boolean = node_id(
        ids,
        fsl_semantics::check::builtin_boolean_type_key(graph.package_name()).map_err(encoding)?,
    );
    let mut types_written = false;
    let mut operation_types_written = false;
    let mut operation_ordinal = 0_u64;
    let mut ordinals = BTreeMap::<String, u64>::new();
    let mut owners = HashMap::<String, (NodeKey, CheckedNodeId)>::new();
    let text = node_id(
        ids,
        fsl_semantics::check::builtin_string_type_key(graph.package_name()).map_err(encoding)?,
    );
    // The claim nodes the operation-contract records name that are written,
    // found in one pass over the nodes.
    let mut wanted = BTreeSet::<CheckedNodeId>::new();
    for (claim, source) in graph.requirements().iter() {
        if source.requirements().kind() == Capability::OperationContract
            && !relation_owned(graph, claim)
        {
            wanted.insert(node_id(ids, claim.node()));
        }
    }
    let written: BTreeSet<CheckedNodeId> = nodes
        .iter()
        .filter(|node| wanted.contains(&node.node_id))
        .map(|node| node.node_id.clone())
        .collect();
    for (claim, source) in graph.requirements().iter() {
        if source.requirements().kind() == Capability::OperationContract
            && relation_owned(graph, claim)
        {
            continue;
        }
        if source.requirements().kind() == Capability::OperationContract {
            append_operation_contract(OperationContractAppend {
                ids,
                graph,
                claim,
                source,
                nodes,
                source_map,
                region: regions,
                omitted,
                written: &written,
                omissions: &mut omissions,
                records: &mut operation_records,
                ordinal: &mut operation_ordinal,
                types_written: &mut operation_types_written,
                text_type: &text,
                integer: &integer,
                boolean: &boolean,
            })?;
            continue;
        }
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
                if !omitted.contains_key(&key) && !nodes.iter().any(|node| node.node_id == id) {
                    return Err(EmitRefusal::SourceMapInvariant);
                }
                entry.insert((key, id))
            }
        };
        // A record is tied to its owner's occurrence: it is omitted exactly
        // when its owner is, and then all of that owner's records are, so
        // the `generated` ordinals of every written owner stay dense.
        let owner_omitted = omitted.contains_key(&*owner_key);
        // The use of an omitted owner is dropped, and its omitted record keeps the
        // checker's ordinal in its id; a written record's use is never dropped.
        let settled = |new: Option<u64>, old: u64| -> Result<u64, EmitRefusal> {
            match new {
                Some(new) => Ok(new),
                None if owner_omitted => Ok(old),
                None => Err(EmitRefusal::SourceMapInvariant),
            }
        };
        if !owner_omitted && names_an_omitted_node(graph, claim, source, omitted)? {
            return Err(EmitRefusal::SourceMapInvariant);
        }
        let application = node_id(ids, claim.node());
        let result_bound = node_id(ids, source.result_bound().node());
        let conversion = source
            .conversion_context()
            .map(|key| -> Result<_, EmitRefusal> {
                Ok((
                    node_id(ids, key.node()),
                    settled(
                        renumbering.ordinal(key.node(), key.origin().ordinal()),
                        key.origin().ordinal(),
                    )?,
                ))
            })
            .transpose()?;
        let occurrence_ordinal = settled(
            renumbering.ordinal(claim.node(), claim.origin().ordinal()),
            claim.origin().ordinal(),
        )?;
        let terminal = if owner_omitted {
            None
        } else {
            frames.terminal(claim)
        };
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
            PathSegment::Text(occurrence_ordinal.to_string()),
            PathSegment::Text("bound".to_owned()),
            PathSegment::Node(source.result_bound().node()),
            PathSegment::Text("conversion".to_owned()),
        ];
        if let Some(context) = source.conversion_context() {
            operands.push(PathSegment::Node(context.node().clone()));
            operands.push(PathSegment::Text(
                renumbering
                    .ordinal(context.node(), context.origin().ordinal())
                    .ok_or(EmitRefusal::SourceMapInvariant)?
                    .to_string(),
            ));
        }
        if let Some(terminal) = terminal {
            operands.push(PathSegment::Text("path_condition".to_owned()));
            operands.push(PathSegment::Text(terminal.0.to_string()));
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
        if owner_omitted {
            omissions.push(OmittedNode {
                node: id,
                cause: OmissionCause::NamesOmittedNode(owner.clone()),
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
        let frame_terms = match terminal {
            Some(terminal) => vec![count(&integer, terminal.0 as u64)?],
            None => Vec::new(),
        };
        let body = claim_aggregate(vec![
            claim_binding("owner", reference(owner)),
            claim_binding("application", reference(&application)),
            claim_binding("occurrence", count(&integer, occurrence_ordinal)?),
            claim_binding("result_bound", reference(&result_bound)),
            claim_binding("conversion", conversion_term),
            claim_binding("path_condition", claim_aggregate(frame_terms)),
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
                    occurrence: occurrence_ordinal,
                    result_bound,
                    conversion,
                    path_condition_frame: terminal,
                    extent_roots: roots,
                    objects,
                },
            )
            .is_some()
        {
            return Err(EmitRefusal::SourceMapInvariant);
        }
    }
    Ok(Appended {
        records,
        operation_records,
        frames: frames.rows,
        omitted: omissions,
        renumbering,
    })
}

pub(super) fn rows(
    ids: &StructuralIds,
    graph: &CheckedGraph,
    emitted: &HashMap<NodeKey, CheckedNodeId>,
    appended: &Appended,
    existing: &[RequirementItemRow],
    omitted: &Omitted,
) -> Result<Vec<RequirementItemRow>, EmitRefusal> {
    let Appended {
        records,
        operation_records,
        omitted: unwritten,
        renumbering,
        ..
    } = appended;
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
        if source.requirements().kind() == Capability::OperationContract
            && relation_owned(graph, claim)
        {
            continue;
        }
        // A `value-validity` record has no row when its owner is omitted,
        // and any other record none when its claim node is; the omission
        // closure leaves no other written record naming an omitted node.
        if !omitted.is_empty() {
            let names = record_names_of(graph, claim, source, None)?;
            let dropped = names
                .owner
                .as_ref()
                .map_or(omitted.contains_key(&names.claim), |owner| {
                    omitted.contains_key(owner)
                });
            if dropped {
                continue;
            }
            if names_an_omitted_node(graph, claim, source, omitted)? {
                return Err(EmitRefusal::SourceMapInvariant);
            }
        }
        // An operation-contract record dropped because the emitter writes no node
        // for its claim has no row either; any other missing record is a writer
        // fault, never a silent drop.
        if source.requirements().kind() == Capability::OperationContract
            && !operation_records.contains_key(claim)
        {
            let claim_id = node_id(ids, claim.node());
            if !unwritten.iter().any(|omission| omission.node == claim_id) {
                return Err(EmitRefusal::SourceMapInvariant);
            }
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
        let mut occurrence =
            checked_occurrence(claim.origin()).ok_or(EmitRefusal::RequirementNotEmitted {
                kind: source.requirements().kind().to_wire(),
                shape: "claim-occurrence",
            })?;
        if occurrence.role == CheckedOccurrenceRole::Expression {
            occurrence.ordinal = renumbering
                .ordinal(claim.node(), occurrence.ordinal)
                .ok_or(EmitRefusal::SourceMapInvariant)?;
        }
        if let Some(row) = existing
            .get(&(claim_id.clone(), occurrence.clone()))
            .filter(|row| row.kind == kind)
        {
            rows.push((claim.node().clone(), (**row).clone()));
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
            Some(WireRecord::ValueValidity(ValueValidityRecord {
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
                path_condition_frame: record.path_condition_frame,
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
            }))
        } else if kind == CapabilityKind::OperationContract {
            let record = operation_records
                .get(claim)
                .ok_or(EmitRefusal::SourceMapInvariant)?;
            Some(WireRecord::OperationContract(OperationContractRecord {
                node: remap(&record.node)?,
                result_bound: remap(&record.result_bound)?,
                invariant: record.invariant,
                operation: record.operation.as_ref().map(&mut remap).transpose()?,
                model_objects: record
                    .model_objects
                    .iter()
                    .map(&mut remap)
                    .collect::<Result<_, _>>()?,
                populations: record
                    .populations
                    .iter()
                    .map(|population| {
                        Ok(OperationPopulation {
                            member_type: remap(&population.member_type)?,
                            ..population.clone()
                        })
                    })
                    .collect::<Result<_, EmitRefusal>>()?,
            }))
        } else {
            None
        };
        rows.push((
            claim.node().clone(),
            RequirementItemRow {
                claim: claim_id,
                occurrence,
                kind,
                extent,
                record,
            },
        ));
    }
    Ok(package_order(rows))
}

/// The frame table of the emitted package, each guard in the final id space of
/// the document-held rows (FSpec FR-825).
pub(super) fn frame_rows(
    ids: &StructuralIds,
    emitted: &HashMap<NodeKey, CheckedNodeId>,
    frames: &[PathConditionFrame],
) -> Result<Vec<PathConditionFrame>, EmitRefusal> {
    let mut keys = ids.key_reader();
    frames
        .iter()
        .map(|frame| {
            let guard = emitted
                .get(
                    &keys
                        .key(&frame.guard)
                        .map_err(|_| EmitRefusal::SourceMapInvariant)?,
                )
                .cloned()
                .ok_or(EmitRefusal::SourceMapInvariant)?;
            Ok(PathConditionFrame {
                guard,
                ..frame.clone()
            })
        })
        .collect()
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
/// names no local node (a dependency node) is kept. A graph that does not
/// hold a node of the base's tag and form at each base position is refused.
/// The rows a writer carries, its frame table and the occurrences it renumbered.
pub(super) type CarriedRows = (
    Vec<RequirementItemRow>,
    Vec<PathConditionFrame>,
    BTreeMap<CheckedNodeId, ordinals::Mapping>,
);

/// Move each record, conversion and row naming a renumbered expression
/// occurrence to its new ordinal.
fn relabel_rows(
    rows: &mut [RequirementItemRow],
    renumbered: &BTreeMap<CheckedNodeId, ordinals::Mapping>,
) {
    let moved = |id: &CheckedNodeId, old: u64| -> u64 {
        renumbered
            .get(id)
            .and_then(|mapping| {
                mapping
                    .binary_search_by_key(&old, |(old, _)| *old)
                    .ok()
                    .and_then(|at| mapping[at].1)
            })
            .unwrap_or(old)
    };
    for row in rows {
        if row.occurrence.role == CheckedOccurrenceRole::Expression {
            row.occurrence.ordinal = moved(&row.claim, row.occurrence.ordinal);
        }
        if let Some(WireRecord::ValueValidity(record)) = &mut row.record {
            record.occurrence = moved(&record.application, record.occurrence);
            if let Some((narrow, occurrence)) = &mut record.conversion {
                *occurrence = moved(narrow, *occurrence);
            }
        }
    }
}

pub(super) fn carry_rows(
    base: &filament_contract_model::CheckedPackageV2,
    nodes: &mut [CheckedSemanticNodeV2],
    source_map: &mut [CheckedSourceMapEntry],
    meter: &mut super::OutputMeter,
) -> Result<CarriedRows, EmitRefusal> {
    let rows = base.requirement_items();
    if rows.is_empty() {
        return Ok((Vec::new(), Vec::new(), BTreeMap::new()));
    }
    let base_nodes = &base.resolved_graph().nodes;
    let finals = nodes
        .get(..base_nodes.len())
        .ok_or(EmitRefusal::RequirementRowCarry(
            "the writer's graph holds fewer nodes than the base package",
        ))?;
    if base_nodes
        .iter()
        .zip(finals)
        .any(|(old, new)| old.node_tag != new.node_tag || old.semantic_form != new.semantic_form)
    {
        return Err(EmitRefusal::RequirementRowCarry(
            "a node of the writer's graph is not the base node at its position",
        ));
    }
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
    // The shared frames keep their parent links; only each guard follows a
    // rename. The frame table is no row: it is carried, never written anew.
    let mut frames = base.path_condition_frames().to_vec();
    for frame in &mut frames {
        follow(&mut frame.guard);
    }
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
        match &mut row.record {
            Some(WireRecord::ValueValidity(record)) => {
                follow(&mut record.node);
                follow(&mut record.owner);
                follow(&mut record.application);
                follow(&mut record.result_bound);
                if let Some((narrow, _)) = &mut record.conversion {
                    follow(narrow);
                }
                record.extent_roots.iter_mut().for_each(follow);
                record.objects.iter_mut().for_each(follow);
            }
            Some(WireRecord::OperationContract(record)) => {
                follow(&mut record.node);
                follow(&mut record.result_bound);
                record.operation.iter_mut().for_each(follow);
                record.model_objects.iter_mut().for_each(follow);
                for population in &mut record.populations {
                    follow(&mut population.member_type);
                }
            }
            None => {}
        }
    }
    // A rekey of declaration-less functions reorders the owner walk, and the
    // expression occurrences follow it (FSpec FR-322).
    let renumbered = ordinals::rewalk(
        &base.resolved_graph().nodes,
        base.source_map(),
        nodes,
        source_map,
        meter,
    )?;
    relabel_rows(&mut rows, &renumbered);
    // A rename changes the structural key of every node holding the renamed
    // node as an operand (a call of a rekeyed entry), so the rows are put in
    // package order again by their final claim keys.
    let graph =
        fsl_semantics::check::wire_id::WireGraph::new(nodes.iter().map(|node| &node.node_id))
            .map_err(|_| {
                EmitRefusal::RequirementRowCarry("the writer's graph ids do not resolve")
            })?;
    let keyed = rows
        .into_iter()
        .map(|row| {
            let key = graph.key(&row.claim).map_err(|_| {
                EmitRefusal::RequirementRowCarry("a carried row's claim is not a node of the graph")
            })?;
            Ok((key, row))
        })
        .collect::<Result<Vec<_>, EmitRefusal>>()?;
    let mut rows = package_order(keyed);
    frames = rederived_frames(frames, &mut rows, nodes, source_map, meter)?;
    renumber_records(&rows, nodes, source_map)?;
    Ok((rows, frames, renumbered))
}

/// The frame table re-derived from the final `nodes` by the reader's walk
/// (FSpec FR-825). A rekey changes the function owners' order, and the
/// canonical table order with it, so each record's terminal frame is looked up
/// again and the carried frames move to their ids in the new table.
fn rederived_frames(
    frames: Vec<PathConditionFrame>,
    rows: &mut [RequirementItemRow],
    nodes: &mut [CheckedSemanticNodeV2],
    source_map: &mut [CheckedSourceMapEntry],
    meter: &mut super::OutputMeter,
) -> Result<Vec<PathConditionFrame>, EmitRefusal> {
    let guarded: Vec<&ValueValidityRecord> = rows
        .iter()
        .filter_map(|row| match &row.record {
            Some(WireRecord::ValueValidity(record)) if record.path_condition_frame.is_some() => {
                Some(record)
            }
            _ => None,
        })
        .collect();
    if guarded.is_empty() {
        return Ok(frames);
    }
    let carried: Vec<frames::CarriedRecord<'_>> = guarded
        .iter()
        .map(|record| frames::CarriedRecord {
            owner: &record.owner,
            application: &record.application,
            occurrence: record.occurrence,
        })
        .collect();
    let (table, terminals) = frames::rederive(nodes, source_map, &carried, meter)?;
    let refuse = |reason| EmitRefusal::RequirementRowCarry(reason);
    let mut new_ids = vec![usize::MAX; frames.len()];
    for (record, terminal) in guarded.iter().zip(&terminals) {
        let mut old = record.path_condition_frame;
        let mut new = Some(*terminal);
        while let (Some(at), Some(to)) = (old, new) {
            let slot = new_ids.get_mut(at.0).ok_or(refuse(
                "a carried record cites a frame the table does not hold",
            ))?;
            if *slot != usize::MAX && *slot != to.0 {
                return Err(refuse(
                    "a carried frame has two places in the derived table",
                ));
            }
            *slot = to.0;
            old = frames[at.0].parent;
            new = table.get(to.0).and_then(|frame| frame.parent);
        }
        if old.is_some() != new.is_some() {
            return Err(refuse("a carried frame chain differs from the derived one"));
        }
    }
    if table.len() != frames.len() || new_ids.contains(&usize::MAX) {
        return Err(refuse("the carried frame table is not the derived one"));
    }
    let mut frames = frames;
    if new_ids.iter().enumerate().any(|(old, &new)| old != new) {
        reorder_frames(&new_ids, &mut frames, rows, nodes, source_map)?;
    }
    Ok(table)
}

/// Put the frames at their new ids and re-point every record's terminal frame,
/// its body binding, its identity path and the rows and source-map entries that
/// name the record node. A parent is never moved after its child: a frame
/// shared across owners would break that, so it is refused.
fn reorder_frames(
    new_ids: &[usize],
    frames: &mut Vec<PathConditionFrame>,
    rows: &mut [RequirementItemRow],
    nodes: &mut [CheckedSemanticNodeV2],
    source_map: &mut [CheckedSourceMapEntry],
) -> Result<(), EmitRefusal> {
    let mut moved: Vec<Option<PathConditionFrame>> = vec![None; frames.len()];
    for (old, frame) in std::mem::take(frames).into_iter().enumerate() {
        let parent = match frame.parent {
            Some(parent) => {
                let parent = new_ids[parent.0];
                if parent >= new_ids[old] {
                    return Err(EmitRefusal::RequirementRowCarry(
                        "a renamed owner order places a frame before its parent",
                    ));
                }
                Some(PathConditionFrameId(parent))
            }
            None => None,
        };
        moved[new_ids[old]] = Some(PathConditionFrame { parent, ..frame });
    }
    *frames = moved.into_iter().flatten().collect();
    let mut renamed = BTreeMap::<CheckedNodeId, (CheckedNodeId, u64)>::new();
    for row in rows.iter_mut() {
        let Some(WireRecord::ValueValidity(record)) = &mut row.record else {
            continue;
        };
        let Some(old) = record.path_condition_frame else {
            continue;
        };
        let new = new_ids[old.0];
        record.path_condition_frame = Some(PathConditionFrameId(new));
        let mut id = record.node.clone();
        let mut segments = id.path.iter_mut();
        let cited = segments
            .by_ref()
            .find(|segment| {
                matches!(segment, filament_contract_model::CheckedNodePathSegment::Name(name) if name.as_ref() == "path_condition")
            })
            .and_then(|_| segments.next())
            .ok_or(EmitRefusal::RequirementRowCarry(
                "a guarded record's identity path does not cite its terminal frame",
            ))?;
        *cited = filament_contract_model::CheckedNodePathSegment::Name(new.to_string().into());
        renamed.insert(
            std::mem::replace(&mut record.node, id.clone()),
            (id, new as u64),
        );
    }
    for node in nodes.iter_mut() {
        let Some((id, frame)) = renamed.get(&node.node_id) else {
            continue;
        };
        node.node_id = id.clone();
        let CheckedTerm::Aggregate { members, .. } = &mut node.body else {
            return Err(EmitRefusal::RequirementRowCarry(
                "a carried record body is no aggregate",
            ));
        };
        let Some(CheckedTerm::Binding { value, .. }) = members.get_mut(5) else {
            return Err(EmitRefusal::RequirementRowCarry(
                "a carried record has no path condition",
            ));
        };
        let CheckedTerm::Aggregate { members, .. } = value.as_mut() else {
            return Err(EmitRefusal::RequirementRowCarry(
                "a carried path condition is no aggregate",
            ));
        };
        let Some(CheckedTerm::Literal { value, .. }) = members.first_mut() else {
            return Err(EmitRefusal::RequirementRowCarry(
                "a carried path condition holds no frame",
            ));
        };
        *value = checked_literal_json(frame)?;
    }
    for entry in source_map.iter_mut() {
        if let Some((id, _)) = renamed.get(&entry.node_id) {
            entry.node_id = id.clone();
        }
    }
    Ok(())
}

/// FSpec FR-825: a function's `claim`/`value_validity` nodes carry
/// `generated` ordinals 0, 1, 2, ... in the package order of the rows that
/// name them. Re-sorted rows can reorder one owner's records, so each record
/// node's occurrence and its `generated` source-map entry take the ordinal of
/// its row's place among its owner's rows.
fn renumber_records(
    rows: &[RequirementItemRow],
    nodes: &mut [CheckedSemanticNodeV2],
    source_map: &mut [CheckedSourceMapEntry],
) -> Result<(), EmitRefusal> {
    let mut next = BTreeMap::<&CheckedNodeId, u64>::new();
    let mut ordinals = BTreeMap::<&CheckedNodeId, u64>::new();
    for record in rows.iter().filter_map(|row| match &row.record {
        Some(WireRecord::ValueValidity(record)) => Some(record),
        _ => None,
    }) {
        let ordinal = next.entry(&record.owner).or_default();
        ordinals.insert(&record.node, *ordinal);
        *ordinal += 1;
    }
    for node in nodes.iter_mut() {
        if let Some(&ordinal) = ordinals.get(&node.node_id) {
            let [occurrence] = node.occurrences.as_mut_slice() else {
                return Err(EmitRefusal::RequirementRowCarry(
                    "a carried record node does not have exactly one occurrence",
                ));
            };
            occurrence.ordinal = ordinal;
        }
    }
    for entry in source_map.iter_mut() {
        if entry.role == filament_contract_model::CheckedOccurrenceRole::Generated {
            if let Some(&ordinal) = ordinals.get(&entry.node_id) {
                entry.ordinal = ordinal;
            }
        }
    }
    Ok(())
}

/// FSpec FR-825 package order, the one order every writer puts
/// `requirement_items` in: ascending by the claim node's structural key, then
/// the occurrence role's wire spelling, the occurrence ordinal and the
/// capability's wire spelling, as the reader compares them. `NodeKey` orders
/// as the reader's structural ids do: package, then path segment by segment
/// (text before nested nodes, a nested node compared in full), then ordinal.
pub(super) fn package_order(
    mut rows: Vec<(NodeKey, RequirementItemRow)>,
) -> Vec<RequirementItemRow> {
    rows.sort_by(|(left_key, left), (right_key, right)| {
        left_key
            .cmp(right_key)
            .then_with(|| {
                crate::checked_v2::role_spelling(&left.occurrence.role)
                    .cmp(crate::checked_v2::role_spelling(&right.occurrence.role))
            })
            .then_with(|| left.occurrence.ordinal.cmp(&right.occurrence.ordinal))
            .then_with(|| capability_wire(left.kind).cmp(capability_wire(right.kind)))
    });
    rows.into_iter().map(|(_, row)| row).collect()
}

/// The FSpec FR-825 wire spelling of a requested capability.
fn capability_wire(kind: CapabilityKind) -> &'static str {
    match kind {
        CapabilityKind::ValueValidity => "value-validity",
        CapabilityKind::OperationContract => "operation-contract",
        CapabilityKind::FiniteReplay => "finite-replay",
        CapabilityKind::TemporalSatisfaction => "temporal-satisfaction",
        CapabilityKind::GlobalConformance => "global-conformance",
        CapabilityKind::Monitorability => "monitorability",
        CapabilityKind::LocalProjection => "local-projection",
        CapabilityKind::Refinement => "refinement",
        CapabilityKind::Realizability => "realizability",
        CapabilityKind::Composition => "composition",
        CapabilityKind::ProbabilisticSatisfaction => "probabilistic-satisfaction",
        CapabilityKind::Schedulability => "schedulability",
    }
}

/// The value-validity record a row carries, for a test reading a package.
#[cfg(test)]
pub(super) fn value_record(row: &RequirementItemRow) -> Option<&ValueValidityRecord> {
    match &row.record {
        Some(WireRecord::ValueValidity(record)) => Some(record),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
