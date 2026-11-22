// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940, FR-120: the bounded dense-time model reader over admitted nodes.

use std::collections::BTreeSet;

use filament_contract_model::{CheckedNodeId, CheckedNodeTag, CheckedOccurrenceRole};
use fsl_semantics::check::{ModelTimeSource, TimeSourceKind, TimedModel};
use fsl_semantics::model::state::StateModel;

use crate::{BarePackage, CheckedTimeSourceKind, CheckedTimedModel, ClockPathStep};

impl BarePackage<'_> {
    /// Read every timed node into dense delay tables. Only a source-owned
    /// base Time dimension, direct non-affine unit and required direct clock
    /// fields are supported. The error identifies an unread timed node; no
    /// partially reconstructed tables are returned.
    pub fn direct_dense_tables(
        &self,
        state: &StateModel,
    ) -> Result<(Vec<TimedModel>, Vec<ModelTimeSource>), CheckedNodeId> {
        let timed = self
            .package()
            .resolved_graph()
            .nodes
            .iter()
            .filter(|node| {
                CheckedNodeTag::from_wire(&node.node_tag) == Some(CheckedNodeTag::TimedModel)
            })
            .collect::<Vec<_>>();
        let Some(first) = timed.first() else {
            return Ok((Vec::new(), Vec::new()));
        };
        let failure = || first.node_id.clone();
        let roots = CheckedTimedModel::from_admitted(self.package()).map_err(|_| failure())?;
        let declarations = self.clock_declarations().map_err(|_| failure())?;
        let mut consumed = BTreeSet::new();
        let mut selections = BTreeSet::new();
        let mut models = Vec::new();
        let mut sources = Vec::new();
        for root in roots {
            let bad = || root.metadata.node_id.clone();
            if root.source.kind != CheckedTimeSourceKind::Dense
                || !root.guards.is_empty()
                || !root.resets.is_empty()
                || !root.invariants.is_empty()
                || !root.urgencies.is_empty()
                || !root.constraints.is_empty()
            {
                return Err(bad());
            }
            let model_node = self.node_key(&root.model).ok_or_else(bad)?;
            let model = state.direct_dense_model(&model_node).ok_or_else(bad)?;
            if !selections.insert(model.model().to_owned()) {
                return Err(bad());
            }
            let unit_node = self.node_key(&root.source.unit).ok_or_else(bad)?;
            let time_dimension = self.node_key(&root.time_dimension).ok_or_else(bad)?;
            let (unit, nominal_nodes) = self
                .units
                .direct_time_nodes(self.package(), &unit_node, &time_dimension)
                .ok_or_else(bad)?;
            let mut regions = self
                .package()
                .source_map()
                .iter()
                .filter(|entry| {
                    entry.node_id == root.source.metadata.node_id
                        && entry.role == CheckedOccurrenceRole::Expression
                })
                .flat_map(|entry| &entry.regions);
            let region = regions.next().ok_or_else(bad)?;
            if regions.next().is_some() {
                return Err(bad());
            }
            let region = crate::checked_v2::source_region(region).map_err(|_| bad())?;
            let source = ModelTimeSource {
                model: model.model().to_owned(),
                model_node,
                span: fsl_foundation::Span {
                    start: usize::try_from(region.start()).map_err(|_| bad())?,
                    end: usize::try_from(region.end()).map_err(|_| bad())?,
                },
                declared_in: Some(region.source().clone()),
                tick_operation_key: None,
                tick_operation_span: None,
                tick_operation_anchor: None,
                source: TimeSourceKind::Dense,
                unit,
                unit_node,
                time_dimension,
                nominal_nodes,
            };
            let mut places = Vec::new();
            let mut joined = BTreeSet::new();
            for clock in model.clocks() {
                let template = declarations
                    .source()
                    .clock_template(clock)
                    .map_err(|_| bad())?;
                let origin = declarations
                    .source()
                    .clock_origin(clock)
                    .map_err(|_| bad())?;
                let mut matching = Vec::new();
                for place in &root.clock_places {
                    if place.model != root.model
                        || !matches!(place.path.as_slice(), [ClockPathStep::Field(_)])
                    {
                        return Err(place.metadata.node_id.clone());
                    }
                    if declarations.clock_template(place).map_err(|_| bad())? == template
                        && declarations.clock_origin(place).map_err(|_| bad())? == origin
                    {
                        matching.push(place);
                    }
                }
                let [place] = matching.as_slice() else {
                    return Err(bad());
                };
                if !joined.insert(place.metadata.node_id.clone()) {
                    return Err(bad());
                }
                places.push(self.node_key(&place.metadata.node_id).ok_or_else(bad)?);
            }
            if joined.len() != root.clock_places.len() {
                return Err(bad());
            }
            for id in std::iter::once(root.metadata.node_id.clone())
                .chain(std::iter::once(root.source.metadata.node_id.clone()))
                .chain(joined)
            {
                if !consumed.insert(id) {
                    return Err(bad());
                }
            }
            models.push(model.with_package_places(places).ok_or_else(bad)?);
            sources.push(source);
        }
        if let Some(node) = timed.iter().find(|node| !consumed.contains(&node.node_id)) {
            return Err(node.node_id.clone());
        }
        Ok((models, sources))
    }
}
