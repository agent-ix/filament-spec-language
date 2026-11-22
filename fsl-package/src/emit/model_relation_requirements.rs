// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940: retain original model-relation and refinement requirement records.

use std::collections::HashMap;

use super::{checked_occurrence, EmitRefusal};
use filament_contract_model::CheckedNodeId;
use filament_contract_model::{
    CapabilityKind, CheckedOccurrenceRole, DomainKey, DomainKind, RequirementExtent,
    RequirementItemRow,
};
use filament_exact::NodeKey;
use fsl_semantics::check::{Capability, CheckedGraph};
use fsl_semantics::family::ClaimExtent;

pub(super) fn rows(
    graph: &CheckedGraph,
    emitted: &HashMap<NodeKey, CheckedNodeId>,
) -> Result<Vec<RequirementItemRow>, EmitRefusal> {
    let mut rows = Vec::new();
    let occurrences = graph
        .step_relations()
        .iter()
        .map(|relation| relation.occurrence())
        .chain(
            graph
                .refinements()
                .iter()
                .map(|refinement| refinement.requirement_occurrence()),
        );
    for occurrence in occurrences {
        let occurrence = occurrence.ok_or(EmitRefusal::SourceMapInvariant)?;
        let record = graph
            .requirements()
            .get_kind(occurrence, Capability::TemporalSatisfaction)
            .ok_or(EmitRefusal::SourceMapInvariant)?;
        let claim = emitted
            .get(occurrence.node())
            .ok_or(EmitRefusal::SourceMapInvariant)?
            .clone();
        // The model obligation is precisely the declaration's infinite-trace
        // domain. Check the source record rather than inventing an extent.
        let ClaimExtent::Unbounded(domains) = record.requirements().extent() else {
            return Err(EmitRefusal::SourceMapInvariant);
        };
        if domains.len() != 1
            || !domains.iter().any(|(domain, kind)| {
                matches!(domain, fsl_foundation::bound::DomainKey::Node { node, path }
                if node == occurrence.node() && path.is_empty())
                    && kind == fsl_foundation::bound::DomainKind::InfiniteTrace
            })
        {
            return Err(EmitRefusal::SourceMapInvariant);
        }
        rows.push(RequirementItemRow {
            claim: claim.clone(),
            occurrence: checked_occurrence(occurrence.origin())
                .ok_or(EmitRefusal::SourceMapInvariant)?,
            kind: CapabilityKind::TemporalSatisfaction,
            extent: RequirementExtent::Unbounded(vec![(
                DomainKey::Node {
                    node: claim,
                    path: Vec::new(),
                },
                DomainKind::InfiniteTrace,
            )]),
            record: None,
        });
    }
    // These declaration IDs contain no operands; their structural order
    // is also their document-held order (FR-825).
    rows.sort_by(|left, right| {
        left.claim
            .cmp(&right.claim)
            .then(role_wire(&left.occurrence.role).cmp(role_wire(&right.occurrence.role)))
            .then(left.occurrence.ordinal.cmp(&right.occurrence.ordinal))
    });
    Ok(rows)
}

fn role_wire(role: &CheckedOccurrenceRole) -> &'static str {
    match role {
        CheckedOccurrenceRole::Declaration => "declaration",
        CheckedOccurrenceRole::Type => "type",
        CheckedOccurrenceRole::Expression => "expression",
        CheckedOccurrenceRole::Anchor => "anchor",
        CheckedOccurrenceRole::Claim => "claim",
        CheckedOccurrenceRole::Generated => "generated",
    }
}
