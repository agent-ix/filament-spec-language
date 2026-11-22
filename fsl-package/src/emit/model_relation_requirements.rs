// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940: retain each HP-1 model requirement in its admitted package.

use std::collections::HashMap;

use super::{checked_occurrence, EmitRefusal};
use filament_contract_model::CheckedNodeId;
use filament_contract_model::{
    CapabilityKind, DomainKey, DomainKind, RequirementExtent, RequirementItemRow,
};
use filament_exact::NodeKey;
use fsl_semantics::check::{Capability, CheckedGraph};
use fsl_semantics::family::ClaimExtent;

pub(super) fn rows(
    graph: &CheckedGraph,
    emitted: &HashMap<NodeKey, CheckedNodeId>,
) -> Result<Vec<RequirementItemRow>, EmitRefusal> {
    let mut rows = Vec::new();
    for relation in graph.step_relations() {
        let occurrence = relation
            .occurrence()
            .ok_or(EmitRefusal::SourceMapInvariant)?;
        let record = graph
            .requirements()
            .get_kind(occurrence, Capability::TemporalSatisfaction)
            .ok_or(EmitRefusal::SourceMapInvariant)?;
        let claim = emitted
            .get(occurrence.node())
            .ok_or(EmitRefusal::SourceMapInvariant)?
            .clone();
        // The model obligation is precisely the relation's infinite-trace
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
    // Relation declaration IDs contain no operands; their structural order
    // is also their document-held order (FR-825).
    rows.sort_by(|left, right| {
        left.claim
            .cmp(&right.claim)
            .then(left.occurrence.ordinal.cmp(&right.occurrence.ordinal))
    });
    Ok(rows)
}
